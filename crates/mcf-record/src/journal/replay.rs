//! Reading a journal back, and saying exactly what could not be read.
//!
//! B62: *where a replay cannot complete, MCF reports what was lost and how much
//! rather than opening with a shorter history — the silent option A2 forbids,
//! aimed at the record itself.* A journal that opens successfully with three
//! months missing is the specific failure this module exists to make
//! impossible.
//!
//! So a replay always returns two things: everything it could read, and — if it
//! stopped early — a [`Loss`] naming the line, the byte offset, the number of
//! bytes it did not read, and why. A4 keeps the first: entries before the
//! damage are a real, usable history and are not discarded because the file
//! ends badly.

use std::path::Path;

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::{Timestamp, UtcOffset};

use crate::json::{self, Value};

use super::{Entry, EntryKind, FORMAT_VERSION};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::replay");

/// What a replay could not read, and how much of it there was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// The one-based line the replay stopped at.
    pub line: usize,
    /// The byte offset that line starts at.
    pub byte_offset: usize,
    /// How many bytes were not read.
    pub bytes_unread: usize,
    /// The classified reason.
    pub failure: Failure,
}

impl core::fmt::Display for Loss {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "stopped at line {} (byte {}), {} bytes unread: {}",
            self.line, self.byte_offset, self.bytes_unread, self.failure
        )
    }
}

/// The result of reading a journal.
#[derive(Debug)]
pub struct Replay {
    /// Everything that could be read, in the order it was written.
    pub entries: Vec<Entry>,
    /// What stopped the replay, if anything did.
    ///
    /// `None` means the whole file was read. It does not mean the file is
    /// undamaged in some larger sense — only that nothing in it was
    /// unreadable, which is the claim this module is able to make.
    pub loss: Option<Loss>,
}

impl Replay {
    /// Whether the whole journal was read.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.loss.is_none()
    }

    /// A sentence stating what was recovered and what was not.
    ///
    /// Always says both. A rendering that mentioned the loss only when there
    /// was one would train a reader to skim past the line that matters.
    #[must_use]
    pub fn statement(&self) -> String {
        match &self.loss {
            None => format!("{} entries, complete", self.entries.len()),
            Some(loss) => format!("{} entries recovered, then {loss}", self.entries.len()),
        }
    }
}

/// Reads a journal back.
///
/// # Errors
///
/// `record.unwritable` when the file cannot be read at all, and
/// `record.corrupt.journal` when its header is missing or names no format —
/// those are failures of the *whole* file rather than a loss partway through
/// it, and there is no partial history to return.
///
/// A damaged or truncated entry is **not** an error: it is a [`Loss`] on an
/// otherwise successful replay, because the entries before it are real.
pub fn replay(path: &Path) -> Result<Replay> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the journal could not be read",
        )
        .with_context("path", path.display().to_string())
        .with_context("os_error", error.to_string())
    })?;

    let mut entries = Vec::new();
    let mut offset = 0_usize;
    let mut line_number = 0_usize;

    for line in text.split_inclusive('\n') {
        line_number += 1;
        let start = offset;
        offset += line.len();
        let complete = line.ends_with('\n');
        let content = line.trim_end_matches('\n');

        // A line with no terminator is a torn write: the process died between
        // the write and the barrier, or the medium filled. It is the expected
        // crash residue, and it is reported as such rather than as corruption.
        if !complete {
            return Ok(Replay {
                entries,
                loss: Some(Loss {
                    line: line_number,
                    byte_offset: start,
                    bytes_unread: text.len() - start,
                    failure: torn(path, content.len()),
                }),
            });
        }

        if content.is_empty() {
            continue;
        }

        let value = match json::parse(content) {
            Ok(value) => value,
            Err(error) => {
                return Ok(Replay {
                    entries,
                    loss: Some(Loss {
                        line: line_number,
                        byte_offset: start,
                        bytes_unread: text.len() - start,
                        failure: unreadable(path, &error.to_string()),
                    }),
                });
            }
        };

        if line_number == 1 {
            check_header(path, &value)?;
            continue;
        }

        match read_entry(&value) {
            Some(entry) => entries.push(entry),
            None => {
                return Ok(Replay {
                    entries,
                    loss: Some(Loss {
                        line: line_number,
                        byte_offset: start,
                        bytes_unread: text.len() - start,
                        failure: unreadable(path, "the line is JSON and is not an entry"),
                    }),
                });
            }
        }
    }

    Ok(Replay {
        entries,
        loss: None,
    })
}

fn check_header(path: &Path, header: &Value) -> Result<()> {
    match header.get("format").and_then(Value::as_integer) {
        Some(FORMAT_VERSION) => Ok(()),
        Some(other) => Err(Failure::new(
            Category::RecordSchemaUnknown,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "the journal was written by a format version this build does not read",
        )
        .with_context("path", path.display().to_string())
        .with_context("found_format", other.to_string())),
        None => Err(Failure::new(
            Category::RecordCorruptJournal,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "the journal's first line is not a header",
        )
        .with_context("path", path.display().to_string())),
    }
}

/// Rebuilds an entry from a line, or `None` if the line is not one.
///
/// The envelope is what is required — identifier, kind, moment — and the body
/// is carried through whatever it contains. A kind this version does not know
/// makes the line unreadable *as an entry*, which is the honest answer under
/// §7.30 rather than a half-understood record admitted into a history.
fn read_entry(value: &Value) -> Option<Entry> {
    let kind = EntryKind::parse(value.get("kind")?.as_text()?)?;
    let nanos = value.get("recorded_at_utc_nanos")?.as_integer()?;
    let body = value.get("body")?.clone();
    value.get("id")?.as_text()?;

    // The offset is not read back from the text rendering: D9 stores it
    // alongside the moment, and a replay that inferred one from a formatted
    // string would be inventing a condition (A7). Entries written by this
    // version carry `unknown`, and that is what comes back.
    let recorded_at = Timestamp::from_utc_nanos(i128::from(nanos), read_offset(value));
    Some(Entry::new(kind, recorded_at, 0, body))
}

fn read_offset(value: &Value) -> Attested<UtcOffset> {
    value
        .get("recorded_at_offset_seconds")
        .and_then(Value::as_integer)
        .and_then(|seconds| i32::try_from(seconds).ok())
        .and_then(UtcOffset::from_seconds_east)
        .map_or(Attested::Unknown, Attested::Known)
}

fn torn(path: &Path, bytes: usize) -> Failure {
    Failure::new(
        Category::RecordReplayIncomplete,
        Attribution::Machine,
        Disposition::Partial,
        WHERE,
        "the journal ends with an unterminated line, which is what a crash mid-append leaves",
    )
    .with_context("path", path.display().to_string())
    .with_context("torn_line_bytes", bytes.to_string())
}

fn unreadable(path: &Path, detail: &str) -> Failure {
    Failure::new(
        Category::RecordCorruptJournal,
        Attribution::Machine,
        Disposition::Partial,
        WHERE,
        "a journal line could not be read",
    )
    .with_context("path", path.display().to_string())
    .with_context("detail", detail.to_owned())
}
