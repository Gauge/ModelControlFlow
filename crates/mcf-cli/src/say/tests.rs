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

#[test]
fn a_refusal_carries_what_it_was_doing_and_what_it_knows() {
    let said = refusal("nothing was acquired", &a_failure());
    assert!(said.starts_with("mcf: nothing was acquired"), "{said}");
    assert!(said.contains("resource.disk.exhausted"), "{said}");
    assert!(said.contains("needs_bytes: 1000"), "{said}");
    assert!(said.contains("available_bytes: 10"), "{said}");
}

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

#[test]
fn the_indented_form_leaves_out_what_was_already_said() {
    let said = beneath(&a_failure());
    assert!(!said.contains("mcf:"), "{said}");
    assert!(said.contains("resource.disk.exhausted"), "{said}");
    assert!(said.contains("needs_bytes: 1000"), "{said}");
}
