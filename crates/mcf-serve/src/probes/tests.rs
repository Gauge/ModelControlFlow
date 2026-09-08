use super::{Addressing, CHAT_TEMPLATE, Trial, chat_template};

fn wrapped(pieces: &[mcf_standin::tokenizer::Piece]) -> bool {
    pieces
        .iter()
        .any(|piece| matches!(piece, mcf_standin::tokenizer::Piece::Marker(marker) if marker == "<|im_start|>"))
}

fn a_vocabulary(tokens: &[String], with_template: bool) -> Vec<u8> {
    a_file(tokens, with_template.then_some("{{ messages }}"), None)
}

fn a_file(tokens: &[String], template: Option<&str>, ending: Option<u32>) -> Vec<u8> {
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

pub(super) fn with_markers(markers: &[&str]) -> Vec<u8> {
    let mut tokens = vec!["<s>", "\u{2581}a", "a"];
    tokens.extend_from_slice(markers);
    a_vocabulary(&with_bytes(&tokens), false)
}

pub(super) fn plain() -> Vec<u8> {
    a_vocabulary(&with_bytes(&["<s>", "\u{2581}a", "a"]), false)
}

fn with_bytes(tokens: &[&str]) -> Vec<String> {
    let mut all: Vec<String> = tokens.iter().map(|token| (*token).to_owned()).collect();
    for byte in 0..=u8::MAX {
        all.push(format!("<0x{byte:02X}>"));
    }
    all
}

pub(crate) fn chatml() -> Vec<u8> {
    a_vocabulary(
        &with_bytes(&["<s>", "\u{2581}a", "a", "<|im_start|>", "<|im_end|>"]),
        true,
    )
}

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

#[test]
fn an_unreadable_model_is_inconclusive() {
    let probed = chat_template(
        std::path::Path::new("/nowhere"),
        b"not a gguf",
        3,
        8,
        "test",
        &mut |_identifiers, _budget| Trial::Stopped {
            after: 4,
            before: None,
        },
    );
    assert!(probed.outcome.is_inconclusive());
    assert_eq!(probed.trials, 0);
    assert_eq!(probed.method.name, CHAT_TEMPLATE.name);
}

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

#[test]
fn what_stopped_is_reported_with_what_it_cost() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &plain(),
        3,
        5,
        "test",
        &mut |_identifiers, _budget| Trial::Stopped {
            after: 4,
            before: None,
        },
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

#[test]
fn the_addressing_the_model_stops_under_is_the_one_reported() {
    let probed = chat_template(
        std::path::Path::new("/fixture"),
        &chatml(),
        4,
        6,
        "test",
        &mut |pieces, _budget| {
            if wrapped(pieces) {
                Trial::Stopped {
                    after: 4,
                    before: None,
                }
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
                Trial::Stopped {
                    after: 4,
                    before: None,
                }
            }
        },
    );
    let observed = raw_stops.outcome.observed().expect("raw stopped");
    assert_eq!(observed.best, "raw");
}

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
                Trial::Stopped {
                    after: 0,
                    before: None,
                }
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
                Trial::Stopped {
                    after: 9,
                    before: None,
                }
            } else {
                Trial::Stopped {
                    after: 0,
                    before: None,
                }
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

#[test]
fn a_ceiling_above_the_claim_is_the_claim() {
    assert_eq!(super::ceiling_of(8192, Some(1_000_000)), 8191);
    assert_eq!(super::ceiling_of(8192, Some(100)), 100);
    assert_eq!(super::ceiling_of(8192, None), 8191);
}

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

    let measured = vocabulary
        .addressed(&chosen.wrap(super::QUESTION))
        .expect("the probe could assemble it");

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
                Trial::Stopped {
                    after: 100,
                    before: None,
                }
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
            before: None,
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

#[test]
fn a_turn_that_thinks_is_reported_by_its_share_before_the_answer() {
    let mut turns = [
        Trial::Stopped {
            after: 40,
            before: Some(30),
        },
        Trial::Stopped {
            after: 60,
            before: None,
        },
        Trial::Stopped {
            after: 50,
            before: Some(45),
        },
    ]
    .into_iter();
    let probed = stop_conditions(
        std::path::Path::new("/fixture"),
        3,
        64,
        64,
        32,
        "test",
        &mut |_question, _budget| turns.next().unwrap_or(Trial::RanOut),
    );
    let observed = probed.outcome.observed().expect("all three ended");
    assert_eq!(observed.longest, 60);
    assert_eq!(
        observed.before,
        Some(45),
        "the most any turn spent before its answer, whichever turn that was"
    );
}

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

#[test]
fn a_role_that_is_renamed_is_not_a_candidate() {
    let template = "{%- if (message['role'] == 'assistant') -%}\n                    {%- set role = \"model\" -%}\n                    {%- else -%}{%- set role = message['role'] -%}{%- endif -%}\n                    {{ '<start_of_turn>' + role + '\n' }}";
    assert_eq!(
        super::assigned_roles(template),
        vec!["model".to_owned()],
        "the word the template writes out, not the word it tests for"
    );
}

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
    let coder = "{%- if tools is defined %}\n    {%- set tools = [] %}\n{%- endif %}\n\n{%- if system_message is defined %}\n    {{- \"<|im_start|>system\\n\" + system_message }}\n{%- else %}{{ '<|im_start|>' + message.role + '\\n' }}";
    assert!(super::opens_a_role(coder, "<|im_start|>"));
    assert!(!super::opens_a_role(coder, "[]"));
}

fn tokens_of_glm() -> Vec<String> {
    with_bytes(&[
        "<|endoftext|>",
        "\u{2581}a",
        "a",
        "[gMASK]",
        "<|user|>",
        "<|assistant|>",
        "<think>",
        "</think>",
    ])
}

#[test]
fn a_template_whose_markers_are_the_roles_is_read_by_them() {
    let template = "[gMASK]<sop>{% for m in messages %}{%- if m.role == 'user' -%}<|user|>{{ m.content }}{%- elif m.role == 'assistant' -%}<|assistant|>{{ '</think>' }}{{ m.content }}{%- endif -%}{%- endfor -%}<|assistant|>{{ '<think>' }}";
    let tokens = tokens_of_glm();
    let file =
        mcf_standin::gguf::parse(&a_file(&tokens, Some(template), Some(0))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let found = super::from_template(&file, &tokens);
    let names: Vec<&str> = found.iter().map(|held| held.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["user…assistant, thinking open"],
        "the form the template writes unswitched, and not the bare role: {names:?}"
    );
    assert_eq!(
        found[0].shown("hello"),
        "<|user|>hello<|assistant|><think>",
        "the turn boundary is the next role's marker, nothing between"
    );

    let closing = "{% for m in messages %}{%- if m.role == 'user' -%}<|user|>{{ m.content }}{%- elif m.role == 'assistant' -%}<|assistant|>{{ '</think>' }}{{ m.content }}{%- endif -%}{%- endfor -%}<|assistant|>{{ '</think>' }}";
    let file = mcf_standin::gguf::parse(&a_file(&tokens_of_glm(), Some(closing), Some(0)))
        .expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let found = super::from_template(&file, &tokens);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "user…assistant, thinking closed");
    assert_eq!(
        found[0].shown("hello"),
        "<|user|>hello<|assistant|></think>"
    );

    let plain = "{% for m in messages %}{%- if m.role == 'user' -%}<|user|>{{ m.content }}{%- elif m.role == 'assistant' -%}<|assistant|>{{ m.content }}{%- endif -%}{%- endfor -%}<|assistant|>";
    let file =
        mcf_standin::gguf::parse(&a_file(&tokens_of_glm(), Some(plain), Some(0))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    let found = super::from_template(&file, &tokens);
    let names: Vec<&str> = found.iter().map(|held| held.name.as_str()).collect();
    assert_eq!(names, vec!["user…assistant"], "{names:?}");
    assert_eq!(found[0].shown("hello"), "<|user|>hello<|assistant|>");

    let bare = with_bytes(&["<|endoftext|>", "\u{2581}a", "a", "[gMASK]"]);
    let file = mcf_standin::gguf::parse(&a_file(&bare, Some(template), Some(0))).expect("a model");
    let tokens = mcf_standin::tokenizer::Tokens::read(&file).expect("a token list");
    assert!(super::from_template(&file, &tokens).is_empty());
}

#[test]
fn a_template_that_assigns_nothing_yields_nothing() {
    let chatml = "{% for message in messages %}                  {{'<|im_start|>' + message['role'] + '\n' + message['content'] }}                  {% endfor %}";
    assert!(
        super::assigned_roles(chatml).is_empty(),
        "nothing is assigned, so nothing is claimed"
    );
}

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

#[test]
fn only_the_role_variable_is_read() {
    let template = "{%- set first_user_prefix = \"model\" -%}";
    assert!(
        super::assigned_roles(template).is_empty(),
        "an assignment to another name says nothing about the role"
    );
}
