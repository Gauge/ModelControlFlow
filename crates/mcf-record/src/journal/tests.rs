//! Tests for the journal.
//!
//! B62's claim is that a damaged journal reports what it lost rather than
//! opening with a shorter history, so most of what is tested here is damage:
//! the file truncated mid-line, a line replaced with rubbish, a header from a
//! version this build does not read. B19 keeps all of it hermetic — a temporary
//! directory, no network, no accelerator.

use std::path::PathBuf;

use mcf_core::failure::Category;
use mcf_core::time::Timestamp;

use super::{Entry, EntryKind, FORMAT_VERSION, Journal, Writer, default_path, replay};
use crate::json::Value;

const AT: Timestamp = Timestamp::from_utc_nanos(
    1_756_058_651_442_000_000,
    mcf_core::attested::Attested::Unknown,
);

/// A directory nothing else is using, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-journal-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        Self(path)
    }

    fn journal(&self) -> PathBuf {
        self.0.join("record.jsonl")
    }
}

impl Drop for Scratch {
    /// A27's habit: what a test creates, it removes.
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

fn entry(_which: u64) -> Entry {
    Entry::new(
        EntryKind::MachineProfile,
        AT,
        Value::map([("cores", Value::Integer(16))]),
    )
}

#[test]
fn a_new_journal_gets_a_header_and_takes_entries() {
    let scratch = Scratch::new("new");
    let mut journal = Journal::open(&scratch.journal()).expect("a fresh journal opens");
    journal.append(&entry(0)).expect("an entry appends");
    journal.append(&entry(1)).expect("a second entry appends");
    assert_eq!(journal.appended(), 2);

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert!(replayed.is_complete(), "{}", replayed.statement());
    assert_eq!(replayed.entries.len(), 2);
    assert_eq!(replayed.entries[0].kind(), EntryKind::MachineProfile);
    assert_eq!(
        replayed.entries[0]
            .body()
            .get("cores")
            .and_then(Value::as_integer),
        Some(16)
    );
}

/// The record is append-only: opening an existing journal adds to it and never
/// truncates it. The property is worth a test of its own because the way to
/// break it is one word in an options builder.
#[test]
fn reopening_appends_and_never_truncates() {
    let scratch = Scratch::new("append");
    {
        let mut journal = Journal::open(&scratch.journal()).expect("opens");
        journal.append(&entry(0)).expect("appends");
    }
    {
        let mut journal = Journal::open(&scratch.journal()).expect("reopens");
        journal.append(&entry(1)).expect("appends");
    }
    let replayed = replay(&scratch.journal()).expect("replays");
    assert_eq!(replayed.entries.len(), 2);
    assert!(replayed.is_complete());
}

/// B62: a crash mid-append leaves an unterminated line, and the replay keeps
/// everything before it and says precisely what it could not read.
#[test]
fn a_torn_last_line_is_reported_and_the_rest_is_kept() {
    let scratch = Scratch::new("torn");
    {
        let mut journal = Journal::open(&scratch.journal()).expect("opens");
        for sequence in 0..3 {
            journal.append(&entry(sequence)).expect("appends");
        }
    }
    // Cut the file mid-way through its last line, which is what a process
    // dying between the write and the barrier leaves behind.
    let text = std::fs::read_to_string(scratch.journal()).expect("readable");
    let cut = text.len() - 20;
    std::fs::write(scratch.journal(), text.split_at(cut).0).expect("writable");

    let replayed = replay(&scratch.journal()).expect("replays");
    assert!(!replayed.is_complete());
    assert_eq!(replayed.entries.len(), 2, "{}", replayed.statement());
    let loss = replayed.loss.expect("the loss is reported");
    assert_eq!(loss.failure.category(), Category::RecordReplayIncomplete);
    assert!(loss.bytes_unread > 0);
    assert!(loss.line >= 3, "{loss}");
}

/// A line that is not JSON at all — the shape a partial-sector write or a
/// filesystem repair leaves — is reported at its line and offset, with
/// everything before it kept.
#[test]
fn rubbish_in_the_middle_is_reported_at_its_line() {
    let scratch = Scratch::new("rubbish");
    {
        let mut journal = Journal::open(&scratch.journal()).expect("opens");
        for sequence in 0..4 {
            journal.append(&entry(sequence)).expect("appends");
        }
    }
    let text = std::fs::read_to_string(scratch.journal()).expect("readable");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines[3] = "\u{0}\u{0}not json\u{0}".to_owned();
    std::fs::write(scratch.journal(), lines.join("\n") + "\n").expect("writable");

    let replayed = replay(&scratch.journal()).expect("replays");
    // Lines 2 and 3 are entries; line 1 is the header.
    assert_eq!(replayed.entries.len(), 2);
    assert!(replayed.statement().contains("2 entries recovered"));
    let loss = replayed.loss.expect("the loss is reported");
    assert_eq!(loss.line, 4);
    assert_eq!(loss.failure.category(), Category::RecordCorruptJournal);
}

/// §7.30: a journal written by a format this build does not read is refused,
/// not appended to. Appending would make the file unreadable to both versions.
#[test]
fn a_journal_from_an_unknown_format_is_refused() {
    let scratch = Scratch::new("format");
    std::fs::create_dir_all(&scratch.0).expect("creatable");
    let future = Value::map([("format", Value::Integer(FORMAT_VERSION + 1))]);
    std::fs::write(scratch.journal(), future.to_line() + "\n").expect("writable");

    let refused = Journal::open(&scratch.journal()).expect_err("an unknown format is refused");
    assert_eq!(refused.category(), Category::RecordSchemaUnknown);
    assert_eq!(refused.context_value("this_format"), Some("1"));

    let replayed = replay(&scratch.journal()).expect_err("and it does not replay either");
    assert_eq!(replayed.category(), Category::RecordSchemaUnknown);
}

/// A file whose first line is not a header at all is corruption of the whole
/// file rather than a loss partway through one, and there is no partial history
/// to hand back.
#[test]
fn a_journal_with_no_header_is_refused() {
    let scratch = Scratch::new("headerless");
    std::fs::create_dir_all(&scratch.0).expect("creatable");
    std::fs::write(scratch.journal(), "{\"not\":\"a header\"}\n").expect("writable");
    let refused = Journal::open(&scratch.journal()).expect_err("refused");
    assert_eq!(refused.category(), Category::RecordCorruptJournal);
}

/// A2: a path that cannot be written names the path and the reason rather than
/// failing quietly or panicking.
#[test]
fn an_unwritable_path_is_classified() {
    let refused = Journal::open(std::path::Path::new(
        "/proc/mcf-cannot-write-here/record.jsonl",
    ))
    .expect_err("a path under /proc is not writable");
    assert_eq!(refused.category(), Category::RecordUnwritable);
    assert!(refused.context_value("os_error").is_some());
}

/// An entry carries its envelope, and the identifier reads as the kind, the
/// moment and the writer — which is what makes a journal legible without a tool
/// and what makes two writers' entries tell apart (DEC-037).
#[test]
fn an_identifier_names_its_kind_its_moment_and_its_writer() {
    let scratch = std::env::temp_dir().join(format!("mcf-id-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&scratch));
    let mut journal = Journal::open(&scratch.join("record.jsonl"))
        .expect("a journal opens")
        .writing_as(Writer::stated("abcd1234"));
    let written = journal
        .append(&Entry::new(EntryKind::MachineProfile, AT, Value::Null))
        .expect("it appends");
    assert_eq!(
        written.id.as_str(),
        "machine_profile_2025-08-24T18-04-11Z_abcd1234_0000"
    );
    drop(std::fs::remove_dir_all(&scratch));
}

/// Two writers on one record never mint the same identifier, however close
/// together they write — which is the whole of what F13 left broken.
#[test]
fn two_writers_never_mint_the_same_identifier() {
    let scratch = std::env::temp_dir().join(format!("mcf-two-writers-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&scratch));
    let path = scratch.join("record.jsonl");

    let mut ids = std::collections::BTreeSet::new();
    for _writer in 0..4 {
        let mut journal = Journal::open(&path).expect("a journal opens");
        for _entry in 0..8 {
            // The same kind and the same moment for every one of them: the
            // worst case, and the one two programs recording the same event
            // actually produce.
            let written = journal
                .append(&Entry::new(EntryKind::SelfCost, AT, Value::Null))
                .expect("it appends");
            assert!(
                ids.insert(written.id.as_str().to_owned()),
                "two entries were given one identifier: {}",
                written.id.as_str()
            );
        }
    }
    assert_eq!(ids.len(), 32);
    drop(std::fs::remove_dir_all(&scratch));
}

/// Kinds round-trip and an unknown one is `None`, never a fallback (§7.30).
#[test]
fn kinds_round_trip_and_an_unknown_one_is_not_guessed() {
    for kind in EntryKind::ALL {
        assert_eq!(EntryKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(EntryKind::parse("something_later"), None);
}

/// A line whose kind this build does not know is unreadable *as an entry*,
/// which is the honest answer rather than a half-understood record admitted
/// into a history.
#[test]
fn a_line_with_an_unknown_kind_is_a_loss_not_a_guess() {
    let scratch = Scratch::new("unknown-kind");
    {
        let mut journal = Journal::open(&scratch.journal()).expect("opens");
        journal.append(&entry(0)).expect("appends");
    }
    let text = std::fs::read_to_string(scratch.journal()).expect("readable");
    let replaced = text.replace("machine_profile", "something_later");
    std::fs::write(scratch.journal(), replaced).expect("writable");

    let outcome = replay(&scratch.journal()).expect("replays");
    assert!(outcome.entries.is_empty());
    let loss = outcome.loss.expect("reported");
    assert_eq!(loss.failure.category(), Category::RecordCorruptJournal);
}

/// The statement always says both halves, so a reader is not trained to skim
/// past the line that matters.
#[test]
fn the_statement_always_says_what_was_recovered() {
    let scratch = Scratch::new("statement");
    {
        let mut journal = Journal::open(&scratch.journal()).expect("opens");
        journal.append(&entry(0)).expect("appends");
    }
    let replayed = replay(&scratch.journal()).expect("replays");
    assert_eq!(replayed.statement(), "1 entries, complete");
}

/// A7: MCF does not invent a place to write the user's evidence. With neither
/// variable set there is no default path.
#[test]
fn the_default_path_is_not_invented() {
    let path = default_path();
    let nothing_is_set =
        std::env::var_os("XDG_DATA_HOME").is_none() && std::env::var_os("HOME").is_none();
    if nothing_is_set {
        assert!(path.is_none());
    } else {
        let path = path.expect("a base directory is set");
        assert!(path.ends_with("mcf/record.jsonl"), "{}", path.display());
    }
}
