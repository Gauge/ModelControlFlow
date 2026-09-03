//! What a well-formed call is, checked against answers written by hand.
//!
//! **The reader is the whole probe, so it is tested where it decides.** Every
//! case below is one answer a model might really produce and the verdict it
//! must get — and the cases that must *not* pass are the point: a reader that
//! called everything well-formed would make the probe report support for every
//! model, which is exactly the claim B-053 exists to stop being made on a
//! metadata field.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::{Attempt, Declared, Form, Found, OFFER, Offering, between, paired, read};
use crate::probes::Trial;

/// An offering that looks for a bare object.
fn plain() -> Offering {
    Offering {
        name: "plain".to_owned(),
        text: String::new(),
        between: None,
        form: Form::Json,
    }
}

/// An offering that looks between a family's own markers.
fn marked() -> Offering {
    Offering {
        name: "marked".to_owned(),
        text: String::new(),
        between: Some(("<tool_call>".to_owned(), "</tool_call>".to_owned())),
        form: Form::Json,
    }
}

/// A turn that ended at the model's own stop token.
fn finished() -> Trial {
    Trial::Stopped { after: 12 }
}

/// The call every model that works emits, in both shapes.
#[test]
fn a_call_that_parses_and_names_the_tool_is_well_formed() {
    let bare = r#"{"name": "get_weather", "arguments": {"city": "Paris"}}"#;
    assert_eq!(read(bare, &plain(), &finished()), Attempt::WellFormed);

    let wrapped = format!("Certainly.\n<tool_call>\n{bare}\n</tool_call>");
    assert_eq!(read(&wrapped, &marked(), &finished()), Attempt::WellFormed);
}

/// Prose is not a call, and is not a malformed one either.
///
/// The distinction is the reason both counts exist: a model that answered from
/// memory has told MCF something different from one that tried and got the
/// form wrong.
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

/// Something call-shaped that does not parse is malformed, and says so.
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

/// A call naming a tool nobody offered is malformed rather than well formed.
///
/// A model that invents a tool has not demonstrated it can call the one it was
/// given, and counting it would let a probe report support on a hallucination.
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

/// An object with no name is malformed.
#[test]
fn an_object_without_a_name_is_malformed() {
    match read(r#"{"arguments": {"city": "Paris"}}"#, &plain(), &finished()) {
        Attempt::Malformed { because } => assert!(because.contains("no \"name\""), "{because}"),
        other => panic!("a nameless object was read as {other:?}"),
    }
}

/// The arguments are never judged.
///
/// A call for the wrong city, or with no arguments at all, is still a
/// well-formed call: whether the arguments are sensible is a judgement and
/// this probe makes none (D42).
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

/// A trial that could not be read is not a verdict about the model.
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

/// A bare object is found even with prose around it, and nesting is balanced.
#[test]
fn a_bare_object_is_found_among_prose_and_balances_its_braces() {
    let said = r#"Sure! {"name": "get_weather", "arguments": {"city": "Paris"}} — done."#;
    let found = between(said, &plain()).expect("an object is in there");
    assert_eq!(
        found,
        Found::AsAsked(r#"{"name": "get_weather", "arguments": {"city": "Paris"}}"#.to_owned())
    );
    // An unbalanced opener yields nothing rather than the rest of the string.
    assert_eq!(between("here we go {\"name\":", &plain()), None);
}

/// A call emitted without the markers it was asked for is malformed, and never
/// "no call".
///
/// **The first real run of this probe got this wrong.** Qwen3-0.6B reported
/// *no call* five times out of five under its own `<tool_call>` markers while
/// calling perfectly under a plain description — and the reader could not tell
/// *it ignored the tool* from *it called without the wrapper*. Those are
/// different facts about a model and A1 forbids discarding the second.
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

    // And prose with no object in it is still `NoCall` — the fallback must not
    // turn every answer into an attempt.
    assert_eq!(
        read("It is sunny in Paris.", &marked(), &finished()),
        Attempt::NoCall
    );
}

/// The fallback says where it found what it found.
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
    // A plain offering asks for a bare object, so finding one is as asked.
    assert_eq!(
        between(outside, &plain()),
        Some(Found::AsAsked("{\"a\":1}".to_owned()))
    );
}

/// A call is read to its closing marker, not to the end of the answer.
#[test]
fn a_marked_call_ends_at_its_closing_marker() {
    let said = "<tool_call>{\"a\":1}</tool_call> and then some chatter";
    assert_eq!(
        between(said, &marked()),
        Some(Found::AsAsked("{\"a\":1}".to_owned()))
    );
}

/// Markers pair by their closing form, and pair with nothing where there is no
/// closing form.
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

/// A declaration is what the file says, and claims support by either route.
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

/// The offered tool's name is what a well-formed call must carry.
#[test]
fn the_offer_is_what_the_reader_checks_against() {
    let call = format!(r#"{{"name": "{}", "arguments": {{}}}}"#, OFFER.name);
    assert_eq!(read(&call, &plain(), &finished()), Attempt::WellFormed);
}

/// A call in the form a model's own template writes is a call, and the
/// offering that asked for the other shape says which one came.
#[test]
fn a_function_block_is_a_call() {
    let block = "<tool_call>\n<function=get_weather>\n<parameter=city>\nParis\n\
                 </parameter>\n</function>\n</tool_call>";
    // Under an offering that asked for that form, it is well formed.
    let mut asked = marked();
    asked.form = Form::Function;
    assert_eq!(read(block, &asked, &finished()), Attempt::WellFormed);
    // Under one that asked for JSON, it is a call in the other form —
    // never *no call*, which is what it was read as before (B-453).
    match read(block, &marked(), &finished()) {
        Attempt::Malformed { because } => {
            assert!(because.contains("nested function block"), "{because}");
        }
        other => panic!("a call in the template's own form read as {other:?}"),
    }
    // And with no markers around it at all, it is still a call.
    let bare = "I will look it up.\n<function=get_weather>\n<parameter=city>\nParis\n\
                </parameter>\n</function>";
    let mut plainly = plain();
    plainly.form = Form::Function;
    assert_eq!(read(bare, &plainly, &finished()), Attempt::WellFormed);
}

/// A function block naming another tool is refused for the name, not the form.
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

/// Prose is still no call, in either form.
#[test]
fn prose_is_no_call_whichever_form_was_asked_for() {
    let said = "It is sunny in Paris today, about 18 degrees.";
    assert_eq!(read(said, &plain(), &finished()), Attempt::NoCall);
    let mut plainly = plain();
    plainly.form = Form::Function;
    assert_eq!(read(said, &plainly, &finished()), Attempt::NoCall);
}
