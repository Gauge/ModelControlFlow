//! The normalizer, against what the reference's would do on the same text.

use super::words;

#[test]
fn whitespace_splits_and_words_survive() {
    assert_eq!(words("hello world", true, true), ["hello", "world"]);
    assert_eq!(
        words("  spaced\tout\nlines  ", true, true),
        ["spaced", "out", "lines"]
    );
    assert_eq!(words("", true, true), Vec::<String>::new());
}

#[test]
fn punctuation_stands_alone() {
    assert_eq!(
        words("it's (quoted), right?", true, true),
        ["it", "'", "s", "(", "quoted", ")", ",", "right", "?"]
    );
}

#[test]
fn lowercase_and_accents_are_the_declared_transformations() {
    assert_eq!(words("Hello WORLD", true, false), ["hello", "world"]);
    assert_eq!(words("Hello", false, false), ["Hello"]);
    // café and naïve by the fold table; the accents are gone, the letters stay.
    assert_eq!(words("Café naïve", true, true), ["cafe", "naive"]);
    // A combining mark written separately is dropped too: e + U+0301.
    assert_eq!(words("cafe\u{301}", true, true), ["cafe"]);
    // Letters whose decoration is not an accent pass through whole, which is
    // what NFD does with them.
    assert_eq!(words("søster łódź", true, true), ["søster", "łodz"]);
}

#[test]
fn cjk_characters_split_one_to_a_word() {
    assert_eq!(words("你好world", true, true), ["你", "好", "world"]);
}

#[test]
fn control_characters_vanish_without_splitting() {
    // A control character is dropped, not a word boundary: the reference
    // `continue`s past it, so the letters on either side join.
    assert_eq!(words("ab\u{1}cd", true, true), ["abcd"]);
}
