//! What a repository claims, against what is true.

use mcf_core::provenance::Licence;

use super::{architecture, arrived_as_promised, ceiling_for, deception, terms_are_legible};
use crate::reference::parse;
use crate::source::{Entry, Listing};
use mcf_core::capability::State;
use mcf_core::failure::Category;

fn listing(licence: Option<&str>, size: u64) -> Listing {
    Listing {
        reference: parse("owner/model").expect("a reference"),
        revision: Some("main".to_owned()),
        entries: vec![Entry::new("model.gguf", size)],
        declared_licence: licence.map(str::to_owned),
        lineage: None,
    }
}

/// A card that agrees with the weights is a verified architecture with a
/// declaration behind it — and still not a verification of anything else about
/// the model (§3.18).
#[test]
fn a_card_that_agrees_with_the_weights_agrees() {
    let compared = architecture(Some("llama"), Some("llama"));
    assert_eq!(compared.state(), State::Verified);
    assert_eq!(compared.observation().map(String::as_str), Some("llama"));
    assert_eq!(compared.declaration().map(String::as_str), Some("llama"));
    assert!(deception(&compared).is_none());
    assert!(
        compared.to_string().contains("the declaration agrees"),
        "{compared}"
    );
}

/// A card that disagrees with the weights is the finding, and the failure names
/// both sides so that a reader knows which to distrust (B-058, A21).
#[test]
fn a_card_that_disagrees_with_the_weights_is_the_finding() {
    let compared = architecture(Some("llama"), Some("mamba"));
    assert_eq!(compared.state(), State::Diverged);
    let failure = deception(&compared).expect("a divergence is a finding");
    assert_eq!(failure.category(), Category::HubMetadataDeceptive);
    assert_eq!(failure.context_value("declared"), Some("llama"));
    assert_eq!(failure.context_value("found_in_the_weights"), Some("mamba"));
}

/// A repository with no card is not deceptive: the weights say what they say,
/// and nothing declared them (A7).
#[test]
fn a_repository_with_no_card_is_not_deceptive() {
    let compared = architecture(None, Some("llama"));
    assert_eq!(compared.state(), State::Verified);
    assert_eq!(compared.declaration(), None);
    assert!(deception(&compared).is_none());
    assert!(
        compared.to_string().contains("nothing declared it"),
        "{compared}"
    );
}

/// Weights MCF has not read leave a *declared* architecture rather than an
/// unknown one: what the card said is a thing MCF knows and must not throw away
/// (B-050, A1).
#[test]
fn weights_that_cannot_be_read_leave_the_question_open() {
    let compared = architecture(Some("llama"), None);
    assert_eq!(compared.state(), State::Declared);
    assert_eq!(compared.declaration().map(String::as_str), Some("llama"));
    assert!(
        !compared.is_established(),
        "a declaration was treated as something to act on"
    );
    assert!(deception(&compared).is_none());

    let nothing = architecture(None, None);
    assert_eq!(nothing.state(), State::Unknown);
}

/// A declared licence is surfaced; an absent one is a state to report, because
/// a repository whose terms nobody can read is one nobody should use blind.
#[test]
fn terms_are_read_or_their_absence_is_reported() {
    assert_eq!(
        terms_are_legible(&listing(Some("apache-2.0"), 10)).expect("declared"),
        Licence::spdx("apache-2.0")
    );
    assert_eq!(
        terms_are_legible(&listing(Some("a licence of their own"), 10)).expect("declared"),
        Licence::Stated,
        "terms MCF cannot name are still terms, and saying so is not a failure"
    );
    terms_are_legible(&listing(Some("   "), 10)).expect_err("a blank declaration declares nothing");
    let failure = terms_are_legible(&listing(None, 10)).expect_err("nothing declared");
    assert_eq!(failure.category(), Category::HubMetadataAbsent);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("owner/model")),
        "the refusal does not name the repository"
    );
}

/// Fewer bytes than promised is a truncation — partial, and B-021's to resume.
#[test]
fn fewer_bytes_than_promised_is_a_truncation() {
    let entry = Entry::new("model.gguf", 1000);
    let failure = arrived_as_promised(&entry, 400).expect_err("short");
    assert_eq!(failure.category(), Category::ArtifactIncomplete);
    let context: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        context.iter().any(|entry| entry == "promised=1000"),
        "{context:?}"
    );
    assert!(
        context.iter().any(|entry| entry == "arrived=400"),
        "{context:?}"
    );
}

/// More bytes than promised is the repository lying about a number MCF plans
/// with, which is a different failure from a short transfer and needs a
/// different response.
#[test]
fn more_bytes_than_promised_is_a_lie_rather_than_a_windfall() {
    let entry = Entry::new("model.gguf", 1000);
    let failure = arrived_as_promised(&entry, 1001).expect_err("over");
    assert_eq!(failure.category(), Category::HubMetadataDeceptive);
    assert!(arrived_as_promised(&entry, 1000).is_ok(), "exactly is fine");
}

/// The ceiling a fetcher reads to is the listing's own figure. A stream that
/// does not end is the cheapest attack a hostile source has, and the only
/// defence is refusing to keep reading.
#[test]
fn the_ceiling_is_what_the_listing_promised() {
    let entry = Entry::new("model.gguf", 4_294_967_296);
    assert_eq!(ceiling_for(&entry), 4_294_967_296);
}
