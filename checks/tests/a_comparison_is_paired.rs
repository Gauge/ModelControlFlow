//! A comparison can only be built out of paired trials (B53, B-250).
//!
//! B53's check is `compiler`: *a comparison result can only be constructed
//! from paired trials carrying a common session id*. The compiler does hold
//! that — `mcf_bench::compare::Comparison` has three constructors, one of
//! which runs the arms alternately itself, one of which verifies the
//! interleaving from the record's positions, and one of which is §3.27's
//! explicitly weaker cross-session assembly and cannot produce a paired
//! difference at all. There is no function anywhere that takes two sequences
//! of timings and calls them a comparison.
//!
//! *Staying* that way is the part the compiler cannot check about itself. A
//! fourth constructor taking two slices, a `Default`, a `From<(Vec, Vec)>`, or
//! a re-export of the statistics module's internals would each reopen the hole
//! quietly, and no existing test would notice. This is what notices.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`: a compile-fail harness is a third-party
//! dependency, and B15 admits weight only against a stated cost.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The statistics that decide a comparison are not reachable from outside the
/// crate.
///
/// This is the whole of *block-then-subtract does not compile*. The two nulls
/// take bare numbers — one takes two slices of nanoseconds — and a caller who
/// could reach them could hand over thirty timings of A and thirty of B with
/// nothing to say they were ever interleaved. They are `pub(super)`, so the
/// only way to a verdict is through a `Comparison`, and the only ways to a
/// `Comparison` are the three that establish or disclaim the pairing.
#[test]
fn the_statistics_are_not_reachable_without_a_comparison() {
    let source = code_only(&bench_source("enough.rs"));
    for entry in ["over_paired_differences", "over_separate_arms"] {
        let declared = format!("pub(super) fn {entry}");
        assert!(
            source.contains(&declared),
            "`{entry}` must stay crate-internal: a public function taking bare timings is \
             block-then-subtract with extra steps (B53, B-250)"
        );
        assert!(
            !source.contains(&format!("pub fn {entry}")),
            "`{entry}` has been made public, which reopens B-250"
        );
    }
    assert!(
        !source.contains("pub fn verdict("),
        "the pre-B-250 entry point took two slices and has been removed; a function of that \
         shape cannot tell an interleaved comparison from two blocks"
    );
}

/// Every constructor of a `Comparison` either establishes the pairing or says
/// it has none.
///
/// The list is read from the source rather than declared here, so a
/// constructor added tomorrow fails this test rather than slipping past a list
/// nobody updated.
#[test]
fn every_constructor_establishes_or_disclaims_the_pairing() {
    /// What each way in is allowed to be, and why it is a pairing.
    const ADMITTED: [&str; 3] = [
        // Runs the arms alternately itself, drawing the order per pair.
        "Interleaving",
        // Verifies the interleaving from the record's positions.
        "from_trials",
        // §3.27's *it may be all that exists*: no pairing, and it says so.
        "from_separate_sessions",
    ];

    let source = code_only(&bench_source("compare.rs"));
    let mut found = Vec::new();
    // Only the two types that can yield a comparison are read: `Side::other`
    // and the rest return `Self` too, and a check that could not tell them
    // apart would be a check about the word `Self`.
    let mut inside = false;
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
            inside = trimmed.contains("Comparison<") || trimmed.contains("Interleaving<");
            continue;
        }
        if !inside || (!trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub const fn ")) {
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
        let returns_one = signature.contains("-> Self")
            || signature.contains("-> Result<Self, NotComparable>")
            || signature.contains("-> Comparison");
        if returns_one {
            found.push(signature);
        }
    }

    assert!(
        found.len() >= 3,
        "fewer than three ways in were found, so this check is reading the wrong file: {found:#?}"
    );
    for constructor in &found {
        assert!(
            ADMITTED.iter().any(|name| constructor.contains(name))
                // `Interleaving::new` and `Interleaving::finish` are the
                // alternating constructor, which is admitted above by name.
                || constructor.contains("fn new(")
                || constructor.contains("fn finish("),
            "a way to build a comparison that neither establishes nor disclaims the pairing: \
             {constructor} (B53, B-250)"
        );
    }
}

/// Nothing manufactures a comparison out of a trait implementation.
///
/// Each of these would be a way to obtain one from something that is not a set
/// of paired trials — which is exactly the shape B-250 exists to make
/// impossible.
#[test]
fn no_trait_implementation_manufactures_a_comparison() {
    let source = code_only(&bench_source("compare.rs"));
    for forbidden in [
        "Default for Comparison",
        "From<(",
        "FromIterator",
        "Extend for Comparison",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a comparison exist without a pairing (B53, B-250)"
        );
    }
}

/// A cross-session comparison is constructible, and cannot report a paired
/// difference.
///
/// The second half of B-250's done-when. The weakness is in the type — the
/// paired-difference accessor returns an `Option` and the assembled
/// construction answers `None` — rather than in a label a caller may forget to
/// print, and the label exists as well.
#[test]
fn a_cross_session_comparison_is_weaker_in_the_type() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("pub fn from_separate_sessions("),
        "§3.27's *it may be all that exists* must remain constructible"
    );
    assert!(
        source.contains("pub fn paired_differences(&self) -> Option<Vec<Difference>>"),
        "a comparison with no pairing must have no paired difference to report"
    );
    assert!(
        source.contains("Assembled {"),
        "the weaker construction must be labelled by a distinct strength"
    );
}

/// The interleaving is verified against the record's positions, not assumed.
///
/// B53's violation is *thirty runs of A, then thirty of B, subtracted*, and a
/// comparison read back out of the record is where that would arrive
/// undetected. The refusal is named, so this checks the name is still there
/// and still reached.
#[test]
fn blocked_trials_are_refused_by_name() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("RanInBlocks {"),
        "the refusal B-250 is about must exist by name"
    );
    assert!(
        source.contains("return Err(NotComparable::RanInBlocks {"),
        "the refusal must be reached, not merely declared"
    );
}

/// The source of one of `mcf-bench`'s modules.
fn bench_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-bench/src")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
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
