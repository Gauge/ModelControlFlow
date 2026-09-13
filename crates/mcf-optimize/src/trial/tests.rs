use super::{Asked, Endpoint, Said, blocks, body};
use crate::corpus::Set;
use crate::dial::{Dial, Step};
use crate::reading::Ending;
use mcf_record::json::Value;

fn asked(dial: Dial, step: Step) -> Asked {
    let Some(set) = Set::numbered(1) else {
        panic!("set one is compiled in");
    };
    Asked {
        set,
        dial,
        step,
        repeat: 0,
        ceiling: 40_000,
        named: Vec::new(),
        timing: false,
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
        counted: None,
        read_in: None,
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

#[test]
fn a_sweep_sends_no_chat_template_switches_at_all() {
    for dial in Dial::ALL {
        let asking = super::body(&asked(dial, dial.step_of(2)));
        assert!(
            asking.get("chat_template_kwargs").is_none(),
            "{} would reach the model as words in its prompt, which a model may honour or \
             ignore as it likes",
            dial.label()
        );
    }
}

#[test]
fn a_sampling_dial_reaches_the_model_as_a_field_the_engine_applies() {
    let asking = super::body(&asked(Dial::Temperature, Step::Thousandths(200)));
    assert!(asking.get("temperature").is_some());
}

#[test]
fn a_level_reaches_the_model_as_the_word_it_named_and_not_as_a_number() {
    let named = vec!["none".to_owned(), "low".to_owned(), "xhigh".to_owned()];
    let mut held = asked(Dial::ThinkingLevel, Step::Whole(2));
    held.named = named;
    let asking = super::body(&held);
    assert_eq!(
        asking
            .get("reasoning_effort")
            .and_then(mcf_record::json::Value::as_text),
        Some("xhigh"),
        "the index is what MCF keeps; the word is what the engine takes"
    );
    assert!(
        asking.get("chat_template_kwargs").is_none(),
        "it goes in the field llama.cpp parses itself, so that 'none' means what it means \
         there rather than being passed straight to a template"
    );
}

#[test]
fn a_level_outside_what_the_model_named_is_never_invented() {
    let mut held = asked(Dial::ThinkingLevel, Step::Whole(9));
    held.named = vec!["low".to_owned()];
    let asking = super::body(&held);
    assert_eq!(
        asking
            .get("reasoning_effort")
            .and_then(mcf_record::json::Value::as_text),
        Some("9"),
        "with nothing to name it, the step is sent as it stands rather than as a guess"
    );
}

#[test]
fn a_sampling_dial_is_still_sent_as_a_number() {
    let asking = super::body(&asked(Dial::Temperature, Step::Thousandths(200)));
    assert!(
        asking
            .get("temperature")
            .and_then(mcf_record::json::Value::as_text)
            .is_none(),
        "a temperature is a number, not a word"
    );
    assert!(asking.get("temperature").is_some());
}

#[test]
fn a_timed_trial_goes_to_the_plain_endpoint_and_asks_for_tokens() {
    let mut held = asked(Dial::DraftDepth, Step::Whole(4));
    held.timing = true;
    held.ceiling = super::TOKENS_TIMED;
    assert_eq!(
        super::the_way_in(&held),
        "/completion",
        "the chat endpoint applies a template and parses the reply; a model told to ignore \
         its own ending walks off the end of that parser and takes the engine with it"
    );
    let asking = super::body(&held);
    assert_eq!(
        asking.get("prompt").and_then(Value::as_text),
        Some(super::TO_BE_TIMED)
    );
    assert!(
        asking.get("messages").is_none(),
        "there is no conversation here, only tokens"
    );
    assert_eq!(
        asking.get("n_predict"),
        Some(&Value::Integer(i64::from(super::TOKENS_TIMED)))
    );
    assert_eq!(
        asking.get("ignore_eos"),
        Some(&Value::Bool(true)),
        "this is what makes the count the count rather than whatever the model felt like"
    );
}

#[test]
fn a_micro_batch_trial_times_reading_a_prompt_rather_than_writing_an_answer() {
    let mut held = asked(Dial::MicroBatch, Step::Whole(512));
    held.timing = true;
    held.ceiling = super::TOKENS_PREFILLED;
    let asking = super::body(&held);
    let sent = asking
        .get("prompt")
        .and_then(Value::as_list)
        .expect("a prompt given as token numbers is exactly as long as it says it is");
    assert_eq!(
        u32::try_from(sent.len()).unwrap(),
        super::TOKENS_PREFILLED,
        "a micro-batch is how many prompt tokens go through the device in one pass, so the \
         prompt is the work being timed and it has to be long enough to take several passes"
    );
    assert_eq!(
        asking.get("n_predict"),
        Some(&Value::Integer(1)),
        "writing an answer is one token at a time whatever the micro-batch is, so timing any \
         more of it would read the same number back at every value"
    );
    assert_eq!(
        asking.get("cache_prompt"),
        Some(&Value::Bool(false)),
        "a cached prompt is not read again, and a reading that reads nothing times nothing"
    );
    assert_eq!(super::the_way_in(&held), "/completion");
}

#[test]
fn a_prompt_to_be_read_is_inside_every_vocabulary_and_fits_the_window() {
    let sent = super::to_be_read(64);
    let numbers: Vec<i64> = sent
        .as_list()
        .expect("a list")
        .iter()
        .filter_map(Value::as_integer)
        .collect();
    assert_eq!(numbers.len(), 64);
    for held in &numbers {
        assert!(
            (0..32_000).contains(held),
            "a token number above the model's vocabulary is refused by the engine, and the \
             smallest vocabulary MCF has met is well above this: {held}"
        );
    }
    assert!(
        numbers
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            > 1,
        "one token repeated is not a prompt an engine reads the way it reads a real one"
    );
    assert_eq!(
        super::prompt_within(4096),
        4096 - 512,
        "a window smaller than the prompt MCF would rather send is the one that decides"
    );
    assert_eq!(super::prompt_within(131_072), super::TOKENS_PREFILLED);
}

#[test]
fn a_trial_of_the_tasks_goes_to_the_chat_endpoint() {
    let held = asked(Dial::ThinkingBudget, Step::Whole(4096));
    assert_eq!(super::the_way_in(&held), "/v1/chat/completions");
    assert!(super::body(&held).get("ignore_eos").is_none());
}
#[test]
fn a_trial_of_the_tasks_asks_for_the_tasks() {
    let asking = super::body(&asked(Dial::ThinkingBudget, Step::Whole(4096)));
    let said = asking
        .get("messages")
        .and_then(mcf_record::json::Value::as_list)
        .and_then(|held| held.first().cloned())
        .and_then(|one| {
            one.get("content")
                .and_then(|held| held.as_text())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    assert!(said.contains("SOLUTION"), "{said}");
}

#[test]
fn a_timed_run_that_stopped_early_is_not_offered_as_a_rate() {
    assert!(
        super::enough_of(super::TOKENS_TIMED) * 2 > u64::from(super::TOKENS_TIMED),
        "most of the tokens asked for is the bar; a run that stopped at a tenth is not the \
         same measurement as one that ran the whole way"
    );
    assert!(super::enough_of(super::TOKENS_TIMED) <= u64::from(super::TOKENS_TIMED));
}

#[test]
fn the_settings_that_only_change_speed_are_the_ones_that_cannot_change_an_answer() {
    assert!(Dial::MicroBatch.only_changes_speed());
    assert!(
        Dial::DraftDepth.only_changes_speed(),
        "a draft head's guesses are checked against the model, so the tokens are identical"
    );
    for dial in [
        Dial::ThinkingBudget,
        Dial::ThinkingLevel,
        Dial::Temperature,
        Dial::TopP,
        Dial::TopK,
    ] {
        assert!(
            !dial.only_changes_speed(),
            "{} changes what comes back",
            dial.label()
        );
    }
}

#[test]
fn a_timed_trial_that_reaches_its_count_is_an_answer_rather_than_a_runaway() {
    let mut held = asked(Dial::MicroBatch, Step::Whole(256));
    held.timing = true;
    held.ceiling = 64;
    assert!(
        held.timing,
        "filling the count is the whole point of a timed trial, not a sign it went wrong"
    );
}

#[test]
fn a_timed_trial_is_not_cut_short_for_repeating_itself() {
    let mut held = asked(Dial::MicroBatch, Step::Whole(1024));
    held.timing = true;
    assert!(
        held.timing,
        "counting upwards repeats by design; a timed trial wants the tokens, not the prose"
    );
    let mut answering = asked(Dial::ThinkingBudget, Step::Whole(4096));
    answering.timing = false;
    assert!(
        !answering.timing,
        "a trial of the tasks is still stopped when it starts going round in circles"
    );
}

#[test]
fn a_timed_run_is_long_enough_to_mean_something_and_taken_more_than_once() {
    assert_eq!(super::TOKENS_TIMED, 1024);
    assert_eq!(
        super::TIMES_TIMED,
        5,
        "a single run of a few thousand tokens is mostly whatever else the machine was doing"
    );
}

#[test]
fn a_read_interrupted_by_a_signal_is_not_the_end_of_the_reply() {
    use std::io::{Read, Result};
    struct Twitchy {
        given: usize,
    }
    impl Read for Twitchy {
        fn read(&mut self, into: &mut [u8]) -> Result<usize> {
            self.given += 1;
            if self.given % 2 == 1 {
                return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
            }
            if self.given > 6 {
                return Ok(0);
            }
            let said = b"x";
            into.get_mut(..1).map_or(Ok(0), |room| {
                room.copy_from_slice(said);
                Ok(1)
            })
        }
    }
    let mut held = Twitchy { given: 0 };
    let mut all = Vec::new();
    let mut room = [0_u8; 8];
    loop {
        match held.read(&mut room) {
            Ok(0) => break,
            Ok(read) => all.extend_from_slice(room.get(..read).unwrap_or_default()),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    assert_eq!(
        all.len(),
        3,
        "every byte after an interruption must still arrive; treating the interruption as \
         the end loses the rest of the reply"
    );
}
