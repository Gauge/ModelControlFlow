#![allow(clippy::panic)]

#[test]
fn nothing_converts_between_the_two_origins() {
    let source = code_only(&origin_source());
    for forbidden in [
        "From<FromCorpus",
        "From<LocallyMeasured",
        "Into<FromCorpus",
        "Into<LocallyMeasured",
        "fn promote",
        "fn confirm",
        "fn into_local",
        "fn assume_local",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a foreign number decide a local question (B34, B43)"
        );
    }
}

#[test]
fn the_two_origins_are_types_and_not_a_flag() {
    let source = code_only(&origin_source());
    assert!(
        source.contains("pub struct LocallyMeasured<T>"),
        "the local origin is no longer its own type"
    );
    assert!(
        source.contains("pub struct FromCorpus<T>"),
        "the corpus origin is no longer its own type"
    );
    for forbidden in ["is_local", "is_corpus", "origin:", "enum Origin"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` turns the distinction back into a field (B43)"
        );
    }
}

#[test]
fn a_corpus_value_cannot_omit_its_sample_count() {
    let source = code_only(&origin_source());
    assert!(
        source.contains("pub const fn new(value: T, reports: usize) -> Self"),
        "the corpus constructor no longer requires a sample count (B44)"
    );
    assert!(
        !source.contains("impl<T> Default for FromCorpus"),
        "a default would let a corpus value claim a sample count nobody supplied"
    );
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn origin_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/origin.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}
