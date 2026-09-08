use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::Path;

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::{Timestamp, UtcOffset};

use crate::json::{self, Value};

use super::{Entry, EntryKind, FORMAT_VERSION};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::replay");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    pub line: usize,
    pub byte_offset: usize,
    pub bytes_unread: usize,
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

#[derive(Debug, Clone)]
pub struct Placed {
    pub entry: Entry,
    pub line: usize,
    pub byte_offset: u64,
    pub byte_length: u32,
}

#[derive(Debug)]
pub struct Placement {
    pub placed: Vec<Placed>,
    pub loss: Option<Loss>,
    pub read_to: u64,
}

#[derive(Debug)]
pub struct Replay {
    pub entries: Vec<Entry>,
    pub loss: Option<Loss>,
}

impl Replay {
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.loss.is_none()
    }

    #[must_use]
    pub fn statement(&self) -> String {
        match &self.loss {
            None => format!("{} entries, complete", self.entries.len()),
            Some(loss) => format!("{} entries recovered, then {loss}", self.entries.len()),
        }
    }
}

pub fn replay(path: &Path) -> Result<Replay> {
    let read = replay_from(path, 0, 0)?;
    Ok(Replay {
        entries: read.placed.into_iter().map(|placed| placed.entry).collect(),
        loss: read.loss,
    })
}

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

fn read_entry(value: &Value) -> Option<Entry> {
    let kind = EntryKind::parse(value.get("kind")?.as_text()?)?;
    let nanos = value.get("recorded_at_utc_nanos")?.as_integer()?;
    let body = value.get("body")?.clone();
    let id = super::EntryId::as_written(value.get("id")?.as_text()?);

    let recorded_at = Timestamp::from_utc_nanos(i128::from(nanos), read_offset(value));
    Some(Entry::recorded(id, kind, recorded_at, body))
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
