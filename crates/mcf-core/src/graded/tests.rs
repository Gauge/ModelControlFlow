//! Four outcomes, no total, and no number out of the three that have none.

use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};

use super::{Graded, LabId, Profile, Score};

fn measured(lab: &str, parts: u64) -> Graded {
    Graded::Measured(Score::new(LabId::new(lab), parts))
}

#[test]
fn only_a_measurement_yields_a_number() {
    assert!(measured("agentic", 800_000).score().is_some());
    for empty in [
        Graded::NotApplicable {
            capability: "tool calling".to_owned(),
        },
        Graded::Unknown {
            why: "nothing has probed this model".to_owned(),
        },
        Graded::Failed(Box::new(Failure::new(
            Category::EngineUnavailable,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-core::graded"),
            "the engine stopped answering",
        ))),
    ] {
        assert_eq!(
            empty.score(),
            None,
            "an outcome with no reading must yield no number: a model with no tool calling \
             that scores 4% on an agentic suite has not been measured badly, it has not been \
             measured (B40)"
        );
        assert!(!empty.is_measured());
    }
}

#[test]
fn not_applicable_and_unknown_are_different_claims() {
    let absent = Graded::NotApplicable {
        capability: "tool calling".to_owned(),
    };
    let unlooked = Graded::Unknown {
        why: "nothing has probed this model".to_owned(),
    };
    assert_ne!(absent, unlooked);
    assert!(absent.to_string().contains("verified absent"));
    assert!(unlooked.to_string().contains("not the same as absent"));
}

#[test]
fn two_scores_from_one_laboratory_compare() {
    let one = Score::new(LabId::new("agentic"), 800_000);
    let other = Score::new(LabId::new("agentic"), 400_000);
    assert_eq!(one.against(&other), Some(core::cmp::Ordering::Greater));
}

#[test]
fn two_scores_from_different_laboratories_do_not() {
    let one = Score::new(LabId::new("agentic"), 800_000);
    let other = Score::new(LabId::new("summarization"), 400_000);
    assert_eq!(
        one.against(&other),
        None,
        "the two scales have no relation, and an ordering here would be one MCF invented (B41)"
    );
}

#[test]
fn a_profile_reports_coverage_and_never_a_total() {
    let held = Profile::empty()
        .and(LabId::new("agentic"), measured("agentic", 800_000))
        .and(
            LabId::new("tools"),
            Graded::NotApplicable {
                capability: "tool calling".to_owned(),
            },
        )
        .and(
            LabId::new("summarization"),
            Graded::Unknown {
                why: "not probed".to_owned(),
            },
        );
    assert_eq!(held.measured().len(), 1);
    assert_eq!(held.inapplicable().len(), 1);
    let shown = held.to_string();
    assert!(shown.contains("measured by 1 of 3"), "{shown}");
    assert!(shown.contains("inapplicable to tools"), "{shown}");
}

#[test]
fn an_empty_profile_says_so_rather_than_reading_as_zero() {
    assert!(
        Profile::empty()
            .to_string()
            .contains("no laboratory has reported")
    );
}
