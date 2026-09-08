#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::io::Write as _;
use std::path::{Path, PathBuf};

use mcf_core::time::Timestamp;
use mcf_record::journal::index::{self, Built};
use mcf_record::journal::{Entry, EntryKind, Index, Journal};
use mcf_record::json::Value;

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

type Damage = dyn Fn(&Path);

fn truncate(path: &Path, bytes: u64) {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("it opens");
    file.set_len(bytes).expect("it truncates");
}

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

        let index = record.open();
        assert!(
            matches!(index.built(), Built::Loaded { .. }),
            "{name}: the rebuilt index was not usable at the next open: {}",
            index.built()
        );
    }
}

#[test]
fn a_damaged_journal_keeps_everything_written_before_the_damage() {
    let record = Record::new("header-only");
    Journal::open(&record.journal()).expect("it opens");
    let index = record.open();
    assert!(index.entries().is_empty());
    assert!(
        index.loss().is_none(),
        "an empty record is not a damaged one"
    );
    drop(record);

    let record = Record::new("mid-entry");
    record.append(5);
    let whole = std::fs::metadata(record.journal()).expect("it stats").len();
    truncate(&record.journal(), whole.saturating_sub(30));
    let index = record.open();
    assert_eq!(index.entries().len(), 4, "the four whole entries are kept");
    let loss = index.loss().expect("the tear is reported");
    assert!(loss.bytes_unread > 0, "{loss}");
    assert_eq!(record.sequences(), vec![0, 1, 2, 3]);

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
