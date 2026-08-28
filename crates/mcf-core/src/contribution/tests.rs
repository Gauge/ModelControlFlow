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

#[test]
fn a_contribution_says_what_it_holds() {
    let held = Contribution::empty()
        .and_comparison(comparison(Workload::Declared))
        .expect("declared")
        .and_absolute(
            Absolute::new(Arm::new("a"), 1, everything_known(), Workload::Declared)
                .expect("complete"),
        );
    assert_eq!(
        held.to_string(),
        "2 row(s): 1 comparison(s) and 1 absolute(s)"
    );
}
