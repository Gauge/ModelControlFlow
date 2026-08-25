//! Scenarios in which no vendored engine will run the artifact.
//!
//! D31: MCF ships a stand-in so that a model no vendored engine runs still
//! runs — marked, behaviour-class only, and never reporting a speed (B65). The
//! failure MCF claims to handle here is `engine.unavailable`, and A13 requires
//! it be reproducible.
//!
//! **What is simulated is the observation** (D26). MCF cannot conjure an
//! artifact no engine supports, and does not need to: what it observes in that
//! case is a run that fell through to the stand-in and a result that carries
//! the mark saying so. That is what this constructs.

use mcf_core::engine::{Run, StandIn};
use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The vendored engine will not run it, so the stand-in did.
pub(super) const NO_VENDORED_ENGINE: Scenario = Scenario {
    id: "engine/no-vendored-engine",
    produces: Category::EngineUnavailable,
    summary: "an artifact no vendored engine runs falls to the stand-in, and every \
              result taken there is marked",
    run: no_vendored_engine,
};

fn no_vendored_engine(_world: &World) -> Outcome {
    let stand_in: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");
    let marked = stand_in.mark(stand_in.behaviour("the tool call parsed"));

    // A5: the mark is what makes the result usable. A stand-in result with no
    // mark would be a corrupted result, so the scenario produces the mark's own
    // cause — which is the classified failure MCF is claiming to handle.
    match marked.degradation().causes().first() {
        Some(failure) => Outcome::Produced(failure.clone()),
        None => Outcome::Unexpected("a stand-in result carried no mark".to_owned()),
    }
}
