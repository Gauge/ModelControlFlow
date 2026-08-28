//! What isolation has to get right.

use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::measurement::{ConditionValue, Conditions, Floor};

use super::{INSTRUMENT, Isolation};

/// A floor in which every condition is known and set to the same value, so
/// that a test changes exactly what it means to change.
fn everything_known() -> Floor {
    Floor {
        hardware_state: known("this machine"),
        thermal_state: known("steady"),
        driver_versions: known("none"),
        runtime_versions: known("stand-in 0.1"),
        quantization: known("q8_0"),
        context_length: Attested::Known(ConditionValue::integer(2048)),
        batch_shape: Attested::Known(ConditionValue::integer(1)),
        mcf_configuration: known("default"),
        realized_placement: known("host"),
        instrumentation: known("recording"),
        artifact_storage: known("tmpfs"),
        seed_set: known("mcf-standard-v1"),
        reuse: known("cold: every trial loaded the model for itself"),
    }
}

fn known(value: &str) -> Attested<ConditionValue> {
    Attested::Known(ConditionValue::text(value))
}

fn conditions(floor: Floor) -> Conditions {
    Conditions::new(BuildIdentity::current(), floor)
}

/// One variable differing is the only shape in which a delta means what a
/// reader will take it to mean.
#[test]
fn one_difference_is_isolated_and_named() {
    let one = conditions(everything_known());
    let mut floor = everything_known();
    floor.quantization = known("q2_k");
    let held = Isolation::between(&one, &conditions(floor));
    assert_eq!(
        held,
        Isolation::Isolated {
            variable: "quantization"
        }
    );
    assert!(held.isolates_a_variable());
    assert!(!held.is_confounded());
}

/// **A8's violation.** Two conditions differing is not a delta, and the
/// refusal names every one of them.
#[test]
fn two_differences_are_not_comparable() {
    let one = conditions(everything_known());
    let mut floor = everything_known();
    floor.quantization = known("q2_k");
    floor.thermal_state = known("hot");
    let held = Isolation::between(&one, &conditions(floor));
    let Isolation::Confounded { differ } = &held else {
        panic!("two differences must be a confound: {held}")
    };
    assert_eq!(differ, &["thermal_state", "quantization"]);
    assert!(!held.isolates_a_variable());
    assert!(
        format!("{held}").contains("not comparable"),
        "A8's words, printed: {held}"
    );
}

/// Two arms of one configuration are not a confound and are not an isolation:
/// they measure the machine, which is the control a comparison runs against
/// itself.
#[test]
fn no_difference_is_one_configuration_measured_twice() {
    let held = Isolation::between(
        &conditions(everything_known()),
        &conditions(everything_known()),
    );
    assert_eq!(held, Isolation::SameConfiguration);
    assert!(!held.isolates_a_variable(), "a control isolates nothing");
    assert!(!held.is_confounded());
}

/// **A7 applied to a comparison.** Two unknowns are not a match — *they were
/// probably the same* is a plausible value substituted for something MCF did
/// not read.
#[test]
fn two_unknowns_are_not_a_match() {
    let mut floor = everything_known();
    floor.thermal_state = Attested::Unknown;
    let one = conditions(floor.clone());
    let held = Isolation::between(&one, &conditions(floor));
    let Isolation::Undetermined { differ, unread } = &held else {
        panic!("an unread condition cannot be called a match: {held}")
    };
    assert!(differ.is_empty());
    assert_eq!(unread, &["thermal_state"]);
    assert!(!held.isolates_a_variable());
}

/// A single difference beside an unread condition is *not* claimed as an
/// isolation: MCF has not read enough to say the other conditions matched.
#[test]
fn one_difference_beside_an_unread_condition_is_undetermined() {
    let one = conditions(everything_known());
    let mut floor = everything_known();
    floor.quantization = known("q2_k");
    floor.artifact_storage = Attested::Unknown;
    let held = Isolation::between(&one, &conditions(floor));
    assert_eq!(
        held,
        Isolation::Undetermined {
            differ: vec!["quantization"],
            unread: vec!["artifact_storage"],
        }
    );
}

/// **Two known differences are a confound whatever else is unread.** A
/// comparison that has lost its meaning does not recover it by MCF failing to
/// read a twelfth condition.
#[test]
fn a_confound_is_not_softened_by_an_unread_condition() {
    let one = conditions(everything_known());
    let mut floor = everything_known();
    floor.quantization = known("q2_k");
    floor.context_length = Attested::Known(ConditionValue::integer(4096));
    floor.artifact_storage = Attested::Unknown;
    let held = Isolation::between(&one, &conditions(floor));
    assert!(
        held.is_confounded(),
        "an unread condition must not turn a confound into an undetermined: {held}"
    );
}

/// A floor of nothing known is not a match with itself, which is what a
/// comparison taken before the condition producers exist honestly is.
#[test]
fn nothing_known_is_undetermined_in_every_condition() {
    let held = Isolation::between(
        &conditions(Floor::nothing_known()),
        &conditions(Floor::nothing_known()),
    );
    let Isolation::Undetermined { differ, unread } = &held else {
        panic!("an entirely unread floor cannot be a match: {held}")
    };
    assert!(differ.is_empty());
    assert_eq!(
        unread.len(),
        Floor::nothing_known().entries().len(),
        "every question is unread, and every one is named"
    );
}

/// The instrument is a variable too: two arms measured by different builds of
/// MCF differ in the thing doing the measuring (B64).
#[test]
fn the_instrument_counts_as_a_condition() {
    let mine = BuildIdentity::current();
    let theirs = BuildIdentity {
        version: "0.0.0-other",
        ..mine
    };
    let held = Isolation::between(
        &Conditions::new(mine, everything_known()),
        &Conditions::new(theirs, everything_known()),
    );
    assert_eq!(
        held,
        Isolation::Isolated {
            variable: INSTRUMENT
        }
    );
}

/// The list of conditions comes from the floor rather than from a copy of it,
/// so a condition added to the floor is checked without anybody editing this
/// module.
#[test]
fn every_condition_in_the_floor_can_be_the_isolated_one() {
    let base = everything_known();
    for (question, _) in base.entries() {
        let mut floor = everything_known();
        for (name, slot) in [
            ("hardware_state", &mut floor.hardware_state),
            ("thermal_state", &mut floor.thermal_state),
            ("driver_versions", &mut floor.driver_versions),
            ("runtime_versions", &mut floor.runtime_versions),
            ("quantization", &mut floor.quantization),
            ("context_length", &mut floor.context_length),
            ("batch_shape", &mut floor.batch_shape),
            ("mcf_configuration", &mut floor.mcf_configuration),
            ("realized_placement", &mut floor.realized_placement),
            ("instrumentation", &mut floor.instrumentation),
            ("artifact_storage", &mut floor.artifact_storage),
            ("seed_set", &mut floor.seed_set),
            ("reuse", &mut floor.reuse),
        ] {
            if name == question {
                *slot = known("something else entirely");
            }
        }
        assert_eq!(
            Isolation::between(&conditions(everything_known()), &conditions(floor)),
            Isolation::Isolated { variable: question },
            "changing {question} alone must isolate {question}"
        );
    }
}
