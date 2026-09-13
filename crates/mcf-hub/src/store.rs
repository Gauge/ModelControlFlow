use std::path::{Path, PathBuf};

use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::provenance::Provenance;
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind, Journal};
use mcf_record::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-hub::store");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub path: PathBuf,
    pub bytes: u64,
    pub parts: u32,
    pub companion: bool,
    pub provenance: std::result::Result<Provenance, Option<Failure>>,
}

impl Held {
    #[must_use]
    pub fn describe(&self) -> String {
        let origin = match &self.provenance {
            Ok(provenance) => provenance.origin().to_string(),
            Err(None) => "origin unknown — nothing beside it says where it came from".to_owned(),
            Err(Some(failure)) => format!("origin unreadable — {failure}"),
        };
        format!("{} ({} bytes) — {origin}", self.path.display(), self.bytes)
    }

    #[must_use]
    pub fn terms(&self) -> String {
        match &self.provenance {
            Ok(provenance) => crate::licence::describe(provenance.licence().known()),
            Err(None) => "licence: unknown — nothing beside it states any terms".to_owned(),
            Err(Some(_)) => {
                "licence: unknown — the provenance beside it could not be read".to_owned()
            }
        }
    }
}

#[must_use]
pub fn provenance_path(artifact: &Path) -> PathBuf {
    let mut name = artifact.file_name().unwrap_or_default().to_os_string();
    name.push(PROVENANCE_SUFFIX);
    artifact.with_file_name(name)
}

const PROVENANCE_SUFFIX: &str = ".mcf-provenance.json";

pub fn record_provenance(artifact: &Path, provenance: &Provenance) -> Result<PathBuf> {
    let path = provenance_path(artifact);
    let line = mcf_record::encode::provenance(provenance).to_line();
    std::fs::write(&path, line + "\n").map_err(|error| {
        Failure::new(
            Category::ResourceDiskReadonly,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "an artifact's provenance could not be written beside it",
        )
        .with_context("path", path.display().to_string())
        .with_context("reason", error.to_string())
    })?;
    Ok(path)
}

pub fn provenance_of(artifact: &Path) -> Result<Provenance> {
    let path = provenance_path(artifact);
    let text = std::fs::read_to_string(&path).map_err(|error| {
        Failure::new(
            if error.kind() == std::io::ErrorKind::NotFound {
                Category::ArtifactMissing
            } else {
                Category::ArtifactUnreadable
            },
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "nothing beside this artifact says where it came from",
        )
        .with_context("path", path.display().to_string())
        .with_context("reason", error.to_string())
    })?;
    let value = mcf_record::json::parse(&text).map_err(|error| {
        Failure::new(
            Category::ArtifactProvenanceIncomplete,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "an artifact's provenance is not readable",
        )
        .with_context("path", path.display().to_string())
        .with_context("reason", error.to_string())
    })?;
    mcf_record::decode::provenance(&value)
        .map_err(|failure| failure.with_context("path", path.display().to_string()))
}

pub fn held(root: &Path) -> Result<Vec<Held>> {
    let mut found = Vec::new();
    walk(root, &mut found)?;
    found.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(gathered(found))
}

#[must_use]
pub fn is_a_model_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if name.ends_with(PROVENANCE_SUFFIX) {
        return false;
    }
    if name.starts_with('.') {
        return false;
    }
    path.extension()
        .and_then(|held| held.to_str())
        .is_some_and(|held| held.eq_ignore_ascii_case("gguf"))
}

#[must_use]
pub fn is_a_companion(path: &Path) -> bool {
    path.file_stem()
        .and_then(|held| held.to_str())
        .is_some_and(|held| held.to_ascii_lowercase().starts_with("mmproj"))
}

#[must_use]
pub fn part_of_a_set(path: &Path) -> Option<(String, u32)> {
    let stem = path.file_stem()?.to_str()?;
    let (before, after) = stem.rsplit_once("-of-")?;
    if !after.chars().all(|c| c.is_ascii_digit()) || after.is_empty() {
        return None;
    }
    let (prefix, number) = before.rsplit_once('-')?;
    if !number.chars().all(|c| c.is_ascii_digit()) || number.is_empty() {
        return None;
    }
    let at: u32 = number.parse().ok()?;
    (at > 0).then(|| (prefix.to_owned(), at))
}

pub fn bytes_of_the_whole(path: &Path) -> Result<u64> {
    let own = std::fs::metadata(path)
        .map_err(|error| {
            Failure::new(
                Category::ArtifactMissing,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "this model could not be measured",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string())
        })?
        .len();
    let Some((prefix, _)) = part_of_a_set(path) else {
        return Ok(own);
    };
    let Some(directory) = path.parent() else {
        return Ok(own);
    };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(own);
    };
    let mut total = 0_u64;
    for entry in entries.flatten() {
        let beside = entry.path();
        if part_of_a_set(&beside).is_some_and(|(held, _)| held == prefix)
            && let Ok(about) = entry.metadata()
        {
            total = total.saturating_add(about.len());
        }
    }
    Ok(if total == 0 { own } else { total })
}

fn gathered(found: Vec<Held>) -> Vec<Held> {
    let mut out: Vec<Held> = Vec::with_capacity(found.len());
    let mut totals: std::collections::BTreeMap<(PathBuf, String), (u64, u32)> =
        std::collections::BTreeMap::new();
    for held in &found {
        if let Some((prefix, _)) = part_of_a_set(&held.path) {
            let directory = held
                .path
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf);
            let entry = totals.entry((directory, prefix)).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(held.bytes);
            entry.1 = entry.1.saturating_add(1);
        }
    }
    for held in found {
        let Some((prefix, at)) = part_of_a_set(&held.path) else {
            out.push(held);
            continue;
        };
        if at != 1 {
            continue;
        }
        let directory = held
            .path
            .parent()
            .map_or_else(PathBuf::new, Path::to_path_buf);
        let (bytes, parts) = totals
            .get(&(directory, prefix))
            .copied()
            .unwrap_or((held.bytes, 1));
        out.push(Held {
            bytes,
            parts,
            ..held
        });
    }
    out
}

fn walk(directory: &Path, into: &mut Vec<Held>) -> Result<()> {
    let entries = std::fs::read_dir(directory).map_err(|error| {
        Failure::new(
            Category::ArtifactUnreadable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "a directory MCF holds artifacts in cannot be read",
        )
        .with_context("path", directory.display().to_string())
        .with_context("reason", error.to_string())
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into)?;
            continue;
        }
        if !is_a_model_file(&path) {
            continue;
        }
        let bytes = std::fs::metadata(&path).map(|metadata| metadata.len());
        into.push(Held {
            bytes: bytes.unwrap_or(0),
            parts: 1,
            companion: is_a_companion(&path),
            provenance: match provenance_of(&path) {
                Ok(provenance) => Ok(provenance),
                Err(failure) if failure.category() == Category::ArtifactMissing => Err(None),
                Err(failure) => Err(Some(failure)),
            },
            path,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doomed {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    doomed: Vec<Doomed>,
    shelf: PathBuf,
    reversible: bool,
    identity: String,
}

impl Plan {
    #[must_use]
    pub fn doomed(&self) -> &[Doomed] {
        &self.doomed
    }

    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        self.doomed
            .iter()
            .try_fold(0_u64, |total, doomed| total.checked_add(doomed.bytes))
    }

    #[must_use]
    pub const fn reversible(&self) -> bool {
        self.reversible
    }

    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    #[must_use]
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        for doomed in &self.doomed {
            lines.push(format!(
                "  {} ({} bytes)",
                doomed.path.display(),
                doomed.bytes
            ));
        }
        let total = match self.bytes() {
            Some(bytes) => format!("{bytes} bytes"),
            None => "an unstatable total — the sizes do not add up".to_owned(),
        };
        let reversal = if self.reversible {
            format!(
                "recoverable from {} until it is purged",
                self.shelf.display()
            )
        } else {
            format!(
                "NOT recoverable: {} is on another filesystem, so this cannot be undone",
                self.shelf.display()
            )
        };
        format!(
            "removing {} file(s), {total}, {reversal}:\n{}",
            self.doomed.len(),
            lines.join("\n")
        )
    }
}

pub fn preview(paths: &[PathBuf], shelf: &Path) -> Result<Plan> {
    let mut doomed: Vec<Doomed> = Vec::new();
    let mut wanted: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !wanted.contains(path) {
            wanted.push(path.clone());
        }
        let sidecar = provenance_path(path);
        if sidecar.exists() && !wanted.contains(&sidecar) {
            wanted.push(sidecar);
        }
    }
    for path in &wanted {
        let metadata = std::fs::metadata(path).map_err(|error| {
            Failure::new(
                if error.kind() == std::io::ErrorKind::NotFound {
                    Category::ArtifactMissing
                } else {
                    Category::ArtifactUnreadable
                },
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "a plan cannot describe what it cannot look at",
            )
            .with_context("path", path.display().to_string())
            .with_context("reason", error.to_string())
        })?;
        doomed.push(Doomed {
            path: path.clone(),
            bytes: metadata.len(),
        });
    }

    let reversible = same_filesystem(&wanted, shelf);
    let identity = identify(&doomed);
    Ok(Plan {
        doomed,
        shelf: shelf.to_path_buf(),
        reversible,
        identity,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    doomed: Vec<Doomed>,
    reason: String,
}

impl Authorization {
    pub fn given(plan: &Plan, reason: &str) -> Result<Self> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(Failure::new(
                Category::ConfigInvalid,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "a removal is authorized with a reason, and none was given",
            )
            .with_context("plan", plan.identity().to_owned()));
        }
        Ok(Self {
            doomed: plan.doomed.clone(),
            reason: reason.to_owned(),
        })
    }

    #[must_use]
    pub fn doomed(&self) -> &[Doomed] {
        &self.doomed
    }

    pub fn covers(&self, plan: &Plan) -> Result<()> {
        let differences = differences_between(&self.doomed, &plan.doomed);
        if differences.is_empty() {
            return Ok(());
        }
        Err(Failure::new(
            Category::ConfigInvalid,
            Attribution::User,
            Disposition::Refused,
            WHERE,
            "this authorization is for a different removal than the plan in front of it",
        )
        .with_context("differences", differences.join("; "))
        .with_context(
            "what_to_do",
            "look again: what would be removed is not what was authorized",
        ))
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    pub shelved: Vec<PathBuf>,
    pub refused: Vec<(PathBuf, String)>,
    pub bytes: u64,
    pub reversible: bool,
}

impl Removed {
    #[must_use]
    pub fn complete(&self) -> bool {
        self.refused.is_empty()
    }
}

pub fn remove(
    plan: &Plan,
    authorization: &Authorization,
    journal: &mut Journal,
    at: Timestamp,
) -> Result<Removed> {
    authorization.covers(plan)?;

    journal.append(&Entry::new(
        EntryKind::ArtifactRemoved,
        at,
        record_of(plan, authorization),
    ))?;

    std::fs::create_dir_all(&plan.shelf).map_err(|error| {
        Failure::new(
            Category::ResourceDiskReadonly,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the shelf a removal moves artifacts to cannot be made",
        )
        .with_context("shelf", plan.shelf.display().to_string())
        .with_context("reason", error.to_string())
    })?;

    let mut shelved = Vec::new();
    let mut refused = Vec::new();
    let mut bytes = 0_u64;
    for doomed in &plan.doomed {
        let destination = shelf_place(&plan.shelf, &doomed.path);
        match std::fs::rename(&doomed.path, &destination) {
            Ok(()) => {
                bytes = bytes.saturating_add(doomed.bytes);
                shelved.push(destination);
            }
            Err(error) => refused.push((doomed.path.clone(), error.to_string())),
        }
    }

    Ok(Removed {
        shelved,
        refused,
        bytes,
        reversible: plan.reversible,
    })
}

pub fn purge(removed: &Removed, authorization: &Authorization, plan: &Plan) -> Result<u64> {
    authorization.covers(plan)?;

    let mut freed = 0_u64;
    let mut stuck = Vec::new();
    for path in &removed.shelved {
        let bytes = std::fs::metadata(path).map(|metadata| metadata.len()).ok();
        match std::fs::remove_file(path) {
            Ok(()) => freed = freed.saturating_add(bytes.unwrap_or(0)),
            Err(error) => stuck.push(format!("{}: {error}", path.display())),
        }
    }

    if stuck.is_empty() {
        Ok(freed)
    } else {
        Err(Failure::new(
            Category::ResourceDiskReadonly,
            Attribution::Machine,
            Disposition::Partial,
            WHERE,
            "part of the shelf could not be deleted",
        )
        .with_context("freed_bytes", freed.to_string())
        .with_context("stuck", stuck.join("; ")))
    }
}

pub fn restore(removed: &Removed, plan: &Plan) -> Result<Vec<PathBuf>> {
    let mut back = Vec::new();
    let mut stuck = Vec::new();
    for (shelved, doomed) in removed.shelved.iter().zip(plan.doomed.iter()) {
        match std::fs::rename(shelved, &doomed.path) {
            Ok(()) => back.push(doomed.path.clone()),
            Err(error) => stuck.push(format!("{}: {error}", doomed.path.display())),
        }
    }
    if stuck.is_empty() {
        Ok(back)
    } else {
        Err(Failure::new(
            Category::ResourceDiskReadonly,
            Attribution::Machine,
            Disposition::Partial,
            WHERE,
            "part of what was shelved could not be put back",
        )
        .with_context("restored", back.len().to_string())
        .with_context("stuck", stuck.join("; ")))
    }
}

fn record_of(plan: &Plan, authorization: &Authorization) -> Value {
    let removed: Vec<Value> = plan
        .doomed
        .iter()
        .map(|doomed| {
            Value::map([
                ("path", Value::text(doomed.path.display().to_string())),
                (
                    "bytes",
                    Value::Integer(i64::try_from(doomed.bytes).unwrap_or(i64::MAX)),
                ),
            ])
        })
        .collect();
    Value::map([
        ("plan", Value::text(plan.identity())),
        ("reason", Value::text(authorization.reason())),
        ("shelf", Value::text(plan.shelf.display().to_string())),
        (
            "reversible",
            Value::text(if plan.reversible { "yes" } else { "no" }),
        ),
        ("removed", Value::List(removed)),
    ])
}

fn differences_between(authorized: &[Doomed], planned: &[Doomed]) -> Vec<String> {
    let mut differences = Vec::new();
    for one in authorized {
        match planned.iter().find(|other| other.path == one.path) {
            None => differences.push(format!(
                "{} was authorized and is not in this removal",
                one.path.display()
            )),
            Some(other) if other.bytes != one.bytes => differences.push(format!(
                "{} was {} bytes when it was looked at and is {} bytes now",
                one.path.display(),
                one.bytes,
                other.bytes
            )),
            Some(_) => {}
        }
    }
    for other in planned {
        if !authorized.iter().any(|one| one.path == other.path) {
            differences.push(format!(
                "{} is in this removal and nobody authorized it",
                other.path.display()
            ));
        }
    }
    differences
}

fn shelf_place(shelf: &Path, original: &Path) -> PathBuf {
    let mut hasher = Sha256::default();
    hasher.update(original.display().to_string().as_bytes());
    let digest = hasher.finish().hex();
    let short: String = digest.chars().take(16).collect();
    let name = original.file_name().map_or_else(
        || "artifact".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    shelf.join(format!("{short}-{name}"))
}

fn identify(doomed: &[Doomed]) -> String {
    let mut hasher = Sha256::default();
    for one in doomed {
        hasher.update(one.path.display().to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(one.bytes.to_string().as_bytes());
        hasher.update(b"\0");
    }
    let digest = hasher.finish().hex();
    digest.chars().take(16).collect()
}

fn same_filesystem(paths: &[PathBuf], shelf: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    let Some(shelf_device) = nearest_existing(shelf).and_then(|existing| {
        std::fs::metadata(existing)
            .ok()
            .map(|metadata| metadata.dev())
    }) else {
        return false;
    };
    !paths.is_empty()
        && paths.iter().all(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| metadata.dev() == shelf_device)
        })
}

fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut candidate = path.to_path_buf();
    loop {
        if candidate.exists() {
            return Some(candidate);
        }
        if !candidate.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests;
