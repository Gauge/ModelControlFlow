//! One shape for a refusal, whichever command produced it.

use super::{beneath, refusal};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

fn a_failure() -> Failure {
    Failure::new(
        Category::ResourceDiskExhausted,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("a test"),
        "there is not enough room",
    )
    .with_context("needs_bytes", "1000")
    .with_context("available_bytes", "10")
}

/// What MCF was doing, what went wrong, and everything the failure carried —
/// because `Display` is one line by design and one line is not enough for
/// somebody who has to act (C1, A2).
#[test]
fn a_refusal_carries_what_it_was_doing_and_what_it_knows() {
    let said = refusal("nothing was acquired", &a_failure());
    assert!(said.starts_with("mcf: nothing was acquired"), "{said}");
    assert!(said.contains("resource.disk.exhausted"), "{said}");
    assert!(said.contains("needs_bytes: 1000"), "{said}");
    assert!(said.contains("available_bytes: 10"), "{said}");
}

/// A cause is printed rather than dropped: *the disk was full* explains *the
/// model was not acquired*, and a reader given only the second is guessing
/// (A1).
#[test]
fn a_cause_is_printed_in_order() {
    let caused = Failure::new(
        Category::ArtifactIncomplete,
        Attribution::Machine,
        Disposition::Partial,
        Subsystem::new("a test"),
        "the transfer did not finish",
    )
    .caused_by(a_failure());

    let said = refusal("nothing was acquired", &caused);
    let transfer = said.find("the transfer did not finish").expect("the outer");
    let disk = said.find("there is not enough room").expect("the cause");
    assert!(
        transfer < disk,
        "the cause came before what it caused: {said}"
    );
    assert!(said.contains("caused by:"), "{said}");
}

/// The indented form says the same thing without repeating what the caller
/// already said.
#[test]
fn the_indented_form_leaves_out_what_was_already_said() {
    let said = beneath(&a_failure());
    assert!(!said.contains("mcf:"), "{said}");
    assert!(said.contains("resource.disk.exhausted"), "{said}");
    assert!(said.contains("needs_bytes: 1000"), "{said}");
}
