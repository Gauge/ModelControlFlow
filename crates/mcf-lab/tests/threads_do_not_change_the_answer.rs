//! A whole forward pass, and a whole generation, at one thread and at many
//! (B-366, D38, §3.12, D19, A13).
//!
//! `mcf-standin`'s own property test establishes this of one operation. This
//! establishes it of the thing a person actually runs: a model file read from
//! bytes, thirty-odd matrix products per token across three blocks, a sampler
//! and a generation loop. The two are not the same claim — an engine can hold
//! the property in the operation and lose it in the assembly, by reducing
//! across heads or across experts somewhere the partition reaches.
//!
//! **The laboratory owns the model because the degenerate fixture cannot ask
//! the question.** [`mcf_lab::fixture::a_model_that_runs`] is one-hot with zero
//! projections, and a sum of zeros is the same number in any order: a partition
//! of its work agrees with the serial path whether or not the partition is
//! sound. So this uses `a_model_with_dense_weights`, whose every weight is
//! non-zero and whose magnitudes span three orders — the condition under which
//! floating-point addition notices what order it was performed in.
//!
//! **The comparison is on bits, and the reason is measured rather than
//! argued.** The threaded path was deliberately given a split reduction — each
//! row summed as two halves and the halves added — and run against this
//! fixture. The logit comparison failed, as it must. *The token comparison did
//! not*: the divergence was too small to move a greedy argmax on this model, so
//! a suite that only compared the text would have called a broken partition
//! sound. Both are asserted, and that is which of the two is load-bearing.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic)]

use mcf_lab::fixture;
use mcf_standin::llama::{Cache, load};
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, generate};
use mcf_standin::threads::Threads;
use mcf_standin::{gguf, tokenizer::Vocabulary};

/// The thread counts every case is run at: one worker, a few, and more workers
/// than the model has rows to give them.
const COUNTS: [usize; 5] = [1, 2, 3, 8, 64];

/// Every logit of every position, identical bytes at every thread count.
#[test]
fn a_forward_pass_is_the_same_bytes_at_every_thread_count() {
    let bytes = fixture::a_model_with_dense_weights(20_366);
    let file = gguf::parse(&bytes).expect("the laboratory's own file reads");

    let definition = logits(&file, &bytes, Threads::definition());
    assert!(
        definition.iter().any(|row| row.iter().any(|v| *v != 0.0)),
        "the dense fixture produced all zeros, which no thread count could disagree about"
    );

    for count in COUNTS {
        let produced = logits(&file, &bytes, Threads::stated(count));
        assert_bits(&produced, &definition, &format!("{count} threads"));
    }

    let machine = logits(&file, &bytes, Threads::what_the_machine_reports());
    assert_bits(&machine, &definition, "what this machine reports");
}

/// And the tokens themselves: a generation is the same text however it was
/// divided.
///
/// The logits are what the property is *about*; the tokens are what a person
/// sees. A divergence too small to move the tokens is still a divergence, which
/// is why both are asserted — but a divergence that does move them is the one
/// somebody would report as MCF answering differently on a busy machine.
#[test]
fn a_generation_is_the_same_tokens_at_every_thread_count() {
    let bytes = fixture::a_model_with_dense_weights(366);
    let file = gguf::parse(&bytes).expect("reads");
    let vocabulary = Vocabulary::read(&file).expect("the vocabulary reads");
    let prompt = vocabulary.encode("maybe yes no", true).expect("segments");

    let definition = tokens(&file, &bytes, &prompt, Threads::definition());
    assert!(
        !definition.is_empty(),
        "the fixture generated nothing, so there is nothing to compare"
    );

    for count in COUNTS {
        let produced = tokens(&file, &bytes, &prompt, Threads::stated(count));
        assert_eq!(
            produced, definition,
            "the model said something different at {count} threads"
        );
    }
}

/// A model carries the count it was given, and says whose number it is.
#[test]
fn a_loaded_model_says_how_many_threads_it_was_given() {
    let bytes = fixture::a_model_with_dense_weights(1);
    let file = gguf::parse(&bytes).expect("reads");
    let model = load(&file, &bytes).expect("loads");
    // One thread at load, which is the definition rather than a choice.
    assert_eq!(model.threads().count(), 1);
    assert!(model.threads().describe().contains("definition"));

    let asked = load(&file, &bytes)
        .expect("loads")
        .across(Threads::stated(4));
    assert_eq!(asked.threads().count(), 4);
    assert!(asked.threads().describe().contains("asked for"));
}

/// Every position's logits, run through the model at a stated thread count.
fn logits(file: &gguf::Model, bytes: &[u8], threads: Threads) -> Vec<Vec<f32>> {
    let model = load(file, bytes).expect("the model loads").across(threads);
    let mut cache = Cache::for_model(&model.shape);
    (0..6_usize)
        .map(|position| {
            model
                .forward(position + 1, position, &mut cache)
                .expect("the forward pass runs")
        })
        .collect()
}

/// What the model says, at a stated thread count.
fn tokens(file: &gguf::Model, bytes: &[u8], prompt: &[usize], threads: Threads) -> Vec<usize> {
    let model = load(file, bytes).expect("loads").across(threads);
    let marked = generate(
        &model,
        "stand-in, this test",
        &Request {
            prompt: prompt.to_vec(),
            limit: 8,
            // Greedy so that a difference in the logits shows as a difference in
            // the tokens rather than being absorbed by a sampler's own arithmetic.
            settings: Settings::Greedy,
            seed: 0,
            stop: Vec::new(),
        },
    )
    .expect("it runs");
    marked.value().observed().tokens.clone()
}

/// Compares two runs bit for bit, and says where they first parted.
fn assert_bits(produced: &[Vec<f32>], expected: &[Vec<f32>], what: &str) {
    assert_eq!(produced.len(), expected.len(), "{what}: different lengths");
    for (position, (one, other)) in produced.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            one.len(),
            other.len(),
            "{what}: position {position} is a different width"
        );
        for (index, (value, against)) in one.iter().zip(other.iter()).enumerate() {
            assert!(
                value.to_bits() == against.to_bits(),
                "{what}: position {position}, logit {index} is {value} and the serial \
                 definition says {against}"
            );
        }
    }
}
