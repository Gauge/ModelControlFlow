#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_bench::planned::{Proposal, Work};
use mcf_core::measurement::{Basis, Estimate};
use mcf_core::time::{Duration, Monotonic};

fn twenty() -> Work {
    Work {
        trials: 20,
        arms: 2,
        tokens: 128,
    }
}

fn each(low: u64, high: u64) -> Estimate<Duration<Monotonic>> {
    Estimate::band(
        Duration::from_nanos(low),
        Duration::from_nanos(high),
        Basis::LocalHistory,
    )
}

#[test]
fn what_was_excluded_is_on_the_page() {
    let held = Proposal::within(twenty(), &each(500, 1_000), Duration::from_nanos(12_000), 2);
    let Proposal::Fewer { running, excluded } = &held else {
        panic!("a budget covering part of the work proposes part of it: {held:?}");
    };
    assert_eq!(running.trials, 6);
    assert_eq!(excluded.trials, 14);
    let shown = held.to_string();
    assert!(
        shown.contains("6 paired trial(s)") && shown.contains("14 paired trial(s)"),
        "both halves must be in the sentence, not only in the type: {shown}"
    );
    assert!(
        shown.contains("EXCLUDED"),
        "and the excluded half must be findable by a reader skimming: {shown}"
    );
}

#[test]
fn too_small_a_budget_is_a_refusal() {
    let held = Proposal::within(twenty(), &each(500, 1_000), Duration::from_nanos(1), 2);
    assert!(
        matches!(held, Proposal::NotEnough { .. }),
        "a budget that cannot buy a comparison must not buy a smaller thing: {held:?}"
    );
    assert_eq!(
        held.running(),
        None,
        "and there must be nothing for a caller to run"
    );
}

#[test]
fn planning_is_conservative() {
    let held = Proposal::within(twenty(), &each(1, 1_000), Duration::from_nanos(12_000), 2);
    assert_eq!(
        held.running().map(|work| work.trials),
        Some(6),
        "six at the slow edge, six thousand at the fast one: {held:?}"
    );
}

#[test]
fn an_unplannable_budget_refuses_in_the_command() {
    let source =
        std::fs::read_to_string(mcf_checks::workspace::root().join("crates/mcf-cli/src/bench.rs"))
            .expect("bench.rs is readable");
    assert!(
        source.contains("a time budget needs a measured rate to plan against"),
        "a budget planned against a rate MCF does not have would be inventing the rate (A7)"
    );
    assert!(
        source.contains("if let Some(why) = planned.refused"),
        "and the refusal must actually stop the run, rather than being a string nobody reads"
    );
}
