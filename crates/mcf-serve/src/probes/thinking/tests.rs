#![allow(clippy::panic, clippy::expect_used)]

use super::*;
use crate::probes::tests::plain;

#[test]
fn a_marker_closes_the_way_its_bracket_says() {
    assert_eq!(closing_form("<think>").as_deref(), Some("</think>"));
    assert_eq!(closing_form("[THINK]").as_deref(), Some("[/THINK]"));
    assert_eq!(closing_form("<|channel|>").as_deref(), Some("</|channel|>"));

    assert_eq!(closing_form("</think>"), None);
    assert_eq!(closing_form("<>"), None);
    assert_eq!(closing_form("think"), None);
    assert_eq!(closing_form(""), None);
    assert_eq!(closing_form("<think]"), None);
}

#[test]
fn a_file_with_no_marker_decides_nothing() {
    let mut ask = |_: usize| (Trial::RanOut, String::new());
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &plain(),
        3,
        200,
        "a test",
        None,
        &mut ask,
    );
    let Outcome::Inconclusive { because } = &probed.outcome else {
        panic!("a file with no paired marker must not report a spend");
    };
    assert!(
        because.contains("could not") || because.contains("no marker of any kind"),
        "{because}"
    );
}

#[test]
fn a_file_that_is_not_a_model_decides_nothing() {
    let mut ask = |_: usize| (Trial::RanOut, String::new());
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        b"not a model at all",
        1,
        200,
        "a test",
        None,
        &mut ask,
    );
    assert!(matches!(probed.outcome, Outcome::Inconclusive { .. }));
}

#[test]
fn opened_and_never_closed_is_not_counted_as_closed() {
    let pair = ("<think>".to_owned(), "</think>".to_owned());
    let said = "<think>working on it and still going".to_owned();
    let counted = count_over(&[pair], &[said]);
    assert_eq!(counted, (1, 0), "opened once, closed never");
}

#[test]
fn a_closed_marker_is_measured_by_what_is_inside_it() {
    let pair = ("<think>".to_owned(), "</think>".to_owned());
    let said = "<think>one two three four</think>The answer is 21.".to_owned();
    assert_eq!(count_over(&[pair], &[said]), (1, 1));
}

#[test]
fn the_budget_is_set_by_the_longest_turn_not_the_mean() {
    let mut said = [
        "<think>a b</think>done".to_owned(),
        "<think>a b c d e f</think>done".to_owned(),
    ]
    .into_iter();
    let mut ask = move |_: usize| {
        (
            Trial::Stopped {
                after: 10,
                before: None,
            },
            said.next().unwrap_or_default(),
        )
    };
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &thinking_model(),
        2,
        200,
        "a test",
        None,
        &mut ask,
    );
    match probed.outcome {
        Outcome::Observed(spends) => {
            assert_eq!(spends.opened, 2);
            assert_eq!(spends.closed, 2);
            assert_eq!(
                spends.longest_inside, 6,
                "the longest inside, not the mean of two and six"
            );
            assert_eq!(spends.used.as_deref(), Some("<think>"));
        }
        other @ Outcome::Inconclusive { .. } => panic!("expected an observation, got {other:?}"),
    }
}

fn count_over(pairs: &[(String, String)], answers: &[String]) -> (usize, usize) {
    let mut opened = 0;
    let mut closed = 0;
    for said in answers {
        let Some((marker, closing)) = pairs.iter().find(|(m, _)| said.contains(m.as_str())) else {
            continue;
        };
        opened += 1;
        if let Some((_, rest)) = said.split_once(marker.as_str())
            && rest.contains(closing.as_str())
        {
            closed += 1;
        }
    }
    (opened, closed)
}

fn thinking_model() -> Vec<u8> {
    crate::probes::tests::with_markers(&["<think>", "</think>"])
}

#[test]
fn a_marker_that_cannot_be_paired_is_still_reported() {
    let bytes = crate::probes::tests::with_markers(&["<|channel|>", "<think>", "</think>"]);
    let file = gguf::parse(&bytes).expect("the fixture parses");
    let tokens = Tokens::read(&file).expect("the fixture lists tokens");

    let paired = pairs(&file, &tokens);
    assert!(
        paired.iter().any(|(marker, _)| marker == "<think>"),
        "a marker whose closing form is present is a pair: {paired:?}"
    );
    let could_not = unpairable(&file, &tokens);
    assert!(
        could_not.iter().any(|marker| marker == "<|channel|>"),
        "a marker whose closing form is absent is reported: {could_not:?}"
    );
    assert!(!could_not.iter().any(|marker| marker == "</think>"));
    assert!(!paired.iter().any(|(marker, _)| marker == "</think>"));
}

#[test]
fn a_file_whose_markers_all_fail_to_pair_says_how_many() {
    let bytes = crate::probes::tests::with_markers(&["<|channel|>", "<|start|>"]);
    let mut ask = |_: usize| (Trial::RanOut, String::new());
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &bytes,
        1,
        200,
        "a test",
        None,
        &mut ask,
    );
    let Outcome::Inconclusive { because } = &probed.outcome else {
        panic!("nothing paired, so nothing may be concluded about what a turn spends");
    };
    assert!(
        because.contains("that it could not"),
        "the count of what could not be paired is the point: {because}"
    );
    assert!(
        because.contains("MCF's reach and not this model's habits"),
        "{because}"
    );
}

#[test]
fn byte_fallback_tokens_are_not_markers() {
    let bytes = crate::probes::tests::with_markers(&["<|channel|>"]);
    let file = gguf::parse(&bytes).expect("the fixture parses");
    let tokens = Tokens::read(&file).expect("the fixture lists tokens");
    let could_not = unpairable(&file, &tokens);
    assert!(
        could_not.iter().any(|marker| marker == "<|channel|>"),
        "a real marker is still reported: {could_not:?}"
    );
    assert!(
        !could_not.iter().any(|marker| marker.starts_with("<0x")),
        "no byte-fallback token is counted as a marker: {could_not:?}"
    );
    assert!(
        could_not.len() < 8,
        "the count is markers, not the byte alphabet: {} found",
        could_not.len()
    );
}

#[test]
fn an_addressing_that_closes_the_thinking_is_asked_with_it_opened() {
    use mcf_standin::tokenizer::Piece;
    let pairs = vec![("<think>".to_owned(), "</think>".to_owned())];
    let closed = super::super::Addressing {
        name: "user…assistant, thinking closed".to_owned(),
        pieces_before: vec![Piece::Marker("<|user|>".to_owned())],
        pieces_after: vec![
            Piece::Marker("<|assistant|>".to_owned()),
            Piece::Marker("</think>".to_owned()),
        ],
    };
    let under = super::under(&closed, &pairs);
    assert!(under.changed);
    assert_eq!(under.opened_by_turn.as_deref(), Some("<think>"));
    assert_eq!(
        under.addressing.name,
        "user…assistant, thinking closed, think opened by the turn"
    );
    assert_eq!(under.addressing.shown("q"), "<|user|>q<|assistant|><think>");

    let open = under.addressing.clone();
    let again = super::under(&open, &pairs);
    assert!(!again.changed);
    assert_eq!(again.opened_by_turn.as_deref(), Some("<think>"));
    assert_eq!(again.addressing, open);

    let plain = super::super::Addressing {
        name: "user…assistant".to_owned(),
        pieces_before: vec![Piece::Marker("<|user|>".to_owned())],
        pieces_after: vec![Piece::Marker("<|assistant|>".to_owned())],
    };
    let under = super::under(&plain, &pairs);
    assert!(!under.changed && under.opened_by_turn.is_none());
    assert_eq!(under.addressing, plain);
    let text = super::super::Addressing {
        name: "typed".to_owned(),
        pieces_before: Vec::new(),
        pieces_after: vec![Piece::Text("</think>".to_owned())],
    };
    assert!(super::under(&text, &pairs).opened_by_turn.is_none());
    assert!(!super::under(&closed, &[]).changed);
}

#[test]
fn the_longest_whole_turn_is_kept_in_tokens() {
    let mut turns = vec![
        (
            Trial::Stopped {
                after: 40,
                before: Some(30),
            },
            "<think>a b</think>done".to_owned(),
        ),
        (
            Trial::Stopped {
                after: 90,
                before: Some(80),
            },
            "<think>a b c</think>done".to_owned(),
        ),
        (Trial::RanOut, "<think>a b c d".to_owned()),
    ]
    .into_iter();
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &thinking_model(),
        3,
        100,
        "a test",
        None,
        &mut |_| turns.next().unwrap_or((Trial::RanOut, String::new())),
    );
    let Outcome::Observed(spends) = &probed.outcome else {
        panic!("observed: {probed:?}");
    };
    assert_eq!(
        spends.longest_turn, 90,
        "the turn that ran out is not a turn"
    );
    assert_eq!(spends.before_in_longest, Some(80));
}

#[test]
fn a_marker_the_turn_opened_is_measured_from_the_first_token() {
    let bytes = crate::probes::tests::with_markers(&["<think>", "</think>"]);
    let mut turns = vec![
        (
            Trial::Stopped {
                after: 9,
                before: None,
            },
            "three times seven</think> 21".to_owned(),
        ),
        (Trial::RanOut, "three times seven is".to_owned()),
    ]
    .into_iter();
    let probed = thinking(
        std::path::Path::new("m"),
        &bytes,
        2,
        16,
        "test",
        Some("<think>"),
        &mut |_| turns.next().unwrap_or((Trial::RanOut, String::new())),
    );
    let Outcome::Observed(spends) = &probed.outcome else {
        panic!("observed: {probed:?}");
    };
    assert_eq!(spends.used.as_deref(), Some("<think>"));
    assert_eq!((spends.opened, spends.closed), (2, 1));
    assert_eq!(spends.longest_inside, 3);
}
