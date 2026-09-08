use mcf_core::failure::Category;
use mcf_core::integrity::{checksum_of, verify};
use mcf_core::provenance::Checksum;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const CORRUPTED: Scenario = Scenario {
    id: "artifact/corrupted-bytes",
    produces: Category::ArtifactCorrupt,
    summary: "one flipped bit in a verified artifact is caught before it is used",
    run: corrupted,
};

pub(super) const MISSING: Scenario = Scenario {
    id: "artifact/missing",
    produces: Category::ArtifactMissing,
    summary: "an artifact that is referenced and not present is missing, not corrupt",
    run: missing,
};

pub(super) const FORMAT_UNSUPPORTED: Scenario = Scenario {
    id: "artifact/format-unsupported",
    produces: Category::ArtifactFormatUnsupported,
    summary: "a file MCF does not read is refused by name, not attempted",
    run: format_unsupported,
};

pub(super) const FORMAT_MALFORMED: Scenario = Scenario {
    id: "artifact/format-malformed",
    produces: Category::ArtifactFormatMalformed,
    summary: "a GGUF that ends before what it says is in it names where it stopped",
    run: format_malformed,
};

pub(super) const PROVENANCE_INCOMPLETE: Scenario = Scenario {
    id: "artifact/model-says-too-little",
    produces: Category::ArtifactProvenanceIncomplete,
    summary: "a model that does not state its own shape is refused, naming what was missing",
    run: provenance_incomplete,
};

pub(super) const UNREADABLE: Scenario = Scenario {
    id: "artifact/unreadable",
    produces: Category::ArtifactUnreadable,
    summary: "an artifact that is present and unreadable is the machine's failure, not the artifact's",
    run: unreadable,
};

fn gguf_header(tensors: u64, metadata: u64) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&tensors.to_le_bytes());
    bytes.extend_from_slice(&metadata.to_le_bytes());
    bytes
}

fn format_unsupported(world: &World) -> Outcome {
    let path = world.path("model.onnx");
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

fn provenance_incomplete(world: &World) -> Outcome {
    let path = world.path("model.gguf");
    let mut bytes = gguf_header(0, 1);
    push_string(&mut bytes, "general.architecture");
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    push_string(&mut bytes, "llama");
    if let Err(error) = std::fs::write(&path, &bytes) {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    let file = match mcf_standin::gguf::read(&path) {
        Ok(file) => file,
        Err(failure) => {
            return Outcome::Unexpected(format!("the file itself should read: {failure}"));
        }
    };
    match mcf_standin::llama::load(&file, &bytes) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a model with no stated shape was loaded".to_owned()),
    }
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&u64::try_from(value.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
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
    match verify(world.scratch(), &any_digest()) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("a directory verified as an artifact".to_owned()),
    }
}
