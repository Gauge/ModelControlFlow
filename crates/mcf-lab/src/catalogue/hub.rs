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

/// The card says one thing and the weights say another.
pub(super) const DECEPTIVE_METADATA: Scenario = Scenario {
    id: "hub/deceptive-metadata",
    produces: Category::HubMetadataDeceptive,
    summary: "a repository declaring an architecture its weights are not is caught by reading them",
    run: deceptive_metadata,
};

/// The repository declares no terms at all.
pub(super) const NO_LICENCE: Scenario = Scenario {
    id: "hub/no-licence",
    produces: Category::HubMetadataAbsent,
    summary: "a repository whose terms nobody can read is a state to report, not one to fill in",
    run: no_licence,
};

/// The transfer ends early.
pub(super) const TRUNCATED_TRANSFER: Scenario = Scenario {
    id: "hub/truncated-transfer",
    produces: Category::ArtifactIncomplete,
    summary: "fewer bytes than the listing promised is a partial artifact, on the disk and in the record",
    run: truncated_transfer,
};

/// The source cannot continue a transfer from where it stopped.
pub(super) const CANNOT_RESUME: Scenario = Scenario {
    id: "hub/cannot-resume",
    produces: Category::HubUnreachable,
    summary: "a hub without ranges says so, and a fetch starts again rather than pretending",
    run: cannot_resume,
};

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

/// A GGUF that declares an architecture, with no tensors — enough for a reader
/// to say what it is, which is all this scenario needs.
fn model_declaring(architecture: &str) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    push_string(&mut bytes, "general.architecture");
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    push_string(&mut bytes, architecture);
    bytes
}

fn deceptive_metadata(world: &World) -> Outcome {
    // The repository's card says llama; the weights it serves say mamba. The
    // whole path runs: list, fetch, read the weights, compare.
    let hub = FakeHub::new().with(
        "owner/mislabelled",
        Repository::holding("model.gguf", &model_declaring("mamba")),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/mislabelled") else {
        return Outcome::Unexpected("owner/mislabelled is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };

    let into = world.path("model.gguf");
    if let Err(failure) = hub.fetch(&reference, entry, &into) {
        return Outcome::Unexpected(format!("the fetch failed: {failure}"));
    }
    let file = match mcf_standin::gguf::read(&into) {
        Ok(file) => file,
        Err(failure) => {
            return Outcome::Unexpected(format!("the weights would not read: {failure}"));
        }
    };

    // The card is what the repository says; the architecture is what the
    // weights say. A21's divergence, and the most useful thing MCF can report
    // about a repository like this.
    let compared = mcf_hub::inspect::Architecture::compare(Some("llama"), file.architecture());
    match compared.divergence() {
        Some(failure) => Outcome::Produced(failure),
        None => Outcome::Unexpected(format!("the mislabelling was not noticed: {compared:?}")),
    }
}

fn cannot_resume(world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789").behaving(Behaviour::NeverResumes),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };
    match hub.fetch_from(&reference, entry, 4, &world.path("model.gguf.partial")) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a hub with no ranges continued a transfer".to_owned()),
    }
}

fn no_licence(_world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/quiet",
        Repository::holding("model.gguf", b"weights").without_licence(),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/quiet") else {
        return Outcome::Unexpected("owner/quiet is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    match mcf_hub::inspect::terms_are_legible(&listing) {
        Err(failure) => Outcome::Produced(failure),
        Ok(licence) => Outcome::Unexpected(format!("a licence appeared from nowhere: {licence}")),
    }
}

fn truncated_transfer(world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789")
            .behaving(Behaviour::Truncates { after: 4 }),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };

    let into = world.path("model.gguf");
    let fetched = match hub.fetch(&reference, entry, &into) {
        Ok(fetched) => fetched,
        Err(failure) => return Outcome::Unexpected(format!("the fetch failed: {failure}")),
    };
    match mcf_hub::inspect::arrived_as_promised(entry, fetched.bytes) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("a short transfer passed as whole".to_owned()),
    }
}

/// A length-prefixed string, as GGUF writes them.
fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&u64::try_from(value.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
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
