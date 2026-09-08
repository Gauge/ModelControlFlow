#![allow(clippy::panic, clippy::expect_used)]

use super::{Attempt, Declared, Form, Found, OFFER, Offering, between, paired, read};
use crate::probes::Trial;

fn plain() -> Offering {
    Offering {
        name: "plain".to_owned(),
        text: String::new(),
        between: None,
        form: Form::Json,
    }
}

fn marked() -> Offering {
    Offering {
        name: "marked".to_owned(),
        text: String::new(),
        between: Some(("<tool_call>".to_owned(), "</tool_call>".to_owned())),
        form: Form::Json,
    }
}

fn finished() -> Trial {
    Trial::Stopped {
        after: 12,
        before: None,
    }
}

#[test]
fn a_call_that_parses_and_names_the_tool_is_well_formed() {
    let bare = r#"{"name": "get_weather", "arguments": {"city": "Paris"}}"#;
    assert_eq!(read(bare, &plain(), &finished()), Attempt::WellFormed);

    let wrapped = format!("Certainly.\n<tool_call>\n{bare}\n</tool_call>");
    assert_eq!(read(&wrapped, &marked(), &finished()), Attempt::WellFormed);
}

#[test]
fn an_answer_with_no_call_in_it_is_no_call() {
    assert_eq!(
        read("It is sunny in Paris today.", &plain(), &finished()),
        Attempt::NoCall
    );
    assert_eq!(
        read("It is sunny in Paris today.", &marked(), &finished()),
        Attempt::NoCall
    );
    assert_eq!(read("", &plain(), &finished()), Attempt::NoCall);
}

#[test]
fn a_call_that_does_not_parse_is_malformed_with_its_reason() {
    let truncated = r#"<tool_call>{"name": "get_weather", "arguments": {"city": </tool_call>"#;
    match read(truncated, &marked(), &finished()) {
        Attempt::Malformed { because } => {
            assert!(
                because.contains("did not parse"),
                "the reason does not say what was wrong: {because}"
            );
        }
        other => panic!("a truncated object was read as {other:?}"),
    }
}

#[test]
fn a_call_naming_another_tool_is_malformed() {
    let wrong = r#"{"name": "lookup_forecast", "arguments": {"city": "Paris"}}"#;
    match read(wrong, &plain(), &finished()) {
        Attempt::Malformed { because } => {
            assert!(because.contains("lookup_forecast"), "{because}");
            assert!(because.contains("not the tool offered"), "{because}");
        }
        other => panic!("a call to another tool was read as {other:?}"),
    }
}

#[test]
fn an_object_without_a_name_is_malformed() {
    match read(r#"{"arguments": {"city": "Paris"}}"#, &plain(), &finished()) {
        Attempt::Malformed { because } => assert!(because.contains("no \"name\""), "{because}"),
        other => panic!("a nameless object was read as {other:?}"),
    }
}

#[test]
fn the_arguments_are_not_read_for_sense() {
    for call in [
        r#"{"name": "get_weather", "arguments": {"city": "Atlantis"}}"#,
        r#"{"name": "get_weather", "arguments": {}}"#,
        r#"{"name": "get_weather"}"#,
    ] {
        assert_eq!(
            read(call, &plain(), &finished()),
            Attempt::WellFormed,
            "the probe formed a judgement about {call}"
        );
    }
}

#[test]
fn a_trial_that_could_not_be_told_is_not_a_no_call() {
    let outcome = read(
        "",
        &plain(),
        &Trial::CouldNotTell("the engine died".to_owned()),
    );
    match outcome {
        Attempt::CouldNotTell { because } => assert!(because.contains("engine died")),
        other => panic!("an unreadable trial was read as {other:?}"),
    }
}

#[test]
fn a_bare_object_is_found_among_prose_and_balances_its_braces() {
    let said = r#"Sure! {"name": "get_weather", "arguments": {"city": "Paris"}} — done."#;
    let found = between(said, &plain()).expect("an object is in there");
    assert_eq!(
        found,
        Found::AsAsked(r#"{"name": "get_weather", "arguments": {"city": "Paris"}}"#.to_owned())
    );
    assert_eq!(between("here we go {\"name\":", &plain()), None);
}

#[test]
fn a_call_without_the_markers_asked_for_is_malformed_not_absent() {
    let bare = r#"{"name": "get_weather", "arguments": {"city": "Paris"}}"#;
    match read(bare, &marked(), &finished()) {
        Attempt::Malformed { because } => {
            assert!(because.contains("without the markers"), "{because}");
            assert!(because.contains("get_weather"), "{because}");
        }
        other => panic!("a call outside its markers was read as {other:?}"),
    }

    assert_eq!(
        read("It is sunny in Paris.", &marked(), &finished()),
        Attempt::NoCall
    );
}

#[test]
fn a_candidate_says_whether_it_was_where_it_was_asked_for() {
    let inside = "<tool_call>{\"a\":1}</tool_call>";
    assert_eq!(
        between(inside, &marked()),
        Some(Found::AsAsked("{\"a\":1}".to_owned()))
    );
    let outside = "{\"a\":1}";
    assert_eq!(
        between(outside, &marked()),
        Some(Found::Elsewhere("{\"a\":1}".to_owned()))
    );
    assert_eq!(
        between(outside, &plain()),
        Some(Found::AsAsked("{\"a\":1}".to_owned()))
    );
}

#[test]
fn a_marked_call_ends_at_its_closing_marker() {
    let said = "<tool_call>{\"a\":1}</tool_call> and then some chatter";
    assert_eq!(
        between(said, &marked()),
        Some(Found::AsAsked("{\"a\":1}".to_owned()))
    );
}

#[test]
fn markers_pair_by_their_closing_form() {
    let both = vec!["<tool_call>".to_owned(), "</tool_call>".to_owned()];
    assert_eq!(
        paired(&both),
        Some(("<tool_call>".to_owned(), "</tool_call>".to_owned()))
    );
    assert_eq!(paired(&["<tool_call>".to_owned()]), None);
    assert_eq!(paired(&[]), None);
}

#[test]
fn a_declaration_claims_support_by_either_route() {
    assert!(
        !Declared {
            template_mentions_tools: false,
            markers: Vec::new(),
            form: Form::Json,
        }
        .claims_support()
    );
    assert!(
        Declared {
            template_mentions_tools: true,
            markers: Vec::new(),
            form: Form::Json,
        }
        .claims_support()
    );
    assert!(
        Declared {
            template_mentions_tools: false,
            markers: vec!["<tool_call>".to_owned()],
            form: Form::Json,
        }
        .claims_support()
    );
}

#[test]
fn the_offer_is_what_the_reader_checks_against() {
    let call = format!(r#"{{"name": "{}", "arguments": {{}}}}"#, OFFER.name);
    assert_eq!(read(&call, &plain(), &finished()), Attempt::WellFormed);
}

#[test]
fn a_function_block_is_a_call() {
    let block = "<tool_call>\n<function=get_weather>\n<parameter=city>\nParis\n\
                 </parameter>\n</function>\n</tool_call>";
    let mut asked = marked();
    asked.form = Form::Function;
    assert_eq!(read(block, &asked, &finished()), Attempt::WellFormed);
    match read(block, &marked(), &finished()) {
        Attempt::Malformed { because } => {
            assert!(because.contains("nested function block"), "{because}");
        }
        other => panic!("a call in the template's own form read as {other:?}"),
    }
    let bare = "I will look it up.\n<function=get_weather>\n<parameter=city>\nParis\n\
                </parameter>\n</function>";
    let mut plainly = plain();
    plainly.form = Form::Function;
    assert_eq!(read(bare, &plainly, &finished()), Attempt::WellFormed);
}

#[test]
fn a_function_block_naming_another_tool_is_malformed() {
    let block = "<function=get_time>\n<parameter=city>\nParis\n</parameter>\n</function>";
    let mut plainly = plain();
    plainly.form = Form::Function;
    match read(block, &plainly, &finished()) {
        Attempt::Malformed { because } => assert!(because.contains("get_time"), "{because}"),
        other => panic!("a call to another tool read as {other:?}"),
    }
}

#[test]
fn prose_is_no_call_whichever_form_was_asked_for() {
    let said = "It is sunny in Paris today, about 18 degrees.";
    assert_eq!(read(said, &plain(), &finished()), Attempt::NoCall);
    let mut plainly = plain();
    plainly.form = Form::Function;
    assert_eq!(read(said, &plainly, &finished()), Attempt::NoCall);
}
