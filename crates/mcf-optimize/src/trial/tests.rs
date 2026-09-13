use super::{Asked, Endpoint, Said, blocks, body};
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
        assert!(
            held.get(field).is_none(),
            "{field} is a launch flag, not a field"
        );
    }
}

#[test]
fn the_body_asks_for_a_stream_and_refuses_the_prompt_cache() {
    let held = body(&asked(Dial::Temperature, Step::Thousandths(0)));
    assert_eq!(
        held.get("stream"),
        Some(&mcf_record::json::Value::Bool(true))
    );
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
    assert_eq!(
        switches
            .get("reasoning_effort")
            .and_then(mcf_record::json::Value::as_text),
        Some("medium")
    );
}

#[test]
fn fenced_blocks_are_lifted_in_order_and_unfenced_prose_is_left_behind() {
    let said = "### SOLUTION 1\n```python\nfirst = 1\n```\nchatter\n### SOLUTION 2\n```\nsecond = \
                2\n```\n";
    assert_eq!(
        blocks(said),
        vec!["first = 1".to_owned(), "second = 2".to_owned()]
    );
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
    assert!(
        held.patience.as_secs() >= 3600,
        "a long answer is not cut off"
    );
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

#[test]
fn an_empty_reply_says_the_endpoint_said_nothing_rather_than_just_failed() {
    let said = super::what_came_back("");
    assert!(said.contains("said nothing at all"), "{said}");
}

#[test]
fn a_refusal_carries_the_status_line_and_what_the_body_said() {
    let said = super::what_came_back(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\n\r\n\
         {\"error\":{\"message\":\"Loading model\",\"code\":503}}",
    );
    assert!(said.contains("503"), "{said}");
    assert!(
        said.contains("Loading model"),
        "the reason the engine gave is the reason to show: {said}"
    );
}

#[test]
fn a_model_that_ran_out_of_room_before_answering_is_told_apart_from_a_broken_endpoint() {
    let said = super::what_came_back(
        "HTTP/1.1 200 OK\r\n\r\ndata: {\"choices\":[{\"finish_reason\":\"length\",\
         \"delta\":{}}]}",
    );
    assert!(
        said.contains("no room left to answer"),
        "a thinking budget that leaves nothing for the answer is a finding, not a fault: \
         {said}"
    );
}

#[test]
fn an_answer_that_is_not_http_at_all_says_so() {
    let said = super::what_came_back("hello there");
    assert!(said.contains("not HTTP"), "{said}");
}

#[test]
fn a_long_body_is_cut_down_to_something_a_person_can_read() {
    let body = "x".repeat(5000);
    let said = super::what_came_back(&format!("HTTP/1.1 500 Oops\r\n\r\n{body}"));
    assert!(said.len() < 500, "{}", said.len());
    assert!(said.contains("500"));
}

#[test]
fn a_reply_body_is_flattened_onto_one_line() {
    let said = super::what_came_back("HTTP/1.1 400 Bad\r\n\r\n{\n  \"error\": \"no\"\n}");
    assert!(!said.contains('\n'), "{said}");
}

#[test]
fn nothing_is_ready_on_a_port_with_nothing_behind_it() {
    assert!(
        !super::is_ready(1),
        "a port nobody is listening on is not an engine that is ready"
    );
}

#[test]
fn waiting_for_a_port_that_never_answers_gives_up_rather_than_waiting_forever() {
    let began = std::time::Instant::now();
    let ready = super::ready_within(1, std::time::Duration::from_millis(600), |_seconds| {});
    assert!(!ready);
    assert!(
        began.elapsed() < std::time::Duration::from_secs(20),
        "it waited {:?}",
        began.elapsed()
    );
}
