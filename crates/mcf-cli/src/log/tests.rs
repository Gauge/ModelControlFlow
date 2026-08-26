//! What a log shows, and what it refuses to hide.

use super::{SHOWN, counted, summarize};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind};
use mcf_record::json::Value;

fn an_entry(kind: EntryKind, body: Value) -> Entry {
    Entry::new(
        kind,
        Timestamp::from_utc_nanos(0, mcf_core::attested::Attested::Unknown),
        0,
        body,
    )
}

/// Every kind gets its own sentence, with the field a reader wants first.
#[test]
fn each_kind_says_the_interesting_thing_first() {
    let acquired = summarize(&an_entry(
        EntryKind::ArtifactAcquired,
        Value::map([
            ("repository", Value::text("owner/model")),
            ("file", Value::text("model.gguf")),
            (
                "verification",
                Value::map([("state", Value::text("verified"))]),
            ),
        ]),
    ));
    assert!(acquired.contains("owner/model:model.gguf"), "{acquired}");
    assert!(acquired.contains("verified"), "{acquired}");

    let removed = summarize(&an_entry(
        EntryKind::ArtifactRemoved,
        Value::map([
            ("reason", Value::text("superseded")),
            ("removed", Value::List(vec![Value::Null, Value::Null])),
        ]),
    ));
    assert!(removed.contains("2 file(s)"), "{removed}");
    assert!(removed.contains("superseded"), "{removed}");

    let stopped = summarize(&an_entry(
        EntryKind::DaemonStopped,
        Value::map([
            ("how", Value::text("asked")),
            ("reason", Value::text("the test")),
        ]),
    ));
    assert!(stopped.contains("because: the test"), "{stopped}");

    let failed = summarize(&an_entry(
        EntryKind::Failure,
        Value::map([
            ("category", Value::text("hub.unreachable")),
            ("attribution", Value::text("machine")),
            ("detail", Value::text("nothing answered")),
        ]),
    ));
    assert!(failed.contains("hub.unreachable"), "{failed}");
    assert!(failed.contains("nothing answered"), "{failed}");
}

/// A stop with no reason says so rather than showing a blank: *no reason given*
/// is a fact about the stop (A7).
#[test]
fn a_stop_with_no_reason_says_that() {
    let stopped = summarize(&an_entry(
        EntryKind::DaemonStopped,
        Value::map([("how", Value::text("asked"))]),
    ));
    assert!(stopped.contains("asked"), "{stopped}");
    assert!(!stopped.contains("because: \n"), "{stopped}");
}

/// The count says what it counted, so a filtered log cannot be mistaken for the
/// whole record.
#[test]
fn the_count_says_what_it_counted() {
    assert_eq!(counted(1, None), "1 entry");
    assert_eq!(counted(4, None), "4 entries");
    assert!(counted(2, Some(EntryKind::Failure)).contains("failure"));
}

/// The default is a stated number rather than everything, because a command
/// that printed a machine's whole life is one people pipe to `tail` — which is
/// MCF choosing twenty with less said about it (§3.15).
#[test]
fn the_default_is_stated() {
    assert_eq!(SHOWN, 20);
}
