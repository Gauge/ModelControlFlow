//! What MCF changed, and how to put it back.
//!
//! A27 is absolute and its test has one right answer: *if this run were
//! interrupted at the worst possible moment, could the machine be returned to
//! how it was found?* §3.25 draws the boundary — what MCF owns it changes
//! freely, what it can restore it may change with permission and reverses when
//! the work ends **including after a crash**, and what it cannot restore it
//! does not touch.
//!
//! "Including after a crash" is the whole difficulty. A `Drop` implementation
//! restores when a process unwinds; it does nothing when a process is killed,
//! and B-220's scenario is exactly that. So the ledger is **durable and written
//! before the change is made**: the record of how to undo something reaches the
//! disk first, and the change happens second. A crash between the two leaves a
//! ledger entry for a change that never happened, and restoring it is harmless;
//! a crash the other way round would leave a changed machine nobody can put
//! back, which is the failure A27 forbids.
//!
//! **Recovery happens on open.** [`Ledger::open`] finds anything a previous run
//! left, restores it, and *reports what it did* — silently cleaning up would be
//! the quiet continuation §3.1 forbids, and an operator whose machine was
//! changed and changed back is owed the fact that it happened.
//!
//! **What can be changed is a short, enumerable list**, which A26's shape
//! applied one level out: [`Change`] has one variant today because a replaced
//! file is the only thing MCF alters outside its own directory. Governors,
//! process priorities, exclusive modes and suspensions join it when the
//! environment ladder is built (§6.39, DEC-041, DEC-042) — and B48 requires
//! each be approved per run, so each arrives with the approval that admits it.
//! The list being short is a feature: what is not in it, MCF cannot change.

use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-record::restore");

/// Something MCF changed about the machine, and what it was before.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    /// A file MCF replaced or created.
    ///
    /// `previous` is `None` when the file did not exist, which is a different
    /// restoration from an empty file and is why it is an option rather than
    /// an empty vector (A7's habit, applied to a rollback).
    FileReplaced {
        /// Which file.
        path: PathBuf,
        /// What was there, or nothing if it was not there.
        previous: Option<String>,
    },
}

impl Change {
    /// Records the current state of a file, before MCF replaces it.
    ///
    /// # Errors
    ///
    /// `artifact.unreadable` when the file is there and cannot be read. MCF
    /// does not change what it could not first capture: A27's *what it cannot
    /// restore it does not touch*, made a precondition rather than a rule.
    pub fn about_to_replace(path: &Path) -> Result<Self> {
        let previous = match std::fs::read_to_string(path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(Failure::new(
                    Category::ArtifactUnreadable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "MCF will not change a file it could not first read, because it \
                     could not then put it back",
                )
                .with_context("path", path.display().to_string())
                .with_context("os_error", error.to_string()));
            }
        };
        Ok(Self::FileReplaced {
            path: path.to_path_buf(),
            previous,
        })
    }

    /// Puts it back.
    ///
    /// # Errors
    ///
    /// `platform.restore_failed`, carrying what could not be restored. A27
    /// requires MCF restore everything it changed; where it cannot, §6.39 and
    /// B-210 require it be *named* rather than claimed.
    pub fn restore(&self) -> Result<()> {
        match self {
            Self::FileReplaced { path, previous } => {
                let outcome = match previous {
                    Some(contents) => std::fs::write(path, contents),
                    None => match std::fs::remove_file(path) {
                        // Already gone is already restored.
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        other => other,
                    },
                };
                outcome.map_err(|error| {
                    Failure::new(
                        Category::PlatformRestoreFailed,
                        Attribution::Machine,
                        Disposition::Partial,
                        WHERE,
                        "something MCF changed could not be put back",
                    )
                    .with_context("path", path.display().to_string())
                    .with_context("os_error", error.to_string())
                })
            }
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::FileReplaced { path, previous } => Value::map([
                ("change", Value::text("file_replaced")),
                ("path", Value::text(path.display().to_string())),
                (
                    "previous",
                    match previous {
                        Some(contents) => Value::text(contents.clone()),
                        None => Value::Null,
                    },
                ),
            ]),
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value.get("change")?.as_text()? {
            "file_replaced" => Some(Self::FileReplaced {
                path: PathBuf::from(value.get("path")?.as_text()?),
                previous: match value.get("previous")? {
                    Value::Null => None,
                    Value::Text(contents) => Some(contents.clone()),
                    _ => return None,
                },
            }),
            // A change kind this version does not know is not guessed at. The
            // ledger says so and the caller decides, exactly as the record does
            // with an unknown entry kind (§7.30).
            _ => None,
        }
    }
}

impl core::fmt::Display for Change {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FileReplaced { path, previous } => write!(
                f,
                "{} ({})",
                path.display(),
                match previous {
                    Some(contents) => format!("was {} bytes", contents.len()),
                    None => "did not exist".to_owned(),
                }
            ),
        }
    }
}

/// What a recovery found and did.
#[derive(Debug, Default)]
pub struct Recovery {
    /// What was put back.
    pub restored: Vec<Change>,
    /// What could not be, and why.
    pub failed: Vec<Failure>,
    /// Entries the ledger held that this version does not understand.
    ///
    /// Neither restored nor discarded: A1 forbids losing the information, and
    /// §7.30 makes a record written by another version the reader's problem to
    /// name rather than the parser's to guess at.
    pub unreadable: usize,
}

impl Recovery {
    /// Whether the previous run had left anything.
    #[must_use]
    pub fn found_anything(&self) -> bool {
        !self.restored.is_empty() || !self.failed.is_empty() || self.unreadable > 0
    }

    /// A sentence stating what happened, whichever way it went.
    #[must_use]
    pub fn statement(&self) -> String {
        if !self.found_anything() {
            return "nothing was left to restore".to_owned();
        }
        let mut said = vec![format!(
            "a previous run left {} change(s); {} restored",
            self.restored.len() + self.failed.len() + self.unreadable,
            self.restored.len()
        )];
        if !self.failed.is_empty() {
            said.push(format!("{} COULD NOT BE", self.failed.len()));
        }
        if self.unreadable > 0 {
            said.push(format!(
                "{} written by a version this one cannot read",
                self.unreadable
            ));
        }
        said.join(", ")
    }
}

/// A durable record of what MCF has changed and not yet put back.
#[derive(Debug)]
pub struct Ledger {
    path: PathBuf,
    pending: Vec<Change>,
}

impl Ledger {
    /// Opens the ledger, restoring anything a previous run left.
    ///
    /// # Errors
    ///
    /// `record.unwritable` when the ledger's directory cannot be made. A
    /// restoration that *failed* is not an error here: it is in the
    /// [`Recovery`], because the ledger opened and the operator needs both
    /// facts.
    pub fn open(path: &Path) -> Result<(Self, Recovery)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(path, &error))?;
        }
        let recovery = recover(path);
        let ledger = Self {
            path: path.to_path_buf(),
            pending: Vec::new(),
        };
        // The ledger is cleared only after every entry has been dealt with, so
        // a crash during recovery leaves the work still to do.
        if recovery.failed.is_empty() && recovery.unreadable == 0 {
            let _cleared = std::fs::remove_file(path);
        }
        Ok((ledger, recovery))
    }

    /// Records a change **before** it is made, durably.
    ///
    /// # Errors
    ///
    /// `record.unwritable`. The change must not be made if this fails: that is
    /// the ordering the whole design rests on, and it is the caller's to
    /// honour because only the caller can not-make-the-change.
    pub fn record(&mut self, change: Change) -> Result<()> {
        self.pending.push(change);
        self.flush()
    }

    /// Everything MCF has changed and not yet put back.
    #[must_use]
    pub fn pending(&self) -> &[Change] {
        &self.pending
    }

    /// Puts everything back, newest first, and clears the ledger.
    ///
    /// Newest first because changes can overlap: a file replaced twice is
    /// restored to what it was before the *first* replacement only if the
    /// second is undone first.
    ///
    /// # Errors
    ///
    /// The first failure encountered, after attempting every restoration.
    /// Attempting all of them matters more than reporting the first: A27 wants
    /// the machine back, and stopping at the first failure would leave the rest
    /// changed.
    pub fn restore_all(&mut self) -> Result<()> {
        let mut first: Option<Failure> = None;
        while let Some(change) = self.pending.pop() {
            if let (Err(failure), None) = (change.restore(), first.as_ref()) {
                first = Some(failure);
            }
        }
        let _cleared = std::fs::remove_file(&self.path);
        match first {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    fn flush(&self) -> Result<()> {
        let lines: Vec<String> = self
            .pending
            .iter()
            .map(|change| change.to_value().to_line())
            .collect();
        let mut text = lines.join("\n");
        text.push('\n');
        std::fs::write(&self.path, text).map_err(|error| unwritable(&self.path, &error))
    }
}

impl Drop for Ledger {
    /// Restores on the ordinary path out.
    ///
    /// This is the *convenience*, not the guarantee. A killed process runs no
    /// destructor, which is why the durable ledger exists and why
    /// [`Ledger::open`] recovers.
    fn drop(&mut self) {
        let _restored = self.restore_all();
    }
}

fn recover(path: &Path) -> Recovery {
    let mut recovery = Recovery::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return recovery;
    };
    // Newest first, for the reason `restore_all` gives.
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = json::parse(line) else {
            recovery.unreadable += 1;
            continue;
        };
        let Some(change) = Change::from_value(&value) else {
            recovery.unreadable += 1;
            continue;
        };
        match change.restore() {
            Ok(()) => recovery.restored.push(change),
            Err(failure) => recovery.failed.push(failure),
        }
    }
    recovery
}

fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the restoration ledger could not be written, so nothing may be changed",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

#[cfg(test)]
mod tests;
