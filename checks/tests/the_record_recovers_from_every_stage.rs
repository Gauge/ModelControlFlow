//! Damage the record at every stage of its life, and check that nothing which
//! was written whole is ever lost quietly (B-300, D20, B62, A2, A4).
//!
//! **What B-300 asks for.** *A scenario corrupts the database at every
//! lifecycle stage and the record rebuilds or states what it could not
//! recover.* There are two files to damage, and they are not equal: the journal
//! is the record (D20) and the index is derived from it. So the claim splits in
//! two, and both halves are checked here:
//!
//! * **The index may be destroyed at any moment and cost nothing but time.**
//!   Deleted, truncated mid-record, filled with garbage, left behind by an
//!   older build, describing a journal that no longer exists — every one of
//!   those is a rebuild that says why, never a failure and never a wrong
//!   answer.
//! * **The journal may be damaged and costs exactly what was damaged.**
//!   Everything written whole before the damage is still readable, and the
//!   damage itself is reported with the line, the offset and the bytes — the
//!   short-history failure B62 names.
//!
//! **The stages are the moments a process can die**, built as the residue they
//! leave rather than as their causes (D26): after the header and before the
//! first entry; mid-entry; between entries; mid-index-record; and after
//! everything, with the two files disagreeing.
//!
//! **Why this is a check and not a laboratory scenario.** The lab's catalogue
//! is a table of scenarios that each produce one classified failure (A13, D26).
//! Most of what is asserted here produces *no* failure — that is the claim —
//! so it belongs with the checks that assert a property of the whole system.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::io::Write as _;
use std::path::{Path, PathBuf};

use mcf_core::time::Timestamp;
use mcf_record::journal::index::{self, Built};
use mcf_record::journal::{Entry, EntryKind, Index, Journal};
use mcf_record::json::Value;

/// A record of its own, in a directory nothing else touches (B19).
struct Record {
    root: PathBuf,
}

impl Record {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("mcf-record-stages-{name}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        std::fs::create_dir_all(&root).expect("a place to write");
        Self { root }
    }

    fn journal(&self) -> PathBuf {
        self.root.join("record.jsonl")
    }

    fn index(&self) -> PathBuf {
        index::default_path(&self.journal())
    }

    /// Appends `count` entries the way MCF does, durably, one line each.
    fn append(&self, count: usize) {
        let mut journal = Journal::open(&self.journal()).expect("it opens");
        for sequence in 0..count {
            journal
                .append(&Entry::new(
                    EntryKind::MachineProfile,
                    Timestamp::now(),
                    Value::map([(
                        "sequence",
                        Value::Integer(i64::try_from(sequence).unwrap_or(-1)),
                    )]),
                ))
                .expect("it appends");
        }
    }

    fn open(&self) -> Index {
        Index::over(&self.journal(), &self.index()).expect("the index opens")
    }

    /// Every sequence number the record can be made to give up.
    fn sequences(&self) -> Vec<i64> {
        let index = self.open();
        index
            .entries()
            .iter()
            .map(|located| {
                index
                    .read(located)
                    .expect("the entry reads back")
                    .body()
                    .get("sequence")
                    .and_then(Value::as_integer)
                    .expect("a sequence")
            })
            .collect()
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.root));
    }
}

/// One way to ruin a file.
type Damage = dyn Fn(&Path);

fn truncate(path: &Path, bytes: u64) {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("it opens");
    file.set_len(bytes).expect("it truncates");
}

/// The index is a convenience and never a source of truth: whatever is done to
/// it, the answers are the journal's.
#[test]
fn the_index_can_be_destroyed_at_any_stage_and_costs_only_time() {
    let damages: [(&str, &Damage); 5] = [
        ("deleted", &|path| {
            std::fs::remove_file(path).expect("it goes");
        }),
        ("truncated mid-record", &|path| {
            let len = std::fs::metadata(path).expect("it stats").len();
            truncate(path, len.saturating_sub(11));
        }),
        ("emptied", &|path| {
            std::fs::write(path, b"").expect("it writes");
        }),
        ("garbage", &|path| {
            std::fs::write(path, vec![0xa5_u8; 500]).expect("it writes");
        }),
        ("half a header", &|path| {
            let bytes = std::fs::read(path).expect("it reads");
            std::fs::write(path, bytes.get(..30).unwrap_or_default()).expect("it writes");
        }),
    ];

    for (name, damage) in damages {
        let record = Record::new(&name.replace(' ', "-"));
        record.append(8);
        let before = record.sequences();
        assert_eq!(before.len(), 8, "{name}");

        damage(&record.index());

        let after = record.sequences();
        assert_eq!(after, before, "{name}: the record answered differently");

        // And the open said what it cost, rather than paying it in silence.
        let index = record.open();
        assert!(
            matches!(index.built(), Built::Loaded { .. }),
            "{name}: the rebuilt index was not usable at the next open: {}",
            index.built()
        );
    }
}

/// Every stage of a journal's life, damaged: what was written whole survives,
/// and what was not is named.
#[test]
fn a_damaged_journal_keeps_everything_written_before_the_damage() {
    // The header is there and nothing else: a process that died between
    // creating the record and its first event.
    let record = Record::new("header-only");
    Journal::open(&record.journal()).expect("it opens");
    let index = record.open();
    assert!(index.entries().is_empty());
    assert!(
        index.loss().is_none(),
        "an empty record is not a damaged one"
    );
    drop(record);

    // Mid-entry: the residue of a process that died between the write and the
    // durability barrier.
    let record = Record::new("mid-entry");
    record.append(5);
    let whole = std::fs::metadata(record.journal()).expect("it stats").len();
    truncate(&record.journal(), whole.saturating_sub(30));
    let index = record.open();
    assert_eq!(index.entries().len(), 4, "the four whole entries are kept");
    let loss = index.loss().expect("the tear is reported");
    assert!(loss.bytes_unread > 0, "{loss}");
    assert_eq!(record.sequences(), vec![0, 1, 2, 3]);

    // Between entries, with a line in the middle made unreadable: everything
    // before it is a real history (A4) and the rest is stated as lost.
    let record = Record::new("bad-line");
    record.append(6);
    let text = std::fs::read_to_string(record.journal()).expect("it reads");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    "not json at all".clone_into(lines.get_mut(4).expect("a fifth line"));
    std::fs::write(record.journal(), lines.join("\n") + "\n").expect("it writes");
    let index = record.open();
    assert_eq!(index.entries().len(), 3);
    assert_eq!(index.loss().expect("the loss is reported").line, 5);
    assert_eq!(record.sequences(), vec![0, 1, 2]);
}

/// The damage is reported at every open, not only at the one that found it.
///
/// An index that covered the entries on both sides of a hole would answer
/// questions cleanly about a history with a piece missing, which is exactly the
/// silent shorter history B62 forbids.
#[test]
fn a_loss_is_reported_again_every_time() {
    let record = Record::new("again");
    record.append(3);
    {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(record.journal())
            .expect("it opens");
        file.write_all(b"{ not json\n").expect("it writes");
    }
    record.append(3);

    for attempt in 0..3 {
        let index = record.open();
        assert!(
            index.loss().is_some(),
            "attempt {attempt}: the loss was forgotten"
        );
        assert_eq!(index.entries().len(), 3, "attempt {attempt}");
    }
}

/// A journal that shrank is not a journal the index describes.
///
/// This is the one case where the index holds something the journal cannot
/// confirm, and the rule is the same as everywhere else in MCF: the evidence
/// wins and the derivation is thrown away.
#[test]
fn an_index_describing_a_journal_that_changed_is_discarded() {
    let record = Record::new("changed");
    record.append(9);
    record.open();

    let text = std::fs::read_to_string(record.journal()).expect("it reads");
    let kept: Vec<&str> = text.lines().take(4).collect();
    std::fs::write(record.journal(), kept.join("\n") + "\n").expect("it writes");

    let index = record.open();
    assert!(
        matches!(index.built(), Built::Rebuilt { .. }),
        "the stale index was kept: {}",
        index.built()
    );
    assert_eq!(record.sequences(), vec![0, 1, 2]);
}

/// What a surface asks for is what the journal says, whether or not an index
/// exists — the property that makes the index safe to delete.
#[test]
fn the_same_questions_get_the_same_answers_without_an_index() {
    let record = Record::new("agreement");
    record.append(20);

    let with = record.open();
    let latest: Vec<i64> = with
        .latest(Some(EntryKind::MachineProfile), 5)
        .iter()
        .map(|located| {
            with.read(located)
                .expect("it reads")
                .body()
                .get("sequence")
                .and_then(Value::as_integer)
                .expect("a sequence")
        })
        .collect();
    assert_eq!(latest, vec![15, 16, 17, 18, 19]);

    let replayed = mcf_record::journal::replay(&record.journal()).expect("it replays");
    let from_the_journal: Vec<i64> = replayed
        .entries
        .iter()
        .rev()
        .take(5)
        .rev()
        .map(|entry| {
            entry
                .body()
                .get("sequence")
                .and_then(Value::as_integer)
                .expect("a sequence")
        })
        .collect();
    assert_eq!(
        latest, from_the_journal,
        "the index and the journal disagree"
    );
}
