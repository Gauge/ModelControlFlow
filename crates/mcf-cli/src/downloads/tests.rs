use super::{About, one_line, queue_lines};
use mcf_record::json::Value;

fn a_row(id: i64, state: &str, arrived: i64, whole: i64) -> Value {
    Value::map([
        ("id", Value::Integer(id)),
        ("reference", Value::text("owner/model")),
        ("file", Value::text("a-model.gguf")),
        ("state", Value::text(state)),
        ("arrived_bytes", Value::Integer(arrived)),
        ("whole_bytes", Value::Integer(whole)),
        ("part", Value::Integer(1)),
        ("parts", Value::Integer(1)),
    ])
}

/// The one shape every answer about the queue has: the rows at the top level, beside
/// whatever else that particular answer says.
fn a_queue(rows: Vec<Value>) -> Value {
    Value::map([
        ("transfers", Value::List(rows)),
        ("at_once", Value::Integer(2)),
    ])
}

#[test]
fn every_queued_transfer_is_listed() {
    let shown = queue_lines(
        &a_queue(vec![
            a_row(1, "fetching", 500, 1_000),
            a_row(2, "queued", 0, 0),
        ]),
        None,
    );
    assert_eq!(shown.len(), 2, "{shown:#?}");
}

#[test]
fn one_transfer_can_be_asked_about_on_its_own() {
    let shown = queue_lines(
        &a_queue(vec![
            a_row(1, "fetching", 500, 1_000),
            a_row(2, "paused", 10, 1_000),
        ]),
        Some(2),
    );
    assert_eq!(shown.len(), 1, "{shown:#?}");
    assert!(shown.join("").contains("paused"), "{shown:#?}");
}

#[test]
fn an_empty_queue_says_how_to_put_something_in_it() {
    let shown = queue_lines(&a_queue(Vec::new()), None).join("\n");
    assert!(shown.contains("nothing is queued"), "{shown}");
    assert!(
        shown.contains("downloads add"),
        "an empty list that does not say what to do next is a dead end: {shown}"
    );
}

#[test]
fn how_far_along_is_only_said_when_there_is_a_whole_to_say_it_of() {
    let unknown = one_line(&a_row(1, "queued", 0, 0));
    assert!(
        !unknown.contains('%'),
        "a share of an unknown whole is a number made up: {unknown}"
    );
    let known = one_line(&a_row(1, "fetching", 250, 1_000));
    assert!(known.contains("25%"), "{known}");
}

#[test]
fn a_refusal_is_carried_onto_the_line_that_failed() {
    let mut row = a_row(1, "failed", 10, 1_000);
    if let Value::Map(fields) = &mut row {
        let _put = fields.insert(
            "why".to_owned(),
            Value::map([
                ("category", Value::text("hub_unreachable")),
                ("detail", Value::text("the hub did not answer")),
            ]),
        );
    }
    let said = one_line(&row);
    assert!(
        said.contains("did not answer"),
        "a failure the operator cannot read is one they cannot act on: {said}"
    );
}

#[test]
fn each_thing_that_can_be_asked_says_what_it_did() {
    for about in [About::Pause, About::Resume, About::Cancel] {
        assert!(!about.word().is_empty());
    }
}
