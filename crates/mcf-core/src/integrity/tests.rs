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

#[test]
fn unchanged_bytes_verify() {
    let scratch = Scratch::new("unchanged");
    let path = scratch.write("weights.bin", b"the same bytes as before");
    let recorded = checksum_of(&path).expect("the file reads");
    assert!(verify(&path, &recorded).is_ok());
}

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

#[test]
fn a_file_that_cannot_be_read_is_unreadable_and_not_corrupt() {
    let scratch = Scratch::new("unreadable");
    let expected =
        Checksum::sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
            .expect("a well-formed digest");
    let failure = verify(&scratch.0, &expected).expect_err("a directory does not verify");
    assert_eq!(failure.category(), Category::ArtifactUnreadable);
    assert_eq!(failure.attribution(), crate::failure::Attribution::Machine);
    assert!(failure.context_value("os_error").is_some());
}

#[test]
fn an_artifact_larger_than_one_block_verifies_the_same_way() {
    let scratch = Scratch::new("large");
    let bytes: Vec<u8> = (0..300_000_u32).map(|i| (i % 251) as u8).collect();
    let path = scratch.write("large.bin", &bytes);
    let recorded = checksum_of(&path).expect("the file reads");
    assert!(verify(&path, &recorded).is_ok());
    assert_eq!(recorded.hex(), crate::digest::sha256(&bytes).hex());
}
