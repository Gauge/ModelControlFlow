//! The probe's own logic, on a generator that answers however the test says.

use super::{Addressing, CHAT_TEMPLATE, Trial, chat_template};

/// `<|im_start|>` in the fixture vocabularies below.
const MARKER: usize = 3;

/// A model file with a vocabulary and nothing else.
///
/// Written here rather than taken from the laboratory: `mcf-lab` depends on
/// this crate, and a dev-dependency the other way would invert the layering
/// the workspace check exists to hold (B-001). What the probe needs from a
/// model is its vocabulary, which is what this carries.
fn a_vocabulary(tokens: &[String], with_template: bool) -> Vec<u8> {
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
        let kind: i32 = if token.starts_with("<|") || token.starts_with("<s") {
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
    a_vocabulary(&with_bytes(&["<s>", "\u{2581}a", "a"]), false)
}

/// A vocabulary that can spell anything.
///
/// Byte-fallback tokens, because the probe's question is real English and a
/// vocabulary of three pieces cannot represent it — without these the probe
/// correctly reports that it could not assemble a turn, which is true and not
/// what these tests are about.
fn with_bytes(tokens: &[&str]) -> Vec<String> {
    let mut all: Vec<String> = tokens.iter().map(|token| (*token).to_owned()).collect();
    for byte in 0..=u8::MAX {
        all.push(format!("<0x{byte:02X}>"));
    }
    all
}

/// A vocabulary that can be addressed as `ChatML`.
fn chatml() -> Vec<u8> {
    a_vocabulary(
        &with_bytes(&["<s>", "\u{2581}a", "a", "<|im_start|>", "<|im_end|>"]),
        true,
    )
}

/// An addressing wraps the question and nothing else — shown as text for a
/// reader, sent as identifiers.
#[test]
fn an_addressing_wraps_the_question() {
    let chatml = Addressing {
        name: "im_start…im_end as assistant".to_owned(),
        pieces_before: vec![
            mcf_standin::tokenizer::Piece::Marker("<|im_start|>".to_owned()),
            mcf_standin::tokenizer::Piece::Text("user\n".to_owned()),
        ],
        pieces_after: vec![
            mcf_standin::tokenizer::Piece::Marker("<|im_end|>".to_owned()),
            mcf_standin::tokenizer::Piece::Text("\n".to_owned()),
            mcf_standin::tokenizer::Piece::Marker("<|im_start|>".to_owned()),
            mcf_standin::tokenizer::Piece::Text("assistant\n".to_owned()),
        ],
    };
    assert_eq!(
        chatml.shown("hello"),
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
        &mut |_identifiers, _budget| Trial::Stopped { after: 4 },
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
        &mut |_identifiers, _budget| Trial::CouldNotTell("the engine did not say".to_owned()),
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
        &mut |_identifiers, _budget| Trial::RanOut,
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
        &mut |_identifiers, _budget| Trial::Stopped { after: 4 },
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
        // The marker is token 3 in this fixture — `<s>`, `▁a`, `a`, then
        // `<|im_start|>`. Its presence is how the test tells the addressings
        // apart, now that one is identifiers rather than text.
        &mut |identifiers, _budget| {
            if identifiers.contains(&MARKER) {
                Trial::Stopped { after: 4 }
            } else {
                Trial::RanOut
            }
        },
    );
    let observed = probed.outcome.observed().expect("one addressing stopped");
    assert!(observed.best.contains("im_start"), "{}", observed.best);
    assert!(observed.declared_a_template, "the file did declare one");
    assert_eq!(
        observed.stopped.len(),
        2,
        "one addressing from the template, and raw"
    );

    // And the other way round: when raw is what stops, raw is what is
    // reported, template or no template.
    let raw_stops = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        4,
        6,
        "test",
        &mut |identifiers, _budget| {
            if identifiers.contains(&MARKER) {
                Trial::RanOut
            } else {
                Trial::Stopped { after: 4 }
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
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).expect("a vocabulary");
    let names: Vec<String> = super::addressings(&file, &vocabulary)
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names[0].contains("im_start"), "{names:?}");
    assert_eq!(names[1], "raw");

    let file = mcf_standin::gguf::parse(&plain()).expect("a model");
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).expect("a vocabulary");
    let names: Vec<String> = super::addressings(&file, &vocabulary)
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(
        names,
        vec!["raw".to_owned()],
        "no chat tokens, no chat addressing"
    );
}

/// The observation F38 corrected: a model that ends its turn having said
/// *nothing* has refused to speak, and scoring that as a finished turn made
/// the probe report the exact opposite of the truth. Here the raw addressing
/// goes silent every time and the template addressing talks past the budget,
/// which is what a small instruct model really did — the probe must not call raw best.
#[test]
fn ending_a_turn_having_said_nothing_is_not_ending_a_turn() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        5,
        6,
        "test",
        &mut |identifiers, _budget| {
            if identifiers.contains(&MARKER) {
                Trial::RanOut
            } else {
                Trial::Stopped { after: 0 }
            }
        },
    );
    assert!(
        probed.outcome.observed().is_none(),
        "silence is not a finished turn, so nothing was observed"
    );
    let said = format!("{:?}", probed.outcome);
    assert!(
        said.contains("said nothing"),
        "the silence is the finding, and has to be in the reason: {said}"
    );
}

/// And the pair of it: when the model *does* speak before stopping, that
/// addressing is the one reported — the fix must not refuse everything.
#[test]
fn speaking_then_stopping_is_what_counts() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        5,
        6,
        "test",
        &mut |identifiers, _budget| {
            if identifiers.contains(&MARKER) {
                Trial::Stopped { after: 9 }
            } else {
                Trial::Stopped { after: 0 }
            }
        },
    );
    let observed = probed
        .outcome
        .observed()
        .expect("the addressing it spoke under is the one that counts");
    assert!(observed.best.contains("im_start"), "{}", observed.best);
    assert_eq!(
        observed.silent.iter().find(|(name, _)| name == "raw"),
        Some(&("raw".to_owned(), 5)),
        "and the silence is kept, not discarded"
    );
}

use super::{Accepted, Context, usable_context};

/// A file whose claim holds costs one trial, not fifteen. The search exists
/// for the case where the claim does not hold, and running it anyway would
/// spend a context's worth of forward passes to learn nothing.
#[test]
fn a_context_that_holds_is_one_question() {
    let mut asked = Vec::new();
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        "test",
        &mut |length| {
            asked.push(length);
            Accepted::Read(length)
        },
    );
    let observed = probed
        .outcome
        .observed()
        .expect("the engine took what the file declares");
    assert_eq!(
        observed,
        &Context {
            declared: 8192,
            accepted: 8191,
            because: None,
        }
    );
    assert_eq!(asked, vec![8191], "one trial, at the declared length");
}

/// Where the claim does not hold, the boundary is found exactly.
#[test]
fn the_boundary_is_found_where_it_is() {
    let ceiling = 2047;
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        "test",
        &mut |length| {
            if length <= ceiling {
                Accepted::Read(length)
            } else {
                Accepted::Refused("exceeds the available context size".to_owned())
            }
        },
    );
    let observed = probed.outcome.observed().expect("a boundary was found");
    assert_eq!(observed.accepted, ceiling, "the largest length that worked");
    assert_eq!(observed.declared, 8192);
    assert!(
        observed
            .because
            .as_deref()
            .is_some_and(|said| said.contains("context size")),
        "the engine's own reason travels with the divergence (A1)"
    );
    assert!(
        probed.trials <= 14,
        "a halving, not a walk: {}",
        probed.trials
    );
}

/// A prompt read shorter than it was sent is the failure this probe is for,
/// and it must not be mistaken for a shorter context that was honestly
/// reported.
#[test]
fn silent_truncation_is_caught_and_named() {
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        "test",
        &mut |length| Accepted::Read(length.min(1000)),
    );
    let observed = probed.outcome.observed().expect("a boundary was found");
    assert_eq!(observed.accepted, 1000);
    assert!(
        observed
            .because
            .as_deref()
            .is_some_and(|said| said.contains("silence")),
        "a prompt quietly shortened has to be named as that: {:?}",
        observed.because
    );
}

/// An engine that cannot say how much it read leaves the question open. It is
/// not *the context is short* and not *the context is fine* (A7, D42).
#[test]
fn an_engine_that_cannot_say_leaves_it_unknown() {
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        "test",
        &mut |_length| Accepted::CouldNotTell("this engine does not say".to_owned()),
    );
    assert!(
        probed.outcome.observed().is_none(),
        "nothing was observed, so nothing may be reported as observed"
    );
}
