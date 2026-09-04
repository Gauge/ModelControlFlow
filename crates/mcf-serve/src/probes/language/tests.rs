//! What a cost is, and the sentence it must never become (B-057, F81, A6, A7).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::{Counted, LANGUAGE_COST, SAMPLES, language_cost};

/// A counter that cannot read the file is inconclusive, not a cost of nought
/// — and the counter's own reason is the probe's (A7, B-442).
#[test]
fn a_counter_that_cannot_read_decides_nothing() {
    let probed = language_cost(
        std::path::Path::new("/nowhere.gguf"),
        "stand-in",
        &mut |_text| Err("a model this machine is not holding: nowhere.gguf".to_owned()),
    );
    assert!(
        probed.outcome.observed().is_none(),
        "a probe that could not count must not report costs"
    );
    assert_eq!(probed.tokens, 0);
    let said = format!("{:?}", probed.outcome);
    assert!(
        said.contains("not holding: nowhere.gguf"),
        "the counter's reason is the reason: {said}"
    );
}

/// The count is the counter's, sample by sample, and the reader is named:
/// six samples in, six costs out against the English one, with the dearest
/// and cheapest read off the counts (B-442).
#[test]
fn the_costs_are_what_the_counter_said() {
    let probed = language_cost(
        std::path::Path::new("/a-model.gguf"),
        "llama-server",
        &mut |text| {
            Ok(Counted {
                tokens: text.chars().count().div_ceil(4),
                by: "the llama-server tokenizer at /engine".to_owned(),
            })
        },
    );
    let spend = probed
        .outcome
        .observed()
        .expect("a counter that answers every sample is a cost");
    assert_eq!(spend.costs.len(), SAMPLES.len());
    assert_eq!(spend.read_by, "the llama-server tokenizer at /engine");
    assert!(spend.unencodable.is_empty(), "{:?}", spend.unencodable);
    let english = spend
        .costs
        .iter()
        .find(|cost| cost.language == "English")
        .expect("the baseline");
    assert_eq!(english.against_english_ppm, 1_000_000);
    let dearest = spend
        .costs
        .iter()
        .max_by_key(|cost| cost.tokens)
        .expect("six costs");
    assert_eq!(spend.dearest, dearest.language);
}

/// One sample the counter refuses is a fact about that sample, in the
/// counter's words, and not a reason to withhold the other five (A7).
#[test]
fn one_unreadable_sample_is_reported_beside_the_rest() {
    let probed = language_cost(
        std::path::Path::new("/a-model.gguf"),
        "stand-in",
        &mut |text| {
            if text.chars().any(|held| held as u32 > 0x2FFF) {
                Err("no token spells this byte".to_owned())
            } else {
                Ok(Counted {
                    tokens: text.split_whitespace().count(),
                    by: "MCF's own tokenizer".to_owned(),
                })
            }
        },
    );
    let spend = probed
        .outcome
        .observed()
        .expect("English counted, so there is a baseline");
    assert!(
        !spend.unencodable.is_empty(),
        "the sample outside the counter's range is named"
    );
    assert!(
        spend
            .unencodable
            .iter()
            .all(|(_, why)| why == "no token spells this byte"),
        "{:?}",
        spend.unencodable
    );
    assert_eq!(spend.costs.len() + spend.unencodable.len(), SAMPLES.len());
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
