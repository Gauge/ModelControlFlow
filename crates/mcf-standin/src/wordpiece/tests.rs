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
    assert_eq!(words("Café naïve", true, true), ["cafe", "naive"]);
    assert_eq!(words("cafe\u{301}", true, true), ["cafe"]);
    assert_eq!(words("søster łódź", true, true), ["søster", "łodz"]);
}

#[test]
fn cjk_characters_split_one_to_a_word() {
    assert_eq!(words("你好world", true, true), ["你", "好", "world"]);
}

#[test]
fn control_characters_vanish_without_splitting() {
    assert_eq!(words("ab\u{1}cd", true, true), ["abcd"]);
}
