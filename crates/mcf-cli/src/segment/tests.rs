//! A segmentation shows where text breaks, and rates nothing.

use super::{Fragment, whole_or_shattered};

fn fragment(text: &str, byte: bool) -> Fragment {
    Fragment {
        identifier: 1,
        text: text.to_owned(),
        byte,
    }
}

#[test]
fn a_word_that_is_one_token_survives_whole() {
    let said = whole_or_shattered(
        "hello world",
        &[fragment(" hello", false), fragment(" world", false)],
    );
    assert!(said.contains("2 of 2"), "{said}");
}

#[test]
fn a_word_broken_into_pieces_is_counted_as_broken() {
    let said = whole_or_shattered(
        "antidisestablishmentarianism",
        &[
            fragment(" antid", false),
            fragment("isestab", false),
            fragment("lishmentarianism", false),
        ],
    );
    assert!(said.contains("0 of 1"), "{said}");
}

#[test]
fn raw_bytes_are_named_as_the_vocabulary_having_no_piece() {
    let said = whole_or_shattered("日本", &[fragment("日", true), fragment("本", true)]);
    assert!(
        said.contains("2 token(s) are raw bytes"),
        "a byte fallback is the strongest signal that this vocabulary has no word for the \
         text, and hiding it in a count would lose it: {said}"
    );
}

#[test]
fn nothing_here_rates_the_segmentation() {
    let said = whole_or_shattered("hello", &[fragment(" hello", false)]);
    assert!(
        said.contains("nothing here rates it"),
        "§3.15: MCF shows where text breaks and does not say whether that is good: {said}"
    );
    for judgement in ["poor", "bad", "inefficient", "wasteful", "should"] {
        assert!(!said.contains(judgement), "{judgement} in {said}");
    }
}

#[test]
fn text_with_no_words_says_so_rather_than_dividing_by_nothing() {
    assert!(whole_or_shattered("   ", &[]).contains("No whitespace-separated words"));
}

/// **The bug the Japanese case found.** A byte-level vocabulary spells one
/// character across several tokens, so the decode of *k* tokens is not the
/// decode of *k-1* with something appended: the replacement mark standing in
/// for an incomplete character is *replaced* by the character. A prefix strip
/// fails there, and with a fallback it reports the whole text as one token's
/// contribution — the last token of a Japanese phrase appearing to have
/// produced the entire phrase.
mod contributions {
    use super::super::added_by;

    #[test]
    fn a_token_that_appends_contributes_what_it_appended() {
        assert_eq!(added_by("hello", "hello world"), " world");
    }

    #[test]
    fn a_token_that_completes_a_character_contributes_the_character() {
        // The first token left an incomplete sequence, which decodes as a
        // replacement mark; the second completes it.
        assert_eq!(added_by("日本語の\u{fffd}", "日本語のテ"), "テ");
    }

    #[test]
    fn nothing_is_ever_reported_as_the_whole_text() {
        // The failing case, pinned: before B-381's byte comparison this
        // returned "日本語のテキスト" for a token that contributed one
        // character.
        assert_eq!(added_by("日本語のテキス\u{fffd}", "日本語のテキスト"), "ト");
    }

    #[test]
    fn a_token_that_adds_nothing_visible_adds_nothing() {
        assert_eq!(added_by("日本", "日本"), "");
    }
}

/// Markers are found by shape and then asked about, never by a table.
mod markers {
    use super::super::marker_shaped;

    #[test]
    fn the_shapes_models_use_are_found() {
        let found = marker_shaped("<|im_start|>user [INST] <s> <start_of_turn>");
        assert_eq!(
            found,
            [
                "<|im_start|>".to_owned(),
                "<s>".to_owned(),
                "<start_of_turn>".to_owned(),
                "[INST]".to_owned(),
            ]
        );
    }

    #[test]
    fn ordinary_prose_containing_a_less_than_is_not_a_marker() {
        // An unbounded scan would call half a sentence a marker.
        assert!(marker_shaped("a < b and c > d").is_empty());
        assert!(marker_shaped("if x < y then").is_empty());
    }

    #[test]
    fn the_same_marker_twice_is_reported_once() {
        assert_eq!(marker_shaped("<s> and <s>").len(), 1);
    }

    #[test]
    fn a_prompt_with_no_markers_produces_no_section() {
        assert!(marker_shaped("just some ordinary text").is_empty());
    }

    #[test]
    fn an_unclosed_marker_is_not_one() {
        assert!(marker_shaped("<|im_start").is_empty());
    }
}

/// What a prompt spends is stated against a context that says where it came
/// from (B-382, A21).
mod cost {
    use super::super::what_it_spends;

    #[test]
    fn a_prompt_that_fits_says_what_is_left() {
        let said = what_it_spends(100, Some(8192));
        assert!(said.contains("leaving 8092 token(s)"), "{said}");
        assert!(
            said.contains("for everything else"),
            "the share alone hides that the answer needs room too: {said}"
        );
    }

    #[test]
    fn a_prompt_that_does_not_fit_says_so_before_anything_is_sent() {
        let said = what_it_spends(1185, Some(128));
        assert!(
            said.contains("does not fit, before a single token of answer"),
            "the question B-382 exists to answer is whether this can be sent at all: {said}"
        );
    }

    #[test]
    fn the_context_is_marked_as_declared() {
        let said = what_it_spends(10, Some(4096));
        assert!(
            said.contains("DECLARED"),
            "A21: a declared figure presented as a measured one is the failure: {said}"
        );
        assert!(
            said.contains("`mcf probe`"),
            "and the command that would verify it must be named, or the marking is a \
             disclaimer rather than a route: {said}"
        );
    }

    #[test]
    fn no_declared_context_is_unknown_and_not_unlimited() {
        let said = what_it_spends(10, None);
        assert!(said.contains("unknown rather than unlimited"), "{said}");
        for wrong in ["0 token(s) of context", "no limit", "unlimited context"] {
            assert!(!said.contains(wrong), "{wrong} in {said}");
        }
    }

    #[test]
    fn a_declared_zero_is_treated_as_no_declaration() {
        // A file declaring zero has declared nothing usable, and dividing by
        // it would be the arithmetic deciding what the sentence says.
        assert!(
            what_it_spends(10, Some(0)).contains("unknown rather than unlimited"),
            "a zero context must not become a division"
        );
    }
}
