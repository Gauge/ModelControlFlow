//! Outcomes leave; tasks, outputs and retractions do not
//! (B-171, B-203, B-251, B-310, B42, B54, B63, D21, §6.30, §3.19, §3.20).
//!
//! **Why the shape and not a filter.** A contribution that strips task content
//! on the way out is one line away from not stripping it, and the line is in
//! the export path where nobody looks twice. A contribution with nowhere to
//! *put* task content cannot leak it however the export is written — so an
//! audit finds no task content because there was never anywhere for it to be.
//!
//! The stakes are not only privacy: a benchmark task that travels ends up in
//! somebody's training data, and a corpus that leaks its own tasks measures
//! memorization from then on.
//!
//! **And no retraction, ever** (B-310, D21). Once something has left, it has
//! left. An affordance suggesting otherwise would be the most consequential
//! false promise MCF could make, because a person would rely on it. This file
//! forbids the method by name so that adding one is a deliberate act against a
//! test rather than a helpful-looking commit.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

fn source() -> String {
    std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-core/src/contribution.rs"),
    )
    .expect("contribution.rs is readable")
    .lines()
    .filter(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
    })
    .collect::<Vec<&str>>()
    .join("\n")
}

/// There is nowhere in the format to put an artifact.
#[test]
fn no_field_can_hold_a_task_or_an_output() {
    // Without the terms, which name every one of these in order to say they
    // are absent — the third time in this repository a check has had to skip
    // the sentence explaining why it passes (F81).
    let held = source();
    let held = held
        .split_once("pub const TERMS")
        .map_or(held.clone(), |(before, after)| {
            let (_, rest) = after.split_once("\";").unwrap_or_default();
            format!("{before}{rest}")
        });
    for forbidden in [
        "prompt",
        "completion",
        "output",
        "text:",
        "task",
        "fixture",
        "document",
        "PathBuf",
        "bytes",
    ] {
        assert!(
            !held.contains(forbidden),
            "`{forbidden}` in the contribution format is somewhere for task content to be put, \
             and a benchmark task that travels ends up in somebody's training data (B-171, \
             §6.30)"
        );
    }
}

/// No code path offers a retraction.
#[test]
fn nothing_here_offers_to_unsend_anything() {
    let held = source();
    for promise in [
        "fn retract",
        "fn unsend",
        "fn withdraw",
        "fn recall",
        "fn delete",
        "fn revoke",
    ] {
        assert!(
            !held.contains(promise),
            "`{promise}` would promise something MCF cannot do: publication cannot be undone \
             (D21, B63, §3.20)"
        );
    }
    assert!(
        held.contains("cannot be undone"),
        "and the terms must say so before anything is sent, rather than leaving a reader to \
         discover it afterwards"
    );
}

/// A custom workload is marked at production and refused at both routes.
#[test]
fn a_custom_workload_is_refused_by_construction() {
    let held = source();
    assert!(
        held.contains("pub workload: Workload") && held.contains("workload: Workload,"),
        "B-203: the marking travels on the row from production — one applied at export is one \
         that can be forgotten at export (B42, A25)"
    );
    assert_eq!(
        held.matches("NotContributable::WorkloadIsCustom").count(),
        2,
        "both routes in — a comparison and an absolute — must refuse it, and a complete \
         condition set must not rescue it"
    );
}

/// An absolute cannot exist without the conditions that make it readable.
#[test]
fn a_bare_number_cannot_be_constructed() {
    let held = source();
    assert!(
        held.contains("pub fn new(") && held.contains("Result<Self, NotContributable>"),
        "B54: an absolute must be fallible to construct, so that a bare duration cannot exist \
         as a contributable row at all"
    );
    assert!(
        held.contains("if known < of"),
        "and the test is completeness of the condition floor, which is what makes somebody \
         else's number scalable (§3.4)"
    );
    assert!(
        !held.contains("impl Default for Absolute"),
        "a default would be a bare number with a straight face"
    );
}

/// The type prefers comparisons, and says why.
#[test]
fn a_comparison_needs_less_than_an_absolute() {
    let held = source();
    let (_, comparison) = held
        .split_once("pub struct Comparison {")
        .expect("a comparison is a row shape");
    let (comparison, _) = comparison.split_once('}').expect("and it ends");
    assert!(
        comparison.contains("pub pairs: usize") && comparison.contains("pub effect:"),
        "§3.27: what travels is both arms, the pairing and the effect size (B-251)"
    );
    assert!(
        !comparison.contains("nanoseconds"),
        "a comparison carries a ratio, not a duration: the duration is the part that does not \
         survive travel"
    );
}
