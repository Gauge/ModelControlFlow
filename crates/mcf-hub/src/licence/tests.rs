//! What MCF says about terms, and what it declines to say.

use super::{Family, describe, family, recognize};
use mcf_core::provenance::Licence;

/// An identifier MCF knows is kept as the repository wrote it, normalized only
/// in case — an identifier is a name, and a record that rewrote it would record
/// something the repository did not say.
#[test]
fn a_known_identifier_is_kept_as_written() {
    assert_eq!(recognize("apache-2.0"), Some(Licence::spdx("apache-2.0")));
    assert_eq!(recognize("MIT"), Some(Licence::spdx("mit")));
    assert_eq!(recognize("  gpl-3.0  "), Some(Licence::spdx("gpl-3.0")));
}

/// Terms MCF cannot match are *present and unmatched*, which is not the same as
/// absent. Collapsing the two would let MCF proceed past terms nobody read.
#[test]
fn terms_that_cannot_be_matched_are_not_the_same_as_none() {
    assert_eq!(recognize("some-bespoke-thing-2024"), Some(Licence::Stated));
    assert_eq!(recognize("other"), Some(Licence::Stated));
    assert_eq!(recognize("proprietary"), Some(Licence::Stated));
    assert_eq!(recognize(""), None, "nothing declared is nothing declared");
    assert_eq!(recognize("   "), None);
}

/// The families are what the identifiers say about themselves.
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

/// Unmatched terms have no family MCF can name, and inventing one would be the
/// summary this module exists not to write.
#[test]
fn unmatched_terms_have_no_family() {
    assert_eq!(family(&Licence::Stated), None);
}

/// A non-commercial licence is recognized as such rather than as permissive,
/// which is the distinction that matters to somebody about to use a model at
/// work — and MCF states the family rather than deciding for them.
#[test]
fn a_non_commercial_licence_is_not_quietly_permissive() {
    let licence = recognize("cc-by-nc-sa-4.0").expect("declared");
    assert_eq!(family(&licence), Some(Family::NonCommercial));
    assert_ne!(family(&licence), Some(Family::Permissive));
}

/// A bespoke licence is recognized by name and not summarized. The family says
/// *read the terms*, which is the only honest thing MCF can say about a licence
/// written for one model.
#[test]
fn a_bespoke_licence_is_named_and_not_summarized() {
    let licence = recognize("llama3.1").expect("declared");
    assert_eq!(family(&licence), Some(Family::Bespoke));
    let described = describe(Some(&licence));
    assert!(described.contains("llama3.1"), "{described}");
    assert!(described.contains("read the terms"), "{described}");
}

/// The three states each render differently, because a surface that showed
/// *unknown* and *unmatched* the same way would be answering a question nobody
/// asked.
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

/// Every row in the table round-trips: it is recognized, and it has the family
/// the table gives it. A row that did not would be a claim nothing checks.
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

/// No identifier appears twice, which would make the family it maps to depend
/// on the order of a list.
#[test]
fn the_table_names_each_identifier_once() {
    let mut seen: Vec<&str> = super::KNOWN.iter().map(|(name, _)| *name).collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), count, "an identifier is in the table twice");
}
