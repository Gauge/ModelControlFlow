//! Artifacts leave this machine because somebody said so (B-027, §3.11).
//!
//! **The failure this exists against is a helpful one.** A disk fills, and a
//! tool that wants to keep working deletes the oldest thing it can find. MCF
//! will not: §3.11 makes disk arbitration a decision rather than a surprise,
//! and an artifact is not a cache entry — it is what a measurement was made
//! against, and re-acquiring it is not always possible (§XII's chain can be
//! withdrawn, gated or relicensed between one week and the next, DEC-038).
//!
//! So there is no path from *space is short* to *bytes are gone*. Removal takes
//! three separate acts, and each one is a type:
//!
//! 1. [`preview`] says exactly what would go, how much it weighs, and whether
//!    it could be brought back. Nothing is touched.
//! 2. [`Authorization::given`] is somebody deciding, with a reason, about *that
//!    plan* — a plan that has since changed is refused rather than applied to
//!    files nobody looked at.
//! 3. [`remove`] writes the record first and then **moves** the artifact to a
//!    shelf. It deletes nothing at all.
//!
//! Deleting is [`purge`], which is a fourth act with an authorization of its
//! own. That is what *reversible where reasonable* buys: a rename inside one
//! filesystem is free, so the reversible case costs nothing and the operator
//! who moved the wrong model has an afternoon to notice.
//!
//! **What is on the disk is what MCF knows.** An artifact's provenance lives
//! beside it in a sidecar rather than in an index, because an index is a second
//! copy that drifts: a model moved by hand, a directory restored from a backup,
//! a machine that lost its journal — in each case the sidecar is still there and
//! still true, and §3.6's *provenance travels with the artifact* is a statement
//! about the artifact rather than about MCF's bookkeeping. [`held`] reads what
//! is there, and an artifact whose provenance is missing or unreadable is
//! listed as exactly that (A7).
//!
//! **The record is written before the artifact moves.** A1: after a removal the
//! artifact is gone and the record is all there is, so a record written
//! afterwards is one that a crash can lose along with the thing it describes.
//! Written first, the worst case is a record of a removal that did not finish —
//! which is exactly what the shelf will show.

use std::path::{Path, PathBuf};

use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::provenance::Provenance;
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind, Journal};
use mcf_record::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-hub::store");

/// What MCF is holding: an artifact, and whatever can be said about where it
/// came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    /// Where the artifact is.
    pub path: PathBuf,
    /// How many bytes it is.
    pub bytes: u64,
    /// Where it came from, if the sidecar beside it can be read.
    ///
    /// The failure is kept rather than flattened to `None`: *there is no
    /// provenance here* and *there is one and MCF cannot read it* are different
    /// states, and an operator can act on the second (A7, A2).
    pub provenance: std::result::Result<Provenance, Option<Failure>>,
}

impl Held {
    /// What a surface says about it, in one line.
    #[must_use]
    pub fn describe(&self) -> String {
        let origin = match &self.provenance {
            Ok(provenance) => provenance.origin().to_string(),
            Err(None) => "origin unknown — nothing beside it says where it came from".to_owned(),
            Err(Some(failure)) => format!("origin unreadable — {failure}"),
        };
        format!("{} ({} bytes) — {origin}", self.path.display(), self.bytes)
    }
}

/// Where an artifact's provenance is written.
///
/// Beside it, under its own name plus a suffix, so that moving the artifact and
/// forgetting the sidecar is visible rather than silent: what is left is an
/// artifact with no provenance, which [`held`] reports as exactly that.
#[must_use]
pub fn provenance_path(artifact: &Path) -> PathBuf {
    let mut name = artifact.file_name().unwrap_or_default().to_os_string();
    name.push(PROVENANCE_SUFFIX);
    artifact.with_file_name(name)
}

/// The suffix a provenance sidecar carries.
const PROVENANCE_SUFFIX: &str = ".mcf-provenance.json";

/// Writes an artifact's provenance beside it.
///
/// # Errors
///
/// `resource.disk.readonly` when the sidecar cannot be written. Failing to
/// record provenance is not a partial success to be shrugged at: an artifact
/// whose origin was never written down is one §3.6 says MCF should not be
/// holding, and the caller is told so it can decide (A2).
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

/// Reads an artifact's provenance from beside it.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the sidecar is there and cannot be
/// read — malformed JSON, a field absent, a link in the chain unreadable.
/// `artifact.missing` when there is no sidecar at all, which is a state rather
/// than a defect: a model an operator put there by hand has no provenance and
/// saying so is the honest answer (A7).
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

/// Everything MCF is holding under a directory, and what is known about each.
///
/// Reads the disk rather than an index. Directories are walked in the order the
/// filesystem gives them and the result is sorted by path, so two runs on one
/// machine list the same things in the same order (§3.17).
///
/// # Errors
///
/// `artifact.unreadable` when the directory itself cannot be read. An artifact
/// inside it that cannot be measured is *listed* with what went wrong rather
/// than dropped: a list that silently shortened itself is the one thing a list
/// must not do (A1, A4).
pub fn held(root: &Path) -> Result<Vec<Held>> {
    let mut found = Vec::new();
    walk(root, &mut found)?;
    found.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(found)
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
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(PROVENANCE_SUFFIX))
        {
            continue;
        }
        let bytes = std::fs::metadata(&path).map(|metadata| metadata.len());
        into.push(Held {
            bytes: bytes.unwrap_or(0),
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

/// One file a plan would remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doomed {
    /// Where it is.
    pub path: PathBuf,
    /// How many bytes it is, read at preview time.
    pub bytes: u64,
}

/// What a removal would do, before anything is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    doomed: Vec<Doomed>,
    shelf: PathBuf,
    reversible: bool,
    identity: String,
}

impl Plan {
    /// Everything that would go.
    #[must_use]
    pub fn doomed(&self) -> &[Doomed] {
        &self.doomed
    }

    /// How much would be freed, or `None` if the sizes do not add up — a total
    /// MCF will not state wrongly (A6).
    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        self.doomed
            .iter()
            .try_fold(0_u64, |total, doomed| total.checked_add(doomed.bytes))
    }

    /// Whether what this removes could be brought back.
    ///
    /// Measured rather than assumed: the shelf and the artifact are on the same
    /// filesystem or they are not, and a rename across filesystems is a copy
    /// nobody asked for. False is not a refusal — it is the fact an operator
    /// authorizes against.
    #[must_use]
    pub const fn reversible(&self) -> bool {
        self.reversible
    }

    /// Which plan this is.
    ///
    /// A digest of what it would remove and how big those things were, so that
    /// an authorization is for *this* removal. A file that changed between the
    /// looking and the deciding produces a different identity, and the removal
    /// is refused rather than performed on something nobody previewed.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// What an operator reads before deciding.
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

/// Looks at what a removal would do, and touches nothing.
///
/// # Errors
///
/// `artifact.missing` when one of the paths is not there: a plan that quietly
/// dropped it would be a plan an operator reads as *this is everything*, and
/// the missing file is a thing to explain rather than to skip (A1, A7).
/// `artifact.unreadable` when a path cannot be measured, for the same reason.
pub fn preview(paths: &[PathBuf], shelf: &Path) -> Result<Plan> {
    let mut doomed: Vec<Doomed> = Vec::new();
    let mut wanted: Vec<PathBuf> = Vec::new();
    for path in paths {
        if !wanted.contains(path) {
            wanted.push(path.clone());
        }
        // An artifact's provenance goes where the artifact goes. Leaving the
        // sidecar behind would strand a record of something that is no longer
        // there and shelve an artifact that can no longer say where it came
        // from — and §3.6 makes the two one thing. It is named in the plan
        // rather than moved quietly, because a preview that hid a file is not a
        // preview.
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

/// Somebody deciding, about one plan, for a stated reason.
///
/// It carries no capability of its own: it is a claim that a human looked at a
/// particular list of files at particular sizes and said yes. [`remove`] checks
/// that the plan in front of it is still that list.
///
/// It holds the list rather than a digest of it so that a refusal can say
/// *which* file changed and *by how much*. A refusal that printed two hex
/// strings would be correct and useless, and §3.1 asks a refusal to be
/// actionable rather than merely right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    doomed: Vec<Doomed>,
    reason: String,
}

impl Authorization {
    /// Authorizes this plan.
    ///
    /// # Errors
    ///
    /// `config.invalid` when the reason is blank. The reason is not decoration:
    /// §3.11's requirement is that a removal be a decision, and *because the
    /// disk was full* and *because I replaced it with the Q6 quantization* are
    /// different decisions that a record with no reason cannot tell apart.
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

    /// What was authorized: the files, at the sizes they were looked at.
    #[must_use]
    pub fn doomed(&self) -> &[Doomed] {
        &self.doomed
    }

    /// Whether this authorization is for the removal in front of it.
    ///
    /// # Errors
    ///
    /// `config.invalid`, naming every difference: a file that changed size, one
    /// that has appeared, one that has gone. Each is a reason the removal about
    /// to happen is not the one somebody looked at.
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

    /// Why.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// What a removal did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    /// Where each artifact went, in the order the plan named them.
    pub shelved: Vec<PathBuf>,
    /// Anything that could not be moved, and what stopped it.
    ///
    /// A4: a removal that moved four files of five is an outcome, not an
    /// error. Both halves are in the record and both halves are here.
    pub refused: Vec<(PathBuf, String)>,
    /// How many bytes are now on the shelf.
    pub bytes: u64,
    /// Whether what is on the shelf can be moved back.
    pub reversible: bool,
}

impl Removed {
    /// Whether everything the plan named was moved.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.refused.is_empty()
    }
}

/// Performs an authorized removal: records it, then moves the artifacts to the
/// shelf.
///
/// Nothing is deleted here. The shelf is where an artifact waits for [`purge`],
/// which is a separate decision — and on a machine where the shelf is on
/// another filesystem, where it waits is a copy nobody made, so the plan says
/// so and the operator authorizes that fact.
///
/// # Errors
///
/// `config.invalid` when the authorization is for a different plan than the one
/// supplied — including the same files after one of them changed size, which is
/// a different removal from the one somebody looked at.
/// `record.unwritable` when the record cannot be written, in which case nothing
/// is moved: an artifact that vanished without a record is the one outcome A1
/// forbids outright.
pub fn remove(
    plan: &Plan,
    authorization: &Authorization,
    journal: &mut Journal,
    at: Timestamp,
) -> Result<Removed> {
    authorization.covers(plan)?;

    // Before anything moves. A record written afterwards is one a crash can
    // lose along with the artifact it describes (A1).
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

/// Deletes what is on the shelf, for a reason, having been asked to.
///
/// The only function in MCF that destroys an artifact, and it takes an
/// authorization naming the plan whose shelf it is emptying. Reclaiming space
/// is a thing an operator does, never a thing MCF does while nobody is looking
/// (§3.11).
///
/// # Errors
///
/// `config.invalid` when the authorization is for a different plan.
/// `resource.disk.readonly` when something on the shelf will not go — reported
/// with what did (A4).
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

/// Puts an artifact back where it came from.
///
/// The reason the shelf exists. It is not an undo of the record — the record
/// says a removal happened, and it did — it is the artifact returning, which is
/// a second event and the caller's to record.
///
/// # Errors
///
/// `resource.disk.readonly` when a file will not move back, naming which.
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

/// What the record says about a removal.
///
/// Everything needed to say what left and on whose word, because after this the
/// artifact is gone and this is all there is (A1).
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

/// What is different between what was authorized and what is planned.
///
/// Every difference, not the first: A1 keeps what was found, and an operator
/// told about one changed file out of three will look at one file.
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

/// Where on the shelf a doomed file goes.
///
/// Named by a digest of its original path rather than by its basename: two
/// repositories both publishing `model.gguf` would otherwise land on top of
/// each other, and a removal that destroyed the artifact it was preserving
/// would be the worst possible way to fail.
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

/// A digest of what a plan would remove.
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

/// Whether everything a plan names lives on the same filesystem as the shelf.
///
/// Read from the device the kernel reports rather than guessed from the paths:
/// a bind mount, a separate `/home` and a FUSE mount all look like ordinary
/// directories, and F5 already cost MCF a day to a filesystem nobody had
/// noticed. Where the shelf does not exist yet, its nearest existing ancestor
/// answers — that is where it will be made.
///
/// Unknown answers `false`, which is the direction that cannot mislead: an
/// operator told a removal is irreversible loses nothing but an afternoon of
/// convenience, and one told it is reversible when it is not loses the model.
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
