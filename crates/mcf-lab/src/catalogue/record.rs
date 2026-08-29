//! Scenarios that damage the record.
//!
//! D20 makes the journal the record itself, so these are the failures that
//! decide whether months of measurements survive a bad afternoon. B62's claim —
//! *a replay reports what was lost rather than opening with a shorter history*
//! — is what they exist to demonstrate rather than assert.
//!
//! Each builds the observable and not its cause (D26): a torn line is a file
//! written short, not a power failure; an unwritable path is a directory the
//! kernel refuses, not a full disk.

use mcf_core::failure::Category;
use mcf_core::time::Timestamp;
use mcf_record::content::{Content, ContentStore};
use mcf_record::journal::{Entry, EntryKind, FORMAT_VERSION, Journal, Writer, replay};
use mcf_record::json::Value;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The record's directory cannot be written.
pub(super) const UNWRITABLE_PATH: Scenario = Scenario {
    id: "record/unwritable-path",
    produces: Category::RecordUnwritable,
    summary: "the record's directory cannot be created, so nothing can be written",
    run: unwritable_path,
};

/// The journal was written by a format this build does not read.
pub(super) const UNKNOWN_FORMAT: Scenario = Scenario {
    id: "record/unknown-format",
    produces: Category::RecordSchemaUnknown,
    summary: "a journal from a later format version is refused rather than appended to",
    run: unknown_format,
};

/// The process died between the write and the durability barrier.
pub(super) const TORN_LAST_LINE: Scenario = Scenario {
    id: "record/torn-last-line",
    produces: Category::RecordReplayIncomplete,
    summary: "the journal ends mid-line; everything before it is kept and the loss is stated",
    run: torn_last_line,
};

/// A line in the middle of the journal is not readable.
pub(super) const CORRUPT_LINE: Scenario = Scenario {
    id: "record/corrupt-line",
    produces: Category::RecordCorruptJournal,
    summary: "a line is not JSON; the replay stops there and names the line and the offset",
    run: corrupt_line,
};

/// The journal's first line is not a header.
pub(super) const HEADERLESS: Scenario = Scenario {
    id: "record/headerless",
    produces: Category::RecordCorruptJournal,
    summary: "the journal's first line is not a header, so the whole file is refused",
    run: headerless,
};

/// Content filed beside the record is there and will not be read.
pub(super) const CONTENT_UNREADABLE: Scenario = Scenario {
    id: "record/content-unreadable",
    produces: Category::RecordContentUnreadable,
    summary: "content was filed and the file will not open; the absence is a failure rather \
              than an empty answer",
    run: content_unreadable,
};

/// A content key that is not a name is refused before anything is written.
pub(super) const CONTENT_KEY_REFUSED: Scenario = Scenario {
    id: "record/content-key-refused",
    produces: Category::InternalInvariantViolated,
    summary: "a content key with a path in it is refused rather than writing outside the store",
    run: content_key_refused,
};

fn entry(sequence: u64) -> Entry {
    // A fixed moment, because §3.17 wants a failure found once to reproduce
    // exactly: a timestamp read from the wall clock would put a different
    // identifier in every run.
    const AT: Timestamp = Timestamp::from_utc_nanos(
        1_756_058_651_442_000_000,
        mcf_core::attested::Attested::Unknown,
    );
    Entry::new(
        EntryKind::MachineProfile,
        AT,
        Value::map([(
            "scenario",
            Value::Integer(i64::try_from(sequence).unwrap_or(0)),
        )]),
    )
}

/// Fills a journal with `count` entries and hands back its text.
fn a_journal_of(world: &World, count: u64) -> Result<String, Outcome> {
    let path = world.path("record.jsonl");
    let mut journal = match Journal::open(&path) {
        // Stated, so that the same scenario writes the same bytes every run
        // (§3.17): an identifier names its writer, and a distinct writer is a
        // different record every time (DEC-037).
        Ok(journal) => journal.writing_as(Writer::stated("labbed01")),
        Err(failure) => {
            return Err(Outcome::Unexpected(format!(
                "could not build one: {failure}"
            )));
        }
    };
    for sequence in 0..count {
        if let Err(failure) = journal.append(&entry(sequence)) {
            return Err(Outcome::Unexpected(format!("could not append: {failure}")));
        }
    }
    drop(journal);
    std::fs::read_to_string(&path)
        .map_err(|error| Outcome::Unexpected(format!("could not read it back: {error}")))
}

fn unwritable_path(_world: &World) -> Outcome {
    // `/proc` is a kernel filesystem that refuses a new directory on every
    // Linux machine. The scenario constructs what MCF observes — a path it
    // cannot create — rather than the cause, which would be a full or
    // read-only volume (D26).
    let path = std::path::Path::new("/proc/mcf-lab-cannot-write-here/record.jsonl");
    match Journal::open(path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("the journal opened under /proc".to_owned()),
    }
}

fn unknown_format(world: &World) -> Outcome {
    let path = world.path("record.jsonl");
    let later = Value::map([("format", Value::Integer(FORMAT_VERSION + 1))]);
    if let Err(error) = std::fs::write(&path, later.to_line() + "\n") {
        return Outcome::Unexpected(format!("could not write the journal: {error}"));
    }
    match Journal::open(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a later format was accepted".to_owned()),
    }
}

fn torn_last_line(world: &World) -> Outcome {
    let text = match a_journal_of(world, 3) {
        Ok(text) => text,
        Err(outcome) => return outcome,
    };
    let path = world.path("record.jsonl");
    // Cut mid-way through the last line: the residue of a process that died
    // between the write and the barrier.
    let cut = text.len().saturating_sub(20);
    if let Err(error) = std::fs::write(&path, text.split_at(cut).0) {
        return Outcome::Unexpected(format!("could not tear it: {error}"));
    }

    match replay(&path) {
        Err(failure) => Outcome::Unexpected(format!("the replay refused the file: {failure}")),
        Ok(replayed) => match replayed.loss {
            Some(loss) if !replayed.entries.is_empty() => Outcome::Produced(loss.failure),
            Some(_) => Outcome::Unexpected("the loss was reported and nothing was kept".to_owned()),
            None => Outcome::Unexpected("a torn journal replayed as complete".to_owned()),
        },
    }
}

fn corrupt_line(world: &World) -> Outcome {
    let text = match a_journal_of(world, 4) {
        Ok(text) => text,
        Err(outcome) => return outcome,
    };
    let path = world.path("record.jsonl");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    match lines.get_mut(3) {
        Some(line) => "\u{0}\u{0}not json\u{0}".clone_into(line),
        None => return Outcome::Unexpected("the journal is shorter than expected".to_owned()),
    }
    if let Err(error) = std::fs::write(&path, lines.join("\n") + "\n") {
        return Outcome::Unexpected(format!("could not corrupt it: {error}"));
    }

    match replay(&path) {
        Err(failure) => Outcome::Unexpected(format!("the replay refused the file: {failure}")),
        Ok(replayed) => match replayed.loss {
            Some(loss) if !replayed.entries.is_empty() => Outcome::Produced(loss.failure),
            Some(_) => Outcome::Unexpected("the loss was reported and nothing was kept".to_owned()),
            None => Outcome::Unexpected("a corrupt journal replayed as complete".to_owned()),
        },
    }
}

fn headerless(world: &World) -> Outcome {
    let path = world.path("record.jsonl");
    if let Err(error) = std::fs::write(&path, "{\"not\":\"a header\"}\n") {
        return Outcome::Unexpected(format!("could not write the journal: {error}"));
    }
    match Journal::open(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a journal with no header was accepted".to_owned()),
    }
}

/// A25's store, holding something it then cannot hand back.
///
/// The observable rather than the cause (D26): a file that will not open, not
/// a permission model or a damaged medium. It is a *directory* where content
/// should be a file, which every platform refuses to read as bytes — and the
/// distinction this scenario exists for is that *there and unreadable* must
/// not come back as `None`, because `None` is what an entry whose content was
/// never kept says, and losing that difference is A1's information loss.
fn content_unreadable(world: &World) -> Outcome {
    let store = match ContentStore::open(&world.path("content")) {
        Ok(store) => store,
        Err(failure) => {
            return Outcome::Unexpected(format!("the content store would not open: {failure}"));
        }
    };
    let key = "generated_lab_0000";
    if let Err(failure) = store.keep(key, &Content::new("what a model said")) {
        return Outcome::Unexpected(format!("content would not be kept: {failure}"));
    }
    // Replace the file with a directory of the same name: present, and not
    // readable as bytes.
    let path = world.path("content").join(key);
    if let Err(error) = std::fs::remove_file(&path) {
        return Outcome::Unexpected(format!("the kept file was not there: {error}"));
    }
    if let Err(error) = std::fs::create_dir(&path) {
        return Outcome::Unexpected(format!("the directory would not be made: {error}"));
    }
    match store.disclose_kept(key) {
        Err(failure) => Outcome::Produced(failure),
        Ok(None) => Outcome::Unexpected(
            "content that is there and unreadable came back as absent, which is what an entry \
             whose content was never kept says (A1)"
                .to_owned(),
        ),
        Ok(Some(_)) => Outcome::Unexpected("a directory was read as content".to_owned()),
    }
}

/// The traversal of §3.7, pointed inward.
///
/// Content is filed under the record's own entry identifier, which is a plain
/// name — but the store takes a `&str`, because naming an `EntryId` here would
/// give the content store a path to the record (B-161). What a `&str` cannot
/// carry is the guarantee that it *is* one, so the store checks, and this is
/// the check firing: a key holding a parent component would write wherever it
/// pointed.
fn content_key_refused(world: &World) -> Outcome {
    let store = match ContentStore::open(&world.path("content")) {
        Ok(store) => store,
        Err(failure) => {
            return Outcome::Unexpected(format!("the content store would not open: {failure}"));
        }
    };
    match store.keep("../escaped", &Content::new("somewhere else")) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected(
            "a key with a parent component was written, so content can leave its store".to_owned(),
        ),
    }
}
