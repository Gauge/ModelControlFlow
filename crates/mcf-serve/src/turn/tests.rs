use super::*;

#[test]
fn a_turn_survives_the_record() {
    let turn = Turn {
        thinking: Some(false),
        effort: Some("low".to_owned()),
        system: Some("Be brief.".to_owned()),
    };
    assert_eq!(Turn::from_value(&turn.to_value()), Some(turn));
    let nothing = Turn::default();
    assert_eq!(Turn::from_value(&nothing.to_value()), Some(nothing));
    assert_eq!(Turn::from_value(&Value::Null), None);
}

#[test]
fn the_switches_are_the_templates_own_words() {
    let turn = Turn {
        thinking: Some(true),
        effort: Some("medium".to_owned()),
        system: None,
    };
    assert_eq!(
        turn.switches(),
        Value::map([
            ("enable_thinking", Value::Bool(true)),
            ("reasoning_effort", Value::text("medium")),
        ])
    );
    assert_eq!(Turn::default().switches(), Value::map::<&str>([]));
    assert!(!Turn::default().asks_anything());
    assert!(turn.asks_anything());
}

#[test]
fn what_was_asked_is_said_in_words() {
    assert_eq!(Turn::default().said(), "nothing switched");
    let turn = Turn {
        thinking: Some(false),
        effort: None,
        system: Some("x".to_owned()),
    };
    assert_eq!(turn.said(), "thinking off, a system turn");
}

#[test]
fn a_system_turn_goes_first_and_the_prompt_has_a_place() {
    let turn = Turn {
        thinking: None,
        effort: None,
        system: Some("Be brief.".to_owned()),
    };
    let messages = turn.messages();
    let listed = messages.as_list().unwrap_or_default();
    assert_eq!(listed.len(), 2);
    assert_eq!(
        listed
            .first()
            .and_then(|m| m.get("role"))
            .and_then(Value::as_text),
        Some("system")
    );
    assert_eq!(
        listed
            .get(1)
            .and_then(|m| m.get("content"))
            .and_then(Value::as_text),
        Some(PLACE)
    );
    assert!(PLACE.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn the_account_of_what_came_before_counts_and_does_not_quote() {
    let before = BeforeTheAnswer {
        inside: "<think>".to_owned(),
        opened_by: OpenedBy::Turn,
        tokens: 12,
        closed: true,
        text: "working".to_owned(),
        answer: "21".to_owned(),
    };
    let value = before.to_value();
    assert_eq!(value.get("tokens"), Some(&Value::Integer(12)));
    assert_eq!(
        value.get("opened_by").and_then(Value::as_text),
        Some("the turn")
    );
    assert_eq!(value.get("closed"), Some(&Value::Bool(true)));
    assert_eq!(value.get("text"), None);
    assert_eq!(value.get("answer"), None);
}

/// A run of identifiers is found where it last sits, and nowhere it does
/// not sit at all — which is how the answer's opener is found among the
/// channels a model wrote before it (B-457).
#[test]
fn a_run_is_found_where_it_last_sits() {
    let held = [1, 2, 3, 9, 9, 1, 2, 3, 7, 7];
    assert_eq!(super::last_run(&held, &[1, 2, 3]), Some(5));
    assert_eq!(super::last_run(&held, &[7, 7]), Some(8));
    assert_eq!(super::last_run(&held, &[9, 9, 9]), None);
    assert_eq!(super::last_run(&held, &[]), None);
    // A run longer than what holds it is not in it.
    assert_eq!(super::last_run(&[1], &[1, 2]), None);
    // The whole of it is a run of itself.
    assert_eq!(super::last_run(&held, &held), Some(0));
}
