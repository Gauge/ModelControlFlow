//! The three properties that keep a rule of thumb from reading as a result
//! (B-380, DEC-002, A21).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::CATALOGUE;

/// A touchstone carries no digit: every number a reader sees came from a
/// measurement.
#[test]
fn no_touchstone_contains_a_number() {
    for held in CATALOGUE {
        for part in [held.subject(), held.bare(), held.unmeasured()] {
            assert!(
                !part.chars().any(|letter| letter.is_ascii_digit()),
                "a touchstone with a number in it is a sentence a reader will take for a \
                 measurement (B-380): {part:?}"
            );
        }
    }
}

/// Every rendering carries the mark and the limits — there is no other one.
#[test]
fn every_rendering_says_it_is_not_a_result_and_what_was_not_measured() {
    for held in CATALOGUE {
        let rendered = held.to_string();
        assert!(
            rendered.starts_with("RULE OF THUMB, not a result — "),
            "{rendered}"
        );
        assert!(
            rendered.contains("MCF has not measured "),
            "a touchstone must say what it did not measure, in the same sentence: {rendered}"
        );
        assert!(
            rendered.contains(held.unmeasured()),
            "and the limits must be the ones this touchstone declared: {rendered}"
        );
    }
}

/// Each names the item that would replace it, and no two describe one subject.
#[test]
fn each_names_what_would_replace_it_and_describes_its_own_subject() {
    let mut subjects = Vec::new();
    for held in CATALOGUE {
        assert!(
            held.until().starts_with("B-") && held.until().len() == 5,
            "a touchstone names the register item whose work replaces it: {:?}",
            held.until()
        );
        assert!(
            !held.unmeasured().is_empty() && !held.bare().is_empty(),
            "both halves are required"
        );
        assert!(
            !subjects.contains(&held.subject()),
            "two touchstones about {:?}: a reader meeting both learns which one MCF meant by \
             guessing",
            held.subject()
        );
        subjects.push(held.subject());
    }
}
