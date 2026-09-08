mod anomaly;
mod entry;
pub mod index;
mod replay;

pub use anomaly::{Reading, TOLERANCE, between as clock_anomaly_between};
pub use entry::{Entry, EntryId, EntryKind, Writer};
pub use index::{Built, Index, Located};
pub use replay::{Loss, Placed, Placement, Replay};

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::{Clock as _, SystemClock, Timestamp};

use crate::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-record::journal");

pub const FORMAT_VERSION: i64 = 1;

#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    file: File,
    appended: u64,
    last_read: Option<Reading>,
    recording_anomaly: bool,
    writer: Writer,
}

#[derive(Debug)]
pub struct Appended {
    pub id: EntryId,
    pub anomaly: Option<Failure>,
}

impl Appended {
    #[must_use]
    pub const fn saw_a_clock_anomaly(&self) -> bool {
        self.anomaly.is_some()
    }
}

impl Journal {
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
            last_read: None,
            recording_anomaly: false,
            writer: Writer::distinct(),
        };
        if fresh {
            journal.write_line(&header())?;
        } else {
            journal.check_header()?;
        }
        Ok(journal)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn writer(&self) -> &Writer {
        &self.writer
    }

    #[must_use]
    pub fn writing_as(mut self, writer: Writer) -> Self {
        self.writer = writer;
        self
    }

    #[must_use]
    pub const fn appended(&self) -> u64 {
        self.appended
    }

    pub fn append(&mut self, entry: &Entry) -> Result<Appended> {
        let now = Reading {
            wall: Timestamp::now(),
            monotonic: SystemClock.now(),
        };
        let anomaly = match self.last_read {
            Some(earlier) if !self.recording_anomaly => anomaly::between(earlier, now),
            Some(_) | None => None,
        };
        self.last_read = Some(now);

        let stamped = entry
            .clone()
            .stamped(entry.identify(&self.writer, self.appended));
        let id = match stamped.id() {
            Some(id) => id.clone(),
            None => EntryId::as_written(""),
        };
        self.write_line(&stamped.to_value())?;
        self.appended = self.appended.saturating_add(1);

        if let Some(failure) = &anomaly {
            self.recording_anomaly = true;
            let recorded = self.append(&Entry::new(
                EntryKind::Failure,
                entry.recorded_at(),
                crate::encode::failure(failure),
            ));
            self.recording_anomaly = false;
            recorded?;
        }

        Ok(Appended { id, anomaly })
    }

    fn write_line(&mut self, value: &Value) -> Result<()> {
        let mut line = value.to_line();
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .map_err(|error| unwritable(&self.path, &error))?;
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

fn header() -> Value {
    Value::map([
        ("format", Value::Integer(FORMAT_VERSION)),
        (
            "created_by",
            crate::encode::build_identity(BuildIdentity::current()),
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

pub use replay::{replay, replay_from};

#[cfg(test)]
mod tests;
