use std::path::{Path, PathBuf};

use super::{Credential, Identity, KNOWN_VARIABLES, Origin, Secret, Sighting, sightings};

fn nothing_set(_variable: &str) -> Option<String> {
    None
}

fn nothing_readable(_path: &Path) -> Option<String> {
    None
}

#[test]
fn debug_shows_the_fingerprint_and_not_the_token() {
    let secret = Secret::new("hf_averyrealtokenindeed");
    let rendered = format!("{secret:?}");
    assert!(
        !rendered.contains("hf_averyrealtokenindeed"),
        "the token appeared in Debug: {rendered}"
    );
    assert!(rendered.contains(secret.fingerprint()), "{rendered}");
    assert!(rendered.contains("redacted"), "{rendered}");
}

#[test]
fn a_credential_inside_something_else_is_still_redacted() {
    let credential = Credential::new(Secret::new("hf_secret"), Origin::Supplied);
    let rendered = format!("{credential:?}");
    assert!(!rendered.contains("hf_secret"), "{rendered}");
    let described = credential.describe();
    assert!(!described.contains("hf_secret"), "{described}");
    assert!(described.contains("supplied directly"), "{described}");
}

#[test]
fn the_fingerprint_tells_one_credential_from_another() {
    let one = Secret::new("hf_one");
    let same = Secret::new("hf_one");
    let other = Secret::new("hf_two");
    assert_eq!(one.fingerprint(), same.fingerprint());
    assert_ne!(one.fingerprint(), other.fingerprint());
    assert!(one.fingerprint().starts_with("sha256:"));
    assert_eq!(
        one.fingerprint().len(),
        "sha256:".len() + 12,
        "a fingerprint long enough to be a token is a token"
    );
}

#[test]
fn the_token_is_reachable_only_by_asking_for_it() {
    assert_eq!(Secret::new("hf_token").reveal(), "hf_token");
}

#[test]
fn a_sighting_reports_what_is_there_without_using_it() {
    let known = KNOWN_VARIABLES.first().expect("MCF knows of a variable");
    let look_up = |variable: &str| (variable == *known).then(|| "hf_environmental".to_owned());
    let seen = sightings(&look_up, None, &nothing_readable);
    assert_eq!(seen.len(), 1);
    let sighting = seen.first().expect("one sighting");
    assert_eq!(
        sighting.origin,
        Origin::Environment {
            variable: (*known).to_owned()
        }
    );
    assert_eq!(
        sighting.fingerprint,
        Secret::new("hf_environmental").fingerprint()
    );
    let described = sighting.describe();
    assert!(!described.contains("hf_environmental"), "{described}");
    assert!(described.contains("has not used it"), "{described}");
}

#[test]
fn every_known_variable_is_looked_at() {
    for variable in KNOWN_VARIABLES {
        let look_up = |asked: &str| (asked == *variable).then(|| "hf_x".to_owned());
        let seen = sightings(&look_up, None, &nothing_readable);
        assert_eq!(seen.len(), 1, "{variable} was not looked at");
    }
}

#[test]
fn a_blank_credential_is_not_a_credential() {
    let blank = |_: &str| Some("   ".to_owned());
    assert!(sightings(&blank, None, &nothing_readable).is_empty());
    let path = PathBuf::from("/nowhere/token");
    let blank_file = |_: &Path| Some("\n".to_owned());
    assert!(sightings(&nothing_set, Some(&path), &blank_file).is_empty());
}

#[test]
fn a_token_file_is_reported_by_name() {
    let path = PathBuf::from("/home/somebody/.cache/token");
    let read = |asked: &Path| (asked == path).then(|| "hf_fromafile\n".to_owned());
    let seen = sightings(&nothing_set, Some(&path), &read);
    let sighting = seen.first().expect("one sighting");
    assert_eq!(sighting.origin, Origin::File { path: path.clone() });
    assert_eq!(
        sighting.fingerprint,
        Secret::new("hf_fromafile").fingerprint(),
        "the trailing newline is not part of the token"
    );
}

#[test]
fn nothing_anywhere_is_an_answer() {
    assert!(sightings(&nothing_set, None, &nothing_readable).is_empty());
}

#[test]
fn the_identities_say_different_things() {
    let anonymous = Identity::Anonymous.to_string();
    let offered = Identity::Offered {
        fingerprint: "sha256:abc".to_owned(),
    }
    .to_string();
    let confirmed = Identity::Confirmed {
        account: "somebody".to_owned(),
        fingerprint: "sha256:abc".to_owned(),
    }
    .to_string();

    assert!(anonymous.contains("no credential"), "{anonymous}");
    assert!(offered.contains("unconfirmed"), "{offered}");
    assert!(confirmed.contains("somebody"), "{confirmed}");
    assert_ne!(offered, confirmed);
}

#[test]
fn an_origin_names_a_place() {
    assert!(
        Origin::Environment {
            variable: "A_VARIABLE".to_owned()
        }
        .to_string()
        .contains("A_VARIABLE")
    );
    assert!(
        Origin::File {
            path: PathBuf::from("/tmp/t")
        }
        .to_string()
        .contains("/tmp/t")
    );
}

#[test]
fn a_sighting_carries_no_token() {
    let sighting = Sighting {
        origin: Origin::Supplied,
        fingerprint: Secret::new("hf_nope").fingerprint().to_owned(),
    };
    assert!(!format!("{sighting:?}").contains("hf_nope"));
}
