//! Four outcomes, no total (B-200, B-201, B40, B41, D2, §3.23, §3.9).
//!
//! **What goes wrong without this.** B40: *a model with no tool-calling that
//! scores 4% on an agentic suite has not been measured badly — it has not been
//! measured.* Once that four percent is a number in a column, nothing
//! downstream can tell it from a real one. It sorts, it averages, it loses a
//! comparison, and it becomes a verdict about a model arrived at by grading it
//! on a capability it does not have.
//!
//! B40 and B41 both name `compiler` as their check, and most of the work is
//! done by the shape: `NotApplicable`, `Unknown` and `Failed` have nowhere to
//! put a score, and `Profile` has no arithmetic. What a compiler cannot check
//! is that nobody *adds* the escape hatch, which is what this file is for —
//! every method named here would be a one-line change that silently restores
//! the failure.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_core::graded::{Graded, LabId, Profile, Score};

/// The code, without the doc comments.
///
/// The prose names the escape hatches in order to say they are absent, so a
/// check reading the whole file would fail on the sentence explaining why it
/// passes. A check that blocks the correct work teaches people to write around
/// it.
fn source() -> String {
    std::fs::read_to_string(mcf_checks::workspace::root().join("crates/mcf-core/src/graded.rs"))
        .expect("graded.rs is readable")
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// There is exactly one way a number leaves an outcome, and it is fallible.
#[test]
fn no_outcome_without_a_reading_yields_a_number() {
    let held = source();
    assert!(
        held.contains("pub const fn score(&self) -> Option<&Score>"),
        "the only way out must be an `Option`, so that a caller meets the three cases with no \
         reading at the point where they were about to flatten them (B40)"
    );
    for hatch in [
        "unwrap_or",
        "score_or",
        "impl Default for Graded",
        "fn as_number",
        "fn value(&self) -> u64",
    ] {
        assert!(
            !held.contains(hatch),
            "`{hatch}` would turn *not measured* into a number, which is the whole failure \
             B40 names"
        );
    }
}

/// The three outcomes with no reading really have nowhere to put one.
#[test]
fn the_empty_outcomes_carry_no_score() {
    for empty in [
        Graded::NotApplicable {
            capability: "tool calling".to_owned(),
        },
        Graded::Unknown {
            why: "not probed".to_owned(),
        },
    ] {
        assert_eq!(empty.score(), None);
        assert!(!empty.is_measured());
    }
}

/// Scores from two laboratories do not compare, and the type says so rather
/// than picking an answer.
#[test]
fn no_ordering_spans_two_laboratories() {
    let one = Score::new(LabId::new("agentic"), 800_000);
    let other = Score::new(LabId::new("summarization"), 400_000);
    assert_eq!(
        one.against(&other),
        None,
        "the two scales have no relation, and an ordering here would be one MCF invented (B41)"
    );

    let held = source();
    for spanning in [
        "impl PartialOrd for Score",
        "impl Ord for Score",
        "derive(PartialOrd",
        "impl core::ops::Add",
        "impl core::iter::Sum",
    ] {
        assert!(
            !held.contains(spanning),
            "`{spanning}` on a score would let two laboratories be added or ranked against each \
             other, which is §5's leaderboard wearing local clothes (B41, §3.9)"
        );
    }
}

/// A profile has no total, and no way to grow one.
#[test]
fn a_profile_has_no_overall_number() {
    let held = source();
    let (_, profile) = held
        .split_once("pub struct Profile")
        .expect("`Profile` is where outcomes are held together");
    for total in [
        "fn overall",
        "fn average",
        "fn mean",
        "fn total",
        "fn rating",
        "fn rank",
        "sum()",
        "fold(",
    ] {
        assert!(
            !profile.contains(total),
            "`{total}` would combine two laboratories into a scalar, and *which model is better* \
             has no referent once quality is plural (B-201, B41, D2)"
        );
    }
}

/// Coverage travels, which is the half a reader is least likely to be shown.
#[test]
fn a_profile_renders_what_it_did_not_measure() {
    let shown = Profile::empty()
        .and(
            LabId::new("agentic"),
            Graded::Measured(Score::new(LabId::new("agentic"), 800_000)),
        )
        .and(
            LabId::new("tools"),
            Graded::NotApplicable {
                capability: "tool calling".to_owned(),
            },
        )
        .to_string();
    assert!(shown.contains("measured by 1 of 2"), "{shown}");
    assert!(
        shown.contains("inapplicable to tools"),
        "B41 makes coverage travel with every answer, and *inapplicable to* is the half most \
         likely to be dropped: {shown}"
    );
}
