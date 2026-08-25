//! The laboratory: deterministic scenarios that produce real failures.
//!
//! §3.17 and D5 put MCF's confidence here rather than in ambient observation,
//! which is what lets B4 refuse telemetry without costing §II anything. A13 is
//! the obligation: *every failure MCF claims to handle has a simulation that
//! produces it*, and D26 binds that to what MCF's own code constructs — a
//! category the code can produce has a scenario, and a category nothing
//! produces yet is a classification waiting for the code that will use it.
//!
//! **What a scenario is.** A named, in-tree function that constructs the
//! conditions for one failure and returns the failure MCF actually produced.
//! Not a mock: the scenario builds a real journal in a real directory and
//! writes real damage into it, and the code under test is the code that ships.
//! What is *simulated* is the world MCF observes — a torn file, an unwritable
//! path, a clock that has stepped — never the cause of it (D26).
//!
//! **What makes it a laboratory rather than a test suite.**
//!
//! * **Determinism is a property of the lab, not of the world** (B27). A
//!   scenario run a hundred times produces the same failure a hundred times,
//!   and [`repeat`] is what says so.
//! * **The clock is supplied, not reached for** (D26). [`World`] holds a
//!   [`SimulatedClock`], so a scenario about a deadline states the passage of
//!   time rather than waiting for it.
//! * **No performance number originates here** (A11). The lab's clock is a
//!   different *type* from the monotonic one (B37), so a simulated duration
//!   cannot become a throughput figure — a compiler check rather than a rule.
//! * **No lab API, no discovery, no configuration language** (B32). The
//!   catalogue is a constant in this crate. Adding a scenario is editing it.
//!
//! **What it declines**, because §6.16 requires the boundary be stated: it does
//! not model what causes a failure, a vendor stack's internals, or the world's
//! timing. A scenario saying *the transfer stalled for thirty seconds* is a
//! statement about what MCF then did, never about how often that happens. D26
//! holds the full list, and A12 governs the consequence: where the simulator
//! and reality disagree, reality is right.

mod catalogue;
pub mod hub;
mod scenario;
mod world;

pub use catalogue::{CATALOGUE, find};
pub use scenario::{Outcome, Repetition, Scenario};
pub use world::{SCRATCH_PLACEHOLDER, World};

use mcf_core::time::SimulatedClock;

/// Runs a scenario once.
///
/// Never fails to the caller: a scenario that did not produce what it claims is
/// an [`Outcome::Unexpected`] carrying what happened instead, because the
/// laboratory reporting *its own* failure as an error would be the manager
/// dying with the managed (A3, one level up).
#[must_use]
pub fn run(scenario: &Scenario) -> Outcome {
    let world = World::for_scenario(scenario.id);
    let outcome = (scenario.run)(&world);
    // Where the run happened is environment; what happened is the failure.
    // Eliding the first is what makes two runs comparable for the second.
    outcome.elide(&world)
}

/// Runs a scenario repeatedly and reports whether every run agreed.
///
/// §3.17's requirement in one function: *a failure found once reproduces
/// exactly, forever*. Divergence is the finding — about the scenario, or about
/// something in MCF that is not as deterministic as it claims — and it is
/// reported with both outcomes rather than as a count.
#[must_use]
pub fn repeat(scenario: &Scenario, times: usize) -> Repetition {
    let mut first: Option<Outcome> = None;
    let mut divergence: Option<(Outcome, usize)> = None;
    for iteration in 0..times {
        let outcome = run(scenario);
        match &first {
            None => first = Some(outcome),
            Some(expected) if expected == &outcome => {}
            Some(_) => {
                if divergence.is_none() {
                    divergence = Some((outcome, iteration));
                }
            }
        }
    }
    Repetition {
        runs: times,
        first,
        divergence,
    }
}

/// A clock a scenario can move.
///
/// Exposed so that a caller outside this crate can build a [`World`] for a
/// scenario of its own — the laboratory's own tests do exactly that, and B27
/// holds the lab to the same standards as everything it tests.
#[must_use]
pub fn clock() -> SimulatedClock {
    SimulatedClock::new()
}
