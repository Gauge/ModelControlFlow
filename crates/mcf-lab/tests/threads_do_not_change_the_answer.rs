#![allow(clippy::expect_used, clippy::panic)]

use mcf_lab::fixture;
use mcf_standin::llama::{Cache, load};
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, generate};
use mcf_standin::threads::Threads;
use mcf_standin::{gguf, tokenizer::Vocabulary};

const COUNTS: [usize; 5] = [1, 2, 3, 8, 64];

fn the_fixture_is_partitioned() {
    let width = 323;
    let earned = Threads::stated(8).worth_starting(width * width);
    assert!(
        earned > 1,
        "the dense fixture's smallest product earns {earned} worker(s), so nothing below is \
         partitioned and this file establishes nothing"
    );
}

#[test]
fn a_forward_pass_is_the_same_bytes_at_every_thread_count() {
    the_fixture_is_partitioned();
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

#[test]
fn a_generation_is_the_same_tokens_at_every_thread_count() {
    the_fixture_is_partitioned();
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

#[test]
fn a_loaded_model_says_how_many_threads_it_was_given() {
    let bytes = fixture::a_model_with_dense_weights(1);
    let file = gguf::parse(&bytes).expect("reads");
    let model = load(&file, &bytes).expect("loads");
    assert_eq!(model.threads().count(), 1);
    assert!(model.threads().describe().contains("definition"));

    let asked = load(&file, &bytes)
        .expect("loads")
        .across(Threads::stated(4));
    assert_eq!(asked.threads().count(), 4);
    assert!(asked.threads().describe().contains("asked for"));
}

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

fn tokens(file: &gguf::Model, bytes: &[u8], prompt: &[usize], threads: Threads) -> Vec<usize> {
    let model = load(file, bytes).expect("loads").across(threads);
    let marked = generate(
        &model,
        "stand-in, this test",
        &Request {
            prompt: prompt.to_vec(),
            limit: 8,
            settings: Settings::Greedy,
            seed: 0,
            stop: Vec::new(),
        },
    )
    .expect("it runs");
    marked.value().observed().tokens.clone()
}

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
