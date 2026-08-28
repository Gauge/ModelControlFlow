//! A comparison in which more than one variable differed has no delta to give
//! (A8, B-085).
//!
//! A8's check is `CI` — *an intentionally confounded comparison is refused by
//! the tooling* — and the behaviour is tested where it lives, in
//! `mcf_bench::compare`'s own tests, against a comparison whose arms
//! deliberately differ in two conditions. What is checked here is the shape
//! that makes those tests hard to defeat, because the behaviour could be
//! restored to a convention by two small edits nobody would flag in review:
//!
//! 1. making `Finding::verdict` return a `Verdict` again rather than an
//!    `Option`, so that a caller who forgets to ask about isolation prints a
//!    number that means nothing;
//! 2. writing the list of conditions out a second time inside the isolation
//!    check, so that the floor grows and the check silently does not.
//!
//! Both are refusals of a plausible simplification, which is exactly the kind
//! B-041 exists to keep from happening quietly.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// A confounded comparison's delta is `None`, in the type.
///
/// Not a flag beside the number and not a warning in the rendering: there is
/// no number. A6's habit — a value that cannot be separated from what
/// qualifies it — applied to the one case where the qualification is that the
/// value should not exist.
#[test]
fn the_delta_is_optional_in_the_type() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("pub const fn verdict(&self) -> Option<&Verdict>"),
        "a confounded comparison must have no delta to hand out, which means the accessor is an \
         `Option` (A8, B-085)"
    );
    assert!(
        source.contains("isolation.is_confounded() && self.declared.is_none()"),
        "the refusal must be reached: a confound nobody declared is an error, and a confound the \
         operator declared is science (A8)"
    );
    assert!(
        source.contains("pub fn declaring("),
        "the operator must be able to declare a confound, or A8's science half is unreachable"
    );
}

/// The isolation check reads the floor rather than a copy of it.
///
/// §3.3 says the floor never shrinks, and does not say it never grows. A
/// second list of condition names would be the thing that quietly stopped
/// growing with it, and the comparison would go on reporting *isolated* about
/// a condition it had stopped looking at.
#[test]
fn isolation_reads_the_floor_itself() {
    let source = code_only(&core_source("measurement/isolation.rs"));
    assert!(
        source.contains(".floor().entries()"),
        "isolation must enumerate the floor rather than name its fields (§3.3, B16)"
    );
    for written_out in ["thermal_state", "quantization", "context_length"] {
        assert!(
            !source.contains(written_out),
            "`{written_out}` is written out in the isolation check, which is a second copy of a \
             list the floor already holds (§3.3)"
        );
    }
}

/// Two unknowns are not a match.
///
/// A7 forbids filling an unknown with a plausible value, and *they were
/// probably the same* is that. The check is that the comparison of two
/// attested values has a branch for it rather than falling through to
/// equality.
#[test]
fn an_unread_condition_is_not_treated_as_a_match() {
    let source = code_only(&core_source("measurement/isolation.rs"));
    assert!(
        source.contains("Undetermined {"),
        "a condition MCF could not read must have its own answer (A7)"
    );
    assert!(
        !source.contains("mine == theirs && "),
        "an unknown must not be compared as if it were a value"
    );
}

/// The source of one of `mcf-bench`'s modules.
fn bench_source(file: &str) -> String {
    read(&format!("crates/mcf-bench/src/{file}"))
}

/// The source of one of `mcf-core`'s modules.
fn core_source(file: &str) -> String {
    read(&format!("crates/mcf-core/src/{file}"))
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// The source with its documentation comments removed, so that a sentence
/// quoting a condition's name is not read as the code naming it.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}
