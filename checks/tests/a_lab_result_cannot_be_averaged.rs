#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_core::graded::{Graded, LabId, Profile, Score};

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

#[test]
fn no_foreign_number_can_reach_a_recommendation() {
    let held = source();
    assert!(
        held.contains("profile: crate::origin::LocallyMeasured<Profile>"),
        "a candidate must hold a locally-measured profile and nothing else: the failure mode \
         is not somebody deliberately ranking on foreign data, it is a number arriving through \
         three layers of helpers with nobody noticing (B34, B43)"
    );
    for foreign in ["FromCorpus", "fn from_corpus", "impl From<FromCorpus"] {
        assert!(
            !held.contains(&format!("{foreign}<Profile>")),
            "`{foreign}` here would be a route for a contributed measurement to become an \
             input to a recommendation (B-167, §6.28, §5)"
        );
    }
    let origin = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-core/src/origin.rs"),
    )
    .expect("origin.rs is readable");
    for conversion in [
        "impl From<FromCorpus",
        "impl From<LocallyMeasured",
        "fn into_local",
        "fn confirm",
    ] {
        assert!(
            !origin.contains(conversion),
            "`{conversion}` would let a corpus value become a local one by being confirmed; \
             A20's shape applies — it is replaced by a measurement, never promoted into one"
        );
    }
}

#[test]
fn a_single_candidate_produces_a_refusal_with_its_reasoning() {
    let held = source();
    assert!(
        held.contains("AFieldOfOne"),
        "§6.23: a frontier with a single point is not a frontier, and the refusal is a named \
         outcome rather than an empty list"
    );
    assert!(
        held.contains("only: String"),
        "and it names the one candidate, because the operator's next move is to name a second"
    );
    assert!(
        held.contains("TooFewMeasured"),
        "a field of six of which one has a reading is a field of one wearing six names (B40)"
    );
}

#[test]
fn coverage_is_computed_from_the_field_it_describes() {
    let held = source();
    assert!(
        held.contains("pub fn coverage(&self, lab: &LabId) -> Coverage"),
        "B41 requires coverage with every answer: a ranking is what a reader takes away and \
         the coverage is what tells them how much it is worth"
    );
    for stored in ["coverage: Coverage", "struct Field {\n    coverage"] {
        assert!(
            !held.contains(stored),
            "a stored coverage can describe a different set of candidates than the ranking it \
             sits beside; computing it from the field is what keeps the two together"
        );
    }
    assert!(
        held.contains("verified absent, which is not a low score"),
        "and the rendering must keep B40's distinction, or the coverage reintroduces the \
         failure the outcome type exists to prevent"
    );
}

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
