use crate::attested::Attested;
use crate::configuration::{Sampling, Thousandths};

use super::{Calibrated, Chosen};

fn warm() -> Sampling {
    Sampling {
        temperature: Attested::Known(Thousandths(700)),
        ..Sampling::nothing_set()
    }
}

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

#[test]
fn what_did_not_pin_stays_comparable() {
    let declared = Calibrated::declared_by_artifact(warm(), "the file's metadata");
    let measured = Calibrated::measured_here(warm(), "a sweep");
    let ours = Calibrated::mcfs_own(warm(), "nothing was declared");
    assert!(declared.comparable_with(&measured));
    assert!(measured.comparable_with(&ours));
    assert!(ours.comparable_with(&declared));
}

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
