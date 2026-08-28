//! What a log shows, and what it refuses to hide.

use super::{SHOWN, counted, summarize};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind};
use mcf_record::json::Value;

fn an_entry(kind: EntryKind, body: Value) -> Entry {
    Entry::new(
        kind,
        Timestamp::from_utc_nanos(0, mcf_core::attested::Attested::Unknown),
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

/// **A9 in the window.** A comparison that found no difference reads as a
/// result, and says so in as many words — a reader must not have to know that
/// an absent number means *we looked and there was nothing there*.
#[test]
fn a_null_comparison_reads_as_a_result() {
    let said = summarize(&an_entry(
        EntryKind::Comparison,
        Value::map([
            ("left", Value::map([("arm", Value::text("q8_0"))])),
            ("right", Value::map([("arm", Value::text("q2_k"))])),
            (
                "outcome",
                Value::map([
                    ("kind", Value::text("same")),
                    ("resolution", Value::Integer(50_000)),
                    ("pairs", Value::Integer(37)),
                ]),
            ),
        ]),
    ));
    assert!(said.contains("q8_0") && said.contains("q2_k"), "{said}");
    assert!(said.contains("no difference as large as 5.0%"), "{said}");
    assert!(
        said.contains("a result, not a failure"),
        "the null result says what it is: {said}"
    );
    assert!(said.contains("37 paired trial(s)"), "{said}");
}

/// A comparison that was refused reads as an outcome and names what differed,
/// with no number anywhere in the line (A8, A9).
#[test]
fn a_refused_comparison_reads_as_an_outcome() {
    let said = summarize(&an_entry(
        EntryKind::Comparison,
        Value::map([
            ("left", Value::map([("arm", Value::text("q8_0"))])),
            ("right", Value::map([("arm", Value::text("q2_k"))])),
            (
                "outcome",
                Value::map([
                    ("kind", Value::text("not_comparable")),
                    ("pairs", Value::Integer(12)),
                ]),
            ),
            (
                "isolation",
                Value::map([
                    ("kind", Value::text("confounded")),
                    (
                        "differ",
                        Value::List(vec![
                            Value::text("thermal_state"),
                            Value::text("quantization"),
                        ]),
                    ),
                ]),
            ),
        ]),
    ));
    assert!(said.contains("not comparable"), "{said}");
    assert!(said.contains("thermal_state, quantization"), "{said}");
    assert!(!said.contains("differ by"), "no delta escapes: {said}");
}

/// *Does not fit here* reads as a finding, with the count of each outcome
/// (A9, §6.3).
#[test]
fn a_plan_reads_as_a_finding() {
    let variant = |outcome: &str| Value::map([("outcome", Value::text(outcome))]);
    let said = summarize(&an_entry(
        EntryKind::FitmentPlanned,
        Value::map([
            ("repository", Value::text("owner/model")),
            (
                "plan",
                Value::map([(
                    "variants",
                    Value::List(vec![
                        variant("fits"),
                        variant("fits_at_a_shorter_context"),
                        variant("does_not_fit"),
                        variant("does_not_fit"),
                    ]),
                )]),
            ),
        ]),
    ));
    assert!(said.contains("planned owner/model"), "{said}");
    assert!(said.contains("1 of 4 variant(s) fit here"), "{said}");
    assert!(said.contains("1 at a shorter context"), "{said}");
    assert!(said.contains("2 do not"), "{said}");
}

/// Neither kind is a failure, which is the whole of A9: a reader filtering the
/// log for failures must not find them, and one filtering for results must.
#[test]
fn neither_new_kind_is_a_failure() {
    for kind in [EntryKind::Comparison, EntryKind::FitmentPlanned] {
        assert_ne!(kind, EntryKind::Failure);
        assert!(
            !summarize(&an_entry(kind, Value::map::<String>([]))).contains("failure"),
            "{kind} must not read as a failure"
        );
    }
}
