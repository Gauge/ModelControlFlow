//! What a look upstream can say, and what it must not.

use super::{Decay, Observation};
use crate::attested::Attested;
use crate::time::Timestamp;

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

/// A hub that will not answer is not a decay: it says nothing about whether
/// anything changed, and calling it one would report an absence as an event
/// (A7, F17).
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

/// And it says so where a person will read it: the sentence names the ambiguity
/// rather than implying the artifact is gone.
#[test]
fn an_unreachable_repository_names_the_ambiguity() {
    let said = Decay::Unreachable {
        said: "hub.auth.required".to_owned(),
    }
    .to_string();
    assert!(said.contains("private"), "{said}");
    assert!(said.contains("never existed"), "{said}");
}

/// *Checked and unchanged* is a fact, distinct from *never checked* — which is
/// the absence of any observation at all.
#[test]
fn unchanged_is_a_finding_rather_than_an_absence() {
    let observed = Observation::new(AT, Decay::Unchanged);
    assert_eq!(observed.found.as_str(), "unchanged");
    assert!(observed.to_string().contains("has changed"));
}

/// Every finding has a name a record can carry, and they are all different.
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
