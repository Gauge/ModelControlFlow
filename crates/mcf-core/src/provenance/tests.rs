//! Tests for provenance.
//!
//! The chain modelled here is §XII's: a GGUF requantization published by one
//! account, of weights published by another. It is used because §XII calls it
//! the hard provenance case rather than the easy one — and because a type that
//! only handles the easy case would pass a test built from the easy case.
//!
//! B28 keeps the reference model a fixture and never a case in the code, so
//! the names below are illustrative: substituting different ones must change
//! nothing about how any of this behaves.

use super::{
    Artifact, ArtifactName, Checksum, DigestAlgorithm, Licence, Origin, Provenance, Repository,
    Revision, ToolIdentity, Transformation, TransformationKind,
};
use crate::attested::Attested;
use crate::time::Timestamp;

const ACQUIRED: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);
const SHA: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn upstream() -> Provenance {
    Provenance::acquired(
        Origin::hub(
            Repository::new("example-org/example-27B"),
            Some(Revision::new("aa11bb22cc33")),
        ),
        ACQUIRED,
    )
    .with_licence(Licence::spdx("Apache-2.0"))
}

fn derivative() -> Provenance {
    Provenance::acquired(
        Origin::hub(
            Repository::new("example-publisher/example-27B-GGUF"),
            Some(Revision::new("dd44ee55ff66")),
        ),
        ACQUIRED,
    )
    .with_integrity(Checksum::sha256(SHA).expect("a 64-character hex digest"))
    .with_licence(Licence::spdx("Apache-2.0"))
    .transformed(Transformation::new(
        TransformationKind::Requantization,
        Attested::Known("Q4_K_M".to_owned()),
        Attested::Known(ToolIdentity::new("llama.cpp quantize", None)),
        Attested::Unknown,
    ))
    .derived_from(upstream())
}

/// B-006: an artifact handle cannot exist without provenance. The compiler
/// enforces it; this records that the provenance an artifact was built with is
/// the provenance it reports.
#[test]
fn an_artifact_carries_the_provenance_it_was_built_with() {
    let artifact = Artifact::new(ArtifactName::new("example.gguf"), derivative());
    assert_eq!(artifact.provenance(), &derivative());
    assert_eq!(artifact.name().as_str(), "example.gguf");
}

/// A1: an amendment adds to what is known and cannot discard what was known,
/// because it is written as a function of the existing provenance.
#[test]
fn an_amendment_adds_and_does_not_replace() {
    let artifact = Artifact::new(
        ArtifactName::new("example.gguf"),
        Provenance::acquired(Origin::Unattributed, ACQUIRED),
    );
    assert!(!artifact.provenance().integrity().is_known());

    let checked = artifact.amend(|provenance| {
        provenance.with_integrity(Checksum::sha256(SHA).expect("a 64-character hex digest"))
    });
    assert!(checked.provenance().integrity().is_known());
    assert_eq!(checked.provenance().origin(), &Origin::Unattributed);
    assert_eq!(checked.provenance().retrieved_at(), ACQUIRED);
}

/// §XII's case: the derivative traces to its source weights through the
/// publisher's pipeline, and the chain is two artifacts deep.
#[test]
fn the_requantization_chain_traverses_to_its_source() {
    let provenance = derivative();
    assert_eq!(provenance.depth(), 2);
    assert!(provenance.traces_to_a_pinned_source());

    let repositories: Vec<String> = provenance
        .chain()
        .map(|link| match link.origin() {
            Origin::Hub { repository, .. } => repository.to_string(),
            other => other.to_string(),
        })
        .collect();
    assert_eq!(
        repositories,
        [
            "example-publisher/example-27B-GGUF".to_owned(),
            "example-org/example-27B".to_owned()
        ]
    );
}

/// A1: the upstream provenance is kept whole rather than summarized. The
/// source's licence and revision are still readable from the derivative.
#[test]
fn the_upstream_provenance_is_kept_whole() {
    let provenance = derivative();
    let source = provenance
        .source()
        .expect("the derivative names its source");
    assert_eq!(source.licence().known(), Some(&Licence::spdx("Apache-2.0")));
    match source.origin() {
        Origin::Hub { revision, .. } => {
            assert_eq!(revision.known().map(Revision::as_str), Some("aa11bb22cc33"));
        }
        other => panic!("the source is not a hub origin: {other}"),
    }
}

/// A9: a chain that stops is a result, not a failure. It says so, and every
/// field is either recorded or unknown (B-019's condition).
#[test]
fn a_chain_that_stops_says_so() {
    let orphan = Provenance::acquired(Origin::Unattributed, ACQUIRED);
    assert_eq!(orphan.depth(), 1);
    assert!(!orphan.traces_to_a_pinned_source());
    assert!(!orphan.licence().is_known());
    assert!(!orphan.integrity().is_known());
    assert_eq!(orphan.origin().to_string(), "unattributed");
}

/// A7: a hub reference with no revision is a hub origin whose revision is
/// unknown — not a hub origin with an invented revision, and not an
/// unattributed one.
#[test]
fn an_unpinned_hub_reference_is_known_but_unpinned() {
    let unpinned = Provenance::acquired(
        Origin::hub(Repository::new("example-org/example"), None),
        ACQUIRED,
    );
    assert!(!unpinned.traces_to_a_pinned_source());
    assert_eq!(unpinned.origin().to_string(), "example-org/example@unknown");
}

/// A7 again, on the licence: nothing read is unknown, and it renders as
/// `unknown` rather than as a permissive default.
#[test]
fn an_unread_licence_is_unknown_and_not_permissive() {
    let provenance = Provenance::acquired(Origin::Unattributed, ACQUIRED);
    assert_eq!(provenance.licence().known(), None);
    assert_eq!(provenance.licence().to_string(), "unknown");
}

/// Terms that are present and unmatched are a different answer from no terms
/// at all. Collapsing the two would let MCF proceed past terms nobody read.
#[test]
fn unmatched_terms_are_not_the_same_as_no_terms() {
    let unmatched =
        Provenance::acquired(Origin::Unattributed, ACQUIRED).with_licence(Licence::Stated);
    assert!(unmatched.licence().is_known());
    assert!(!Licence::Stated.is_identified());
    assert!(Licence::spdx("MIT").is_identified());
    assert_eq!(unmatched.licence().to_string(), "stated, unmatched");
}

/// A19: the digest is validated against independently known properties — a
/// SHA-256 digest is exactly 64 hexadecimal characters — rather than kept as
/// whatever text arrived.
#[test]
fn a_malformed_digest_is_not_a_checksum() {
    assert!(Checksum::sha256(SHA).is_some());
    assert!(Checksum::sha256("").is_none());
    assert!(Checksum::sha256(&SHA[..63]).is_none());
    assert!(Checksum::sha256(&format!("{SHA}0")).is_none());
    let non_hex = format!("{}z", &SHA[..63]);
    assert!(Checksum::sha256(&non_hex).is_none());
    assert_eq!(DigestAlgorithm::Sha256.hex_length(), 64);
}

/// Case is a spelling, not a difference: two records of one digest compare
/// equal whichever way the hub wrote it.
#[test]
fn digest_case_is_normalized_but_the_digest_is_not_changed() {
    let lower = Checksum::sha256(SHA).expect("a 64-character hex digest");
    let upper = Checksum::sha256(&SHA.to_ascii_uppercase()).expect("still a digest");
    assert_eq!(lower, upper);
    assert_eq!(lower.hex(), SHA);
    assert_eq!(lower.to_string(), format!("sha256:{SHA}"));
}

/// A transformation whose tool version nobody published is recorded as
/// unknown, which is a reproducibility gap stated rather than hidden (P3).
#[test]
fn a_transformation_with_an_unknown_tool_version_says_so() {
    let provenance = derivative();
    let transformation = provenance
        .transformations()
        .first()
        .expect("the derivative records its requantization");
    assert_eq!(transformation.kind(), &TransformationKind::Requantization);
    assert_eq!(
        transformation.detail().known().map(String::as_str),
        Some("Q4_K_M")
    );
    let tool = transformation
        .performed_by()
        .known()
        .expect("the tool is named");
    assert_eq!(tool.name(), "llama.cpp quantize");
    assert_eq!(tool.version(), &Attested::Unknown);
    assert!(!transformation.performed_at().is_known());
    assert!(
        transformation.to_string().contains("unknown"),
        "{transformation}"
    );
}

/// Transformations read forwards: oldest first, so the vector is the history
/// in the order it happened.
#[test]
fn transformations_are_kept_oldest_first() {
    let provenance = Provenance::acquired(Origin::Unattributed, ACQUIRED)
        .transformed(Transformation::new(
            TransformationKind::FormatConversion,
            Attested::Known("to GGUF".to_owned()),
            Attested::Unknown,
            Attested::Unknown,
        ))
        .transformed(Transformation::new(
            TransformationKind::Quantization,
            Attested::Known("Q4_K_M".to_owned()),
            Attested::Unknown,
            Attested::Unknown,
        ));
    let kinds: Vec<String> = provenance
        .transformations()
        .iter()
        .map(|t| t.kind().to_string())
        .collect();
    assert_eq!(kinds, ["format conversion", "quantization"]);
}

/// A transformation MCF has no name for is recorded as what it was called,
/// never filed under the nearest known kind.
#[test]
fn an_unnamed_transformation_keeps_the_words_it_arrived_with() {
    let kind = TransformationKind::Other("pruned by a script nobody kept".to_owned());
    assert_eq!(kind.to_string(), "pruned by a script nobody kept");
    assert_ne!(kind, TransformationKind::Quantization);
}
