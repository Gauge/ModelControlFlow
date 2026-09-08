use super::{Family, describe, family, recognize};
use mcf_core::provenance::Licence;

#[test]
fn a_known_identifier_is_kept_as_written() {
    assert_eq!(recognize("apache-2.0"), Some(Licence::spdx("apache-2.0")));
    assert_eq!(recognize("MIT"), Some(Licence::spdx("mit")));
    assert_eq!(recognize("  gpl-3.0  "), Some(Licence::spdx("gpl-3.0")));
}

#[test]
fn terms_that_cannot_be_matched_are_not_the_same_as_none() {
    assert_eq!(recognize("some-bespoke-thing-2024"), Some(Licence::Stated));
    assert_eq!(recognize("other"), Some(Licence::Stated));
    assert_eq!(recognize("proprietary"), Some(Licence::Stated));
    assert_eq!(recognize(""), None, "nothing declared is nothing declared");
    assert_eq!(recognize("   "), None);
}

#[test]
fn the_family_is_what_the_identifier_says_about_itself() {
    for (identifier, expected) in [
        ("mit", Family::Permissive),
        ("apache-2.0", Family::Permissive),
        ("gpl-3.0", Family::Copyleft),
        ("agpl-3.0", Family::Copyleft),
        ("cc-by-nc-4.0", Family::NonCommercial),
        ("llama3.1", Family::Bespoke),
        ("gemma", Family::Bespoke),
        ("creativeml-openrail-m", Family::Bespoke),
    ] {
        let licence = recognize(identifier).expect("declared");
        assert_eq!(family(&licence), Some(expected), "{identifier}");
    }
}

#[test]
fn unmatched_terms_have_no_family() {
    assert_eq!(family(&Licence::Stated), None);
}

#[test]
fn a_non_commercial_licence_is_not_quietly_permissive() {
    let licence = recognize("cc-by-nc-sa-4.0").expect("declared");
    assert_eq!(family(&licence), Some(Family::NonCommercial));
    assert_ne!(family(&licence), Some(Family::Permissive));
}

#[test]
fn a_bespoke_licence_is_named_and_not_summarized() {
    let licence = recognize("llama3.1").expect("declared");
    assert_eq!(family(&licence), Some(Family::Bespoke));
    let described = describe(Some(&licence));
    assert!(described.contains("llama3.1"), "{described}");
    assert!(described.contains("read the terms"), "{described}");
}

#[test]
fn every_state_renders_as_itself() {
    let absent = describe(None);
    let unmatched = describe(Some(&Licence::Stated));
    let identified = describe(Some(&Licence::spdx("mit")));

    assert!(absent.contains("unknown"), "{absent}");
    assert!(absent.contains("declared none"), "{absent}");
    assert!(unmatched.contains("could not identify"), "{unmatched}");
    assert!(
        identified.contains("mit") && identified.contains("permissive"),
        "{identified}"
    );
    assert_ne!(absent, unmatched);
    assert_ne!(unmatched, identified);
}

#[test]
fn every_row_in_the_table_is_recognized_and_has_a_family() {
    for (identifier, expected) in super::KNOWN {
        let licence = recognize(identifier)
            .unwrap_or_else(|| panic!("{identifier} is in the table and was not recognized"));
        assert!(
            licence.is_identified(),
            "{identifier} matched as unidentified"
        );
        assert_eq!(family(&licence), Some(*expected), "{identifier}");
    }
}

#[test]
fn the_table_names_each_identifier_once() {
    let mut seen: Vec<&str> = super::KNOWN.iter().map(|(name, _)| *name).collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), count, "an identifier is in the table twice");
}
