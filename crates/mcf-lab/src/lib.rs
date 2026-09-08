mod catalogue;
pub mod fixture;
pub mod hub;
mod scenario;
pub mod serving;
mod world;

pub use catalogue::{CATALOGUE, find};
pub use scenario::{Outcome, Repetition, Scenario};
pub use world::{SCRATCH_PLACEHOLDER, World};

use mcf_core::time::SimulatedClock;

#[must_use]
pub fn run(scenario: &Scenario) -> Outcome {
    let world = World::for_scenario(scenario.id);
    let outcome = (scenario.run)(&world);
    outcome.elide(&world)
}

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

#[must_use]
pub fn clock() -> SimulatedClock {
    SimulatedClock::new()
}
