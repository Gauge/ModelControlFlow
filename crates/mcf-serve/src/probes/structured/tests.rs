#![allow(clippy::panic, clippy::expect_used)]

use super::{Attempt, Kind, SHAPE, framings, read};
use crate::probes::Trial;

fn finished() -> Trial {
    Trial::Stopped {
        after: 24,
        before: None,
    }
}

#[test]
fn an_object_with_every_field_of_its_kind_conforms() {
    let object = r#"{"sentence": "The cat sat on the mat.", "words": 6, "is_question": false}"#;
    assert_eq!(
        read(object, &finished()),
        Attempt::Conformed { extra: Vec::new() }
    );

    let with_prose = format!("Here you go:\n{object}\nHope that helps.");
    assert_eq!(
        read(&with_prose, &finished()),
        Attempt::Conformed { extra: Vec::new() }
    );
}

#[test]
fn a_wrong_answer_of_the_right_shape_still_conforms() {
    let wrong = r#"{"sentence": "something else entirely", "words": 4, "is_question": true}"#;
    assert_eq!(
        read(wrong, &finished()),
        Attempt::Conformed { extra: Vec::new() }
    );
}

#[test]
fn an_answer_with_no_object_in_it_is_no_object() {
    assert_eq!(
        read(
            "The sentence has six words and is not a question.",
            &finished()
        ),
        Attempt::NoObject
    );
    assert_eq!(read("", &finished()), Attempt::NoObject);
}

#[test]
fn a_missing_field_departs_and_is_named() {
    let short = r#"{"sentence": "The cat sat on the mat.", "words": 6}"#;
    let Attempt::Departed { because } = read(short, &finished()) else {
        panic!("an object missing a field must depart");
    };
    assert!(because.contains("is_question"), "{because}");
}

#[test]
fn a_field_of_the_wrong_kind_departs() {
    let stringly = r#"{"sentence": "The cat sat on the mat.", "words": "6", "is_question": "no"}"#;
    let Attempt::Departed { because } = read(stringly, &finished()) else {
        panic!("a string where a number was asked for must depart");
    };
    assert!(because.contains("words"), "{because}");
    assert!(because.contains("number"), "{because}");
}

#[test]
fn an_object_that_will_not_parse_departs_with_what_came_out() {
    let broken = r#"{"sentence": "The cat sat on the mat.", words: 6}"#;
    let Attempt::Departed { because } = read(broken, &finished()) else {
        panic!("unparseable JSON must depart rather than be absent");
    };
    assert!(because.contains("did not parse"), "{because}");
    assert!(because.contains("words"), "{because}");
}

#[test]
fn a_field_it_added_is_recorded_and_still_conforms() {
    let extra = r#"{"sentence": "The cat sat on the mat.", "words": 6, "is_question": false,
                    "confidence": "high"}"#;
    assert_eq!(
        read(extra, &finished()),
        Attempt::Conformed {
            extra: vec!["confidence".to_owned()]
        }
    );
}

#[test]
fn a_trial_that_could_not_be_told_is_not_a_negative() {
    let undecidable = Trial::CouldNotTell("the daemon did not answer".to_owned());
    assert_eq!(
        read("anything at all", &undecidable),
        Attempt::CouldNotTell {
            because: "the daemon did not answer".to_owned()
        }
    );
}

#[test]
fn every_framing_asks_for_the_whole_shape() {
    let framings = framings();
    assert_eq!(framings.len(), 3, "three framings, each its own condition");
    for framing in &framings {
        assert!(
            framing.text.contains(super::SENTENCE),
            "{}: the sentence is not in the question",
            framing.name
        );
        for field in SHAPE {
            assert!(
                framing.text.contains(field.name),
                "{}: does not ask for {}",
                framing.name,
                field.name
            );
        }
    }
}

#[test]
fn a_kind_matches_only_its_own_values() {
    let text = mcf_record::json::parse(r#""a""#).expect("a string parses");
    let number = mcf_record::json::parse("6").expect("a number parses");
    let boolean = mcf_record::json::parse("true").expect("a boolean parses");

    assert!(Kind::Text.matches(&text));
    assert!(!Kind::Text.matches(&number));
    assert!(Kind::Number.matches(&number));
    assert!(!Kind::Number.matches(&boolean));
    assert!(Kind::Boolean.matches(&boolean));
    assert!(!Kind::Boolean.matches(&text));
}

#[test]
fn a_turn_the_budget_cut_short_is_not_no_object() {
    assert_eq!(
        read("Sure! Here is the ", &Trial::RanOut),
        Attempt::Unfinished
    );
    assert_eq!(
        read(
            "Sure! Here is the ",
            &Trial::Stopped {
                after: 5,
                before: None
            }
        ),
        Attempt::NoObject
    );
    let object = r#"{"sentence": "The cat sat on the mat.", "words": 6, "is_question": false}"#;
    assert_eq!(
        read(object, &Trial::RanOut),
        Attempt::Conformed { extra: Vec::new() }
    );
}
