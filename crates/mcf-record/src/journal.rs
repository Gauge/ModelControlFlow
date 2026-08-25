//! The journal: an append-only file that *is* the record.
//!
//! D20 settles the shape and B62 is the rule: **the journal is the record; the
//! database is derived.** Entries are appended as they happen, the queryable
//! store is built from them and may be discarded and rebuilt, and where a
//! replay cannot complete MCF reports what was lost and how much rather than
//! opening with a shorter history — which is the silent option A2 forbids,
//! aimed at the record itself.
//!
//! **Written at events, never on a timer** (B4, §3.3). There is no flush
//! thread, no batching timer and no background writer in this module, and the
//! absence is the enforcement: nothing here can be scheduled, so an idle MCF
//! writes nothing and wakes for nothing.
//!
//! **Crash safety is configuration, not assumption** (D20). Every append is a
//! single write of one complete line followed by a durability barrier, so the
//! failure mode a crash produces is a *torn last line* — which [`replay`] can
//! identify, bound and report. Anything cleverer would produce failure modes
//! that are harder to describe rather than less likely.
//!
//! **One line per entry.** A torn write is then a torn line: replay finds the
//! boundary, reports the loss precisely, and keeps everything before it (A4).
//!
//! At M0 the journal is the whole record. D6's SQLite index is derived from it
//! and arrives with B-042; nothing here depends on that, which is D20's point.

mod entry;
mod replay;

pub use entry::{Entry, EntryId, EntryKind};
pub use replay::{Loss, Replay};

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-record::journal");

/// The journal format's version.
///
/// Written into the header of every journal at creation, and checked on open.
/// §7.30 makes a schema a public interface the moment it is shared, so this is
/// present from the first write rather than added when it first changes —
/// B-042's *schema-versioned from the first write*, applied to the journal that
/// precedes the database.
pub const FORMAT_VERSION: i64 = 1;

/// An open journal.
///
/// Holds the file open for appending and nothing else: no buffer that could be
/// lost, no queue that could be reordered, no state that a crash could leave
/// disagreeing with the file.
#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    file: File,
    appended: u64,
}

impl Journal {
    /// Opens a journal, creating it if it does not exist.
    ///
    /// A new journal gets a header line naming the format version and the build
    /// that created it (§3.4 — MCF's own version is a condition of everything
    /// that follows). An existing one is checked: a journal written by a format
    /// version this build cannot read is `record.schema.unknown`, refused
    /// rather than appended to, because appending would make the file
    /// unreadable to both versions.
    ///
    /// # Errors
    ///
    /// `record.unwritable` when the path cannot be created or opened;
    /// `record.schema.unknown` when the existing header names a format this
    /// build does not read; `record.corrupt.journal` when the header is absent
    /// or unreadable.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(path, &error))?;
        }
        let fresh = !path.exists();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open(path)
            .map_err(|error| unwritable(path, &error))?;

        let mut journal = Self {
            path: path.to_path_buf(),
            file,
            appended: 0,
        };
        if fresh {
            journal.write_line(&header())?;
        } else {
            journal.check_header()?;
        }
        Ok(journal)
    }

    /// Where this journal lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// How many entries this handle has appended.
    ///
    /// Not how many the file holds: that is [`replay`]'s answer, and it comes
    /// from the file rather than from anything held in memory.
    #[must_use]
    pub const fn appended(&self) -> u64 {
        self.appended
    }

    /// Appends an entry, durably.
    ///
    /// Returns only after the entry is on the medium. §VII would prefer this be
    /// cheaper and P2 settles it: *MCF may not become fast by recording less
    /// than the science requires*. D24 budgets the cost at 2 ms per event, and
    /// B-011 is where that is asserted rather than hoped.
    ///
    /// # Errors
    ///
    /// `record.unwritable` when the write or the durability barrier fails. The
    /// caller is told; nothing is retried silently, because a retried write
    /// that succeeded is a different event from one that succeeded first time
    /// (B2).
    pub fn append(&mut self, entry: &Entry) -> Result<()> {
        self.write_line(&entry.to_value())?;
        self.appended = self.appended.saturating_add(1);
        Ok(())
    }

    fn write_line(&mut self, value: &Value) -> Result<()> {
        let mut line = value.to_line();
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .map_err(|error| unwritable(&self.path, &error))?;
        // The durability barrier. `sync_data` rather than `sync_all`: the
        // entry's bytes are what must survive, and the file's metadata is
        // rewritten by the next append anyway.
        self.file
            .sync_data()
            .map_err(|error| unwritable(&self.path, &error))
    }

    fn check_header(&self) -> Result<()> {
        let text = std::fs::read_to_string(&self.path).map_err(|error| {
            Failure::new(
                Category::RecordCorruptJournal,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the journal exists and could not be read",
            )
            .with_context("path", self.path.display().to_string())
            .with_context("os_error", error.to_string())
        })?;

        let Some(first) = text.lines().next() else {
            return Err(Failure::new(
                Category::RecordCorruptJournal,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the journal is empty and has no header",
            )
            .with_context("path", self.path.display().to_string()));
        };

        let header = crate::json::parse(first).map_err(|error| {
            Failure::new(
                Category::RecordCorruptJournal,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the journal's header line cannot be read",
            )
            .with_context("path", self.path.display().to_string())
            .with_context("detail", error.to_string())
        })?;

        let version = header.get("format").and_then(Value::as_integer);
        match version {
            Some(FORMAT_VERSION) => Ok(()),
            Some(other) => Err(Failure::new(
                Category::RecordSchemaUnknown,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the journal was written by a format version this build does not read",
            )
            .with_context("path", self.path.display().to_string())
            .with_context("found_format", other.to_string())
            .with_context("this_format", FORMAT_VERSION.to_string())),
            None => Err(Failure::new(
                Category::RecordCorruptJournal,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the journal's header names no format version",
            )
            .with_context("path", self.path.display().to_string())),
        }
    }
}

/// The header every journal opens with.
fn header() -> Value {
    let identity = BuildIdentity::current();
    Value::map([
        ("format", Value::Integer(FORMAT_VERSION)),
        (
            "created_by",
            Value::map([
                ("version", Value::text(identity.version)),
                ("revision", Value::text(identity.revision.to_string())),
                ("rustc", Value::text(identity.rustc)),
                ("target", Value::text(identity.target)),
                ("profile", Value::text(identity.profile)),
            ]),
        ),
    ])
}

fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the record could not be written",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

/// Where the record lives on this machine.
///
/// `$XDG_DATA_HOME/mcf/record.jsonl`, falling back to `$HOME/.local/share`.
/// Returns `None` when neither is set: A7's habit applied to a path — MCF does
/// not invent a location to write the user's evidence into.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        })?;
    Some(base.join("mcf").join("record.jsonl"))
}

pub use replay::replay;

#[cfg(test)]
mod tests;
