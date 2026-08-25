//! Tests for the bundle.
//!
//! What is being checked is that a bundle survives leaving the machine and
//! coming back — and, more to the point, that one which did *not* survive says
//! so rather than reading as a shorter but valid record.

use std::path::PathBuf;

use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::time::Timestamp;

use super::{FORMAT_VERSION, Kind, read, write};
use crate::journal::{Entry, EntryKind, Journal};
use crate::json::Value;

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-export-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        let _made = std::fs::create_dir_all(&path);
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A journal holding `count` entries.
    fn journal_of(&self, count: u64) -> PathBuf {
        let path = self.path("record.jsonl");
        let mut journal = Journal::open(&path).expect("a journal opens");
        for sequence in 0..count {
            journal
                .append(&Entry::new(
                    EntryKind::MachineProfile,
                    AT,
                    sequence,
                    Value::map([("n", Value::Integer(i64::try_from(sequence).unwrap_or(0)))]),
                ))
                .expect("an entry appends");
        }
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_bundle_round_trips() {
    let scratch = Scratch::new("round-trip");
    let journal = scratch.journal_of(5);
    let bundle = scratch.path("out.mcf");

    let written = write(&journal, &bundle, Kind::Export).expect("the bundle writes");
    assert_eq!(written.entries, 5);

    let (kind, manifest, entries) = read(&bundle).expect("the bundle reads");
    assert_eq!(kind, Kind::Export);
    assert_eq!(manifest, written);
    assert_eq!(entries.len(), 5);
    assert_eq!(
        entries
            .first()
            .and_then(|e| e.get("kind"))
            .and_then(Value::as_text),
        Some("machine_profile")
    );
}

/// The entries are copied verbatim rather than re-encoded. A bundle whose
/// digest depended on the version that wrote it would defeat §XV: a
/// configuration found elsewhere has to be checkable here.
#[test]
fn the_entries_are_carried_unchanged() {
    let scratch = Scratch::new("verbatim");
    let journal = scratch.journal_of(3);
    let bundle = scratch.path("out.mcf");
    write(&journal, &bundle, Kind::Export).expect("writes");

    let source = std::fs::read_to_string(&journal).expect("readable");
    let carried = std::fs::read_to_string(&bundle).expect("readable");
    for line in source.lines().skip(1) {
        assert!(
            carried.contains(line),
            "an entry was rewritten on the way out"
        );
    }
}

/// A bundle that arrived damaged is not a bundle with fewer rows. Reading it as
/// one would be B62's silent shortening, arriving by post.
#[test]
fn a_damaged_bundle_is_refused_rather_than_read_short() {
    let scratch = Scratch::new("damaged");
    let journal = scratch.journal_of(4);
    let bundle = scratch.path("out.mcf");
    write(&journal, &bundle, Kind::Export).expect("writes");

    // One entry removed in transit — the shape a truncated transfer has.
    let text = std::fs::read_to_string(&bundle).expect("readable");
    let shortened: Vec<&str> = text.lines().take(text.lines().count() - 1).collect();
    std::fs::write(&bundle, shortened.join("\n") + "\n").expect("writable");

    let failure = read(&bundle).expect_err("a short bundle does not verify");
    assert_eq!(failure.category(), Category::ArtifactCorrupt);
    assert_eq!(failure.context_value("stated_entries"), Some("4"));
    assert!(
        failure
            .context_value("detail")
            .is_some_and(|d| d.contains("3 entries"))
    );
}

/// And one whose contents were altered rather than truncated.
#[test]
fn an_altered_bundle_is_refused() {
    let scratch = Scratch::new("altered");
    let journal = scratch.journal_of(3);
    let bundle = scratch.path("out.mcf");
    write(&journal, &bundle, Kind::Export).expect("writes");

    let text = std::fs::read_to_string(&bundle).expect("readable");
    std::fs::write(&bundle, text.replace("machine_profile", "self_cost")).expect("writable");

    let failure = read(&bundle).expect_err("an altered bundle does not verify");
    assert_eq!(failure.category(), Category::ArtifactCorrupt);
}

/// §7.30: a bundle written by a format this build does not read is refused,
/// and says which format it was.
#[test]
fn a_bundle_from_another_format_is_refused_by_name() {
    let scratch = Scratch::new("format");
    let bundle = scratch.path("out.mcf");
    let header = Value::map([
        ("format", Value::Integer(FORMAT_VERSION + 1)),
        ("kind", Value::text("export")),
    ]);
    std::fs::write(&bundle, header.to_line() + "\n").expect("writable");

    let failure = read(&bundle).expect_err("a later format is refused");
    assert_eq!(failure.category(), Category::ExchangeSchemaUnreadable);
    assert!(
        failure
            .context_value("detail")
            .is_some_and(|d| d.contains("bundle format 2"))
    );
}

/// A4 and B62: a journal with a loss exports what could be read, and the bundle
/// says the source was not complete rather than looking like a whole one.
#[test]
fn a_bundle_from_a_damaged_journal_says_the_source_was_incomplete() {
    let scratch = Scratch::new("partial");
    let journal = scratch.journal_of(4);

    // Tear the last line, which is what a crash mid-append leaves.
    let text = std::fs::read_to_string(&journal).expect("readable");
    std::fs::write(&journal, text.split_at(text.len() - 20).0).expect("writable");

    let bundle = scratch.path("out.mcf");
    let manifest = write(&journal, &bundle, Kind::Export).expect("writes what it could read");
    assert_eq!(manifest.entries, 3, "the readable entries are carried");

    let carried = std::fs::read_to_string(&bundle).expect("readable");
    assert!(
        carried.contains("\"source_was_complete\":false"),
        "the bundle does not say its source was incomplete"
    );
    // And it still verifies as a bundle: it is an honest partial, not a
    // damaged whole.
    let (_, _, entries) = read(&bundle).expect("a partial bundle is still a valid bundle");
    assert_eq!(entries.len(), 3);
}

/// A25, stated as well as structural: a reader of a bundle is told what is not
/// in it, and the guarantee is that this module reads the journal and the
/// journal is not the content store.
#[test]
fn a_bundle_says_it_holds_no_user_content() {
    let scratch = Scratch::new("content");
    let journal = scratch.journal_of(1);
    let bundle = scratch.path("out.mcf");
    write(&journal, &bundle, Kind::Export).expect("writes");

    let carried = std::fs::read_to_string(&bundle).expect("readable");
    assert!(carried.contains("\"contains_user_content\":false"));
}

/// The three kinds are one format. A reader that understands one understands
/// all three, which is what "one mechanism" means.
#[test]
fn every_kind_writes_the_same_format() {
    let scratch = Scratch::new("kinds");
    let journal = scratch.journal_of(2);
    for kind in [Kind::Export, Kind::Contribution, Kind::ReproBundle] {
        let bundle = scratch.path(&format!("{}.mcf", kind.as_str()));
        write(&journal, &bundle, kind).expect("writes");
        let (read_back, _, entries) = read(&bundle).expect("reads");
        assert_eq!(read_back, kind);
        assert_eq!(entries.len(), 2);
    }
    assert_eq!(Kind::parse("export"), Some(Kind::Export));
    assert_eq!(Kind::parse("something_later"), None);
}

/// An empty record exports an empty bundle rather than failing. A9: a null
/// result is a result, and "there is nothing recorded yet" is one.
#[test]
fn an_empty_record_exports_an_empty_bundle() {
    let scratch = Scratch::new("empty");
    let journal = scratch.journal_of(0);
    let bundle = scratch.path("out.mcf");

    let manifest = write(&journal, &bundle, Kind::Export).expect("writes");
    assert_eq!(manifest.entries, 0);
    let (_, _, entries) = read(&bundle).expect("reads");
    assert!(entries.is_empty());
}
