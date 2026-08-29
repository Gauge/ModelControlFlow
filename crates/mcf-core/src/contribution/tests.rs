//! Outcomes leave; artifacts, custom workloads and bare numbers do not.

use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::measurement::{ConditionValue, Conditions, Floor, PartsPerMillion};
use crate::trial::Arm;

use super::{Absolute, Comparison, Contribution, NotContributable, Row, Workload};

fn nothing_known() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

fn everything_known() -> Conditions {
    let stated = || Attested::Known(ConditionValue::text("stated"));
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: stated(),
            thermal_state: stated(),
            driver_versions: stated(),
            runtime_versions: stated(),
            quantization: stated(),
            context_length: stated(),
            batch_shape: stated(),
            mcf_configuration: stated(),
            realized_placement: stated(),
            instrumentation: stated(),
            artifact_storage: stated(),
            seed_set: stated(),
            reuse: stated(),
        },
    )
}

fn comparison(workload: Workload) -> Comparison {
    Comparison {
        left: Arm::new("a"),
        right: Arm::new("b"),
        pairs: 40,
        effect: PartsPerMillion(120_000),
        left_quicker: true,
        conditions: nothing_known(),
        workload,
    }
}

#[test]
fn a_comparison_travels_without_a_complete_floor() {
    // §3.27: both arms met the same afternoon, so what differs between them
    // is the arm — which is why a ratio survives travel where a bare number
    // does not.
    let held = Contribution::empty()
        .and_comparison(comparison(Workload::Declared))
        .expect("a declared workload is contributable");
    assert_eq!(held.rows().len(), 1);
    assert!(matches!(held.rows().first(), Some(Row::Compared(_))));
}

#[test]
fn an_absolute_without_its_conditions_is_refused() {
    let held = Absolute::new(
        Arm::new("a"),
        380_000_000,
        nothing_known(),
        Workload::Declared,
    );
    let Err(NotContributable::ConditionsIncomplete { known, of }) = held else {
        panic!("a bare number is a fact about somebody else's hardware (B54): {held:?}");
    };
    assert_eq!(known, 0);
    assert_eq!(of, 13);
}

#[test]
fn an_absolute_with_its_conditions_is_allowed() {
    let held = Absolute::new(
        Arm::new("a"),
        380_000_000,
        everything_known(),
        Workload::Declared,
    )
    .expect("every condition is stated");
    assert_eq!(held.nanoseconds(), 380_000_000);
}

#[test]
fn a_custom_workload_cannot_be_contributed_by_either_route() {
    assert_eq!(
        Contribution::empty()
            .and_comparison(comparison(Workload::Custom))
            .err(),
        Some(NotContributable::WorkloadIsCustom),
        "B42: nobody else has that workload, so nobody else can reproduce the result"
    );
    assert_eq!(
        Absolute::new(Arm::new("a"), 1, everything_known(), Workload::Custom).err(),
        Some(NotContributable::WorkloadIsCustom),
        "and a complete condition set does not rescue it: the objection is not that the \
         conditions are unknown"
    );
}

#[test]
fn the_marking_is_on_the_row_rather_than_applied_at_export() {
    // B-203: the marking exists before export, not at it. A row carries where
    // its workload came from, so the refusal is a consequence of what the row
    // already says.
    assert!(!Workload::Custom.is_contributable());
    assert!(Workload::Declared.is_contributable());
    assert!(
        Workload::Custom.to_string().contains("not contributable"),
        "and it says so where somebody reads it"
    );
}

#[test]
fn the_terms_say_what_leaves_and_that_it_cannot_be_undone() {
    let terms = Contribution::terms();
    assert!(terms.contains("outcomes only"), "{terms}");
    assert!(
        terms.contains("nowhere in the format to put one"),
        "the guarantee is structural, and saying so is what makes it believable: {terms}"
    );
    assert!(
        terms.contains("cannot be undone"),
        "D21: publication cannot be undone, and the terms must say so before anything is \
         sent: {terms}"
    );
}

/// A contribution renders **the rows**, not a description of them (A24, B-160).
///
/// This asserted the count alone, because that was all there was to render. A24
/// requires that a person deciding whether to publish something irreversible be
/// shown what it says — and a count is exactly the substitute the rule names.
#[test]
fn a_contribution_shows_the_rows_it_would_send_and_then_counts_them() {
    let held = Contribution::empty()
        .and_comparison(comparison(Workload::Declared))
        .expect("declared")
        .and_absolute(
            Absolute::new(Arm::new("a"), 1, everything_known(), Workload::Declared)
                .expect("complete"),
        );
    let rendered = held.to_string();

    // The rows themselves: both arms, the direction, the size and the pairs.
    assert!(
        rendered.contains("comparison · a quicker than b by 12.0%, over 40 pair(s)"),
        "{rendered}"
    );
    assert!(rendered.contains("absolute · a took 1 ns"), "{rendered}");
    // Each row says where its workload came from, because a row from somebody's
    // own workload is one nobody else can interpret (B42).
    assert_eq!(rendered.matches("· workload ").count(), 2, "{rendered}");
    // And the count, after them rather than instead of them.
    assert!(
        rendered.ends_with("2 row(s): 1 comparison(s) and 1 absolute(s)"),
        "{rendered}"
    );
}

/// An import is a claim until this machine measures it (B-166, B-172).
mod imports {
    use crate::contribution::{Imported, Reproduction};
    use crate::origin::{FromCorpus, LocallyMeasured};

    #[test]
    fn an_import_renders_as_declared_and_names_what_would_change_that() {
        let held = Imported::new(
            "a-configuration-somebody-else-named",
            FromCorpus::new(380_u64, 12),
        );
        let shown = held.to_string();
        assert!(
            shown.contains("DECLARED elsewhere"),
            "A21: an imported figure must not render as though MCF measured it: {shown}"
        );
        assert!(
            shown.contains("`mcf probe` and `mcf bench`"),
            "and must name what would make it a measurement, or the marking is a disclaimer: \
             {shown}"
        );
        assert_eq!(
            held.claimed().reports(),
            12,
            "B44: the sample count travels"
        );
    }

    #[test]
    fn nothing_attempted_is_not_agreement() {
        let shown = Reproduction::<u64>::NotAttempted.to_string();
        assert!(shown.contains("not agreement"), "{shown}");
        for wrong in ["confirmed", "agrees", "holds"] {
            assert!(!shown.contains(wrong), "{wrong} in {shown}");
        }
    }

    #[test]
    fn a_divergence_keeps_both_figures() {
        let held = Reproduction::Measured {
            theirs: FromCorpus::new(380_u64, 12),
            ours: LocallyMeasured::new(520_u64),
        };
        let shown = held.to_string();
        assert!(shown.contains("520"), "{shown}");
        assert!(
            shown.contains("380"),
            "keeping only the local figure throws away the comparison, and keeping only the \
             difference throws away what was compared (A1): {shown}"
        );
        assert!(
            shown.contains("not an error"),
            "§6.29: a difference between two machines is evidence about how far a result \
             travels: {shown}"
        );
    }

    #[test]
    fn not_fitting_here_is_a_complete_answer() {
        let held = Reproduction::<u64>::WillNotFitHere {
            needs: "48 GiB".to_owned(),
            has: "24 GiB".to_owned(),
        };
        let shown = held.to_string();
        assert!(
            shown.contains("48 GiB") && shown.contains("24 GiB"),
            "{shown}"
        );
        assert!(
            shown.contains("complete answer"),
            "§6.3, A9: *this needs 48 GiB and you have 24* answers the question that was \
             asked, and storing it as a failure would make the register unable to say what \
             this machine has been told it cannot run: {shown}"
        );
    }
}
