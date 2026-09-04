//! What a thinking probe may and may not conclude (B-421, D42, A21, A7).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use super::*;
use crate::probes::tests::plain;

/// A marker closes the way its own bracket says, or it is not a pair.
#[test]
fn a_marker_closes_the_way_its_bracket_says() {
    assert_eq!(closing_form("<think>").as_deref(), Some("</think>"));
    assert_eq!(closing_form("[THINK]").as_deref(), Some("[/THINK]"));
    assert_eq!(closing_form("<|channel|>").as_deref(), Some("</|channel|>"));

    // Not pairs: a closing form is not itself an opener, an empty marker has
    // no inside, and a thing that is not bracketed is not a marker.
    assert_eq!(closing_form("</think>"), None);
    assert_eq!(closing_form("<>"), None);
    assert_eq!(closing_form("think"), None);
    assert_eq!(closing_form(""), None);
    // Brackets that do not match are not a marker either.
    assert_eq!(closing_form("<think]"), None);
}

/// A file that holds no pair concludes nothing, rather than *does not think*.
///
/// A7's distinction, and the one this probe is most likely to get wrong: MCF
/// found no marker, which is not the same claim as the model having no mode to
/// be in.
#[test]
fn a_file_with_no_marker_decides_nothing() {
    let mut ask = |_: usize| (Trial::RanOut, String::new());
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &plain(),
        3,
        200,
        "a test",
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

/// A file MCF cannot read is inconclusive, never a negative result.
#[test]
fn a_file_that_is_not_a_model_decides_nothing() {
    let mut ask = |_: usize| (Trial::RanOut, String::new());
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        b"not a model at all",
        1,
        200,
        "a test",
        &mut ask,
    );
    assert!(matches!(probed.outcome, Outcome::Inconclusive { .. }));
}

/// A turn still inside its marker when the budget ran out is counted apart.
///
/// F106's shape: ten trials read as *produced nothing* were every one of them
/// still going. Opened-and-not-closed is a fact about the budget, and folding
/// it into the closed count would hide the very thing this probe exists to
/// surface.
#[test]
fn opened_and_never_closed_is_not_counted_as_closed() {
    let pair = ("<think>".to_owned(), "</think>".to_owned());
    let said = "<think>working on it and still going".to_owned();
    let counted = count_over(&[pair], &[said]);
    assert_eq!(counted, (1, 0), "opened once, closed never");
}

/// A turn that opens and closes is counted, and its inside is measured.
#[test]
fn a_closed_marker_is_measured_by_what_is_inside_it() {
    let pair = ("<think>".to_owned(), "</think>".to_owned());
    let said = "<think>one two three four</think>The answer is 21.".to_owned();
    assert_eq!(count_over(&[pair], &[said]), (1, 1));
}

/// The longest turn decides the budget, not the average one.
///
/// A budget has to cover the worst turn seen; an average budget truncates half
/// of them, and a truncated turn is the measurement F106 warns about.
#[test]
fn the_budget_is_set_by_the_longest_turn_not_the_mean() {
    let mut said = [
        "<think>a b</think>done".to_owned(),
        "<think>a b c d e f</think>done".to_owned(),
    ]
    .into_iter();
    let mut ask = move |_: usize| {
        (
            Trial::Stopped { after: 10 },
            said.next().unwrap_or_default(),
        )
    };
    let probed = thinking(
        std::path::Path::new("/nowhere.gguf"),
        &thinking_model(),
        2,
        200,
        "a test",
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

/// Counting helper: what the probe would count over these answers.
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

/// A model whose vocabulary holds a thinking pair.
fn thinking_model() -> Vec<u8> {
    crate::probes::tests::with_markers(&["<think>", "</think>"])
}

/// A marker with no closing form is reported, not passed over.
///
/// Some families delimit a region without a `</x>` — a channel opened by one
/// token and ended by a different one. A turn inside one of those is a turn
/// this probe does not measure, and a report that mentioned only what it could
/// pair would let a reader conclude the model spends nothing before its answer
/// when MCF simply did not look (A7).
#[test]
fn a_marker_that_cannot_be_paired_is_still_reported() {
    // `<|channel|>` is a real token here and `</|channel|>` is not, which is
    // exactly the shape that used to vanish from the report.
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
    // And a closing form is never itself counted as something to be inside.
    assert!(!could_not.iter().any(|marker| marker == "</think>"));
    assert!(!paired.iter().any(|(marker, _)| marker == "</think>"));
}

/// A file whose every marker is unpairable says so, rather than reading as a
/// file with no markers.
///
/// This is where withholding the count did real harm: a channel-format family
/// holds `<|channel|>` and no `</|channel|>`, so nothing pairs — and the report
/// said "no marker it could be inside", which a reader takes to mean the model
/// answers from its first token. What is true is that MCF could not look.
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

/// A byte-fallback token is not a marker a turn can be inside.
///
/// `<0xNN>` is how a vocabulary spells a byte it has no other token for. All
/// 256 of them are marker-shaped, and counting them made the report claim two
/// hundred and fifty-nine markers where a file had two — a number that reads
/// as a finding and is noise.
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
