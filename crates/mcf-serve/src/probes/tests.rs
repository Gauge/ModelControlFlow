//! The probe's own logic, on a generator that answers however the test says.

use super::{Addressing, CHAT_TEMPLATE, Trial, chat_template};

/// `<|im_start|>` in the fixture vocabularies below.
/// Whether a turn the probe built carries the fixture's chat marker — the
/// way the tests tell the template addressing from raw, now that a turn is
/// markers and text for the answering engine to read (B-442).
fn wrapped(pieces: &[mcf_standin::tokenizer::Piece]) -> bool {
    pieces
        .iter()
        .any(|piece| matches!(piece, mcf_standin::tokenizer::Piece::Marker(marker) if marker == "<|im_start|>"))
}

/// A model file with a vocabulary and nothing else.
///
/// Written here rather than taken from the laboratory: `mcf-lab` depends on
/// this crate, and a dev-dependency the other way would invert the layering
/// the workspace check exists to hold (B-001). What the probe needs from a
/// model is its vocabulary, which is what this carries.
fn a_vocabulary(tokens: &[String], with_template: bool) -> Vec<u8> {
    a_file(tokens, with_template.then_some("{{ messages }}"), None)
}

/// A model file with the given tokens, template and end-of-turn token.
fn a_file(tokens: &[String], template: Option<&str>, ending: Option<u32>) -> Vec<u8> {
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

    if let Some(template) = template {
        let mut value = length(template.len()).to_vec();
        value.extend_from_slice(template.as_bytes());
        pairs.push(("tokenizer.chat_template", 8, value));
    }
    if let Some(ending) = ending {
        pairs.push((
            "tokenizer.ggml.eos_token_id",
            4,
            ending.to_le_bytes().to_vec(),
        ));
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

/// A vocabulary holding the given markers as real tokens.
///
/// A spelling that is not a token is text and cannot open anything (D46, F26),
/// so a probe about markers needs a file that actually holds them.
pub(super) fn with_markers(markers: &[&str]) -> Vec<u8> {
    let mut tokens = vec!["<s>", "\u{2581}a", "a"];
    tokens.extend_from_slice(markers);
    a_vocabulary(&with_bytes(&tokens), false)
}

/// A vocabulary with no chat tokens at all.
pub(super) fn plain() -> Vec<u8> {
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
pub(crate) fn chatml() -> Vec<u8> {
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
        // The marker's presence is how the test tells the addressings apart.
        &mut |pieces, _budget| {
            if wrapped(pieces) {
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
        &mut |pieces, _budget| {
            if wrapped(pieces) {
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
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let names: Vec<String> = super::addressings(&file, &tokens)
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names[0].contains("im_start"), "{names:?}");
    assert_eq!(names[1], "raw");

    let file = mcf_standin::gguf::parse(&plain()).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let names: Vec<String> = super::addressings(&file, &tokens)
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
        &mut |pieces, _budget| {
            if wrapped(pieces) {
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
        &mut |pieces, _budget| {
            if wrapped(pieces) {
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

/// A file whose claim holds costs one cheap question and one real one, not
/// fifteen. The search exists for the case where the claim does not hold, and
/// running it anyway would spend a context's worth of forward passes to learn
/// nothing.
///
/// The cheap one first is the instrument being asked whether it can answer at
/// all, which is worth a single token and was worth eight thousand before
/// (F44).
#[test]
fn a_context_that_holds_is_one_question() {
    let mut asked = Vec::new();
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        None,
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
            ceiling: 8191,
            accepted: 8191,
            because: None,
        }
    );
    assert_eq!(
        asked,
        vec![1, 8191],
        "the instrument, then the claim — and no search"
    );
}

/// A caller who asked for less than the file declares is asked exactly
/// that, and the report keeps the ceiling apart from the claim: a trial
/// that stopped where it was told to has not tested the declaration
/// (B-461).
#[test]
fn asking_for_less_asks_for_exactly_that() {
    let mut asked = Vec::new();
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        262_144,
        Some(4096),
        "test",
        &mut |length| {
            asked.push(length);
            Accepted::Read(length)
        },
    );
    let observed = probed
        .outcome
        .observed()
        .expect("the engine took the ceiling");
    assert_eq!(observed.declared, 262_144, "the claim is still the claim");
    assert_eq!(observed.ceiling, 4096, "the ceiling is what was asked for");
    assert_eq!(observed.accepted, 4096);
    assert_eq!(
        asked,
        vec![1, 4096],
        "the instrument, then the ceiling, and never the claim"
    );
}

/// A ceiling above the declaration is the declaration: the caller cannot
/// ask for more than the file has.
#[test]
fn a_ceiling_above_the_claim_is_the_claim() {
    assert_eq!(super::ceiling_of(8192, Some(1_000_000)), 8191);
    assert_eq!(super::ceiling_of(8192, Some(100)), 100);
    assert_eq!(super::ceiling_of(8192, None), 8191);
}

/// The projection is arithmetic on two stated figures, says it is a
/// projection, and puts a floor rather than a figure on the trial.
#[test]
fn a_projection_says_it_is_one() {
    let projection = super::Projection {
        sample: 512,
        nanos: 4_000_000_000,
        target: 262_143,
    };
    assert_eq!(projection.nanos_at_the_rate(), Some(2_047_992_187_500));
    let sentence = projection.sentence();
    assert!(sentence.starts_with("projected:"), "{sentence}");
    assert!(sentence.contains("512 identifiers took 4 s"), "{sentence}");
    assert!(sentence.contains("at least 34 min 7 s"), "{sentence}");
    assert!(sentence.contains("not a measurement"), "{sentence}");
    let empty = super::Projection {
        sample: 0,
        nanos: 1,
        target: 10,
    };
    assert_eq!(empty.nanos_at_the_rate(), None, "no rate from no sample");
}

/// Where the claim does not hold, the boundary is found exactly.
#[test]
fn the_boundary_is_found_where_it_is() {
    let ceiling = 2047;
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        None,
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
        probed.trials <= 15,
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
        None,
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
///
/// And it costs one token to learn. MCF's own engine is this engine, and the
/// first version of this probe sent it the whole declared context before
/// finding out — eight thousand forward passes to reach *could not tell*
/// (F44).
#[test]
fn an_engine_that_cannot_say_leaves_it_unknown() {
    let mut asked = Vec::new();
    let probed = usable_context(
        std::path::Path::new("/fixture"),
        8192,
        None,
        "test",
        &mut |length| {
            asked.push(length);
            Accepted::CouldNotTell("this engine does not say".to_owned())
        },
    );
    assert!(
        probed.outcome.observed().is_none(),
        "nothing was observed, so nothing may be reported as observed"
    );
    assert_eq!(
        asked,
        vec![1],
        "and it was learned from one token, not from a context's worth"
    );
}

/// What is applied must address the model exactly as the probe did.
///
/// The improvement M3's first exit criterion asks for is *attributable to a
/// named probe*, and it is only attributable if the thing applied is the thing
/// measured. A configuration that rebuilt the turn slightly differently —
/// another marker, a lost newline — would be a different addressing wearing
/// the probe's provenance, which is worse than no provenance at all (A21).
#[test]
fn what_is_applied_addresses_it_as_the_probe_did() {
    let bytes = chatml();
    let file = mcf_standin::gguf::parse(&bytes).expect("the fixture reads");
    let vocabulary =
        mcf_standin::tokenizer::Vocabulary::read(&file).expect("the fixture has a vocabulary");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("the fixture lists tokens");
    let candidates = super::addressings(&file, &tokens);
    let chosen = candidates
        .iter()
        .find(|candidate| candidate.name != "raw")
        .expect("the fixture declares a template");

    // What the probe sent, as the engine that answered read it (B-442).
    let measured = vocabulary
        .addressed(&chosen.wrap(super::QUESTION))
        .expect("the probe could assemble it");

    // The same thing, through the file a person's decision writes.
    let stored = crate::configured::Addressing {
        name: chosen.name.clone(),
        before: chosen.pieces_before.clone(),
        after: chosen.pieces_after.clone(),
        probe: super::CHAT_TEMPLATE.name.to_owned(),
        at: "2026-08-27T00:00:00Z".to_owned(),
        build: "0.1.0-m0".to_owned(),
        conditions: "test".to_owned(),
    };
    let value = stored.to_value();
    let read_back =
        crate::configured::Addressing::from_value(&value).expect("it survives the round trip");

    let mut pieces = read_back.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(
        super::QUESTION.to_owned(),
    ));
    pieces.extend(read_back.after.iter().cloned());
    let applied = vocabulary
        .addressed(&pieces)
        .expect("the stored addressing assembles");

    assert_eq!(
        applied, measured,
        "the turn a configuration builds must be the turn the probe measured, identifier for \
         identifier"
    );
}

use super::{Stopping, stop_conditions};

/// A model that ends its turns is reported by the longest one, and the budget
/// doubles rather than starting large.
#[test]
fn a_model_that_stops_is_reported_by_its_longest_turn() {
    let mut budgets = Vec::new();
    let probed = stop_conditions(
        std::path::Path::new("/fixture"),
        2,
        32,
        1024,
        32,
        "test",
        &mut |_question, budget| {
            budgets.push(budget);
            if budget >= 128 {
                Trial::Stopped { after: 100 }
            } else {
                Trial::RanOut
            }
        },
    );
    let observed = probed.outcome.observed().expect("both turns ended");
    assert_eq!(
        observed,
        &Stopping {
            longest: 100,
            stopped: 2,
            of: 2,
            ceiling: 128,
            default_budget: 32,
        }
    );
    assert_eq!(
        budgets,
        vec![32, 64, 128, 32, 64, 128],
        "it doubles from the floor for each trial rather than starting large (B49)"
    );
}

/// A model that never stops within the ceiling is *not* reported as one that
/// never stops.
///
/// The claim the trials support is *not within this many tokens*, and the
/// number travels so a reader can judge whether it was large enough (A7). It
/// also names the likelier cause, because a model addressed wrongly does not
/// stop at any budget (F38).
#[test]
fn never_stopping_names_the_ceiling_and_not_the_model() {
    let probed = stop_conditions(
        std::path::Path::new("/fixture"),
        2,
        32,
        128,
        32,
        "test",
        &mut |_question, _budget| Trial::RanOut,
    );
    assert!(probed.outcome.observed().is_none());
    let said = format!("{:?}", probed.outcome);
    assert!(said.contains("128"), "the ceiling has to be named: {said}");
    assert!(
        said.contains("chat-template"),
        "and the likelier cause: {said}"
    );
}

/// The ceiling is a ceiling: the budget never exceeds it.
#[test]
fn the_budget_never_passes_the_ceiling() {
    let mut budgets = Vec::new();
    let _probed = stop_conditions(
        std::path::Path::new("/fixture"),
        1,
        32,
        100,
        32,
        "test",
        &mut |_question, budget| {
            budgets.push(budget);
            Trial::RanOut
        },
    );
    assert!(
        budgets.iter().all(|budget| *budget <= 100),
        "a doubling that overshot the ceiling would spend past what was asked: {budgets:?}"
    );
    assert_eq!(budgets, vec![32, 64, 100]);
}

/// A template that names a role in order to *rename* it must not yield the
/// name it renamed.
///
/// gemma's template mentions `assistant` exactly once and does it to map it to
/// `model`. A bag-of-words read produced both as candidates, the probe could
/// not tell them apart because *ending a turn* does not, and the tie was
/// reported as though the file were ambiguous when it is explicit (F48,
/// B-375).
#[test]
fn a_role_that_is_renamed_is_not_a_candidate() {
    let template = "{%- if (message['role'] == 'assistant') -%}\n                    {%- set role = \"model\" -%}\n                    {%- else -%}{%- set role = message['role'] -%}{%- endif -%}\n                    {{ '<start_of_turn>' + role + '\n' }}";
    assert_eq!(
        super::assigned_roles(template),
        vec!["model".to_owned()],
        "the word the template writes out, not the word it tests for"
    );
}

/// The opener is the marker the template writes a role after, not the first
/// marker that is not the closer: Qwen3-Coder's template says `[]` before
/// it says `<|im_start|>`, and its vocabulary holds `[]` as a token (F160).
#[test]
fn the_opener_is_the_marker_a_role_follows() {
    let template = "{%- set ns = namespace(tools=[]) %}{% for message in messages %}{{ '<|im_start|>' + message['role'] + '\n' + message['content'] + '<|im_end|>\n' }}{% endfor %}";
    let tokens = with_bytes(&["<s>", "\u{2581}a", "a", "[]", "<|im_start|>", "<|im_end|>"]);
    let file =
        mcf_standin::gguf::parse(&a_file(&tokens, Some(template), Some(5))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let found = super::from_template(&file, &tokens);
    let names: Vec<&str> = found.iter().map(|held| held.name.as_str()).collect();
    assert_eq!(names, vec!["im_start…im_end as assistant"], "{names:?}");
    assert_eq!(
        found[0].pieces_before[0],
        mcf_standin::tokenizer::Piece::Marker("<|im_start|>".to_owned())
    );
    assert!(super::opens_a_role(template, "<|im_start|>"));
    assert!(!super::opens_a_role(template, "[]"));
    // A coder model's own (F160): the role is a word away from the marker on
    // every line that writes one, and `[]` is a line away from *system*.
    let coder = "{%- if tools is defined %}\n    {%- set tools = [] %}\n{%- endif %}\n\n{%- if system_message is defined %}\n    {{- \"<|im_start|>system\\n\" + system_message }}\n{%- else %}{{ '<|im_start|>' + message.role + '\\n' }}";
    assert!(super::opens_a_role(coder, "<|im_start|>"));
    assert!(!super::opens_a_role(coder, "[]"));
}

/// A template that writes no end-of-turn marker is read by its roles: the
/// marker spelled *user* opens, the one spelled *assistant* follows the
/// question, and where `</think>` is a token the template writes, a second
/// candidate closes the thinking first (F171).
#[test]
fn a_template_whose_markers_are_the_roles_is_read_by_them() {
    let template = "[gMASK]<sop>{% for m in messages %}{%- if m.role == 'user' -%}<|user|>{{ m.content }}{%- elif m.role == 'assistant' -%}<|assistant|>{{ '</think>' }}{{ m.content }}{%- endif -%}{%- endfor -%}<|assistant|>";
    let tokens = with_bytes(&[
        "<|endoftext|>",
        "\u{2581}a",
        "a",
        "[gMASK]",
        "<|user|>",
        "<|assistant|>",
        "</think>",
    ]);
    // The file's ending is a token the template never writes.
    let file =
        mcf_standin::gguf::parse(&a_file(&tokens, Some(template), Some(0))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let found = super::from_template(&file, &tokens);
    let names: Vec<&str> = found.iter().map(|held| held.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["user…assistant", "user…assistant, thinking closed"],
        "{names:?}"
    );
    assert_eq!(
        found[0].shown("hello"),
        "<|user|>hello<|assistant|>",
        "the turn boundary is the next role's marker, nothing between"
    );
    assert_eq!(
        found[1].shown("hello"),
        "<|user|>hello<|assistant|></think>"
    );

    // The same template on a vocabulary that spells no role as a marker
    // yields nothing — the shape cannot be sent as itself (F37).
    let bare = with_bytes(&["<|endoftext|>", "\u{2581}a", "a", "[gMASK]"]);
    let file = mcf_standin::gguf::parse(&a_file(&bare, Some(template), Some(0))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    assert!(super::from_template(&file, &tokens).is_empty());
}

/// A template that emits the role it was given assigns nothing, and the
/// ordinary names stay candidates for the model to decide between.
#[test]
fn a_template_that_assigns_nothing_yields_nothing() {
    let chatml = "{% for message in messages %}                  {{'<|im_start|>' + message['role'] + '\n' + message['content'] }}                  {% endfor %}";
    assert!(
        super::assigned_roles(chatml).is_empty(),
        "nothing is assigned, so nothing is claimed"
    );
}

/// Single quotes count, and the first assignment is not the only one.
#[test]
fn both_quotings_are_read_and_every_assignment_is_kept() {
    let template = "{%- set role = 'model' -%}{%- set role = \"agent\" -%}";
    assert_eq!(
        super::assigned_roles(template),
        vec!["model".to_owned(), "agent".to_owned()],
        "a template naming two is ambiguous and both are candidates — that is a tie MCF has \
         evidence for, unlike the one it invented"
    );
}

/// A `set` of something other than the role is not a role.
#[test]
fn only_the_role_variable_is_read() {
    let template = "{%- set first_user_prefix = \"model\" -%}";
    assert!(
        super::assigned_roles(template).is_empty(),
        "an assignment to another name says nothing about the role"
    );
}
