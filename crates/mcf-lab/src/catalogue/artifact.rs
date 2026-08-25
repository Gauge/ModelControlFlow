//! Scenarios that damage an artifact.
//!
//! §7.49's case: an artifact verified once at acquisition and never again,
//! quietly losing a sector, and a twenty-hour evaluation spent measuring the
//! fault. B-301's re-verification is what catches it; these are what
//! demonstrate that it does.
//!
//! Each builds the observable and not its cause (D26): corruption is a byte
//! rewritten, not a failing disk; unreadability is a path that is not a file,
//! not a revoked permission on a machine the laboratory does not own (A27).

use mcf_core::failure::Category;
use mcf_core::integrity::{checksum_of, verify};
use mcf_core::provenance::Checksum;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The bytes changed under a recorded digest.
pub(super) const CORRUPTED: Scenario = Scenario {
    id: "artifact/corrupted-bytes",
    produces: Category::ArtifactCorrupt,
    summary: "one flipped bit in a verified artifact is caught before it is used",
    run: corrupted,
};

/// The artifact is gone.
pub(super) const MISSING: Scenario = Scenario {
    id: "artifact/missing",
    produces: Category::ArtifactMissing,
    summary: "an artifact that is referenced and not present is missing, not corrupt",
    run: missing,
};

/// The artifact is there and will not be read.
pub(super) const UNREADABLE: Scenario = Scenario {
    id: "artifact/unreadable",
    produces: Category::ArtifactUnreadable,
    summary: "an artifact that is present and unreadable is the machine's failure, not the artifact's",
    run: unreadable,
};

/// A digest of nothing, for the two scenarios where the file is never read.
///
/// Computed rather than written out, so there is no parse to fail and no
/// unreachable arm to explain (see `Checksum::of`).
fn any_digest() -> Checksum {
    Checksum::of(mcf_core::digest::sha256(b""))
}

fn corrupted(world: &World) -> Outcome {
    let path = world.path("weights.bin");
    let mut bytes = vec![0_u8; 8192];
    if let Err(error) = std::fs::write(&path, &bytes) {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    let recorded = match checksum_of(&path) {
        Ok(checksum) => checksum,
        Err(failure) => return Outcome::Unexpected(format!("could not verify it: {failure}")),
    };

    // One bit, in the middle, which is what silent disk corruption looks like.
    if let Some(byte) = bytes.get_mut(4096) {
        *byte ^= 0x01;
    }
    if let Err(error) = std::fs::write(&path, &bytes) {
        return Outcome::Unexpected(format!("could not corrupt it: {error}"));
    }

    match verify(&path, &recorded) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("a flipped bit verified".to_owned()),
    }
}

fn missing(world: &World) -> Outcome {
    let path = world.path("gone.bin");
    match verify(&path, &any_digest()) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("an absent artifact verified".to_owned()),
    }
}

fn unreadable(world: &World) -> Outcome {
    // A directory is present, is not a file, and refuses a read. It produces
    // what MCF observes without changing anything on a machine the laboratory
    // does not own (D26, A27).
    match verify(world.scratch(), &any_digest()) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("a directory verified as an artifact".to_owned()),
    }
}
