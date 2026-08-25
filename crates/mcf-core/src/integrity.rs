//! Checking that the bytes are still the bytes.
//!
//! §7.49 and B-301: *artifact checksums are verified at acquisition and never
//! again, so re-verifying before a long run would catch silent disk corruption
//! before it produces a garbage result rather than after.* That is the whole
//! of it — a twenty-hour evaluation against weights that lost a sector is
//! twenty hours spent measuring a fault, and §3.4's conditions would record
//! everything about the run except the one thing that made it wrong.
//!
//! **Three outcomes, and they are different questions.** The bytes verify; the
//! bytes are there and differ (`artifact.corrupt`, attributed to the artifact);
//! or the file is there and cannot be read at all (`artifact.unreadable`,
//! attributed to the machine). The last was a gap in the taxonomy until B-301
//! needed it: a permission error and a media error are claims about the
//! machine, and filing them under *fails verification* would put the wrong
//! attribution on a failure — B24's difference, one level down.
//!
//! **Read a block at a time.** D24 budgets MCF at twenty megabytes resident
//! and §XII's reference model is measured in gigabytes, so the only available
//! shape is streaming.

use std::io::Read as _;
use std::path::Path;

use crate::digest::Sha256;
use crate::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use crate::provenance::Checksum;

const WHERE: Subsystem = Subsystem::new("mcf-core::integrity");

/// How much is read at a time.
///
/// Sixty-four kibibytes: large enough that the read syscall is not the cost,
/// small enough to be invisible against D24's twenty-megabyte ceiling.
const BLOCK: usize = 64 * 1024;

/// Re-computes an artifact's digest and compares it with what provenance says.
///
/// # Errors
///
/// `artifact.missing` when the file is not there; `artifact.unreadable` when it
/// is there and cannot be read; `artifact.corrupt` when it reads and does not
/// match. Each carries the path, and the last carries both digests, because
/// B21 measures a failure record by whether the laboratory can rebuild the
/// failure from it — and *which* bytes it got is the thing a reader needs.
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

/// Computes an artifact's digest.
///
/// # Errors
///
/// `artifact.missing` or `artifact.unreadable`, as above.
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

/// The digest of an artifact, as a [`Checksum`] provenance can hold.
///
/// # Errors
///
/// As [`digest_of`].
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
