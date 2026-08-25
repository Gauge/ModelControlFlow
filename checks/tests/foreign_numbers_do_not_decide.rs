//! A number from somebody else's machine cannot back a local decision.
//!
//! B43's check is `compiler`: *a corpus-sourced value and a locally-measured
//! value are distinct types, and only the second can back a recommendation*
//! (B-221). B34 is the rule — *MCF contributes outward and decides inward* —
//! and B43's violation is a sorted candidate list whose ordering nobody
//! explains, which is a foreign conclusion wearing a local interface.
//!
//! The compiler holds the separation while the two types stay unrelated. What
//! it cannot hold is somebody adding the conversion later, for one call site,
//! after a corpus value turned out to be the only number available.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// Nothing converts a corpus value into a local one, or the reverse.
///
/// A corpus value does not become local by being confirmed. It is *replaced* by
/// the local measurement, exactly as A20 replaces an estimate — and replacement
/// needs no mechanism, which is why the absence of one is the enforcement.
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

/// The two are separate types, not one type with a flag.
///
/// B9's violation, one level up: a single type with an `is_local` field is a
/// filter, and a filter can be misconfigured.
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

/// A corpus value cannot be built without its sample count.
///
/// B44: every corpus statement renders its `n`, so a claim resting on two
/// reports reads differently from one resting on four hundred and a claim
/// resting on nothing says so.
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

/// A file with its documentation removed; the documentation names the
/// forbidden constructs in order to say they are absent.
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
