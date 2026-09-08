use std::io::Read as _;
use std::path::Path;

use crate::digest::Sha256;
use crate::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use crate::provenance::Checksum;

const WHERE: Subsystem = Subsystem::new("mcf-core::integrity");

const BLOCK: usize = 64 * 1024;

pub fn verify(path: &Path, expected: &Checksum) -> Result<()> {
    let found = digest_of(path)?;
    if found.hex() == expected.hex() {
        return Ok(());
    }
    Err(Failure::new(
        Category::ArtifactCorrupt,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the artifact's bytes no longer match the digest recorded for them",
    )
    .with_context("path", path.display().to_string())
    .with_context("expected", expected.to_string())
    .with_context("found", format!("{}:{}", expected.algorithm(), found)))
}

pub fn digest_of(path: &Path) -> Result<crate::digest::Digest> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Failure::new(
                Category::ArtifactMissing,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the artifact is referenced and is not present",
            )
            .with_context("path", path.display().to_string())
        } else {
            unreadable(path, &error)
        }
    })?;

    let mut hasher = Sha256::new();
    let mut block = vec![0_u8; BLOCK];
    loop {
        let read = file
            .read(&mut block)
            .map_err(|error| unreadable(path, &error))?;
        if read == 0 {
            break;
        }
        hasher.update(block.get(..read).unwrap_or(&[]));
    }
    Ok(hasher.finish())
}

pub fn checksum_of(path: &Path) -> Result<Checksum> {
    Ok(Checksum::of(digest_of(path)?))
}

fn unreadable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::ArtifactUnreadable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the artifact is present and could not be read",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

#[cfg(test)]
mod tests;
