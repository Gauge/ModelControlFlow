#![allow(clippy::panic, clippy::expect_used)]

use super::{EMBEDDING, embedding};
use crate::probes::tests::plain;

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

#[test]
fn the_method_says_what_it_does_not_decide() {
    assert!(
        EMBEDDING
            .decides
            .contains("nothing about how good they are"),
        "an embedding probe must deny the reading a reader will otherwise make"
    );
}
