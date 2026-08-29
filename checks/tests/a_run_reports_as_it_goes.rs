//! A run says what it has while it still has it (B-227, A4, §3.1).
//!
//! **The failure.** A benchmark that takes minutes and says nothing until it
//! finishes is one an operator cannot tell from a hung one, and one whose
//! forty pairs are first heard of when it stops. A4 already keeps what an
//! interrupted run produced — the pairs are kept and what stopped it travels
//! with them — but *keeping* is not *reporting*, and B-227 asks for both.
//!
//! **Three properties, each of which could quietly be lost:**
//!
//! 1. progress goes to standard error, because the result is what goes to
//!    standard output and a pipeline reading a verdict must not have to
//!    filter progress out of it;
//! 2. an interim line says how many pairs it rests on, so that a reader who
//!    scrolls back cannot mistake it for the answer;
//! 3. reporting cannot change what the run does — no branch, no early exit, no
//!    pass condition wearing a progress line (A18).

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

fn bench() -> String {
    std::fs::read_to_string(mcf_checks::workspace::root().join("crates/mcf-cli/src/bench.rs"))
        .expect("bench.rs is readable")
}

/// Progress is on standard error, and the verdict is not.
#[test]
fn progress_does_not_reach_standard_output() {
    let source = bench();
    assert!(
        source.contains("fn so_far(") && source.contains("eprintln!"),
        "the run must report as it goes, on standard error (B-227)"
    );
    // `eprintln!` ends in `println!`, so a substring check would forbid the
    // very thing it is asking for. Counting is what tells them apart.
    assert_eq!(
        source.matches("println!").count(),
        source.matches("eprintln!").count(),
        "and never on standard output: the result is what goes there, and a pipeline reading a \
         verdict must not have to filter progress out of it"
    );
}

/// An interim line carries the count it rests on.
#[test]
fn an_interim_line_says_how_far_it_got() {
    let source = bench();
    let (_, said) = source
        .split_once("fn so_far(")
        .expect("`so_far` is what reports as it goes");
    let (said, _) = said.split_once("\n}").expect("and it ends somewhere");
    assert!(
        said.contains("after {pairs} pair(s), so far"),
        "an interim line must say how many pairs it rests on and that it is interim — a reader \
         who scrolls back must not be able to read it as the answer (§3.1)"
    );
    assert!(
        said.contains("if pairs < 2"),
        "and must say nothing below two pairs, which is the floor at which there is a \
         comparison to report at all"
    );
}

/// Reporting is not a decision.
#[test]
fn reporting_cannot_change_what_the_run_does() {
    let source = bench();
    let (_, said) = source
        .split_once("fn so_far(")
        .expect("`so_far` is what reports as it goes");
    let (said, _) = said.split_once("\n}").expect("and it ends somewhere");
    for forbidden in ["break", "std::process::exit", "panic!", "return Err"] {
        assert!(
            !said.contains(forbidden),
            "`so_far` contains `{forbidden}`: A18 gives a benchmark no pass condition, and a \
             progress line that could stop a run would be one"
        );
    }
    assert!(
        !said.contains("&mut "),
        "and it takes the comparison by reference: a reporter that could change what it \
         reports on is not a reporter"
    );
}
