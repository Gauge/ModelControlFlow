//! The derived index: where every entry is, without reading the history to
//! find out (B-300, D20, D6, §3.3).
//!
//! **The journal is the record and this is not.** D20 settles the shape:
//! entries are appended to a journal, and the queryable thing over it is
//! *derived* — discardable, rebuildable, and never a second place where a fact
//! lives. Everything here can be deleted with `rm` and the next open rebuilds
//! it from the journal; nothing here is consulted for what happened, only for
//! **where it is written**.
//!
//! **Why it exists, in numbers.** A replay parses every line: 10 µs an entry on
//! the machine that measured it, so a million-entry record costs 8.5 s at every
//! daemon start and 390 MiB of parsed values held to answer *what happened
//! recently* ([findings.md](../../../../doc/findings.md) F14). The index is
//! 32 bytes an entry, is read as bytes rather than parsed, and turns *the last
//! twenty acquisitions* into a scan of a few tens of megabytes and twenty
//! seeks. That is the whole of its justification, and B15 asks for exactly that
//! before weight is admitted.
//!
//! **It is append-only too, and for the same reason.** A crash during an
//! extension leaves a torn final record, which is arithmetic to detect —
//! the record area is a multiple of a fixed width or it is not — and the
//! remedy is to drop the incomplete tail and read the journal forward from
//! where the whole records stop. There is no mutable header: what the index
//! covers is read from its last record, so no update ever rewrites a byte that
//! was already correct.
//!
//! **Nothing here is durable, deliberately.** The journal pays for a barrier
//! per entry because it is the record (D24 budgets it); an index that did the
//! same would double that cost to protect something a rebuild reproduces
//! exactly. A2 is satisfied by saying what happened instead: every open reports
//! whether it loaded, extended, repaired or rebuilt, and why.
//!
//! **A damaged journal stops the index where the damage is.** The index never
//! covers past a [`Loss`], so the loss is found again at the next open and
//! reported again rather than being indexed around — a history with a hole in
//! the middle that queried cleanly would be the silent failure B62 names.

use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use super::replay::{Loss, read_entry_at, replay_from};
use super::{Entry, EntryKind};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::index");

/// What an index file starts with, so that a file which is not one is not read
/// as one.
const MAGIC: [u8; 8] = *b"MCFINDEX";

/// The index format's version.
///
/// Separate from the journal's: this is a derived file, and changing its layout
/// costs a rebuild rather than a migration. A file naming any other version is
/// rebuilt rather than read (C5's stability obligation does not reach a
/// derivative nobody can cite).
pub const FORMAT_VERSION: u32 = 1;

/// How many bytes one located entry takes.
///
/// Fixed, because a torn write is then detectable by division: a record area
/// whose length is not a multiple of this ends in a record that was being
/// written when the process died.
const RECORD: usize = 32;

/// Where one entry is, and the little about it a query needs before reading it.
///
/// Everything here is what an index can honestly hold: *where* the entry is,
/// *when* it was recorded, and *what kind* it was. The entry's body is not
/// here, because a copy of the body is a second place for a fact to live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Located {
    kind: EntryKind,
    at_utc_nanos: i64,
    line: u32,
    byte_offset: u64,
    byte_length: u32,
}

impl Located {
    /// What kind of event it was.
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    /// When it was recorded, in nanoseconds since the epoch, UTC.
    #[must_use]
    pub const fn at_utc_nanos(&self) -> i64 {
        self.at_utc_nanos
    }

    /// Which line of the journal it is.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Where its line begins in the journal.
    #[must_use]
    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    /// How long its line is, terminator included.
    #[must_use]
    pub const fn byte_length(&self) -> u32 {
        self.byte_length
    }

    /// The byte after its line.
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

/// How this index came to be what it is.
///
/// Always reported, never inferred: an index that quietly rebuilt itself is an
/// index whose cost nobody can see, and a rebuild is the most expensive thing
/// that happens on an open.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Built {
    /// There was no index; one was built from the whole journal.
    Fresh {
        /// How many entries it covers.
        entries: usize,
    },
    /// The index was current and was read as it stood.
    Loaded {
        /// How many entries it covers.
        entries: usize,
    },
    /// The index was behind the journal and the difference was read.
    Extended {
        /// How many entries it already covered.
        had: usize,
        /// How many the journal had added since.
        added: usize,
    },
    /// The index ended mid-record and the incomplete tail was dropped.
    Repaired {
        /// How many whole records survived.
        kept: usize,
        /// How many bytes of a partial record were discarded.
        discarded: usize,
        /// How many entries were read from the journal afterwards.
        added: usize,
    },
    /// The index could not be used and was built again from the journal.
    Rebuilt {
        /// Why, in a sentence: what was wrong with the file that was there.
        why: String,
        /// How many entries the new one covers.
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

/// A derived index over one journal.
#[derive(Debug)]
pub struct Index {
    journal: PathBuf,
    path: PathBuf,
    located: Vec<Located>,
    built: Built,
    loss: Option<Loss>,
}

impl Index {
    /// Opens the index over a journal, building, extending or rebuilding it as
    /// the two files require.
    ///
    /// The journal is the authority in every case: nothing an index file says
    /// survives disagreeing with it.
    ///
    /// # Errors
    ///
    /// Whatever reading the *journal* produces — `record.unwritable`,
    /// `record.corrupt.journal`, `record.schema.unknown`. A damaged **index**
    /// is never an error, because a derived file that cannot be read is a file
    /// to rebuild rather than a failure to report (A4); which of those
    /// happened is in [`Index::built`].
    pub fn over(journal: &Path, index: &Path) -> Result<Self> {
        let fingerprint = fingerprint(journal)?;

        let (kept, why) = match read_existing(index, &fingerprint) {
            Ok(read) => read,
            Err(why) => (Kept::nothing(), Some(why)),
        };

        // The journal is shorter than what the index claims to cover: it was
        // truncated, replaced or restored from somewhere older. Nothing about
        // the index can be trusted against a file it does not describe.
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
                // The line the reading found, rather than a count: a journal
                // with a blank line in it would otherwise shift every number
                // after it, and a line number that is nearly right is worse
                // than none (A6's habit at the smallest scale).
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

    /// How this index came to be what it is.
    #[must_use]
    pub const fn built(&self) -> &Built {
        &self.built
    }

    /// What the journal would not give up, if anything.
    ///
    /// The index stops where a loss starts, so this is both *what is missing*
    /// and *why the index goes no further* (B62).
    #[must_use]
    pub const fn loss(&self) -> Option<&Loss> {
        self.loss.as_ref()
    }

    /// Every entry the index covers, in the order they were written.
    #[must_use]
    pub fn entries(&self) -> &[Located] {
        &self.located
    }

    /// How many bytes of the journal are covered.
    #[must_use]
    pub fn covers(&self) -> u64 {
        self.located.last().map_or(0, Located::ends_at)
    }

    /// How many entries match, of one kind or of every kind.
    #[must_use]
    pub fn count_matching(&self, kind: Option<EntryKind>) -> usize {
        match kind {
            Some(kind) => self.count(kind),
            None => self.located.len(),
        }
    }

    /// How many entries of one kind there are.
    #[must_use]
    pub fn count(&self, kind: EntryKind) -> usize {
        self.located
            .iter()
            .filter(|located| located.kind == kind)
            .count()
    }

    /// The last `wanted` entries, of one kind or of every kind.
    ///
    /// In the order they were written, so that a surface printing them reads
    /// forwards even though the question was asked backwards.
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

    /// Every entry recorded at or after a moment.
    #[must_use]
    pub fn since(&self, utc_nanos: i64) -> Vec<Located> {
        self.located
            .iter()
            .filter(|located| located.at_utc_nanos >= utc_nanos)
            .copied()
            .collect()
    }

    /// Reads one entry out of the journal.
    ///
    /// This is where the index stops being an answer and becomes a pointer: the
    /// entry comes from the journal, parsed from the bytes the index says it
    /// occupies (D20 — the record is the journal).
    ///
    /// # Errors
    ///
    /// `record.unwritable` when the journal cannot be read, and
    /// `record.corrupt.journal` when the bytes at that offset are not an entry
    /// — which means the index is describing a journal that changed under it,
    /// and the remedy is a rebuild.
    pub fn read(&self, located: &Located) -> Result<Entry> {
        read_entry_at(&self.journal, located.byte_offset, located.byte_length)
    }

    /// Whether the index file is on the disk.
    ///
    /// For a test to assert what a rebuild costs; nothing in MCF branches on
    /// it, because the answer to a missing index is to build one.
    #[must_use]
    pub fn index_file_exists(&self) -> bool {
        self.path.exists()
    }

    /// Where this index lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes the index out.
    ///
    /// Appends where it can and rewrites where it cannot, and never barriers:
    /// see the module's note on durability.
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

/// Where an index lives when the journal is at `journal`.
///
/// Beside it, named after it: the two belong together, and an index somewhere
/// else is one that outlives the journal it describes.
#[must_use]
pub fn default_path(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().unwrap_or_default().to_os_string();
    name.push(".index");
    journal.with_file_name(name)
}

/// What survived reading an existing index file.
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

/// How long the fixed part of an index file is.
const HEADER: usize = 8 + 4 + 32 + 2 + KIND_NAMES;

/// How many bytes the kind dictionary takes.
///
/// The names this build knows, written into the file so that a later build
/// whose list differs rebuilds rather than misreading a number as a kind. The
/// dictionary is fixed-width for the same reason the records are: a file whose
/// length is arithmetic is a file a torn write cannot hide in.
const KIND_NAMES: usize = 8 * NAME;
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

/// Reads an index file, or says in one sentence why it cannot be used.
///
/// Every refusal here is a rebuild rather than a failure, so each one is a
/// string a person can read rather than a classified failure: the operator is
/// being told what the open cost, not what went wrong with the record.
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

/// Which position in `EntryKind::ALL` a kind has.
fn position_of(kind: EntryKind) -> usize {
    EntryKind::ALL
        .iter()
        .position(|one| *one == kind)
        .unwrap_or(usize::MAX)
}

/// What identifies the journal this index is over.
///
/// The header line, digested: it carries the format version and the build that
/// created the journal, and it is the one part of a journal that never changes
/// after the file exists. An index whose fingerprint does not match is an index
/// over some other file, whatever its name says.
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
