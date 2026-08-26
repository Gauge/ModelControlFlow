//! What the byte-level half has to get right, checked against what it means
//! rather than against itself.

use super::{Ranks, Split, byte_of, character_of, merge, pieces, spell};

#[test]
fn the_alphabet_is_a_bijection() {
    let mut seen = std::collections::BTreeSet::new();
    for byte in 0..=u8::MAX {
        let character = character_of(byte);
        assert!(
            seen.insert(character),
            "byte {byte:#04x} was spelled {character:?}, which another byte was already spelled"
        );
        assert_eq!(
            byte_of(character),
            Some(byte),
            "byte {byte:#04x} spelled {character:?} did not come back"
        );
    }
    assert_eq!(seen.len(), 256, "every byte value must have a character");
}

#[test]
fn the_alphabet_is_the_one_the_vocabularies_are_written_in() {
    // Three spellings taken from what a real vocabulary carries — a space is
    // `Ġ`, a newline `Ċ`, a tab `ĉ` — which is the only way to know this
    // construction is the one those files were built with rather than a
    // consistent invention of MCF's own.
    assert_eq!(character_of(b' '), '\u{120}');
    assert_eq!(character_of(b'\n'), '\u{10a}');
    assert_eq!(character_of(b'\t'), '\u{109}');
    assert_eq!(character_of(b'A'), 'A');
    assert_eq!(spell(" the"), "\u{120}the");
}

#[test]
fn the_modern_split_is_the_one_the_expression_describes() {
    fn split(text: &str) -> Vec<&str> {
        pieces(text, Split::Modern)
    }
    // A leading space joins the word after it, not the blanks before it.
    assert_eq!(
        split("The capital of France"),
        ["The", " capital", " of", " France"]
    );
    // At most three digits to a piece.
    assert_eq!(split("12345"), ["123", "45"]);
    // A lead character that is not a space still joins the letters after it,
    // which is where this differs from GPT-2's expression.
    assert_eq!(split("(hello)"), ["(hello", ")"]);
    // Contractions, either case, are their own piece.
    assert_eq!(split("it's IT'S"), ["it", "'s", " IT", "'S"]);
    // Whitespace ending in a newline is one piece; the run before a word is
    // not.
    assert_eq!(split("a  \n  b"), ["a", "  \n", " ", " b"]);
}

#[test]
fn the_gpt2_split_differs_where_it_is_supposed_to() {
    assert_eq!(pieces("(hello)", Split::Gpt2), ["(", "hello", ")"]);
    assert_eq!(pieces("12345", Split::Gpt2), ["12345"]);
    assert_eq!(pieces("it'S", Split::Gpt2), ["it", "'", "S"]);
}

#[test]
fn every_piece_of_the_text_survives_the_split() {
    // A1 at the smallest scale: the pieces must reassemble into the text, for
    // both expressions and for text with characters neither was written for.
    for text in [
        "The capital of France is",
        "  \n\ttabs\r\nand\u{a0}nbsp \u{1f600}\u{1f600} emoji 4096 tokens!!",
        "\u{4f60}\u{597d}\u{ff0c}\u{4e16}\u{754c}",
        "",
    ] {
        for split in [Split::Gpt2, Split::Modern] {
            assert_eq!(
                pieces(text, split).concat(),
                text,
                "{split:?} did not cover {text:?}"
            );
        }
    }
}

#[test]
fn merges_are_applied_in_the_order_they_are_listed() {
    // "a b" before "b c": `abc` must become `ab` + `c`, and reversing the list
    // must change the answer. A merge algorithm that used any other order
    // would pass a test that only looked at the first list.
    let earlier = Ranks::read(&["a b".to_owned(), "b c".to_owned()]);
    let later = Ranks::read(&["b c".to_owned(), "a b".to_owned()]);
    assert_eq!(merge("abc", &earlier), ["ab", "c"]);
    assert_eq!(merge("abc", &later), ["a", "bc"]);
}

#[test]
fn a_tie_goes_to_the_leftmost() {
    // Both pairs are the same merge, so the ranks are equal and only the
    // tie-break decides. Leftmost first means `aaa` becomes `aa` + `a`.
    let ranks = Ranks::read(&["a a".to_owned()]);
    assert_eq!(merge("aaa", &ranks), ["aa", "a"]);
}

#[test]
fn a_piece_with_no_merges_stays_characters() {
    let ranks = Ranks::read(&["x y".to_owned()]);
    assert_eq!(merge("abc", &ranks), ["a", "b", "c"]);
    assert!(!ranks.is_empty());
    assert_eq!(ranks.len(), 1);
}

#[test]
fn merging_never_loses_or_reorders_the_piece() {
    let ranks = Ranks::read(&[
        "t h".to_owned(),
        "th e".to_owned(),
        "\u{120} the".to_owned(),
        "e r".to_owned(),
    ]);
    for piece in ["\u{120}there", "the", "\u{120}t", "e", ""] {
        assert_eq!(
            merge(piece, &ranks).concat(),
            piece,
            "{piece:?} did not survive"
        );
    }
}
