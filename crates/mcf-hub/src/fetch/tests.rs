//! Acquisitions against sources that behave badly.
//!
//! The source here is a stand-in written in this file rather than the
//! laboratory's — `mcf-hub` is below `mcf-lab` in the layering, and a crate
//! cannot test against something above it. The laboratory's scenarios drive the
//! same code through the real simulated hub, which is where the end-to-end
//! claim is made; these are the unit-level cases.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::{ATTEMPTS, Verification, acquire, partial_path};
use crate::reference::{Reference, parse};
use crate::source::{Entry, Fetched, Listing, Source};
use mcf_core::digest::sha256;
use mcf_core::failure::{Category, Result};

/// How a stand-in source behaves on each attempt.
enum Serves {
    /// The whole file, every time.
    Everything,
    /// This many bytes per attempt, continuing where asked.
    ThisMany(u64),
    /// The whole file, but a different one from what it declared.
    SomethingElse,
    /// Everything, and refuses to continue from an offset.
    EverythingButNeverResumes,
}

struct StandIn {
    bytes: Vec<u8>,
    serves: Serves,
    attempts: RefCell<usize>,
}

impl StandIn {
    fn new(bytes: &[u8], serves: Serves) -> Self {
        Self {
            bytes: bytes.to_vec(),
            serves,
            attempts: RefCell::new(0),
        }
    }

    fn entry(&self) -> Entry {
        Entry::new("model.gguf", self.bytes.len() as u64).declaring(sha256(&self.bytes).hex())
    }

    fn write(into: &Path, bytes: &[u8], append: bool) -> Fetched {
        if append {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(into)
                .expect("the partial file opens");
            file.write_all(bytes).expect("it writes");
        } else {
            std::fs::write(into, bytes).expect("it writes");
        }
        Fetched {
            bytes: bytes.len() as u64,
            digest: sha256(bytes).hex(),
        }
    }
}

impl Source for StandIn {
    fn describe(&self) -> String {
        "a stand-in source".to_owned()
    }

    fn list(&self, reference: &Reference) -> Result<Listing> {
        Ok(Listing {
            reference: reference.clone(),
            revision: Some("main".to_owned()),
            entries: vec![self.entry()],
            declared_licence: Some("apache-2.0".to_owned()),
            lineage: None,
        })
    }

    fn fetch(&self, _reference: &Reference, _entry: &Entry, into: &Path) -> Result<Fetched> {
        *self.attempts.borrow_mut() += 1;
        let served: Vec<u8> = match self.serves {
            Serves::Everything | Serves::EverythingButNeverResumes => self.bytes.clone(),
            Serves::ThisMany(count) => self
                .bytes
                .get(..usize::try_from(count).unwrap_or(0).min(self.bytes.len()))
                .unwrap_or(&[])
                .to_vec(),
            Serves::SomethingElse => self.bytes.iter().map(|byte| byte ^ 0xFF).collect(),
        };
        Ok(Self::write(into, &served, false))
    }

    fn fetch_from(
        &self,
        _reference: &Reference,
        _entry: &Entry,
        from: u64,
        into: &Path,
    ) -> Result<Fetched> {
        if matches!(self.serves, Serves::EverythingButNeverResumes) {
            return Err(mcf_core::failure::Failure::new(
                Category::HubUnreachable,
                mcf_core::failure::Attribution::Machine,
                mcf_core::failure::Disposition::Refused,
                mcf_core::failure::Subsystem::new("stand-in"),
                "this source cannot continue a transfer from an offset",
            ));
        }
        *self.attempts.borrow_mut() += 1;
        let start = usize::try_from(from)
            .unwrap_or(usize::MAX)
            .min(self.bytes.len());
        let rest = self.bytes.get(start..).unwrap_or(&[]);
        let served: Vec<u8> = match self.serves {
            Serves::ThisMany(count) => rest
                .get(..usize::try_from(count).unwrap_or(0).min(rest.len()))
                .unwrap_or(&[])
                .to_vec(),
            Serves::SomethingElse => rest.iter().map(|byte| byte ^ 0xFF).collect(),
            Serves::Everything | Serves::EverythingButNeverResumes => rest.to_vec(),
        };
        Ok(Self::write(into, &served, true))
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-fetch-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self(path)
    }
    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

fn a_reference() -> Reference {
    parse("owner/model").expect("a reference")
}

/// The ordinary case: it arrives, it verifies, it gets the artifact's name.
#[test]
fn a_whole_transfer_is_verified_and_named() {
    let scratch = Scratch::new("whole");
    let source = StandIn::new(b"0123456789", Serves::Everything);
    let into = scratch.at("model.gguf");

    let acquired = acquire(&source, &a_reference(), &source.entry(), &into).expect("it arrives");
    assert_eq!(acquired.bytes, 10);
    assert_eq!(acquired.attempts, 1);
    assert!(!acquired.resumed);
    assert_eq!(
        acquired.verification,
        Verification::Digest {
            digest: sha256(b"0123456789").hex()
        }
    );
    assert!(into.exists(), "the artifact has its name");
    assert!(!partial_path(&into).exists(), "nothing is left behind");
}

/// A source that stops repeatedly is carried across by resuming, and the result
/// says how many attempts it took — a fact about the source worth keeping.
#[test]
fn a_transfer_that_keeps_stopping_is_resumed_to_the_end() {
    let scratch = Scratch::new("resumes");
    let source = StandIn::new(b"0123456789", Serves::ThisMany(3));
    let into = scratch.at("model.gguf");

    let acquired = acquire(&source, &a_reference(), &source.entry(), &into).expect("it arrives");
    assert_eq!(acquired.bytes, 10);
    assert!(acquired.resumed, "it continued rather than starting again");
    assert!(acquired.attempts > 1, "{} attempts", acquired.attempts);
    assert_eq!(
        std::fs::read(&into).expect("it is there"),
        b"0123456789",
        "the file is the file, not a repetition of its first bytes"
    );
}

/// Interrupted at ninety per cent, it continues from ninety per cent — which is
/// B-021's condition, stated as a partial file that a second call finishes.
#[test]
fn a_transfer_interrupted_near_the_end_continues_from_there() {
    let scratch = Scratch::new("ninety");
    let whole = b"0123456789012345678901234567890123456789";
    let source = StandIn::new(whole, Serves::Everything);
    let into = scratch.at("model.gguf");

    // What a crash leaves: thirty-six of forty bytes in the partial file.
    std::fs::write(partial_path(&into), &whole[..36]).expect("a partial transfer");

    let acquired = acquire(&source, &a_reference(), &source.entry(), &into).expect("it finishes");
    assert!(acquired.resumed, "it started again instead of continuing");
    assert_eq!(acquired.attempts, 1, "one attempt was enough to finish it");
    assert_eq!(std::fs::read(&into).expect("it is there"), whole);
}

/// A file that changed under the transfer is a classified failure, and the
/// mixture is not kept: half of one file and half of another is the one thing
/// worse than no file.
#[test]
fn a_file_that_changed_under_the_transfer_is_refused_and_not_kept() {
    let scratch = Scratch::new("changed");
    let whole = b"0123456789";
    let source = StandIn::new(whole, Serves::SomethingElse);
    let into = scratch.at("model.gguf");

    let failure = acquire(&source, &a_reference(), &source.entry(), &into).expect_err("it differs");
    assert_eq!(failure.category(), Category::ArtifactCorrupt);
    assert!(
        !into.exists(),
        "a file that did not verify was given the artifact's name"
    );
    assert!(
        !partial_path(&into).exists(),
        "a mixture of two files was kept for a later attempt to resume into"
    );

    let context: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        context.iter().any(|entry| entry.starts_with("declared=")),
        "{context:?}"
    );
    assert!(
        context.iter().any(|entry| entry.starts_with("computed=")),
        "{context:?}"
    );
}

/// A source that cannot resume is a fact about the source, not a failure: MCF
/// starts again and finishes.
#[test]
fn a_source_that_cannot_resume_is_restarted_rather_than_refused() {
    let scratch = Scratch::new("norange");
    let whole = b"0123456789";
    let source = StandIn::new(whole, Serves::EverythingButNeverResumes);
    let into = scratch.at("model.gguf");
    std::fs::write(partial_path(&into), b"012").expect("a partial transfer");

    let acquired = acquire(&source, &a_reference(), &source.entry(), &into).expect("it finishes");
    assert!(
        !acquired.resumed,
        "it claimed to resume against a source that cannot"
    );
    assert_eq!(std::fs::read(&into).expect("it is there"), whole);
}

/// A source that never delivers enough gives up, saying how far it got and
/// leaving the partial file to continue from later.
#[test]
fn a_transfer_that_never_finishes_says_how_far_it_got() {
    let scratch = Scratch::new("never");
    // One byte per attempt against a ten-byte file: five attempts reach five.
    let source = StandIn::new(b"0123456789", Serves::ThisMany(1));
    let into = scratch.at("model.gguf");

    let failure = acquire(&source, &a_reference(), &source.entry(), &into).expect_err("too slow");
    assert_eq!(failure.category(), Category::ArtifactIncomplete);
    assert!(
        !into.exists(),
        "an unfinished transfer was given the artifact's name"
    );
    assert!(
        partial_path(&into).exists(),
        "the partial file is what a later attempt continues"
    );

    let context: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        context.iter().any(|entry| entry == "promised=10"),
        "{context:?}"
    );
    assert!(
        context.iter().any(|entry| entry == "reached=5"),
        "{context:?}"
    );
    assert!(
        context
            .iter()
            .any(|entry| *entry == format!("attempts={ATTEMPTS}")),
        "{context:?}"
    );
}

/// A hub that declares no digest leaves the artifact *held* rather than
/// verified, and says so — A21 keeps that a state rather than a weaker success.
#[test]
fn an_artifact_nobody_could_check_is_held_rather_than_verified() {
    let scratch = Scratch::new("undeclared");
    let source = StandIn::new(b"0123456789", Serves::Everything);
    let entry = Entry::new("model.gguf", 10);
    let into = scratch.at("model.gguf");

    let acquired = acquire(&source, &a_reference(), &entry, &into).expect("it arrives");
    match acquired.verification {
        Verification::LengthOnly { digest } => {
            assert_eq!(
                digest,
                sha256(b"0123456789").hex(),
                "the digest MCF computed travels"
            );
        }
        Verification::Digest { .. } => panic!("nothing was declared to verify against"),
    }
}

/// Acquiring something already held and verified costs nothing.
#[test]
fn an_artifact_already_held_is_not_fetched_again() {
    let scratch = Scratch::new("already");
    let source = StandIn::new(b"0123456789", Serves::Everything);
    let into = scratch.at("model.gguf");

    let first = acquire(&source, &a_reference(), &source.entry(), &into).expect("it arrives");
    assert_eq!(first.attempts, 1);
    let again = acquire(&source, &a_reference(), &source.entry(), &into).expect("it is held");
    assert_eq!(again.attempts, 0, "it fetched something it already had");
    assert_eq!(*source.attempts.borrow(), 1, "the source was asked twice");
}

/// The artifact's own name means verified, and the partial name means in
/// progress. That is the whole structural claim.
#[test]
fn the_partial_name_is_never_the_artifacts_name() {
    let into = Path::new("/models/owner/model.gguf");
    let partial = partial_path(into);
    assert_eq!(partial, Path::new("/models/owner/model.gguf.partial"));
    assert_ne!(partial, into);
    assert_eq!(
        partial.parent(),
        into.parent(),
        "it stays beside the artifact"
    );
}
