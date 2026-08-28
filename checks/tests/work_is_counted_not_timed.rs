//! A laboratory declares its work in countable units, never in minutes
//! (B-224, B-225, B46, D14, A20).
//!
//! **What goes wrong without this.** A lab that declares *twenty minutes* has
//! declared a property of the machine it was written on. The declaration is
//! wrong on a slower machine and wrong in the other direction on a faster one,
//! and in neither case has the work changed. Worse, it is wrong *silently*:
//! nothing in the record says the number was a guess about somebody else's
//! hardware.
//!
//! So the declaration is counts — trials, arms, tokens — and the duration is
//! **derived** from those counts and a rate this machine measured, which makes
//! it an `Estimate` and puts A20's wall between it and any measurement.
//!
//! **Why a source check.** Rust cannot express *this struct has no field whose
//! meaning is a duration*: `usize` is `usize` whether it counts trials or
//! seconds. What can be checked is that the declaration type mentions no time
//! unit, that its rendering does not, and that the derivation goes through
//! `Estimate` — the three places the rule could quietly be lost.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The declaration is counts, and nothing in it is a time.
#[test]
fn the_declaration_carries_no_duration() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    let (_, after) = source
        .split_once("pub struct Work {")
        .expect("`Work` is the declaration type, and it is declared in planned.rs");
    let (declaration, _) = after.split_once('}').expect("and its fields end somewhere");
    for forbidden in [
        "Duration",
        "Instant",
        "Timestamp",
        "from_secs",
        "from_millis",
        "seconds",
        "minutes",
        "deadline",
        "timeout",
    ] {
        assert!(
            !declaration.contains(forbidden),
            "`Work` mentions `{forbidden}`: a laboratory declares countable units, and a field \
             that is a duration is a declaration in minutes however it is spelled (B-224)"
        );
    }
    for counted in ["pub trials: usize", "pub arms: usize", "pub tokens: u32"] {
        assert!(
            declaration.contains(counted),
            "the declaration is missing `{counted}`: trials, arms and tokens are what a run \
             can be counted in (B-224)"
        );
    }
}

/// The duration is derived, banded, and carries the basis it came from.
#[test]
fn the_duration_is_derived_and_banded() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    assert!(
        source.contains("fn expected(") && source.contains("Estimate<Duration<Monotonic>>"),
        "the duration must arrive as an `Estimate`, which is the type A20's wall is built from"
    );
    assert!(
        source.contains("Estimate::band("),
        "and banded: a duration predicted from a rate is a range, and one number is the \
         smallest possible version of a confident wrong number (B46)"
    );
    assert!(
        source.contains("each.basis().clone()"),
        "carrying the basis of the rate it was derived from, rather than asserting a stronger \
         one — a reader who is told the wrong basis cannot tell how much to believe it"
    );
    assert!(
        !source.contains("Estimate::point("),
        "a point estimate of a duration is exactly what B46 forbids"
    );
}

/// The multiplication saturates, because a wrapped one reads as reassuring.
#[test]
fn an_impossible_amount_of_work_does_not_wrap() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    assert_eq!(
        source.matches("_mul(").count(),
        source.matches("saturating_mul(").count(),
        "every multiplication in the derivation must saturate: a wrapped one turns a century \
         of work into a microsecond, which is the one wrong answer that reads as good news"
    );
    assert!(
        source.matches("saturating_mul(").count() >= 4,
        "the counts and the derivation each multiply, and this check is worth nothing if \
         they stop"
    );
}

/// What the operator is shown is counts, and the estimate is marked in the
/// sentence rather than only in a type nobody sees.
#[test]
fn the_operator_is_shown_counts_and_a_marked_estimate() {
    let rendering = code_only(&read("crates/mcf-bench/src/planned.rs"));
    let (_, after) = rendering
        .split_once("impl fmt::Display for Work")
        .expect("`Work` renders itself");
    let (shown, _) = after
        .split_once("\n}")
        .expect("and that rendering ends somewhere");
    for unit in ["second", "minute", "hour", " ms", "Duration"] {
        assert!(
            !shown.contains(unit),
            "the declaration renders `{unit}`: what a lab says it will do is countable, and \
             the minutes are a separate derived line (B-224)"
        );
    }

    let bench = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        bench.contains("an ESTIMATE from") && bench.contains("never a declaration"),
        "the derived duration must be marked as an estimate in the sentence a reader meets, \
         not only in the type (A20)"
    );
    assert!(
        bench.contains("no expected duration: {why}"),
        "and absent by name where there is no history to derive it from: a figure MCF chose \
         would be indistinguishable on the page from one it measured (A7, B46)"
    );
}

/// Doc comments carry the reasoning and would otherwise trip every check
/// above; the rules are about the code.
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
