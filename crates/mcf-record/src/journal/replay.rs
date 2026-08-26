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

use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
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

/// One entry, and exactly where in the journal it was.
///
/// The offset and the length are what makes an index possible (B-300, D20): a
/// derived index stores where each entry is and reads back only the bytes it
/// needs, rather than paying for the whole history at every open.
#[derive(Debug, Clone)]
pub struct Placed {
    /// The entry.
    pub entry: Entry,
    /// Which line of the journal it is, one-based.
    pub line: usize,
    /// Where its line begins.
    pub byte_offset: u64,
    /// How long its line is, terminator included.
    pub byte_length: u32,
}

/// The result of reading part or all of a journal, with each entry placed.
#[derive(Debug)]
pub struct Placement {
    /// Everything that could be read, in the order it was written.
    pub placed: Vec<Placed>,
    /// What stopped the reading, if anything did.
    pub loss: Option<Loss>,
    /// The byte the reading got to.
    ///
    /// Where a loss stopped it, this is where the loss begins: a caller that
    /// indexes what came before knows exactly what it has covered.
    pub read_to: u64,
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
    let read = replay_from(path, 0, 0)?;
    Ok(Replay {
        entries: read.placed.into_iter().map(|placed| placed.entry).collect(),
        loss: read.loss,
    })
}

/// Reads a journal from a byte offset, keeping where each entry was.
///
/// `from` must be the start of a line — an index records the end of the last
/// entry it covered, which is exactly that. `lines_before` is how many lines
/// precede it, so that a [`Loss`] names the line as the whole file numbers it
/// rather than as this reading numbers it: a report that said *line 3* about
/// the four hundred thousandth line would be worse than no number at all.
///
/// Reading from an offset does **not** check the header, because the header is
/// not there to read. A caller resuming into the middle of a file is asserting
/// it already knows which file this is; [`crate::journal::index`] does that by
/// fingerprint before it ever calls this.
///
/// # Errors
///
/// `record.unwritable` when the file cannot be opened or measured, and
/// `record.corrupt.journal` / `record.schema.unknown` from the header when
/// reading from the beginning. A damaged entry partway through is a [`Loss`]
/// rather than an error, exactly as in [`replay`].
#[allow(
    clippy::too_many_lines,
    reason = "one loop with one exit per way a line can be wrong; splitting it \
              would put the four losses somewhere other than where they are found"
)]
pub fn replay_from(path: &Path, from: u64, lines_before: usize) -> Result<Placement> {
    let file = std::fs::File::open(path).map_err(|error| unreadable_file(path, &error))?;
    let total = file
        .metadata()
        .map_err(|error| unreadable_file(path, &error))?
        .len();
    let mut reader = BufReader::new(file);
    if from > 0 {
        reader
            .seek(SeekFrom::Start(from))
            .map_err(|error| unreadable_file(path, &error))?;
    }

    let mut placed = Vec::new();
    let mut offset = from;
    let mut line_number = lines_before;
    let mut raw = Vec::new();

    loop {
        raw.clear();
        // Read as bytes rather than as text: a journal with a non-UTF-8 byte in
        // it is damaged *at that line*, and reading the whole file as a string
        // would turn one bad byte into a file nothing can read — which is the
        // shorter-history failure B62 is about, in its most complete form.
        let read = reader
            .read_until(b'\n', &mut raw)
            .map_err(|error| unreadable_file(path, &error))?;
        if read == 0 {
            break;
        }
        line_number += 1;
        let start = offset;
        offset = offset.saturating_add(read as u64);
        let complete = raw.last() == Some(&b'\n');
        let unread = || total.saturating_sub(start);

        // A line with no terminator is a torn write: the process died between
        // the write and the barrier, or the medium filled. It is the expected
        // crash residue, and it is reported as such rather than as corruption.
        if !complete {
            return Ok(Placement {
                placed,
                loss: Some(Loss {
                    line: line_number,
                    byte_offset: usize::try_from(start).unwrap_or(usize::MAX),
                    bytes_unread: usize::try_from(unread()).unwrap_or(usize::MAX),
                    failure: torn(path, raw.len()),
                }),
                read_to: start,
            });
        }

        let Ok(content) = core::str::from_utf8(raw.get(..read.saturating_sub(1)).unwrap_or(&[]))
        else {
            return Ok(Placement {
                placed,
                loss: Some(Loss {
                    line: line_number,
                    byte_offset: usize::try_from(start).unwrap_or(usize::MAX),
                    bytes_unread: usize::try_from(unread()).unwrap_or(usize::MAX),
                    failure: unreadable(path, "the line is not text"),
                }),
                read_to: start,
            });
        };

        if content.is_empty() {
            continue;
        }

        let value = match json::parse(content) {
            Ok(value) => value,
            Err(error) => {
                return Ok(Placement {
                    placed,
                    loss: Some(Loss {
                        line: line_number,
                        byte_offset: usize::try_from(start).unwrap_or(usize::MAX),
                        bytes_unread: usize::try_from(unread()).unwrap_or(usize::MAX),
                        failure: unreadable(path, &error.to_string()),
                    }),
                    read_to: start,
                });
            }
        };

        if line_number == 1 {
            check_header(path, &value)?;
            continue;
        }

        match read_entry(&value) {
            Some(entry) => placed.push(Placed {
                entry,
                line: line_number,
                byte_offset: start,
                byte_length: u32::try_from(read).unwrap_or(u32::MAX),
            }),
            None => {
                return Ok(Placement {
                    placed,
                    loss: Some(Loss {
                        line: line_number,
                        byte_offset: usize::try_from(start).unwrap_or(usize::MAX),
                        bytes_unread: usize::try_from(unread()).unwrap_or(usize::MAX),
                        failure: unreadable(path, "the line is JSON and is not an entry"),
                    }),
                    read_to: start,
                });
            }
        }
    }

    Ok(Placement {
        placed,
        loss: None,
        read_to: offset,
    })
}

/// Reads exactly one entry, from bytes an index says it occupies.
///
/// Bounded by the length it is given rather than by the end of the file: this
/// is the read a query makes after the index has told it where to look, and a
/// query that read to the end of a million-entry journal would be the cost the
/// index exists to avoid.
pub(super) fn read_entry_at(path: &Path, offset: u64, length: u32) -> Result<Entry> {
    let mut file = std::fs::File::open(path).map_err(|error| unreadable_file(path, &error))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| unreadable_file(path, &error))?;
    let mut line = vec![0_u8; length as usize];
    file.read_exact(&mut line)
        .map_err(|error| unreadable_file(path, &error))?;

    let content = core::str::from_utf8(&line)
        .map(|text| text.trim_end_matches('\n'))
        .map_err(|_| not_an_entry(path, offset, "the bytes there are not text"))?;
    let value =
        json::parse(content).map_err(|error| not_an_entry(path, offset, &error.to_string()))?;
    read_entry(&value).ok_or_else(|| not_an_entry(path, offset, "the line there is not an entry"))
}

fn not_an_entry(path: &Path, offset: u64, why: &str) -> Failure {
    Failure::new(
        Category::RecordCorruptJournal,
        Attribution::Mcf,
        Disposition::Refused,
        WHERE,
        "the journal does not hold an entry where the index says it does",
    )
    .with_context("path", path.display().to_string())
    .with_context("byte_offset", offset.to_string())
    .with_context("detail", why.to_owned())
}

fn unreadable_file(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the journal could not be read",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
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
