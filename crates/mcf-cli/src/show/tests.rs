use mcf_record::json::Value;

use super::{scalar, unfolded};

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

#[test]
fn an_identifier_the_record_does_not_hold_is_refused() {
    let response = super::run("comparison_1970-01-01T00-00-00Z_deadbeef_9999");
    assert!(!response.served, "{}", response.text);
}
