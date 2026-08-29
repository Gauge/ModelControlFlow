//! What a cost is, and the sentence it must never become (B-057, F81, A6, A7).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::{LANGUAGE_COST, SAMPLES, language_cost};

/// A file that is not a model is inconclusive, not a cost of nought.
#[test]
fn a_file_that_is_not_a_model_decides_nothing() {
    let probed = language_cost(std::path::Path::new("/nowhere.gguf"), b"not a model at all");
    assert!(
        probed.outcome.observed().is_none(),
        "a probe that could not read the file must not report costs"
    );
    assert_eq!(probed.tokens, 0);
}

/// Every sample is the same meaning, so that what differs between two counts is
/// the vocabulary rather than the sentence.
#[test]
fn the_samples_are_one_meaning_in_six_languages() {
    assert_eq!(SAMPLES.len(), 6);
    let mut languages: Vec<&str> = SAMPLES.iter().map(|(language, _)| *language).collect();
    languages.sort_unstable();
    languages.dedup();
    assert_eq!(
        languages.len(),
        6,
        "two samples of one language is one sample"
    );
    assert!(
        SAMPLES.iter().any(|(language, _)| *language == "English"),
        "English is the baseline every other cost is expressed against"
    );
    for (language, sample) in SAMPLES {
        assert!(
            !sample.trim().is_empty(),
            "{language} has no sample, so its cost would be a fact about nothing"
        );
    }
}

/// The method says what it decides — and, more importantly, what it does not.
///
/// F81's care, kept as a test because it is a sentence and sentences drift: a
/// cost is a fact about a vocabulary, and *fluency* is the reading it must
/// never acquire.
#[test]
fn the_method_refuses_the_reading_that_would_make_it_a_judgement() {
    let decides = LANGUAGE_COST.decides.to_lowercase();
    assert!(
        decides.contains("nothing whatever about how well"),
        "the method must deny the reading a reader will otherwise make: {decides}"
    );
    assert!(
        decides.contains("laboratory"),
        "and must name what would answer the question it declines"
    );
    for grading in ["good at", "better", "fluent", "quality", "grasp"] {
        assert!(
            !LANGUAGE_COST.asks.to_lowercase().contains(grading),
            "a cost is not a grade, and the question must not be phrased as one: {grading}"
        );
    }
}
