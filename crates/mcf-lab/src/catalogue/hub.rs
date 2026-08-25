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
use mcf_hub::source::Source as _;

use crate::hub::{Behaviour, FakeHub, Repository};
use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A reference that is not one.
pub(super) const REFERENCE_IS_NOT_ONE: Scenario = Scenario {
    id: "hub/reference-is-not-one",
    produces: Category::HubRefNotFound,
    summary: "a string that does not name a repository is refused, saying what it saw",
    run: reference_is_not_one,
};

/// The repository is readable and this caller is not.
pub(super) const NEEDS_CREDENTIALS: Scenario = Scenario {
    id: "hub/needs-credentials",
    produces: Category::HubAuthRequired,
    summary: "a private repository asked for without credentials says which is missing",
    run: needs_credentials,
};

/// The credentials are fine and the terms are not accepted.
pub(super) const GATED: Scenario = Scenario {
    id: "hub/gated",
    produces: Category::HubAccessGated,
    summary: "a gated repository is a different answer from an unauthenticated one",
    run: gated,
};

/// The account is throttled.
pub(super) const RATE_LIMITED: Scenario = Scenario {
    id: "hub/rate-limited",
    produces: Category::HubRateLimited,
    summary: "throttling carries the hint the hub gave, so waiting is a decision",
    run: rate_limited,
};

/// Asks the simulated hub for a repository that behaves in a stated way.
///
/// The scenario supplies the *observation* — a hub that answers this way — and
/// not the cause of it (D26). What is being reproduced is what MCF does with
/// the answer.
fn ask(behaviour: Behaviour, authenticated: bool) -> Outcome {
    let repository = Repository::holding("model.gguf", b"weights").behaving(behaviour);
    let hub = FakeHub::new().with("owner/model", repository);
    let hub = if authenticated {
        hub.authenticated()
    } else {
        hub
    };
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    match hub.list(&reference) {
        Err(failure) => Outcome::Produced(failure),
        Ok(listing) => Outcome::Unexpected(format!(
            "the hub answered with {} entries",
            listing.entries.len()
        )),
    }
}

fn needs_credentials(_world: &World) -> Outcome {
    ask(Behaviour::NeedsCredentials, false)
}

fn gated(_world: &World) -> Outcome {
    // Authenticated on purpose: gated is *credentials accepted, terms not*, and
    // running it unauthenticated would reproduce the other failure.
    ask(Behaviour::Gated, true)
}

fn rate_limited(_world: &World) -> Outcome {
    ask(Behaviour::RateLimited { retry_after: 30 }, true)
}

fn reference_is_not_one(_world: &World) -> Outcome {
    // A path traversal dressed as a reference: the exact input §3.7 exists for,
    // and the one that would reach the filesystem if the syntax were not where
    // it stopped.
    match mcf_hub::reference::parse("../../etc/passwd") {
        Err(failure) => Outcome::Produced(failure),
        Ok(reference) => Outcome::Unexpected(format!("a path traversal parsed as {reference}")),
    }
}
