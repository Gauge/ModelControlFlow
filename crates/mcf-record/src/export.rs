use std::path::Path;

use mcf_core::build_identity::BuildIdentity;
use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::Timestamp;

use crate::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-record::export");

pub const FORMAT_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    Export,
    Contribution,
    ReproBundle,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Contribution => "contribution",
            Self::ReproBundle => "repro_bundle",
        }
    }

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Export, Self::Contribution, Self::ReproBundle]
            .into_iter()
            .find(|kind| kind.as_str() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub entries: usize,
    pub digest: String,
    pub content_entries: usize,
}

pub fn write(journal: &Path, to: &Path, kind: Kind) -> Result<Manifest> {
    write_selected(journal, to, kind, |_| true)
}

pub fn write_selected(
    journal: &Path,
    to: &Path,
    kind: Kind,
    keep: impl Fn(&Value) -> bool,
) -> Result<Manifest> {
    let replayed = crate::journal::replay(journal)?;
    let text = std::fs::read_to_string(journal).map_err(|error| unreadable(journal, &error))?;

    let lines: Vec<&str> = text
        .lines()
        .skip(1)
        .take(replayed.entries.len())
        .zip(replayed.entries.iter())
        .filter(|(_, entry)| keep(&entry.to_value()))
        .map(|(line, _)| line)
        .collect();

    let mut hasher = Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    let carrying = replayed
        .entries
        .iter()
        .zip(text.lines().skip(1))
        .filter(|(_, line)| lines.contains(line))
        .filter(|(entry, _)| holds_written_text(&entry.to_value()))
        .count();
    let manifest = Manifest {
        entries: lines.len(),
        digest: hasher.finish().hex(),
        content_entries: carrying,
    };

    let mut out = String::new();
    out.push_str(&header(kind, &manifest, replayed.loss.is_some(), carrying > 0).to_line());
    out.push('\n');
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }

    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|error| unwritable(to, &error))?;
    }
    std::fs::write(to, out).map_err(|error| unwritable(to, &error))?;
    Ok(manifest)
}

pub fn read(from: &Path) -> Result<(Kind, Manifest, Vec<Value>)> {
    let text = std::fs::read_to_string(from).map_err(|error| unreadable(from, &error))?;
    let mut lines = text.lines();

    let Some(header) = lines.next() else {
        return Err(unreadable_bundle(from, "the bundle is empty"));
    };
    let header = json::parse(header).map_err(|error| {
        unreadable_bundle(from, &format!("its header is not readable: {error}"))
    })?;

    match header.get("format").and_then(Value::as_integer) {
        Some(FORMAT_VERSION) => {}
        Some(other) => {
            return Err(unreadable_bundle(
                from,
                &format!(
                    "it was written by bundle format {other}, and this build reads {FORMAT_VERSION}"
                ),
            ));
        }
        None => {
            return Err(unreadable_bundle(
                from,
                "its header names no format version",
            ));
        }
    }

    let kind = header
        .get("kind")
        .and_then(Value::as_text)
        .and_then(Kind::parse)
        .ok_or_else(|| unreadable_bundle(from, "its header names no kind this build knows"))?;

    let stated = header
        .get("manifest")
        .and_then(|manifest| {
            Some(Manifest {
                entries: usize::try_from(manifest.get("entries")?.as_integer()?).ok()?,
                digest: manifest.get("digest")?.as_text()?.to_owned(),
                content_entries: 0,
            })
        })
        .ok_or_else(|| unreadable_bundle(from, "its header carries no manifest"))?;

    let mut hasher = Sha256::new();
    let mut entries = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
        entries.push(json::parse(line).map_err(|error| {
            damaged(from, &format!("an entry is not readable: {error}"), &stated)
        })?);
    }

    let found = hasher.finish().hex();
    if found != stated.digest || entries.len() != stated.entries {
        return Err(damaged(
            from,
            &format!(
                "it holds {} entries digesting to {found}, and states {} digesting to {}",
                entries.len(),
                stated.entries,
                stated.digest
            ),
            &stated,
        ));
    }

    Ok((kind, stated, entries))
}

fn header(kind: Kind, manifest: &Manifest, partial: bool, carries_written_text: bool) -> Value {
    Value::map([
        ("format", Value::Integer(FORMAT_VERSION)),
        ("kind", Value::text(kind.as_str())),
        (
            "created_by",
            crate::encode::build_identity(BuildIdentity::current()),
        ),
        ("created_at", Value::text(Timestamp::now().to_string())),
        (
            "manifest",
            Value::map([
                (
                    "entries",
                    Value::Integer(i64::try_from(manifest.entries).unwrap_or(i64::MAX)),
                ),
                ("digest", Value::text(manifest.digest.clone())),
            ]),
        ),
        ("source_was_complete", Value::Bool(!partial)),
        ("contains_user_content", Value::Bool(carries_written_text)),
    ])
}

fn holds_written_text(entry: &Value) -> bool {
    const WRITTEN: [&[&str]; 3] = [
        &["body", "method", "prompt"],
        &["body", "prompt"],
        &["body", "text"],
    ];
    WRITTEN.iter().any(|path| {
        let mut held = entry;
        for step in *path {
            let Some(next) = held.get(step) else {
                return false;
            };
            held = next;
        }
        held.as_text().is_some_and(|text| !text.is_empty())
    })
}

fn unreadable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::ArtifactUnreadable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the file could not be read",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the bundle could not be written",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

fn unreadable_bundle(path: &Path, why: &str) -> Failure {
    Failure::new(
        Category::ExchangeSchemaUnreadable,
        Attribution::Mcf,
        Disposition::Refused,
        WHERE,
        "the bundle cannot be interpreted by this build",
    )
    .with_context("path", path.display().to_string())
    .with_context("detail", why.to_owned())
}

fn damaged(path: &Path, why: &str, stated: &Manifest) -> Failure {
    Failure::new(
        Category::ArtifactCorrupt,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the bundle does not match the manifest it carries",
    )
    .with_context("path", path.display().to_string())
    .with_context("detail", why.to_owned())
    .with_context("stated_entries", stated.entries.to_string())
    .with_context("stated_digest", stated.digest.clone())
}

#[cfg(test)]
mod tests;
