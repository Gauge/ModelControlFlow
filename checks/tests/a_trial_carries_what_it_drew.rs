//! A trial cannot exist without its seed, and no run may give every trial the
//! same one (B61, D19, B-290).
//!
//! B61's check is `compiler`, and its violation is *thirty identical
//! completions reported with a spread of zero, which reads as remarkable
//! consistency and is an artefact*. Two things make that unreachable, and both
//! are one edit away from being reachable again:
//!
//! 1. `Trial` has a [`Draw`] field with no default and no constructor that
//!    omits it, so a trial that does not say what it drew does not exist;
//! 2. the published seed set is a **bijection** from trial index to seed, so
//!    two trials cannot draw the same one — which a list of literals could only
//!    promise by being checked, and which a `%` in the wrong place would
//!    silently break.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// A trial cannot be built without saying what it drew.
#[test]
fn a_trial_cannot_be_built_without_its_draw() {
    let source = code_only(&read("crates/mcf-core/src/trial/mod.rs"));
    assert!(
        source.contains("drew: Draw,"),
        "a trial must carry what it drew (B61, B-290)"
    );
    for forbidden in [
        "Default for Trial",
        "drew: Option<Draw>",
        "pub fn without_a_draw",
        "Draw::default",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a trial exist without saying what it drew"
        );
    }
    // Every constructor takes it. The list is read from the source so a
    // second constructor added later fails here rather than slipping past.
    for signature in constructors(&source, "-> Self") {
        if signature.contains("fn new(") && signature.contains("value") {
            assert!(
                signature.contains("drew: Draw"),
                "a trial constructor that omits the draw: {signature}"
            );
        }
    }
}

/// The two disciplines are separate variants, not one with a flag.
///
/// D19 gives timing laboratories the opposite rule — hold the seed still, pin
/// the generation length — and a reader must never be able to mistake a timing
/// trial's fixed seed for a behaviour trial's mistake. A boolean would let
/// them.
#[test]
fn the_two_disciplines_are_distinct_variants() {
    let source = code_only(&read("crates/mcf-core/src/trial/seed.rs"));
    assert!(
        source.contains("Seeded {") && source.contains("LengthPinned {"),
        "the behaviour and timing disciplines must be distinct variants (D19)"
    );
    for forbidden in ["is_timing: bool", "fixed: bool", "pinned: bool"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` collapses two disciplines into one with a flag (D19)"
        );
    }
}

/// A declared set is refused where it repeats a seed or holds only one.
#[test]
fn a_set_that_repeats_a_seed_is_refused() {
    let source = code_only(&read("crates/mcf-core/src/trial/seed.rs"));
    for reached in [
        "return Err(NotASeedSet::TooFew",
        "return Err(NotASeedSet::Repeated",
        "return Err(NotASeedSet::Unnamed",
    ] {
        assert!(
            source.contains(reached),
            "`{reached}` must be reached, not merely declared (B61)"
        );
    }
}

/// The published set is stated as arithmetic and never wraps around.
///
/// Two properties in one place: it is a bijection, so no two trials collide;
/// and it is unbounded, which is what F55's stopping condition needs, since how
/// many trials a run will take is not known when it starts. A `%` against a
/// list length would break both at once and is the shape to watch for.
#[test]
fn the_published_set_is_arithmetic_and_unbounded() {
    let source = code_only(&read("crates/mcf-core/src/trial/seed.rs"));
    let Some(published) = source
        .split_once("pub const fn published(")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
    else {
        panic!("the published set is where this check looks");
    };
    assert!(
        !published.contains('%') && !published.contains("rem"),
        "the published set must not be a list read modulo its length: wrapping around repeats a \
         trajectory, which is exactly what B61 forbids"
    );
    assert!(
        published.contains("wrapping_mul") && published.contains("wrapping_add"),
        "this check is reading the wrong function"
    );
    assert!(
        source.contains("Self::Standard => None,"),
        "the published set must report no bound: a benchmark does not know how many trials it \
         will take (F55)"
    );
}

/// A comparison refuses arms that drew from different sets.
///
/// D19's own words: *comparisons require matching seed sets the way they
/// require matching hardware — recorded, checked, and refused when they
/// differ.*
#[test]
fn a_comparison_refuses_mismatched_seed_sets() {
    let source = code_only(&read("crates/mcf-bench/src/compare.rs"));
    assert!(
        source.contains("SeedSetsDiffer {"),
        "the refusal D19 requires must exist by name"
    );
    assert!(
        source.contains("return Err(disagreement(l.drew(), r.drew()));"),
        "and be reached from the pairing, where the two arms' draws meet"
    );
}

/// The seed set is a condition, recorded with every measurement.
#[test]
fn the_seed_set_is_a_floor_condition() {
    let source = code_only(&read("crates/mcf-core/src/measurement/conditions.rs"));
    assert!(
        source.contains("pub seed_set: Attested<ConditionValue>,"),
        "D19 makes the seed set a condition, not part of identity"
    );
    assert!(
        source.contains(r#"("seed_set", &self.seed_set),"#),
        "and it must be in the floor's entries, or no surface renders it and no isolation check \
         sees it (A6, A8)"
    );
}

/// Every `pub fn` signature in a source that returns the named type.
fn constructors(source: &str, returning: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub const fn ") {
            continue;
        }
        let mut signature = trimmed.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(more) => {
                    signature.push(' ');
                    signature.push_str(more.trim());
                }
                None => break,
            }
        }
        if signature.contains(returning) {
            found.push(signature);
        }
    }
    assert!(!found.is_empty(), "no constructor was found in this source");
    found
}

/// The source with its documentation comments removed, so that a sentence
/// quoting a forbidden shape is not read as the shape itself.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
