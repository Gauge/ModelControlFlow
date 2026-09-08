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
    assert_eq!(character_of(b' '), '\u{120}');
    assert_eq!(character_of(b'\n'), '\u{10a}');
    assert_eq!(character_of(b'\t'), '\u{109}');
    assert_eq!(character_of(b'A'), 'A');
    assert_eq!(spell(" the"), "\u{120}the");
}

#[test]
fn the_modern_split_is_the_one_the_expression_describes() {
    fn split(text: &str) -> Vec<&str> {
        pieces(text, Split::ModernThreeDigits)
    }
    assert_eq!(
        split("The capital of France"),
        ["The", " capital", " of", " France"]
    );
    assert_eq!(split("12345"), ["123", "45"]);
    assert_eq!(split("(hello)"), ["(hello", ")"]);
    assert_eq!(split("it's IT'S"), ["it", "'s", " IT", "'S"]);
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
    for text in [
        "The capital of France is",
        "  \n\ttabs\r\nand\u{a0}nbsp \u{1f600}\u{1f600} emoji 4096 tokens!!",
        "\u{4f60}\u{597d}\u{ff0c}\u{4e16}\u{754c}",
        "",
    ] {
        for split in [
            Split::Gpt2,
            Split::Gpt2DigitsApart,
            Split::ModernThreeDigits,
            Split::ModernOneDigit,
        ] {
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
    let earlier = Ranks::read(&["a b".to_owned(), "b c".to_owned()]);
    let later = Ranks::read(&["b c".to_owned(), "a b".to_owned()]);
    assert_eq!(merge("abc", &earlier), ["ab", "c"]);
    assert_eq!(merge("abc", &later), ["a", "bc"]);
}

#[test]
fn a_tie_goes_to_the_leftmost() {
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

#[test]
fn each_expression_cuts_digits_its_own_way() {
    assert_eq!(
        pieces(" 12345 x", Split::ModernThreeDigits),
        [" ", "123", "45", " x"]
    );
    assert_eq!(
        pieces(" 12345 x", Split::ModernOneDigit),
        [" ", "1", "2", "3", "4", "5", " x"]
    );
    assert_eq!(pieces(" 12345 x", Split::Gpt2), [" 12345", " x"]);
    assert_eq!(
        pieces(" 12345 x", Split::Gpt2DigitsApart),
        [" ", "1", "2", "3", "4", "5", " x"]
    );
    assert_ne!(
        pieces("(hello)", Split::ModernOneDigit),
        pieces("(hello)", Split::Gpt2DigitsApart)
    );
}

#[test]
fn the_digits_apart_form_is_gpt2_away_from_digits() {
    for text in ["The capital of France", "it's (hello) world\n\n  ok"] {
        assert_eq!(
            pieces(text, Split::Gpt2DigitsApart),
            pieces(text, Split::Gpt2),
            "{text:?}"
        );
    }
}

#[test]
fn a_symbol_run_takes_the_newline_or_leaves_it() {
    assert_eq!(
        pieces("a!!!\nb", Split::ModernOneDigit),
        vec!["a", "!!!\n", "b"],
        "the one-digit expression's symbol run swallows the newline after it"
    );
    assert_eq!(
        pieces("a!!!\nb", Split::ModernOneDigitSymbolsAlone),
        vec!["a", "!!!", "\n", "b"],
        "the symbols-alone expression stops at the punctuation"
    );
    assert_eq!(
        pieces("a!!!b", Split::ModernOneDigit),
        pieces("a!!!b", Split::ModernOneDigitSymbolsAlone)
    );
}

#[test]
fn symbols_alone_still_cuts_every_digit() {
    assert_eq!(
        pieces("123", Split::ModernOneDigitSymbolsAlone),
        vec!["1", "2", "3"]
    );
}

#[test]
fn letters_are_cut_where_their_case_changes() {
    assert_eq!(
        pieces("HTTPServer", Split::CasePartitionedThreeDigits),
        vec!["HTTPServer"],
        "capitals then lowercase is one piece"
    );
    assert_eq!(
        pieces("helloWorld", Split::CasePartitionedThreeDigits),
        vec!["hello", "World"],
        "a capital starts a new piece"
    );
    assert_eq!(
        pieces("hello", Split::CasePartitionedThreeDigits),
        vec!["hello"]
    );
    assert_ne!(
        pieces("helloWorld", Split::CasePartitionedThreeDigits),
        pieces("helloWorld", Split::ModernThreeDigits)
    );
}

#[test]
fn a_slash_after_a_newline_joins_the_run_only_here() {
    assert_eq!(
        pieces("a!//b", Split::CasePartitionedThreeDigits),
        pieces("a!//b", Split::ModernThreeDigits),
        "a slash is a symbol, so both take it in the run"
    );
    assert_eq!(
        pieces("a!\n/b", Split::CasePartitionedThreeDigits),
        vec!["a", "!\n/", "b"],
        "the trailing class takes the newline and then the slash"
    );
    assert_eq!(
        pieces("a!\n/b", Split::ModernThreeDigits),
        vec!["a", "!\n", "/b"],
        "without the slash the run ends at the newline"
    );
}

#[test]
fn the_case_partitioned_expression_takes_three_digits() {
    assert_eq!(
        pieces("1234", Split::CasePartitionedThreeDigits),
        vec!["123", "4"]
    );
}

#[test]
fn every_expression_covers_what_it_is_given() {
    let texts = [
        "Hello, World!\n\ttabs and  spaces",
        "HTTPServer/v2 handles 1234 requests\r\n",
        "don't stop — ünïcode, 日本語, and !!!\n\n",
        "",
        "   ",
    ];
    for split in [
        Split::Gpt2,
        Split::Gpt2DigitsApart,
        Split::ModernThreeDigits,
        Split::ModernOneDigit,
        Split::ModernOneDigitSymbolsAlone,
        Split::CasePartitionedThreeDigits,
    ] {
        for text in texts {
            let out = pieces(text, split);
            assert_eq!(
                out.concat(),
                text,
                "{split:?} did not cover {text:?} exactly: {out:?}"
            );
        }
    }
}
