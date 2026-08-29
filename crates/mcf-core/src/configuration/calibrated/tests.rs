//! What attribution has to get right.

use crate::attested::Attested;
use crate::configuration::{Sampling, Thousandths};

use super::{Calibrated, Chosen};

fn warm() -> Sampling {
    Sampling {
        temperature: Attested::Known(Thousandths(700)),
        ..Sampling::nothing_set()
    }
}

/// **B60's rule, in one assertion.** There is no way to hold a sampling
/// configuration without saying whose choice it is — every constructor names a
/// source, and there is no other constructor.
#[test]
fn every_way_in_names_a_source() {
    for held in [
        Calibrated::declared_by_artifact(warm(), "generation_config.json"),
        Calibrated::measured_here(warm(), "the quantization sweep"),
        Calibrated::pinned_by_laboratory(warm(), "L26"),
        Calibrated::mcfs_own(warm(), "the artifact recommends nothing"),
    ] {
        assert_eq!(held.sampling(), &warm());
        assert!(
            !format!("{}", held.chosen()).is_empty(),
            "every choice says whose it is"
        );
    }
}

/// An artifact's recommendation is adopted *and* marked, and the mark says
/// where it was read — because *the artifact says so* is not checkable and
/// *this file says so* is (A21).
#[test]
fn a_recommendation_is_adopted_and_marked_with_where_it_was_read() {
    let held = Calibrated::declared_by_artifact(warm(), "generation_config.json");
    assert_eq!(
        held.chosen(),
        &Chosen::DeclaredByArtifact {
            read_from: "generation_config.json".to_owned()
        }
    );
    assert!(!held.chosen().is_measured(), "declared is not measured");
    let text = format!("{held}");
    assert!(text.contains("unverified"), "{text}");
    assert!(text.contains("generation_config.json"), "{text}");
}

/// **The state B60 is most wary of, named rather than absent.** A house choice
/// that says it is a house choice is a condition a reader can weigh; one that
/// does not is a hidden default (§3.15).
#[test]
fn mcfs_own_choice_says_it_is_mcfs_and_why() {
    let held = Calibrated::mcfs_own(warm(), "the artifact recommends nothing");
    let text = format!("{held}");
    assert!(text.contains("MCF's own"), "{text}");
    assert!(text.contains("the artifact recommends nothing"), "{text}");
    assert!(
        text.contains("not measured"),
        "a house choice must not read as a finding: {text}"
    );
    assert!(!held.chosen().is_measured());
}

/// **Kept apart** (B60): a laboratory's pinned method is quarantined from
/// anything that inherited the artifact's recommendation, in both directions.
#[test]
fn a_pinned_laboratory_method_stays_apart() {
    let pinned = Calibrated::pinned_by_laboratory(warm(), "L26");
    let inherited = Calibrated::declared_by_artifact(warm(), "generation_config.json");
    assert!(!pinned.comparable_with(&inherited));
    assert!(!inherited.comparable_with(&pinned));
    assert!(
        !pinned.comparable_with(&Calibrated::mcfs_own(warm(), "nothing was declared")),
        "a pin is apart from MCF's own choice too"
    );
    assert!(pinned.chosen().is_pinned());
}

/// And two laboratories that each pinned their own are two methods, not one.
#[test]
fn two_laboratories_that_each_pinned_are_two_methods() {
    let one = Calibrated::pinned_by_laboratory(warm(), "L26");
    let other = Calibrated::pinned_by_laboratory(warm(), "L19");
    assert!(!one.comparable_with(&other));
    assert!(
        one.comparable_with(&Calibrated::pinned_by_laboratory(warm(), "L26")),
        "one laboratory's own results are comparable with themselves"
    );
}

/// Everything that did not pin is comparable: a recommendation, a measurement
/// and MCF's own choice are all answers to *how should this be sampled*, and
/// A8 already refuses the case where the answers differ.
#[test]
fn what_did_not_pin_stays_comparable() {
    let declared = Calibrated::declared_by_artifact(warm(), "the file's metadata");
    let measured = Calibrated::measured_here(warm(), "a sweep");
    let ours = Calibrated::mcfs_own(warm(), "nothing was declared");
    assert!(declared.comparable_with(&measured));
    assert!(measured.comparable_with(&ours));
    assert!(ours.comparable_with(&declared));
}

/// Attribution is not identity (D17, D18). Two configurations with the same
/// values are the same runnable thing however each arrived at them, which is
/// exactly why the attribution has to travel separately.
#[test]
fn where_a_value_came_from_is_not_part_of_the_value() {
    let declared = Calibrated::declared_by_artifact(warm(), "the file's metadata");
    let ours = Calibrated::mcfs_own(warm(), "nothing was declared");
    assert_eq!(
        declared.sampling(),
        ours.sampling(),
        "the same numbers are the same distribution"
    );
    assert_ne!(
        declared, ours,
        "and the two claims about them are not the same claim"
    );
}
