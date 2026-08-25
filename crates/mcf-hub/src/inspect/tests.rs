//! What a repository claims, against what is true.

use super::{Architecture, arrived_as_promised, ceiling_for, terms_are_legible};
use crate::reference::parse;
use crate::source::{Entry, Listing};
use mcf_core::failure::Category;

fn listing(licence: Option<&str>, size: u64) -> Listing {
    Listing {
        reference: parse("owner/model").expect("a reference"),
        revision: Some("main".to_owned()),
        entries: vec![Entry {
            path: "model.gguf".to_owned(),
            size,
        }],
        declared_licence: licence.map(str::to_owned),
    }
}

/// A card that agrees with the weights is an agreement, and not a verification
/// of anything else about the model.
#[test]
fn a_card_that_agrees_with_the_weights_agrees() {
    assert_eq!(
        Architecture::compare(Some("llama"), Some("llama")),
        Architecture::Agreed {
            architecture: "llama".to_owned()
        }
    );
    assert!(
        Architecture::compare(Some("llama"), Some("llama"))
            .divergence()
            .is_none()
    );
}

/// A card that disagrees with the weights is the finding, and it names both
/// sides — which is what makes it actionable rather than an accusation.
#[test]
fn a_card_that_disagrees_with_the_weights_is_the_finding() {
    let compared = Architecture::compare(Some("llama"), Some("mamba"));
    let failure = compared.divergence().expect("this is a divergence");
    assert_eq!(failure.category(), Category::HubMetadataDeceptive);

    let context: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        context.iter().any(|entry| entry == "declared=llama"),
        "{context:?}"
    );
    assert!(
        context
            .iter()
            .any(|entry| entry == "found_in_the_weights=mamba"),
        "{context:?}"
    );
}

/// A repository with no card is not a deceptive one. Plenty carry none, and A7
/// makes the absence a state rather than a defect.
#[test]
fn a_repository_with_no_card_is_not_deceptive() {
    let compared = Architecture::compare(None, Some("llama"));
    assert_eq!(
        compared,
        Architecture::OnlyTheWeights {
            found: "llama".to_owned()
        }
    );
    assert!(compared.divergence().is_none());
}

/// Weights MCF cannot read leave the question unanswered rather than answered
/// badly — a card MCF cannot check is not thereby true.
#[test]
fn weights_that_cannot_be_read_leave_the_question_open() {
    assert_eq!(
        Architecture::compare(Some("llama"), None),
        Architecture::Unknown
    );
    assert_eq!(Architecture::compare(None, None), Architecture::Unknown);
    assert!(
        Architecture::compare(Some("llama"), None)
            .divergence()
            .is_none()
    );
}

/// A declared licence is surfaced; an absent one is a state to report, because
/// a repository whose terms nobody can read is one nobody should use blind.
#[test]
fn terms_are_read_or_their_absence_is_reported() {
    assert_eq!(
        terms_are_legible(&listing(Some("apache-2.0"), 10)).expect("declared"),
        "apache-2.0"
    );
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
    let entry = Entry {
        path: "model.gguf".to_owned(),
        size: 1000,
    };
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
    let entry = Entry {
        path: "model.gguf".to_owned(),
        size: 1000,
    };
    let failure = arrived_as_promised(&entry, 1001).expect_err("over");
    assert_eq!(failure.category(), Category::HubMetadataDeceptive);
    assert!(arrived_as_promised(&entry, 1000).is_ok(), "exactly is fine");
}

/// The ceiling a fetcher reads to is the listing's own figure. A stream that
/// does not end is the cheapest attack a hostile source has, and the only
/// defence is refusing to keep reading.
#[test]
fn the_ceiling_is_what_the_listing_promised() {
    let entry = Entry {
        path: "model.gguf".to_owned(),
        size: 4_294_967_296,
    };
    assert_eq!(ceiling_for(&entry), 4_294_967_296);
}
