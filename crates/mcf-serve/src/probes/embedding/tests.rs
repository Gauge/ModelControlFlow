//! What an embedding probe may and may not conclude (B-057, D42, A21, A19).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::{EMBEDDING, embedding};
use crate::probes::tests::plain;

/// A model that produces nothing is inconclusive, never *cannot embed*.
///
/// A7's distinction, and the one this probe is most likely to get wrong: MCF
/// asked and got nothing back, which is what an ordinary text model does — and
/// is not the same claim as *this artifact is incapable*.
#[test]
fn an_artifact_that_produced_nothing_decides_nothing() {
    let mut nothing = |_: &str| None;
    let probed = embedding(
        std::path::Path::new("/nowhere.gguf"),
        &plain(),
        "stand-in",
        &mut nothing,
    );
    let mcf_core::probe::Outcome::Inconclusive { because } = &probed.outcome else {
        panic!("a probe that got no vector must not report a width");
    };
    assert!(
        because.contains("rather than reporting that it cannot"),
        "{because}"
    );
}

/// Two identical calls that differ are reported as differing, to the last bit.
#[test]
fn two_calls_that_differ_are_not_rounded_together() {
    let mut counter = 0_usize;
    let mut drifting = |_: &str| {
        counter = counter.saturating_add(1);
        Some((384, format!("digest-{counter}"), 7))
    };
    let probed = embedding(
        std::path::Path::new("/nowhere.gguf"),
        &plain(),
        "stand-in",
        &mut drifting,
    );
    let observed = probed
        .outcome
        .observed()
        .expect("two vectors came back, so there is an observation");
    assert!(
        !observed.identical_twice,
        "two different vectors for one text must be reported as different: MCF's own engine is \
         deterministic, so this is a defect rather than noise (A19)"
    );
    assert_eq!(observed.width, 384);
    assert_eq!(probed.tokens, 0, "an embedding generates nothing");
}

/// The same vector twice is what a deterministic engine produces.
#[test]
fn the_same_vector_twice_is_observed_as_such() {
    let mut steady = |_: &str| Some((768, "one-digest".to_owned(), 7));
    let probed = embedding(
        std::path::Path::new("/nowhere.gguf"),
        &plain(),
        "stand-in",
        &mut steady,
    );
    let observed = probed.outcome.observed().expect("an observation");
    assert!(observed.identical_twice);
    assert_eq!(observed.width, 768);
}

/// The method declines the reading that would make it a grade.
#[test]
fn the_method_says_what_it_does_not_decide() {
    assert!(
        EMBEDDING
            .decides
            .contains("nothing about how good they are"),
        "an embedding probe must deny the reading a reader will otherwise make"
    );
}
