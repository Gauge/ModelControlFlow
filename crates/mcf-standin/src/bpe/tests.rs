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
        pieces(text, Split::ModernThreeDigits)
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

/// The four expressions cut digits differently, and that is not decoration.
///
/// One takes up to three at a time, one exactly one, GPT-2 the whole run with
/// the space before it, and the fourth one at a time with no space. A
/// tokenizer that used the wrong one puts a number in different pieces, so a
/// different set of merges can apply to it. This is the defect F23 found in
/// MCF's own first version, kept as the test that would have caught it.
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
    // The last two agree on digits and disagree elsewhere, so neither is the
    // other under another name.
    assert_ne!(
        pieces("(hello)", Split::ModernOneDigit),
        pieces("(hello)", Split::Gpt2DigitsApart)
    );
}

/// The digits-apart form is GPT-2's expression everywhere a digit is not
/// involved.
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

/// A symbol run that takes nothing after it is a different cut (F23).
///
/// One ends its symbol alternative ` ?[^\s\p{L}\p{N}]+[\r\n]*` and the other
/// ends it ` ?[^\s\p{L}\p{N}\r\n]+`. So a newline after
/// punctuation joins the punctuation under one and stands alone under the
/// other — two pieces where there was one, and the merges cannot span them.
/// This is the whole reason it is its own expression rather than being run
/// through the one it resembles.
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
    // Where there is no newline the two agree, which is why the difference is
    // easy to miss by eye.
    assert_eq!(
        pieces("a!!!b", Split::ModernOneDigit),
        pieces("a!!!b", Split::ModernOneDigitSymbolsAlone)
    );
}

/// Digits are still one at a time in the expression that takes symbols alone.
#[test]
fn symbols_alone_still_cuts_every_digit() {
    assert_eq!(
        pieces("123", Split::ModernOneDigitSymbolsAlone),
        vec!["1", "2", "3"]
    );
}

/// Letters are cut where their case changes, and a run of capitals holds.
///
/// `((?=[\p{L}])([^a-z]))*((?=[\p{L}])([^A-Z]))+` — a run of letters that are
/// not lowercase, then a run that are not uppercase. `HTTPServer` is one piece
/// because the capitals are the first run and `erver` the second; `helloWorld`
/// is two because the second word starts a new first run.
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
    // Where nothing changes case, it agrees with the plainer expressions.
    assert_eq!(
        pieces("hello", Split::CasePartitionedThreeDigits),
        vec!["hello"]
    );
    // And that is a real difference from the expression it most resembles.
    assert_ne!(
        pieces("helloWorld", Split::CasePartitionedThreeDigits),
        pieces("helloWorld", Split::ModernThreeDigits)
    );
}

/// Its symbol run swallows a slash *after* the newlines, where others stop.
///
/// ` ?[^\s\p{L}\p{N}]+[\r\n/]*` against ` ?[^\s\p{L}\p{N}]+[\r\n]*`. The
/// slash is in the trailing class, and where that shows is narrower than it
/// looks: a slash is itself a symbol, so an ordinary run takes it already. It
/// is only once the run has ended and a newline has been taken that the two
/// expressions part — which is exactly the kind of difference that is invisible
/// by eye and changes the tokenization.
#[test]
fn a_slash_after_a_newline_joins_the_run_only_here() {
    // A slash among punctuation is the run itself, under both.
    assert_eq!(
        pieces("a!//b", Split::CasePartitionedThreeDigits),
        pieces("a!//b", Split::ModernThreeDigits),
        "a slash is a symbol, so both take it in the run"
    );
    // After a newline the trailing class decides, and only one has the slash.
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

/// Digits come in threes here, as the expression says.
#[test]
fn the_case_partitioned_expression_takes_three_digits() {
    assert_eq!(
        pieces("1234", Split::CasePartitionedThreeDigits),
        vec!["123", "4"]
    );
}

/// Every expression covers its input completely and without overlap.
///
/// A scanner that dropped a character would lose information silently (A1),
/// and a new expression is the most likely place for that to happen.
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
