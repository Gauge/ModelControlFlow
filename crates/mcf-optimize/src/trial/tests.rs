use super::{blocks, body, Asked, Endpoint, Said};
use crate::corpus::Set;
use crate::dial::{Dial, Step};
use crate::reading::Ending;

fn asked(dial: Dial, step: Step) -> Asked {
    let Some(set) = Set::numbered(1) else {
        panic!("set one is compiled in");
    };
    Asked {
        set,
        dial,
        step,
        repeat: 0,
        thinking: Some(true),
        effort: Some("medium".to_owned()),
        ceiling: 40_000,
    }
}

#[test]
fn a_request_field_dial_travels_in_the_body() {
    let held = body(&asked(Dial::Temperature, Step::Thousandths(600)));
    assert!(held.get("temperature").is_some(), "temperature is sent");
    assert!(held.get("top_p").is_none(), "only the swept field is sent");
}

#[test]
fn an_engine_flag_dial_never_travels_in_the_body() {
    let held = body(&asked(Dial::ThinkingBudget, Step::Whole(2048)));
    for field in ["temperature", "top_p", "top_k", "reasoning_budget"] {
        assert!(held.get(field).is_none(), "{field} is a launch flag, not a field");
    }
}

#[test]
fn the_body_asks_for_a_stream_and_refuses_the_prompt_cache() {
    let held = body(&asked(Dial::Temperature, Step::Thousandths(0)));
    assert_eq!(held.get("stream"), Some(&mcf_record::json::Value::Bool(true)));
    assert_eq!(
        held.get("cache_prompt"),
        Some(&mcf_record::json::Value::Bool(false)),
        "a cached prefix makes two trials incomparable"
    );
}

#[test]
fn the_thinking_switches_ride_in_the_template_arguments() {
    let held = body(&asked(Dial::Temperature, Step::Thousandths(0)));
    let Some(switches) = held.get("chat_template_kwargs") else {
        panic!("the switches are sent");
    };
    assert_eq!(switches.get("reasoning_effort").and_then(mcf_record::json::Value::as_text), Some("medium"));
}

#[test]
fn fenced_blocks_are_lifted_in_order_and_unfenced_prose_is_left_behind() {
    let said = "### SOLUTION 1\n```python\nfirst = 1\n```\nchatter\n### SOLUTION 2\n```\nsecond = \
                2\n```\n";
    assert_eq!(blocks(said), vec!["first = 1".to_owned(), "second = 2".to_owned()]);
}

#[test]
fn an_unclosed_fence_yields_the_blocks_before_it_rather_than_failing() {
    let said = "```python\ngood = 1\n```\n```python\ntruncated";
    assert_eq!(blocks(said), vec!["good = 1".to_owned()]);
}

#[test]
fn an_endpoint_defaults_to_loopback_with_no_key() {
    let held = Endpoint::default();
    assert!(held.key.is_none(), "a key is supplied, never assumed");
    assert!(held.patience.as_secs() >= 3600, "a long answer is not cut off");
}

#[test]
fn a_said_carries_why_it_ended_when_it_did_not_simply_answer() {
    let held = Said {
        answer: String::new(),
        produced: 0,
        ending: Ending::Failed,
        why: Some("the endpoint produced nothing".to_owned()),
    };
    assert!(held.why.is_some(), "a failure says what happened");
}
