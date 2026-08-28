//! One portable file, and the one mechanism that makes all of them.
//!
//! D20: *nothing leaves the machine automatically; export is an explicit act
//! producing one portable file.* B-302 adds the condition that matters —
//! **one mechanism serves export, contribution and repro bundles** — because
//! three serializations of the same evidence would eventually disagree about
//! what the evidence was.
//!
//! **What a bundle is.** The journal's own lines, unchanged, between a header
//! that says what this is and a manifest that says what it should contain. The
//! entries are copied verbatim rather than re-encoded: a bundle that
//! re-serialized its contents would be a bundle whose digest depended on the
//! version that wrote it, and §XV's whole promise is that a configuration found
//! elsewhere can be reproduced here.
//!
//! **Three kinds, one shape** ([`Kind`]). They differ in *what is selected*,
//! never in how it is written — which is what makes them one mechanism. §XIV's
//! contribution and PR2's repro bundle are selections over the same entries.
//!
//! **What is structurally absent.** User content. Not filtered out — never
//! present: this module reads the journal and the journal is not the content
//! store (A25, B-161). There is no code path here that could reach content,
//! which is the guarantee §6.8 asks for rather than the one a filter provides.
//!
//! **Exporting is not publishing.** A24 gates *publication* — the irreversible,
//! itemized act of sending something off this machine — and writing a file to a
//! path the operator named is not that. So export is not a gated category
//! (A16) and flows like any other default (B1). The gate belongs to whatever
//! *sends* a bundle, and it is B-160's.

use std::path::Path;

use mcf_core::build_identity::BuildIdentity;
use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::Timestamp;

use crate::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-record::export");

/// The bundle format's version.
///
/// Separate from the journal's, because they are separate public interfaces
/// with separate readers: §XIV's is another machine, and §7.30 makes a schema
/// an interface the moment it is shared.
pub const FORMAT_VERSION: i64 = 1;

/// What a bundle is for.
///
/// The kinds differ in what is *selected*, never in how it is written. A reader
/// that understands one understands all three, which is the point of there
/// being one mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// Everything in the record, for the operator's own use.
    Export,
    /// The rows §XIV permits to leave, chosen per share (A24).
    ///
    /// Not implemented here: B-160 builds the selection and the confirmation
    /// that shows the rows leaving. What is settled is that when it does, it
    /// writes this format.
    Contribution,
    /// One claim and everything needed to re-run it (PR2).
    ReproBundle,
}

impl Kind {
    /// The name that appears in a bundle's header.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Contribution => "contribution",
            Self::ReproBundle => "repro_bundle",
        }
    }

    /// The kind a written name refers to, if this build knows it.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Export, Self::Contribution, Self::ReproBundle]
            .into_iter()
            .find(|kind| kind.as_str() == name)
    }
}

/// What a bundle turned out to contain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// How many entries.
    pub entries: usize,
    /// The digest of those entries, exactly as they are written.
    pub digest: String,
}

/// Writes a bundle from a journal.
///
/// The journal is read rather than the in-memory state of a running MCF,
/// because D20 makes the journal *the record*: exporting anything else would be
/// exporting a second opinion about what happened.
///
/// # Errors
///
/// Whatever reading the journal or writing the bundle fails with. A journal
/// with a loss in it is exported **up to the loss**, and the manifest says how
/// many entries that was — A4 keeps a partial outcome, and B62 requires the
/// extent be stated rather than the history quietly shortened.
pub fn write(journal: &Path, to: &Path, kind: Kind) -> Result<Manifest> {
    write_selected(journal, to, kind, |_| true)
}

/// The same, carrying only the entries a selector keeps.
///
/// **The kinds differ in what is selected, never in how it is written**, and
/// this is where that stops being a sentence in the module header. A repro
/// bundle is one claim and what it rests on (PR2); a contribution is the rows
/// §XIV permits to leave; an export is everything. All three are this function
/// with a different predicate, so a reader that understands one understands
/// all three.
///
/// The selector sees each entry as the record holds it, and the lines are still
/// copied **verbatim**: re-encoding a kept line would make a bundle's digest
/// depend on the version that wrote it.
///
/// # Errors
///
/// As [`write()`].
pub fn write_selected(
    journal: &Path,
    to: &Path,
    kind: Kind,
    keep: impl Fn(&Value) -> bool,
) -> Result<Manifest> {
    let replayed = crate::journal::replay(journal)?;
    let text = std::fs::read_to_string(journal).map_err(|error| unreadable(journal, &error))?;

    // The entry lines, verbatim. The header is line one and is not an entry;
    // anything after a loss is not carried, because it could not be read.
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
    let manifest = Manifest {
        entries: lines.len(),
        digest: hasher.finish().hex(),
    };

    let mut out = String::new();
    // Whether anything carried holds text the operator wrote. Asked of the
    // entries rather than assumed from the kind: a bundle's honesty about this
    // has to survive somebody adding a field to an entry.
    let written = replayed
        .entries
        .iter()
        .zip(text.lines().skip(1))
        .filter(|(_, line)| lines.contains(line))
        .any(|(entry, _)| holds_written_text(&entry.to_value()));
    out.push_str(&header(kind, &manifest, replayed.loss.is_some(), written).to_line());
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

/// Reads a bundle back, checking it against its own manifest.
///
/// # Errors
///
/// `exchange.schema.unreadable` when the header is absent or names a format
/// this build does not read; `artifact.corrupt` when the entries do not match
/// the digest the manifest states. The second is the one worth having: a
/// bundle that arrived damaged is not a bundle with fewer rows, and reading it
/// as one would be the silent shortening B62 forbids, arriving by post.
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
        // B62's extent, carried outward. A bundle from a journal that could not
        // be fully replayed says so, rather than being a shorter history nobody
        // can tell from a complete one.
        ("source_was_complete", Value::Bool(!partial)),
        // A25, stated as well as structural — and **not a constant**. It was
        // one, and the claim stopped being true the day a comparison began
        // recording the prompt both arms were asked as part of its method
        // (PR2, B-211): the journal is still not the content store, and the
        // *method* of a measurement is text the operator wrote.
        //
        // The header says what the bundle holds rather than what this module
        // hopes it holds. A reader deciding whether to send a file is entitled
        // to the first (A24, §3.20), and a `false` that is sometimes wrong is
        // worse than no field at all.
        ("contains_user_content", Value::Bool(carries_written_text)),
    ])
}

/// Whether an entry holds text the operator wrote.
///
/// The fields are named rather than guessed at, because guessing is how this
/// gets it wrong in the direction that matters: an unrecognized field read as
/// *not user content* is a bundle telling somebody it is safe to send.
///
/// A field added later that holds written text and is not named here is a
/// defect, and `checks/tests/a_bundle_says_what_it_holds.rs` is what catches
/// it — by requiring that every place a surface writes operator text into the
/// record be one of these.
fn holds_written_text(entry: &Value) -> bool {
    /// Where operator text reaches the record today.
    const WRITTEN: [&[&str]; 2] = [
        // A comparison's method: what both arms were asked (PR2, B-211).
        &["body", "method", "prompt"],
        // A generation: what was asked, where a surface recorded it.
        &["body", "prompt"],
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
