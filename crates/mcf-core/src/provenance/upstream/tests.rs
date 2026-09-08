use super::{Decay, Observation};
use crate::attested::Attested;
use crate::time::Timestamp;

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

#[test]
fn an_unreachable_repository_is_not_a_change() {
    assert!(
        !Decay::Unreachable {
            said: "hub.auth.required".to_owned()
        }
        .is_a_change()
    );
    assert!(!Decay::Unchanged.is_a_change());
    assert!(
        Decay::RevisionGone {
            revision: "abc123".to_owned()
        }
        .is_a_change()
    );
}

#[test]
fn a_refused_repository_names_the_ambiguity() {
    let said = Decay::Unreachable {
        said: "hub.auth.required".to_owned(),
    }
    .to_string();
    assert!(said.contains("private"), "{said}");
    assert!(said.contains("never existed"), "{said}");
}

#[test]
fn a_hub_that_could_not_be_reached_does_not_borrow_that_sentence() {
    let said = Decay::Unreachable {
        said: "hub.unreachable".to_owned(),
    }
    .to_string();
    assert!(!said.contains("private"), "{said}");
    assert!(!said.contains("never existed"), "{said}");
    assert!(said.contains("reaching the hub"), "{said}");
}

#[test]
fn unchanged_is_a_finding_rather_than_an_absence() {
    let observed = Observation::new(AT, Decay::Unchanged);
    assert_eq!(observed.found.as_str(), "unchanged");
    assert!(observed.to_string().contains("has changed"));
}

#[test]
fn every_finding_has_its_own_name() {
    let all = [
        Decay::Unchanged,
        Decay::RevisionGone {
            revision: "a".to_owned(),
        },
        Decay::Relicensed {
            was: "a".to_owned(),
            now: "b".to_owned(),
        },
        Decay::Gated {
            how: "manual".to_owned(),
        },
        Decay::Replaced {
            file: "a".to_owned(),
            was: "b".to_owned(),
            now: "c".to_owned(),
        },
        Decay::Unreachable {
            said: "a".to_owned(),
        },
    ];
    let mut names: Vec<&str> = all.iter().map(Decay::as_str).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), all.len(), "two findings share a name");
}
