//! What conformance is, checked against answers written by hand.
//!
//! **The reader is the whole probe, so it is tested where it decides.** Every
//! case below is one answer a model might really produce and the verdict it
//! must get. The cases that must *not* conform are the point: a reader that
//! called everything conforming would report that every model produces
//! structured output, which is the claim B-054 exists to stop being made on a
//! metadata field or a hunch.
//!
//! The other half is the one F101 paid for — a probe that folds two facts into
//! one and keeps the wrong one. *No object at all*, *an object of the wrong
//! shape*, and *an object with something extra* are three different things a
//! model does, and each is asserted separately here.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::{Attempt, Kind, SHAPE, framings, read};
use crate::probes::Trial;

/// A turn that ended at the model's own stop token.
fn finished() -> Trial {
    Trial::Stopped { after: 24 }
}

/// The answer a model that works produces, bare and wrapped in prose.
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

/// The values are not read for sense, and this is where that is asserted.
///
/// Six is the right number of words and four is not, and both conform: whether
/// a model can count is a capability a laboratory grades against a task, and a
/// probe that failed the second would be reporting *cannot produce JSON* about
/// a model that produced JSON (D42, §XIII).
#[test]
fn a_wrong_answer_of_the_right_shape_still_conforms() {
    let wrong = r#"{"sentence": "something else entirely", "words": 4, "is_question": true}"#;
    assert_eq!(
        read(wrong, &finished()),
        Attempt::Conformed { extra: Vec::new() }
    );
}

/// Prose is not an object, and is not a departure either.
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

/// A missing field is a departure, and the reader says which field.
#[test]
fn a_missing_field_departs_and_is_named() {
    let short = r#"{"sentence": "The cat sat on the mat.", "words": 6}"#;
    let Attempt::Departed { because } = read(short, &finished()) else {
        panic!("an object missing a field must depart");
    };
    assert!(because.contains("is_question"), "{because}");
}

/// A field of the wrong kind is a departure, and the reader says both kinds.
///
/// This is the half-conformance a single-kind shape could not observe: every
/// key present, every value a string.
#[test]
fn a_field_of_the_wrong_kind_departs() {
    let stringly = r#"{"sentence": "The cat sat on the mat.", "words": "6", "is_question": "no"}"#;
    let Attempt::Departed { because } = read(stringly, &finished()) else {
        panic!("a string where a number was asked for must depart");
    };
    assert!(because.contains("words"), "{because}");
    assert!(because.contains("number"), "{because}");
}

/// Something object-shaped that will not parse is a departure, with what came
/// out quoted — never *no object*, because the model tried (A1).
#[test]
fn an_object_that_will_not_parse_departs_with_what_came_out() {
    let broken = r#"{"sentence": "The cat sat on the mat.", words: 6}"#;
    let Attempt::Departed { because } = read(broken, &finished()) else {
        panic!("unparseable JSON must depart rather than be absent");
    };
    assert!(because.contains("did not parse"), "{because}");
    assert!(because.contains("words"), "{because}");
}

/// A key nobody asked for is recorded and does not fail the trial.
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

/// A trial nobody could read is the third state and carries its reason.
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

/// Every framing names the sentence and asks for every field.
///
/// A framing that forgot one would make the probe report a departure that MCF
/// caused — the measurement-error §X is about, pointed at the probe itself.
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

/// The kinds are what they say they are, checked against the record's own
/// parser rather than against an opinion (A19).
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
