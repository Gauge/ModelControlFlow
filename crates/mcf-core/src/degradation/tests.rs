use super::{Degradation, Degraded, MaybeDegraded};
use crate::build_identity::BuildIdentity;
use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use crate::measurement::{Conditions, Count, Floor, Measurement};

const WHERE: Subsystem = Subsystem::new("mcf-core::degradation::tests");

fn no_accelerator() -> Degradation {
    Degradation::because(
        Category::AccelAbsent,
        Attribution::Machine,
        WHERE,
        "no accelerator is present, so this ran on the processor",
    )
}

fn thermal() -> Degradation {
    Degradation::because(
        Category::AccelThermalCeiling,
        Attribution::Machine,
        WHERE,
        "the accelerator was throttling throughout",
    )
}

#[test]
fn a_degradation_is_a_classified_failure() {
    let degradation = no_accelerator();
    let cause = degradation.causes().first().expect("one cause");
    assert_eq!(cause.category(), Category::AccelAbsent);
    assert_eq!(cause.attribution(), Attribution::Machine);
    assert_eq!(cause.disposition(), Disposition::Degraded);
    assert_eq!(cause.subsystem(), WHERE);
}

#[test]
fn only_a_failure_mcf_continued_past_can_become_a_degradation() {
    let refused = Failure::new(
        Category::AccelAbsent,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "refused",
    );
    assert!(Degradation::from_failure(refused).is_none());

    let degraded = Failure::new(
        Category::AccelAbsent,
        Attribution::Machine,
        Disposition::Degraded,
        WHERE,
        "continued on the processor",
    );
    assert!(Degradation::from_failure(degraded).is_some());
}

#[test]
fn degradations_compose_and_neither_is_lost() {
    let both = no_accelerator().and(thermal());
    let categories: Vec<Category> = both.causes().iter().map(Failure::category).collect();
    assert_eq!(
        categories,
        [Category::AccelAbsent, Category::AccelThermalCeiling]
    );
}

#[test]
fn the_rendering_carries_the_mark_and_names_the_loss() {
    let conditions = Conditions::new(BuildIdentity::current(), Floor::nothing_known());
    let measurement = Measurement::of(Count(10), Count(30), [Count(20)], conditions);
    let rendered = Degraded::new(measurement, no_accelerator()).to_string();
    assert!(rendered.contains("DEGRADED"), "{rendered}");
    assert!(rendered.contains("accel.absent"), "{rendered}");
    assert!(rendered.contains("n=3"), "{rendered}");
}

#[test]
fn the_mark_survives_a_transformation() {
    let doubled = Degraded::new(Count(21), no_accelerator()).map(|Count(n)| Count(n * 2));
    assert_eq!(doubled.value(), &Count(42));
    assert_eq!(doubled.degradation(), &no_accelerator());
}

#[test]
fn combining_degraded_values_keeps_both_marks() {
    let combined =
        Degraded::new(Count(1), no_accelerator()).zip(Degraded::new(Count(2), thermal()));
    assert_eq!(combined.value(), &(Count(1), Count(2)));
    assert_eq!(combined.degradation().causes().len(), 2);
}

#[test]
fn taking_the_value_hands_back_the_mark() {
    let (value, degradation) = Degraded::new(Count(7), no_accelerator()).into_parts();
    assert_eq!(value, Count(7));
    assert_eq!(degradation, no_accelerator());
}

#[test]
fn an_undegraded_value_reports_no_degradation() {
    let full = MaybeDegraded::Full(Count(3));
    assert!(!full.is_degraded());
    assert_eq!(full.degradation(), None);
    assert_eq!(full.value(), &Count(3));
    assert_eq!(full.to_string(), "3");
}

#[test]
fn marking_an_already_marked_value_keeps_both_marks() {
    let once = MaybeDegraded::Full(Count(1)).degrade(no_accelerator());
    let twice = MaybeDegraded::Reduced(once).degrade(thermal());
    let categories: Vec<Category> = twice
        .degradation()
        .causes()
        .iter()
        .map(Failure::category)
        .collect();
    assert_eq!(
        categories,
        [Category::AccelAbsent, Category::AccelThermalCeiling]
    );
}

#[test]
fn the_sum_type_renders_the_mark_too() {
    let reduced = MaybeDegraded::Reduced(Degraded::new(Count(5), no_accelerator()));
    assert!(reduced.to_string().contains("DEGRADED"));
    assert!(reduced.is_degraded());
}
