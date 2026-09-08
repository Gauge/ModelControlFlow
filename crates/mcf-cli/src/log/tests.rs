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

#[test]
fn a_stop_with_no_reason_says_that() {
    let stopped = summarize(&an_entry(
        EntryKind::DaemonStopped,
        Value::map([("how", Value::text("asked"))]),
    ));
    assert!(stopped.contains("asked"), "{stopped}");
    assert!(!stopped.contains("because: \n"), "{stopped}");
}

#[test]
fn the_count_says_what_it_counted() {
    assert_eq!(counted(1, None), "1 entry");
    assert_eq!(counted(4, None), "4 entries");
    assert!(counted(2, Some(EntryKind::Failure)).contains("failure"));
}

#[test]
fn the_default_is_stated() {
    assert_eq!(SHOWN, 20);
}

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

#[test]
fn a_prompt_report_says_the_floor_and_its_spread_where_there_is_one() {
    let clause = |moved: i64| Value::map([("moved_parts_per_million", Value::Integer(moved))]);
    let mut body = Value::map([
        ("floor_parts_per_million", Value::Integer(849_000)),
        (
            "prompt",
            Value::map([
                ("parts", Value::Integer(5)),
                ("characters", Value::Integer(397)),
            ]),
        ),
        (
            "conditions",
            Value::map([
                ("unit", Value::text("paragraph")),
                ("model", Value::text("dm.gguf")),
            ]),
        ),
        (
            "clauses",
            Value::List(vec![clause(869_000), clause(836_000)]),
        ),
    ]);
    let one = summarize(&an_entry(EntryKind::PromptReported, body.clone()));
    assert!(
        one.contains("floor 84.9%, 1 of 2 removed moved the answer past it"),
        "{one}"
    );
    assert!(!one.contains("every position"), "{one}");
    if let Value::Map(fields) = &mut body {
        fields.insert(
            "floor_spread".to_owned(),
            Value::map([
                ("least_parts_per_million", Value::Integer(732_000)),
                ("middle_parts_per_million", Value::Integer(843_000)),
                ("most_parts_per_million", Value::Integer(933_000)),
            ]),
        );
    }
    let spread = summarize(&an_entry(EntryKind::PromptReported, body));
    assert!(
        spread.contains("floor 84.9% (drawn at every position: 73.2% to 93.3%), 1 of 2"),
        "{spread}"
    );
}
