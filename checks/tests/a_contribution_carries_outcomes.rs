#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

fn source() -> String {
    std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-core/src/contribution.rs"),
    )
    .expect("contribution.rs is readable")
    .lines()
    .filter(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
    })
    .collect::<Vec<&str>>()
    .join("\n")
}

#[test]
fn no_field_can_hold_a_task_or_an_output() {
    let held = source();
    let held = held
        .split_once("pub const TERMS")
        .map_or(held.clone(), |(before, after)| {
            let (_, rest) = after.split_once("\";").unwrap_or_default();
            format!("{before}{rest}")
        });
    for forbidden in [
        "prompt",
        "completion",
        "output",
        "text:",
        "task",
        "fixture",
        "document",
        "PathBuf",
        "bytes",
    ] {
        assert!(
            !held.contains(forbidden),
            "`{forbidden}` in the contribution format is somewhere for task content to be put, \
             and a benchmark task that travels ends up in somebody's training data (B-171, \
             §6.30)"
        );
    }
}

#[test]
fn nothing_here_offers_to_unsend_anything() {
    let held = source();
    for promise in [
        "fn retract",
        "fn unsend",
        "fn withdraw",
        "fn recall",
        "fn delete",
        "fn revoke",
    ] {
        assert!(
            !held.contains(promise),
            "`{promise}` would promise something MCF cannot do: publication cannot be undone \
             (D21, B63, §3.20)"
        );
    }
    assert!(
        held.contains("cannot be undone"),
        "and the terms must say so before anything is sent, rather than leaving a reader to \
         discover it afterwards"
    );
}

#[test]
fn a_custom_workload_is_refused_by_construction() {
    let held = source();
    assert!(
        held.contains("pub workload: Workload") && held.contains("workload: Workload,"),
        "B-203: the marking travels on the row from production — one applied at export is one \
         that can be forgotten at export (B42, A25)"
    );
    assert_eq!(
        held.matches("NotContributable::WorkloadIsCustom").count(),
        2,
        "both routes in — a comparison and an absolute — must refuse it, and a complete \
         condition set must not rescue it"
    );
}

#[test]
fn a_bare_number_cannot_be_constructed() {
    let held = source();
    assert!(
        held.contains("pub fn new(") && held.contains("Result<Self, NotContributable>"),
        "B54: an absolute must be fallible to construct, so that a bare duration cannot exist \
         as a contributable row at all"
    );
    assert!(
        held.contains("if known < of"),
        "and the test is completeness of the condition floor, which is what makes somebody \
         else's number scalable (§3.4)"
    );
    assert!(
        !held.contains("impl Default for Absolute"),
        "a default would be a bare number with a straight face"
    );
}

#[test]
fn an_import_arrives_as_a_claim() {
    let held = source();
    assert!(
        held.contains("claimed: crate::origin::FromCorpus<T>"),
        "an import must hold a `FromCorpus`, which cannot back a recommendation (B-167) and \
         cannot render as a local measurement (B43)"
    );
    for promotion in [
        "fn verify(self)",
        "fn promote",
        "fn into_local",
        "impl From<Imported",
    ] {
        assert!(
            !held.contains(promotion),
            "`{promotion}` would convert a claim into a measurement; verification *replaces* \
             it and records what both said, exactly as A20 replaces an estimate"
        );
    }
    assert!(
        held.contains("theirs:") && held.contains("ours:"),
        "B-172: a reproduction keeps both figures — only the local one throws away the \
         comparison, only the difference throws away what was compared (A1)"
    );
}

#[test]
fn a_configuration_that_will_not_run_is_a_complete_answer() {
    let held = source();
    assert!(
        held.contains("WillNotFitHere"),
        "*this needs 48 GiB and you have 24* answers the question that was asked"
    );
    assert!(
        held.contains("NotAttempted"),
        "and *nothing was tried* must be its own state: unattempted is not agreement (A21, A7)"
    );
}

#[test]
fn a_comparison_needs_less_than_an_absolute() {
    let held = source();
    let (_, comparison) = held
        .split_once("pub struct Comparison {")
        .expect("a comparison is a row shape");
    let (comparison, _) = comparison.split_once('}').expect("and it ends");
    assert!(
        comparison.contains("pub pairs: usize") && comparison.contains("pub effect:"),
        "§3.27: what travels is both arms, the pairing and the effect size (B-251)"
    );
    assert!(
        !comparison.contains("nanoseconds"),
        "a comparison carries a ratio, not a duration: the duration is the part that does not \
         survive travel"
    );
}
