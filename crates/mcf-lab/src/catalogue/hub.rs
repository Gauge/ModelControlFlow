//! Scenarios about the model source.
//!
//! The hub is unbounded, heterogeneous and changes without notice (§6.3), and
//! §III commits MCF to a defined, actionable outcome for every reference to it.
//! These are the failures that can be produced without a hub: a reference is a
//! string, and what MCF does with a string it cannot use is decidable here and
//! now.
//!
//! What is *not* here yet is the rest of B7's check — the hostile-hub fixtures
//! that need a simulated hub to serve them (B-028). D26 draws that line: this
//! scenario simulates what MCF observes, which is a reference that names
//! nothing, rather than a hub that behaves badly.

use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A reference that is not one.
pub(super) const REFERENCE_IS_NOT_ONE: Scenario = Scenario {
    id: "hub/reference-is-not-one",
    produces: Category::HubRefNotFound,
    summary: "a string that does not name a repository is refused, saying what it saw",
    run: reference_is_not_one,
};

fn reference_is_not_one(_world: &World) -> Outcome {
    // A path traversal dressed as a reference: the exact input §3.7 exists for,
    // and the one that would reach the filesystem if the syntax were not where
    // it stopped.
    match mcf_hub::reference::parse("../../etc/passwd") {
        Err(failure) => Outcome::Produced(failure),
        Ok(reference) => Outcome::Unexpected(format!("a path traversal parsed as {reference}")),
    }
}
