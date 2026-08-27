//! The probe's own logic, on a generator that answers however the test says.

use super::{Addressing, CHAT_TEMPLATE, Trial, chat_template};

/// A model file with a vocabulary and nothing else.
///
/// Written here rather than taken from the laboratory: `mcf-lab` depends on
/// this crate, and a dev-dependency the other way would invert the layering
/// the workspace check exists to hold (B-001). What the probe needs from a
/// model is its vocabulary, which is what this carries.
fn a_vocabulary(tokens: &[&str], with_template: bool) -> Vec<u8> {
    // Every token that looks like a marker is USER_DEFINED, which is what
    // makes it tokenize as itself — a real vocabulary marks them and the
    // probe's marker check depends on it (F26, F37).
    fn length(value: usize) -> [u8; 8] {
        (value as u64).to_le_bytes()
    }
    let mut pairs: Vec<(&str, u32, Vec<u8>)> = Vec::new();

    let mut model = length("llama".len()).to_vec();
    model.extend_from_slice(b"llama");
    pairs.push(("general.architecture", 8, model));

    let mut kind = length("llama".len()).to_vec();
    kind.extend_from_slice(b"llama");
    pairs.push(("tokenizer.ggml.model", 8, kind));

    let mut list = 8_u32.to_le_bytes().to_vec();
    list.extend_from_slice(&length(tokens.len()));
    for token in tokens {
        list.extend_from_slice(&length(token.len()));
        list.extend_from_slice(token.as_bytes());
    }
    pairs.push(("tokenizer.ggml.tokens", 9, list));

    let mut scores = 6_u32.to_le_bytes().to_vec();
    scores.extend_from_slice(&length(tokens.len()));
    for _ in tokens {
        scores.extend_from_slice(&0.0_f32.to_le_bytes());
    }
    pairs.push(("tokenizer.ggml.scores", 9, scores));

    let mut types = 5_u32.to_le_bytes().to_vec();
    types.extend_from_slice(&length(tokens.len()));
    for token in tokens {
        let kind: i32 = if token.starts_with('<') && token.len() > 3 {
            4
        } else {
            1
        };
        types.extend_from_slice(&kind.to_le_bytes());
    }
    pairs.push(("tokenizer.ggml.token_type", 9, types));

    if with_template {
        let template = "{{ messages }}";
        let mut value = length(template.len()).to_vec();
        value.extend_from_slice(template.as_bytes());
        pairs.push(("tokenizer.chat_template", 8, value));
    }

    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(0));
    out.extend_from_slice(&length(pairs.len()));
    for (key, kind, value) in &pairs {
        out.extend_from_slice(&length(key.len()));
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(value);
    }
    out
}

/// A vocabulary with no chat tokens at all.
fn plain() -> Vec<u8> {
    a_vocabulary(&["<s>", "\u{2581}a", "a"], false)
}

/// A vocabulary that can be addressed as `ChatML`.
fn chatml() -> Vec<u8> {
    a_vocabulary(
        &["<s>", "\u{2581}a", "a", "<|im_start|>", "<|im_end|>"],
        true,
    )
}

/// An addressing wraps the question and nothing else.
#[test]
fn an_addressing_wraps_the_question() {
    let chatml = Addressing {
        name: "chatml",
        before: "<|im_start|>user\n".to_owned(),
        after: "<|im_end|>\n<|im_start|>assistant\n".to_owned(),
    };
    assert_eq!(
        chatml.wrap("hello"),
        "<|im_start|>user\nhello<|im_end|>\n<|im_start|>assistant\n"
    );
}

/// A file that is not a model is inconclusive, not negative (D42).
#[test]
fn an_unreadable_model_is_inconclusive() {
    let probed = chat_template(
        std::path::Path::new("/nowhere"),
        b"not a gguf",
        3,
        8,
        "test",
        &mut |_prompt, _budget| Trial::Stopped,
    );
    assert!(probed.outcome.is_inconclusive());
    assert_eq!(probed.trials, 0);
    assert_eq!(probed.method.name, CHAT_TEMPLATE.name);
}

/// A trial that could not be told apart is inconclusive, says which addressing
/// it was on *and why* — the probe never turns *could not tell* into *does not
/// work* (D42, A7).
#[test]
fn a_trial_that_does_not_run_is_inconclusive() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &plain(),
        2,
        4,
        "test",
        &mut |_prompt, _budget| Trial::CouldNotTell("the engine did not say".to_owned()),
    );
    match &probed.outcome {
        mcf_core::probe::Outcome::Inconclusive { because } => {
            assert!(because.contains("the engine did not say"), "{because}");
        }
        mcf_core::probe::Outcome::Observed(_) => panic!("a trial that did not run decided"),
    }
}

/// Nothing stopping anywhere is *could not tell*, because the budget may be
/// the reason — §3.18's third state, and the distinction D42 turns on.
#[test]
fn nothing_stopping_anywhere_is_inconclusive_rather_than_negative() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &plain(),
        2,
        4,
        "test",
        &mut |_prompt, _budget| Trial::RanOut,
    );
    match &probed.outcome {
        mcf_core::probe::Outcome::Inconclusive { because } => {
            assert!(because.contains("larger budget"), "{because}");
        }
        mcf_core::probe::Outcome::Observed(_) => {
            panic!("no addressing stopping was read as an answer");
        }
    }
}

/// The probe reports what stopped, and its cost is in tokens (B49).
#[test]
fn what_stopped_is_reported_with_what_it_cost() {
    // No chat tokens, so `raw` is the only candidate: a model that can only be
    // addressed one way is answered with that way rather than with an invented
    // alternative.
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &plain(),
        3,
        5,
        "test",
        &mut |_prompt, _budget| Trial::Stopped,
    );
    let observed = probed
        .outcome
        .observed()
        .expect("every trial stopped, so the probe decided");
    assert_eq!(observed.best, "raw");
    assert_eq!(observed.of, 3);
    assert_eq!(probed.trials, 3);
    assert_eq!(probed.tokens, 15, "three trials of five tokens");
}

/// A vocabulary that can be addressed two ways, where only one stops: the
/// probe answers with the one the *model* ended a turn under, and the tie-break
/// never invents a wrapping for a model that does not need one.
#[test]
fn the_addressing_the_model_stops_under_is_the_one_reported() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        4,
        6,
        "test",
        &mut |prompt, _budget| {
            if prompt.contains("<|im_start|>") {
                Trial::Stopped
            } else {
                Trial::RanOut
            }
        },
    );
    let observed = probed.outcome.observed().expect("one addressing stopped");
    assert_eq!(observed.best, "chatml");
    assert!(observed.declared_a_template, "the file did declare one");
    assert_eq!(
        observed.stopped,
        vec![("chatml".to_owned(), 4), ("raw".to_owned(), 0)]
    );

    // And the other way round: when raw is what stops, raw is what is
    // reported, template or no template.
    let raw_stops = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        4,
        6,
        "test",
        &mut |prompt, _budget| {
            if prompt.contains("<|im_start|>") {
                Trial::RanOut
            } else {
                Trial::Stopped
            }
        },
    );
    let observed = raw_stops.outcome.observed().expect("raw stopped");
    assert_eq!(observed.best, "raw");
}

/// Every candidate is drawn from the model's own vocabulary.
#[test]
fn addressings_come_from_the_vocabulary_not_from_a_family() {
    let file = mcf_standin::gguf::parse(&chatml()).expect("a model");
    let names: Vec<&str> = super::addressings(&file).iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["chatml", "raw"]);

    let file = mcf_standin::gguf::parse(&plain()).expect("a model");
    let names: Vec<&str> = super::addressings(&file).iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["raw"], "no chat tokens, no chat addressing");
}
