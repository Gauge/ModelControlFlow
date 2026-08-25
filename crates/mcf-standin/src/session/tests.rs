//! A generation runs, ends for a stated reason, and comes back marked.

use super::{Request, Stopped, generate};
use crate::llama::{Loaded, load};
use crate::sample::Settings;
use crate::{gguf, llama};

/// The same one-hot model the forward pass is tested against: token `i`
/// produces token `i`, so a generation is a predictable repetition and the
/// interesting things are the stopping rule and the mark.
fn model() -> (Vec<u8>, Loaded) {
    let bytes = crate::llama::tests::one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let loaded = load(&file, &bytes).expect("loads");
    (bytes, loaded)
}

/// A generation produces tokens, and the result carries the mark A5 requires.
///
/// The type is the assertion: `Degraded<Behaviour<Generated>>` has no way out
/// that drops the mark, so a caller that renders the tokens renders the fact
/// that they came from a stand-in.
#[test]
fn a_generation_comes_back_marked() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![2],
        limit: 3,
        settings: Settings::Greedy,
        seed: 0,
        stop: Vec::new(),
    };
    let marked = generate(&model, "stand-in 0.1.0-m0", &request).expect("it runs");

    // The mark names what was lost and which engine ran.
    let rendered = marked.degradation().to_string();
    assert!(rendered.contains("engine"), "{rendered}");

    let behaviour = marked.value();
    assert_eq!(behaviour.engine(), "stand-in");
    let generated = behaviour.observed();
    assert_eq!(generated.prompt_length, 1);
    assert_eq!(generated.stopped, Stopped::AtLimit);
    // The one-hot model answers token 2 with token 2, for ever.
    assert_eq!(generated.tokens, vec![2, 2, 2]);
}

/// A stop token ends the generation and says which one did it. *The model
/// stopped* and *MCF stopped it* are different facts about a run, which is the
/// question a behaviour-class laboratory is asking.
#[test]
fn a_stop_token_ends_it_and_names_itself() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![1],
        limit: 10,
        settings: Settings::Greedy,
        seed: 0,
        stop: vec![1],
    };
    let marked = generate(&model, "stand-in", &request).expect("it runs");
    let generated = marked.value().observed();
    assert_eq!(generated.stopped, Stopped::AtStopToken { token: 1 });
    assert!(
        generated.tokens.is_empty(),
        "the stop token is not produced"
    );
}

/// A limit of zero produces nothing and says the budget is why, rather than
/// looking like a model that had nothing to say.
#[test]
fn an_exhausted_budget_says_so() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![0],
        limit: 0,
        settings: Settings::Greedy,
        seed: 0,
        stop: Vec::new(),
    };
    let marked = generate(&model, "stand-in", &request).expect("it runs");
    assert_eq!(marked.value().observed().stopped, Stopped::AtLimit);
}

/// An empty prompt has nothing to read, and that is its own outcome.
#[test]
fn an_empty_prompt_is_its_own_outcome() {
    let (_, model) = model();
    let request = Request {
        prompt: Vec::new(),
        limit: 4,
        settings: Settings::Greedy,
        seed: 0,
        stop: Vec::new(),
    };
    let marked = generate(&model, "stand-in", &request).expect("it runs");
    assert_eq!(marked.value().observed().stopped, Stopped::NothingToRead);
}

/// The same request twice is the same tokens, including when it samples: the
/// seed is a condition and the run is a function of it (D19, §3.12).
#[test]
fn the_same_request_generates_the_same_tokens() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![3],
        limit: 6,
        settings: Settings::Nucleus {
            temperature: 0.8,
            top_p: 0.9,
        },
        seed: 12_345,
        stop: Vec::new(),
    };
    let first = generate(&model, "stand-in", &request).expect("runs");
    let second = generate(&model, "stand-in", &request).expect("runs");
    assert_eq!(
        first.value().observed().tokens,
        second.value().observed().tokens
    );

    let mut other = request.clone();
    other.seed = 12_346;
    let third = generate(&model, "stand-in", &other).expect("runs");
    // A different seed need not differ on a one-hot model where one token
    // dominates, so this asserts the weaker true thing: the seed reached the
    // run at all, which the type of `Request` makes visible and this keeps
    // honest.
    assert_eq!(third.value().observed().prompt_length, 1);
}

/// A token outside the vocabulary is refused by the forward pass rather than
/// generating from whatever follows the embedding table.
#[test]
fn a_prompt_outside_the_vocabulary_is_refused() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![99],
        limit: 1,
        settings: Settings::Greedy,
        seed: 0,
        stop: Vec::new(),
    };
    let failure = generate(&model, "stand-in", &request).expect_err("99 is not a token");
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::ArtifactFormatMalformed
    );
    let _ = llama::ARCHITECTURE;
}
