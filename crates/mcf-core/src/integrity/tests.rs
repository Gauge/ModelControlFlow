//! Tests for re-verification.
//!
//! B19 keeps them hermetic: files this test writes, in a directory it removes.

use std::path::PathBuf;

use super::{checksum_of, verify};
use crate::failure::Category;
use crate::provenance::Checksum;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("mcf-integrity-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        let _made = std::fs::create_dir_all(&path);
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("a temporary file is writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// A19: the digest of a known file against the published vector for those
/// bytes, so the file path is checked and not just the hasher.
#[test]
fn a_files_digest_is_the_digest_of_its_bytes() {
    let scratch = Scratch::new("known");
    let path = scratch.write("abc.bin", b"abc");
    let checksum = checksum_of(&path).expect("the file reads");
    assert_eq!(
        checksum.hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// The file that has not changed verifies.
#[test]
fn unchanged_bytes_verify() {
    let scratch = Scratch::new("unchanged");
    let path = scratch.write("weights.bin", b"the same bytes as before");
    let recorded = checksum_of(&path).expect("the file reads");
    assert!(verify(&path, &recorded).is_ok());
}

/// §7.49's case: a handful of flipped bits, caught before the run rather than
/// after it. The failure names both digests, because which bytes it got is
/// what a reader needs (B21).
#[test]
fn a_single_flipped_bit_is_caught_and_both_digests_are_named() {
    let scratch = Scratch::new("corrupt");
    let mut bytes = vec![0_u8; 8192];
    let path = scratch.write("weights.bin", &bytes);
    let recorded = checksum_of(&path).expect("the file reads");

    if let Some(byte) = bytes.get_mut(4096) {
        *byte ^= 0x01;
    }
    let path = scratch.write("weights.bin", &bytes);

    let failure = verify(&path, &recorded).expect_err("corrupted bytes do not verify");
    assert_eq!(failure.category(), Category::ArtifactCorrupt);
    assert_eq!(failure.attribution(), crate::failure::Attribution::Artifact);
    assert!(failure.context_value("expected").is_some());
    assert!(failure.context_value("found").is_some());
    assert_ne!(
        failure.context_value("expected"),
        failure.context_value("found")
    );
}

/// A file that is not there is missing, not corrupt: A2 wants the failure that
/// happened rather than the nearest one.
#[test]
fn an_absent_file_is_missing_and_not_corrupt() {
    let expected =
        Checksum::sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            .expect("a well-formed digest");
    let failure = verify(
        std::path::Path::new("/nonexistent/mcf-no-such-artifact"),
        &expected,
    )
    .expect_err("an absent artifact does not verify");
    assert_eq!(failure.category(), Category::ArtifactMissing);
}

/// A file that is there and cannot be read is neither missing nor corrupt.
/// That distinction is why `artifact.unreadable` was added: the attribution
/// differs, and B24's whole point is that attribution is a verdict.
#[test]
fn a_file_that_cannot_be_read_is_unreadable_and_not_corrupt() {
    // A directory is present, is not a file, and refuses a read — the most
    // portable way to produce "there and unreadable" without changing
    // permissions on a machine the suite does not own (A27).
    let scratch = Scratch::new("unreadable");
    let expected =
        Checksum::sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            .expect("a well-formed digest");
    let failure = verify(&scratch.0, &expected).expect_err("a directory does not verify");
    assert_eq!(failure.category(), Category::ArtifactUnreadable);
    assert_eq!(failure.attribution(), crate::failure::Attribution::Machine);
    assert!(failure.context_value("os_error").is_some());
}

/// Verification streams, so an artifact larger than one block is handled the
/// same as one smaller. §XII's reference model is measured in gigabytes and
/// D24 budgets MCF at twenty megabytes resident.
#[test]
fn an_artifact_larger_than_one_block_verifies_the_same_way() {
    let scratch = Scratch::new("large");
    let bytes: Vec<u8> = (0..300_000_u32).map(|i| (i % 251) as u8).collect();
    let path = scratch.write("large.bin", &bytes);
    let recorded = checksum_of(&path).expect("the file reads");
    assert!(verify(&path, &recorded).is_ok());
    assert_eq!(recorded.hex(), crate::digest::sha256(&bytes).hex());
}
