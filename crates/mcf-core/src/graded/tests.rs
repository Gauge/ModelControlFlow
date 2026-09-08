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

mod fields {
    use crate::graded::{Candidate, Field, Graded, LabId, NoRecommendation, Profile, Score};
    use crate::origin::LocallyMeasured;

    fn candidate(name: &str, lab: &str, parts: Option<u64>) -> Candidate {
        let graded = parts.map_or(
            Graded::Unknown {
                why: "not probed".to_owned(),
            },
            |held| Graded::Measured(Score::new(LabId::new(lab), held)),
        );
        Candidate::new(
            name,
            LocallyMeasured::new(Profile::empty().and(LabId::new(lab), graded)),
        )
    }

    #[test]
    fn a_field_of_one_is_refused_and_names_the_one() {
        let field = Field::empty().and(candidate("a", "agentic", Some(800_000)));
        let held = field.ordered_by(&LabId::new("agentic"));
        assert_eq!(
            held,
            Err(NoRecommendation::AFieldOfOne {
                only: "a".to_owned()
            }),
            "a frontier with a single point is not a frontier (§6.23)"
        );
        let Err(why) = held else {
            panic!("checked just above");
        };
        assert!(
            why.to_string()
                .contains("recommends whatever it was handed"),
            "and the refusal must say why, since *the best of one* reads exactly like a \
             comparison: {why}"
        );
    }

    #[test]
    fn an_empty_field_is_not_a_field_of_one() {
        assert_eq!(
            Field::empty().ordered_by(&LabId::new("agentic")),
            Err(NoRecommendation::AFieldOfNone)
        );
    }

    #[test]
    fn a_field_of_six_with_one_reading_is_a_field_of_one() {
        let mut field = Field::empty().and(candidate("a", "agentic", Some(800_000)));
        for named in ["b", "c", "d", "e", "f"] {
            field = field.and(candidate(named, "agentic", None));
        }
        assert_eq!(
            field.ordered_by(&LabId::new("agentic")),
            Err(NoRecommendation::TooFewMeasured {
                lab: LabId::new("agentic"),
                measured: 1
            }),
            "the other five were not measured badly, they were not measured (B40)"
        );
    }

    #[test]
    fn a_real_field_is_ordered_best_first() {
        let field = Field::empty()
            .and(candidate("slow", "agentic", Some(200_000)))
            .and(candidate("quick", "agentic", Some(800_000)))
            .and(candidate("unprobed", "agentic", None));
        let ordered = field
            .ordered_by(&LabId::new("agentic"))
            .expect("two candidates were measured");
        assert_eq!(
            ordered
                .iter()
                .map(|candidate| candidate.name())
                .collect::<Vec<&str>>(),
            ["quick", "slow"],
            "and the unmeasured candidate is absent rather than last: a missing reading is not \
             a low one (B-200)"
        );
    }

    #[test]
    fn a_laboratory_nobody_measured_yields_a_refusal_not_an_empty_ranking() {
        let field = Field::empty()
            .and(candidate("a", "agentic", Some(1)))
            .and(candidate("b", "agentic", Some(2)));
        assert!(
            matches!(
                field.ordered_by(&LabId::new("summarization")),
                Err(NoRecommendation::TooFewMeasured { .. })
            ),
            "an empty list would read as *nothing is any good* rather than *nothing was asked*"
        );
    }
}

mod coverage {
    use crate::graded::{Candidate, Field, Graded, LabId, Profile, Score};
    use crate::origin::LocallyMeasured;

    fn candidate(name: &str, entries: Vec<(&str, Graded)>) -> Candidate {
        let mut profile = Profile::empty();
        for (lab, graded) in entries {
            profile = profile.and(LabId::new(lab), graded);
        }
        Candidate::new(name, LocallyMeasured::new(profile))
    }

    fn measured(lab: &str, parts: u64) -> Graded {
        Graded::Measured(Score::new(LabId::new(lab), parts))
    }

    #[test]
    fn a_ranking_on_two_of_eleven_laboratories_says_so() {
        let field = Field::empty()
            .and(candidate(
                "a",
                vec![
                    ("agentic", measured("agentic", 800_000)),
                    (
                        "tools",
                        Graded::NotApplicable {
                            capability: "tool calling".to_owned(),
                        },
                    ),
                    (
                        "vision",
                        Graded::Unknown {
                            why: "not probed".to_owned(),
                        },
                    ),
                ],
            ))
            .and(candidate(
                "b",
                vec![("agentic", measured("agentic", 200_000))],
            ));
        let held = field.coverage(&LabId::new("agentic"));
        assert_eq!(held.measured, 2);
        assert_eq!(held.considered, 2);
        assert_eq!(held.inapplicable, [LabId::new("tools")]);
        assert_eq!(held.silent, [LabId::new("vision")]);

        let shown = held.to_string();
        assert!(shown.contains("measured 2 of 2 candidate(s)"), "{shown}");
        assert!(
            shown.contains("inapplicable to tools"),
            "the negative half is the one most likely to be dropped (B41): {shown}"
        );
        assert!(
            shown.contains("informed nothing here"),
            "and *not run* is not *no difference* (A7): {shown}"
        );
    }

    #[test]
    fn a_laboratory_with_one_reading_is_not_silent() {
        let field = Field::empty()
            .and(candidate("a", vec![("agentic", measured("agentic", 1))]))
            .and(candidate(
                "b",
                vec![(
                    "agentic",
                    Graded::Unknown {
                        why: "not probed".to_owned(),
                    },
                )],
            ));
        let held = field.coverage(&LabId::new("agentic"));
        assert!(
            held.silent.is_empty(),
            "what a reader needs is *nothing came from here at all*, and a laboratory with one \
             reading is not that: {:?}",
            held.silent
        );
        assert_eq!(held.measured, 1);
    }

    #[test]
    fn coverage_counts_every_candidate_considered_not_only_the_ranked_ones() {
        let field = Field::empty()
            .and(candidate("a", vec![("agentic", measured("agentic", 1))]))
            .and(candidate("b", vec![("agentic", measured("agentic", 2))]))
            .and(candidate(
                "c",
                vec![(
                    "agentic",
                    Graded::Unknown {
                        why: "not probed".to_owned(),
                    },
                )],
            ));
        let held = field.coverage(&LabId::new("agentic"));
        assert_eq!(
            (held.measured, held.considered),
            (2, 3),
            "*two of three* is the claim; *two* alone hides the one that was not measured (A1)"
        );
    }
}
