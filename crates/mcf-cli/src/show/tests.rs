//! What expanding a statement has to get right.

use mcf_record::json::Value;

use super::{scalar, unfolded};

/// **A question asked and unanswered is not a question nobody asked.** `null`
/// is printed rather than skipped: dropping it would erase the difference,
/// which is what A7 exists to keep (§3.4).
#[test]
fn an_unanswered_condition_is_printed_as_unanswered() {
    let held = Value::map([
        ("thermal_state", Value::Null),
        ("quantization", Value::text("Q4_K_M")),
    ]);
    let lines = unfolded(&held, 2);
    assert_eq!(lines.len(), 2, "both fields appear: {lines:#?}");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("thermal_state") && line.contains("not answered")),
        "an unanswered condition says so: {lines:#?}"
    );
    assert_eq!(scalar(&Value::Null), "— not answered");
}

/// The pairs are numbered, so *the seventh pair* is a thing a reader can point
/// at.
#[test]
fn a_list_of_pairs_is_numbered() {
    let pair = |ns: i64| Value::map([("left_ns", Value::Integer(ns))]);
    let held = Value::map([("pairs", Value::List(vec![pair(1), pair(2), pair(3)]))]);
    let lines = unfolded(&held, 0);
    for at in 0..3 {
        assert!(
            lines.iter().any(|line| line.trim() == format!("#{at}")),
            "pair {at} is numbered: {lines:#?}"
        );
    }
}

/// Nothing is summarized: every scalar the record holds appears, and a nested
/// value is unfolded rather than printed as one line of JSON.
#[test]
fn every_recorded_value_appears() {
    let held = Value::map([
        (
            "outcome",
            Value::map([
                ("kind", Value::text("differ")),
                ("difference", Value::Integer(262_000)),
            ]),
        ),
        (
            "reuse",
            Value::text("cold: every trial loaded the model for itself"),
        ),
    ]);
    let lines = unfolded(&held, 0).join("\n");
    for expected in ["kind", "differ", "difference", "262000", "reuse", "cold:"] {
        assert!(
            lines.contains(expected),
            "`{expected}` is missing from:\n{lines}"
        );
    }
    assert!(
        !lines.contains(r#"{"difference""#),
        "a nested value is unfolded rather than printed as a line of JSON:\n{lines}"
    );
}

/// A statement with no evidence behind it is refused rather than invented: an
/// identifier the record does not hold gets a refusal that says how to find
/// one.
#[test]
fn an_identifier_the_record_does_not_hold_is_refused() {
    let response = super::run("comparison_1970-01-01T00-00-00Z_deadbeef_9999");
    // The record may or may not exist on the machine running the suite; either
    // way, asking for an entry that is not there must not be served.
    assert!(!response.served, "{}", response.text);
}
