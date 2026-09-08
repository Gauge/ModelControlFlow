use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use super::replay::{Loss, read_entry_at, replay_from};
use super::{Entry, EntryKind};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::index");

const MAGIC: [u8; 8] = *b"MCFINDEX";

pub const FORMAT_VERSION: u32 = 1;

const RECORD: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Located {
    kind: EntryKind,
    at_utc_nanos: i64,
    line: u32,
    byte_offset: u64,
    byte_length: u32,
}

impl Located {
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    #[must_use]
    pub const fn at_utc_nanos(&self) -> i64 {
        self.at_utc_nanos
    }

    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    #[must_use]
    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    #[must_use]
    pub const fn byte_length(&self) -> u32 {
        self.byte_length
    }

    #[must_use]
    pub const fn ends_at(&self) -> u64 {
        self.byte_offset.saturating_add(self.byte_length as u64)
    }

    fn to_bytes(self) -> [u8; RECORD] {
        let mut bytes = [0_u8; RECORD];
        bytes[0..8].copy_from_slice(&self.at_utc_nanos.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.byte_offset.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.byte_length.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.line.to_le_bytes());
        let kind = u16::try_from(position_of(self.kind)).unwrap_or(u16::MAX);
        bytes[24..26].copy_from_slice(&kind.to_le_bytes());
        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let kind = u16::from_le_bytes(read_two(bytes, 24)?);
        Some(Self {
            at_utc_nanos: i64::from_le_bytes(read_eight(bytes, 0)?),
            byte_offset: u64::from_le_bytes(read_eight(bytes, 8)?),
            byte_length: u32::from_le_bytes(read_four(bytes, 16)?),
            line: u32::from_le_bytes(read_four(bytes, 20)?),
            kind: *EntryKind::ALL.get(usize::from(kind))?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Built {
    Fresh {
        entries: usize,
    },
    Loaded {
        entries: usize,
    },
    Extended {
        had: usize,
        added: usize,
    },
    Repaired {
        kept: usize,
        discarded: usize,
        added: usize,
    },
    Rebuilt {
        why: String,
        entries: usize,
    },
}

impl core::fmt::Display for Built {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Fresh { entries } => write!(f, "built from the journal: {entries} entries"),
            Self::Loaded { entries } => write!(f, "read as it stood: {entries} entries"),
            Self::Extended { had, added } => {
                write!(f, "extended: {had} entries already, {added} read since")
            }
            Self::Repaired {
                kept,
                discarded,
                added,
            } => write!(
                f,
                "repaired: {kept} whole records kept, {discarded} bytes of a torn record dropped, \
                 {added} entries read again"
            ),
            Self::Rebuilt { why, entries } => {
                write!(f, "rebuilt ({why}): {entries} entries")
            }
        }
    }
}

#[derive(Debug)]
pub struct Index {
    journal: PathBuf,
    path: PathBuf,
    located: Vec<Located>,
    built: Built,
    loss: Option<Loss>,
}

impl Index {
    pub fn over(journal: &Path, index: &Path) -> Result<Self> {
        let fingerprint = fingerprint(journal)?;

        let (kept, why) = match read_existing(index, &fingerprint) {
            Ok(read) => read,
            Err(why) => (Kept::nothing(), Some(why)),
        };

        let journal_bytes = std::fs::metadata(journal)
            .map_err(|error| unreadable(journal, &error))?
            .len();
        let (kept, why) = match kept.located.last() {
            Some(last) if last.ends_at() > journal_bytes => (
                Kept::nothing(),
                Some(format!(
                    "the journal is {journal_bytes} bytes and the index covers {}",
                    last.ends_at()
                )),
            ),
            Some(_) | None => (kept, why),
        };

        let from = kept.located.last().map_or(0, Located::ends_at);
        let lines_before = kept.located.last().map_or(0, |last| last.line as usize);
        let read = replay_from(journal, from, lines_before)?;

        let added = read.placed.len();
        let mut located = kept.located;
        let had = located.len();
        for placed in &read.placed {
            located.push(Located {
                kind: placed.entry.kind(),
                at_utc_nanos: i64::try_from(placed.entry.recorded_at().utc_nanos())
                    .unwrap_or(i64::MAX),
                line: u32::try_from(placed.line).unwrap_or(u32::MAX),
                byte_offset: placed.byte_offset,
                byte_length: placed.byte_length,
            });
        }

        let built = if kept.discarded > 0 {
            Built::Repaired {
                kept: had,
                discarded: kept.discarded,
                added,
            }
        } else if let Some(why) = &why {
            Built::Rebuilt {
                why: why.clone(),
                entries: located.len(),
            }
        } else if kept.was_absent {
            Built::Fresh {
                entries: located.len(),
            }
        } else if added == 0 {
            Built::Loaded {
                entries: located.len(),
            }
        } else {
            Built::Extended { had, added }
        };

        let mine = Self {
            journal: journal.to_path_buf(),
            path: index.to_path_buf(),
            located,
            built,
            loss: read.loss,
        };
        mine.write(
            had == 0 || kept.discarded > 0 || why.is_some(),
            &fingerprint,
        )?;
        Ok(mine)
    }

    #[must_use]
    pub const fn built(&self) -> &Built {
        &self.built
    }

    #[must_use]
    pub const fn loss(&self) -> Option<&Loss> {
        self.loss.as_ref()
    }

    #[must_use]
    pub fn entries(&self) -> &[Located] {
        &self.located
    }

    #[must_use]
    pub fn covers(&self) -> u64 {
        self.located.last().map_or(0, Located::ends_at)
    }

    #[must_use]
    pub fn count_matching(&self, kind: Option<EntryKind>) -> usize {
        match kind {
            Some(kind) => self.count(kind),
            None => self.located.len(),
        }
    }

    #[must_use]
    pub fn count(&self, kind: EntryKind) -> usize {
        self.located
            .iter()
            .filter(|located| located.kind == kind)
            .count()
    }

    #[must_use]
    pub fn latest(&self, kind: Option<EntryKind>, wanted: usize) -> Vec<Located> {
        let mut found: Vec<Located> = self
            .located
            .iter()
            .rev()
            .filter(|located| kind.is_none_or(|kind| located.kind == kind))
            .take(wanted)
            .copied()
            .collect();
        found.reverse();
        found
    }

    #[must_use]
    pub fn since(&self, utc_nanos: i64) -> Vec<Located> {
        self.located
            .iter()
            .filter(|located| located.at_utc_nanos >= utc_nanos)
            .copied()
            .collect()
    }

    pub fn read(&self, located: &Located) -> Result<Entry> {
        read_entry_at(&self.journal, located.byte_offset, located.byte_length)
    }

    #[must_use]
    pub fn index_file_exists(&self) -> bool {
        self.path.exists()
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, from_scratch: bool, fingerprint: &[u8; 32]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(&self.path, &error))?;
        }
        let mut file = if from_scratch {
            let mut fresh = std::fs::File::create(&self.path)
                .map_err(|error| unwritable(&self.path, &error))?;
            fresh
                .write_all(&header(fingerprint))
                .map_err(|error| unwritable(&self.path, &error))?;
            fresh
        } else {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .map_err(|error| unwritable(&self.path, &error))?
        };

        let already = if from_scratch {
            0
        } else {
            let covered = file
                .metadata()
                .map_err(|error| unwritable(&self.path, &error))?
                .len()
                .saturating_sub(HEADER as u64);
            usize::try_from(covered).unwrap_or(0).saturating_div(RECORD)
        };
        let mut bytes = Vec::with_capacity(self.located.len().saturating_sub(already) * RECORD);
        for located in self.located.iter().skip(already) {
            bytes.extend_from_slice(&located.to_bytes());
        }
        file.write_all(&bytes)
            .map_err(|error| unwritable(&self.path, &error))
    }
}

#[must_use]
pub fn default_path(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().unwrap_or_default().to_os_string();
    name.push(".index");
    journal.with_file_name(name)
}

struct Kept {
    located: Vec<Located>,
    discarded: usize,
    was_absent: bool,
}

impl Kept {
    const fn nothing() -> Self {
        Self {
            located: Vec::new(),
            discarded: 0,
            was_absent: false,
        }
    }
}

const HEADER: usize = 8 + 4 + 32 + 2 + KIND_NAMES;

const KIND_NAMES: usize = EntryKind::ALL.len() * NAME;
const NAME: usize = 32;

fn header(fingerprint: &[u8; 32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(fingerprint);
    bytes.extend_from_slice(
        &u16::try_from(EntryKind::ALL.len())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    for kind in EntryKind::ALL {
        let mut name = [0_u8; NAME];
        let text = kind.as_str().as_bytes();
        let end = text.len().min(NAME);
        if let (Some(slot), Some(source)) = (name.get_mut(..end), text.get(..end)) {
            slot.copy_from_slice(source);
        }
        bytes.extend_from_slice(&name);
    }
    bytes
}

fn read_existing(
    path: &Path,
    fingerprint: &[u8; 32],
) -> core::result::Result<(Kept, Option<String>), String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                Kept {
                    located: Vec::new(),
                    discarded: 0,
                    was_absent: true,
                },
                None,
            ));
        }
        Err(error) => return Err(format!("the index could not be read: {error}")),
    };

    if bytes.get(..8) != Some(&MAGIC) {
        return Err("the file does not begin as an index does".to_owned());
    }
    let version = read_four(&bytes, 8).map(u32::from_le_bytes);
    if version != Some(FORMAT_VERSION) {
        return Err(format!(
            "the index names format {} and this build writes {FORMAT_VERSION}",
            version.map_or_else(|| "nothing".to_owned(), |found| found.to_string())
        ));
    }
    if bytes.get(12..44) != Some(fingerprint.as_slice()) {
        return Err("the index was built over a different journal".to_owned());
    }
    let Some(names) = bytes.get(46..HEADER) else {
        return Err("the index has no kind dictionary".to_owned());
    };
    if names != header(fingerprint).get(46..HEADER).unwrap_or_default() {
        return Err(
            "the index was written by a build that knew different kinds of entry".to_owned(),
        );
    }

    let Some(area) = bytes.get(HEADER..) else {
        return Err("the index stops before its records".to_owned());
    };
    let whole = area.len().saturating_div(RECORD);
    let discarded = area.len().saturating_sub(whole.saturating_mul(RECORD));
    let mut located = Vec::with_capacity(whole);
    for at in 0..whole {
        let start = at.saturating_mul(RECORD);
        let Some(record) = area.get(start..start.saturating_add(RECORD)) else {
            break;
        };
        match Located::from_bytes(record) {
            Some(one) => located.push(one),
            None => {
                return Err(format!(
                    "record {at} of the index names a kind of entry that is not in its dictionary"
                ));
            }
        }
    }
    Ok((
        Kept {
            located,
            discarded,
            was_absent: false,
        },
        None,
    ))
}

fn position_of(kind: EntryKind) -> usize {
    EntryKind::ALL
        .iter()
        .position(|one| *one == kind)
        .unwrap_or(usize::MAX)
}

fn fingerprint(journal: &Path) -> Result<[u8; 32]> {
    let mut file = std::fs::File::open(journal).map_err(|error| unreadable(journal, &error))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| unreadable(journal, &error))?;
    let mut head = [0_u8; 4096];
    let read = file
        .read(&mut head)
        .map_err(|error| unreadable(journal, &error))?;
    let line = head
        .get(..read)
        .unwrap_or_default()
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if line.is_empty() {
        return Err(Failure::new(
            Category::RecordCorruptJournal,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "the journal has no header line to identify it by",
        )
        .with_context("path", journal.display().to_string()));
    }
    Ok(*sha256(line).bytes())
}

fn read_two(bytes: &[u8], at: usize) -> Option<[u8; 2]> {
    bytes.get(at..at.saturating_add(2))?.try_into().ok()
}

fn read_four(bytes: &[u8], at: usize) -> Option<[u8; 4]> {
    bytes.get(at..at.saturating_add(4))?.try_into().ok()
}

fn read_eight(bytes: &[u8], at: usize) -> Option<[u8; 8]> {
    bytes.get(at..at.saturating_add(8))?.try_into().ok()
}

fn unreadable(path: &Path, error: &std::io::Error) -> Failure {
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

fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the index could not be written",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

#[cfg(test)]
mod tests;
