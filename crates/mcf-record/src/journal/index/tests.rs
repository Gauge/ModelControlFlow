use std::io::Write as _;

use mcf_core::time::Timestamp;

use super::{Built, Index, default_path};

const HALF_RECORD: usize = 16;
use crate::journal::{Entry, EntryKind, Journal};
use crate::json::Value;

struct Place {
    root: std::path::PathBuf,
}

impl Place {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mcf-index-{name}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        std::fs::create_dir_all(&root).expect("a place to write");
        Self { root }
    }

    fn journal(&self) -> std::path::PathBuf {
        self.root.join("record.jsonl")
    }

    fn index(&self) -> std::path::PathBuf {
        default_path(&self.journal())
    }

    fn write(&self, entries: usize) {
        let mut journal = Journal::open(&self.journal()).expect("it opens");
        for sequence in 0..entries {
            journal
                .append(&Entry::new(
                    if sequence % 2 == 0 {
                        EntryKind::ArtifactAcquired
                    } else {
                        EntryKind::Failure
                    },
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
}

impl Drop for Place {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.root));
    }
}

#[test]
fn a_journal_with_no_index_gets_one() {
    let place = Place::new("fresh");
    place.write(6);

    let index = place.open();
    assert_eq!(*index.built(), Built::Fresh { entries: 6 });
    assert_eq!(index.entries().len(), 6);
    assert_eq!(index.count(EntryKind::ArtifactAcquired), 3);
    assert_eq!(index.count(EntryKind::Failure), 3);
    assert!(index.index_file_exists());
}

#[test]
fn what_the_index_points_at_is_the_entry() {
    let place = Place::new("points");
    place.write(4);
    let index = place.open();

    for (position, located) in index.entries().iter().enumerate() {
        let entry = index.read(located).expect("the entry reads back");
        assert_eq!(entry.kind(), located.kind());
        assert_eq!(
            entry.body().get("sequence").and_then(Value::as_integer),
            i64::try_from(position).ok()
        );
    }
}

#[test]
fn an_index_is_extended_rather_than_rebuilt() {
    let place = Place::new("extend");
    place.write(3);
    assert_eq!(*place.open().built(), Built::Fresh { entries: 3 });
    assert_eq!(*place.open().built(), Built::Loaded { entries: 3 });

    place.write(2);
    let index = place.open();
    assert_eq!(*index.built(), Built::Extended { had: 3, added: 2 });
    assert_eq!(index.entries().len(), 5);
}

#[test]
fn a_torn_index_record_is_dropped_and_read_again() {
    let place = Place::new("torn");
    place.write(5);
    place.open();

    let mut bytes = std::fs::read(place.index()).expect("it reads");
    bytes.truncate(bytes.len() - HALF_RECORD);
    std::fs::write(place.index(), &bytes).expect("it writes");

    let index = place.open();
    match index.built() {
        Built::Repaired {
            kept,
            discarded,
            added,
        } => {
            assert_eq!(*kept, 4);
            assert_eq!(*discarded, HALF_RECORD);
            assert_eq!(*added, 1);
        }
        other => panic!("the tear was not repaired: {other}"),
    }
    assert_eq!(index.entries().len(), 5);
}

#[test]
fn a_useless_index_is_rebuilt_with_the_reason_said() {
    for (name, damage) in [
        (
            "garbage",
            &(|path: &std::path::Path| {
                std::fs::write(path, b"this is not an index at all, not even close")
                    .expect("it writes");
            }) as &dyn Fn(&std::path::Path),
        ),
        ("version", &|path: &std::path::Path| {
            let mut bytes = std::fs::read(path).expect("it reads");
            bytes.splice(8..12, 99_u32.to_le_bytes());
            std::fs::write(path, bytes).expect("it writes");
        }),
        ("fingerprint", &|path: &std::path::Path| {
            let mut bytes = std::fs::read(path).expect("it reads");
            bytes.splice(12..13, [0xff]);
            std::fs::write(path, bytes).expect("it writes");
        }),
        ("dictionary", &|path: &std::path::Path| {
            let mut bytes = std::fs::read(path).expect("it reads");
            bytes.splice(46..47, *b"z");
            std::fs::write(path, bytes).expect("it writes");
        }),
        ("truncated header", &|path: &std::path::Path| {
            let bytes = std::fs::read(path).expect("it reads");
            std::fs::write(path, bytes.get(..20).unwrap_or_default()).expect("it writes");
        }),
    ] {
        let place = Place::new(&format!("damaged-{}", name.replace(' ', "-")));
        place.write(4);
        place.open();
        damage(&place.index());

        let index = place.open();
        match index.built() {
            Built::Rebuilt { why, entries } => {
                assert_eq!(*entries, 4, "{name}");
                assert!(!why.is_empty(), "{name}: the rebuild gave no reason");
            }
            other => panic!("{name}: a damaged index was not rebuilt: {other}"),
        }
        assert_eq!(index.entries().len(), 4, "{name}");
    }
}

#[test]
fn an_index_ahead_of_its_journal_is_thrown_away() {
    let place = Place::new("ahead");
    place.write(6);
    place.open();

    let text = std::fs::read_to_string(place.journal()).expect("it reads");
    let mut lines: Vec<&str> = text.lines().collect();
    lines.truncate(3);
    std::fs::write(place.journal(), format!("{}\n", lines.join("\n"))).expect("it writes");

    let index = place.open();
    match index.built() {
        Built::Rebuilt { why, entries } => {
            assert_eq!(*entries, 2);
            assert!(why.contains("covers"), "{why}");
        }
        other => panic!("an index ahead of its journal was kept: {other}"),
    }
}

#[test]
fn the_index_never_covers_past_a_loss() {
    let place = Place::new("loss");
    place.write(4);
    {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(place.journal())
            .expect("it opens");
        file.write_all(b"{this is not json}\n").expect("it writes");
    }
    place.write(2);

    let index = place.open();
    assert_eq!(index.entries().len(), 4, "it indexed past the damage");
    let loss = index.loss().expect("the loss is reported");
    assert_eq!(loss.line, 6, "the header is line one");
    assert!(index.covers() < std::fs::metadata(place.journal()).expect("it stats").len());

    let again = place.open();
    assert!(again.loss().is_some());
    assert_eq!(again.entries().len(), 4);
}

#[test]
fn the_index_answers_the_questions_the_surfaces_ask() {
    let place = Place::new("queries");
    place.write(10);
    let index = place.open();

    let latest = index.latest(Some(EntryKind::Failure), 3);
    assert_eq!(latest.len(), 3);
    assert!(latest.iter().all(|one| one.kind() == EntryKind::Failure));
    let ordered: Vec<i64> = latest.iter().map(super::Located::at_utc_nanos).collect();
    let mut sorted = ordered.clone();
    sorted.sort_unstable();
    assert_eq!(
        ordered, sorted,
        "the last three are shown in the order written"
    );

    let all = index.entries();
    let from = all.get(5).expect("a sixth entry").at_utc_nanos();
    assert!(index.since(from).len() >= 5);
}
