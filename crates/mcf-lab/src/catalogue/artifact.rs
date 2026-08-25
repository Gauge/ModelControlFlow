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

/// The file is not a format MCF reads.
pub(super) const FORMAT_UNSUPPORTED: Scenario = Scenario {
    id: "artifact/format-unsupported",
    produces: Category::ArtifactFormatUnsupported,
    summary: "a file MCF does not read is refused by name, not attempted",
    run: format_unsupported,
};

/// The file is the format MCF reads, and disagrees with itself.
pub(super) const FORMAT_MALFORMED: Scenario = Scenario {
    id: "artifact/format-malformed",
    produces: Category::ArtifactFormatMalformed,
    summary: "a GGUF that ends before what it says is in it names where it stopped",
    run: format_malformed,
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
/// A GGUF header, as far as a scenario needs one: the magic, a version this
/// reader reads, and counts. Written out here rather than borrowed from the
/// reader's tests, because a scenario that shared a builder with the code under
/// test would be a scenario about the builder.
fn gguf_header(tensors: u64, metadata: u64) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&tensors.to_le_bytes());
    bytes.extend_from_slice(&metadata.to_le_bytes());
    bytes
}

fn format_unsupported(world: &World) -> Outcome {
    let path = world.path("model.onnx");
    // A real file, in a format MCF has no reader for. The observable, not its
    // cause (D26): what MCF meets is bytes that are not GGUF.
    if let Err(error) = std::fs::write(&path, b"ONNX\x00\x00\x00\x08not a gguf") {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    match mcf_standin::gguf::read(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a file that is not GGUF was read as one".to_owned()),
    }
}

fn format_malformed(world: &World) -> Outcome {
    let path = world.path("model.gguf");
    // A GGUF that says it holds one metadata entry and then stops. This is what
    // a truncated download leaves behind, and what a reader must refuse rather
    // than fill in.
    let mut bytes = gguf_header(0, 1);
    bytes.extend_from_slice(&12_u64.to_le_bytes());
    bytes.extend_from_slice(b"general.arch");
    if let Err(error) = std::fs::write(&path, &bytes) {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    match mcf_standin::gguf::read(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a truncated GGUF was read as complete".to_owned()),
    }
}

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
