use super::{Request, Stopped, generate};
use crate::llama::{Loaded, load};
use crate::sample::Settings;
use crate::{gguf, llama};

fn model() -> (Vec<u8>, Loaded) {
    let bytes = crate::llama::tests::one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let loaded = load(&file, &bytes).expect("loads");
    (bytes, loaded)
}

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

    let rendered = marked.degradation().to_string();
    assert!(rendered.contains("engine"), "{rendered}");

    let behaviour = marked.value();
    assert_eq!(behaviour.engine(), "stand-in");
    let generated = behaviour.observed();
    assert_eq!(generated.prompt_length, 1);
    assert_eq!(generated.stopped, Stopped::AtLimit);
    assert_eq!(generated.tokens, vec![2, 2, 2]);
}

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

#[test]
fn the_same_request_generates_the_same_tokens() {
    let (_, model) = model();
    let request = Request {
        prompt: vec![3],
        limit: 6,
        settings: Settings::Nucleus {
            temperature: 0.8,
            top_k: 0,
            top_p: 0.9,
            min_p: 0.0,
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
    assert_eq!(third.value().observed().prompt_length, 1);
}

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
