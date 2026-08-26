//! What `pull` says, and what it refuses to do.
//!
//! The acquisition path itself is exercised against a real server in
//! `mcf_hub::client`'s tests and end to end in `tests/whole_system.rs`; what is
//! here is the surface's own decisions — what it offers when nobody has chosen,
//! and what it will not attempt.

use mcf_hub::reference;
use mcf_hub::source::{Entry, Listing};

use super::{DEFAULT_HUB, PLANNING_CONTEXT, licence_of, offer, run};

fn a_listing() -> Listing {
    Listing {
        reference: reference::parse("owner/model").expect("a reference"),
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        entries: vec![
            Entry::new("Q4_K_M.gguf", 396_705_472).declaring("a".repeat(64)),
            Entry::new("Q8_0.gguf", 700_000_000),
        ],
        declared_licence: Some("apache-2.0".to_owned()),
    }
}

/// Nobody's quantization is chosen for them: what a repository publishes is put
/// in front of the operator with the sizes, which is the question they were
/// actually asking (§3.13, A7).
#[test]
fn without_a_file_it_offers_the_choice_and_acquires_nothing() {
    let offered = offer(&a_listing(), None);
    assert!(offered.contains("Q4_K_M.gguf"), "{offered}");
    assert!(offered.contains("396705472"), "{offered}");
    assert!(offered.contains("50968a44"), "{offered}");
    assert!(offered.contains("nothing was acquired"), "{offered}");
    assert!(offered.contains("mcf pull owner/model:<file>"), "{offered}");
}

/// A file the hub declares no digest for is named as such, because an artifact
/// nobody can check is a condition of every measurement taken on it (A21).
#[test]
fn a_file_with_no_declared_digest_is_pointed_out() {
    let offered = offer(&a_listing(), None);
    let undeclared = offered
        .lines()
        .find(|line| line.contains("Q8_0.gguf"))
        .unwrap_or_default();
    assert!(undeclared.contains("no digest"), "{undeclared}");
    let declared = offered
        .lines()
        .find(|line| line.contains("Q4_K_M.gguf"))
        .unwrap_or_default();
    assert!(!declared.contains("no digest"), "{declared}");
}

/// The terms are in front of the operator before they choose, which is what
/// §III asks and B-023 built.
#[test]
fn the_terms_are_offered_with_the_files() {
    let offered = offer(&a_listing(), None);
    assert!(offered.contains("apache-2.0"), "{offered}");
    assert!(offered.contains("permissive"), "{offered}");
    assert_eq!(
        licence_of(&a_listing()),
        Some(mcf_core::provenance::Licence::spdx("apache-2.0"))
    );
}

/// A string that is not a reference is refused by name rather than attempted.
#[test]
fn a_reference_that_is_not_one_is_refused() {
    let response = run("not a reference at all", None);
    assert!(!response.served);
    assert!(
        response.text.contains("not a reference"),
        "{}",
        response.text
    );
}

/// The default hub is the real one, and reaching it needs TLS MCF has not
/// vendored — said in as many words rather than attempted and failed obscurely
/// (B-322, F9).
#[test]
fn the_encrypted_hub_is_refused_in_as_many_words() {
    assert!(DEFAULT_HUB.starts_with("https://"), "{DEFAULT_HUB}");
    let response = run("owner/model", None);
    assert!(!response.served);
    assert!(
        response.text.contains("no TLS stack is vendored"),
        "{}",
        response.text
    );
    assert!(response.text.contains("B-322"), "{}", response.text);
}

/// And a hub that is not a URL is refused before anything is opened.
#[test]
fn a_hub_that_is_not_a_url_is_refused() {
    let response = run("owner/model", Some("not-a-hub"));
    assert!(!response.served);
    assert!(response.text.contains("not a hub"), "{}", response.text);
}

/// A plan is offered at a stated context, because *this fits* means nothing
/// without the length it fits at (A6, §3.4).
#[test]
fn a_plan_is_offered_at_a_stated_context() {
    let plan = vec!["  Q4_K_M.gguf — fits: needs 1 of 2 usable, 1 left".to_owned()];
    let offered = offer(&a_listing(), Some(&plan));
    assert!(
        offered.contains(&format!("at {PLANNING_CONTEXT} tokens of context")),
        "{offered}"
    );
    assert!(offered.contains("fits: needs"), "{offered}");
}

/// And where no plan can be made, the surface says so rather than showing an
/// empty one — a missing plan and a plan that found nothing are different
/// answers (A7).
#[test]
fn no_plan_is_said_rather_than_shown_empty() {
    let offered = offer(&a_listing(), None);
    assert!(
        offered.contains("cannot say which of these would run here"),
        "{offered}"
    );
    assert!(offered.contains("configuration"), "{offered}");
}
