//! Each of §7.38's four decays, and the answer a hub gives for none of them.

use mcf_core::attested::Attested;
use mcf_core::provenance::{
    Checksum, Decay, DigestAlgorithm, Licence, Origin, Provenance, Repository,
};
use mcf_core::time::Timestamp;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use super::look;
use crate::reference::Reference;
use crate::source::{Entry, Fetched, Listing, Source};

/// A source that answers with one prepared listing, or with one prepared
/// refusal.
///
/// A test double rather than the laboratory's hub, which lives above this crate
/// and cannot be reached from inside it. What is under test here is the
/// *comparison* — what MCF concludes from what a hub said — and the whole-system
/// tier drives the same code against a hub that is a real server (D26).
struct Says(Result<Listing>);

impl Says {
    fn listing(gated: Option<&str>, licence: Option<&str>, digest: Option<&str>) -> Self {
        Self(Ok(Listing {
            reference: a_reference(),
            revision: Some("main".to_owned()),
            entries: vec![Entry {
                path: "model.gguf".to_owned(),
                size: 16,
                digest: digest.map(str::to_owned),
            }],
            gated: gated.map(str::to_owned),
            declared_licence: licence.map(str::to_owned),
            lineage: None,
        }))
    }

    fn refusing(category: Category) -> Self {
        Self(Err(Failure::new(
            category,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("a test"),
            "the hub said no",
        )))
    }
}

impl Source for Says {
    fn describe(&self) -> String {
        "a source that says one thing".to_owned()
    }

    fn list(&self, _reference: &Reference) -> Result<Listing> {
        match &self.0 {
            Ok(listing) => Ok(listing.clone()),
            Err(failure) => Err(failure.clone()),
        }
    }

    fn fetch(
        &self,
        _reference: &Reference,
        _entry: &Entry,
        _into: &std::path::Path,
    ) -> Result<Fetched> {
        unreachable!("a decay check fetches nothing")
    }
}

fn a_reference() -> Reference {
    Reference {
        owner: "owner".to_owned(),
        name: "model".to_owned(),
        revision: Some("main".to_owned()),
        file: None,
    }
}

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

const WEIGHTS: &[u8] = b"GGUF the weights";

fn digest_of(bytes: &[u8]) -> String {
    mcf_core::digest::sha256(bytes).hex()
}

/// What MCF wrote down when it acquired the file: a pinned revision, the
/// licence the repository declared, and the digest it verified.
fn acquired(revision: Option<&str>, licence: &str, digest: &str) -> Provenance {
    let origin = Origin::hub(
        Repository::new("owner/model"),
        revision.map(mcf_core::provenance::Revision::new),
    );
    let mut provenance = Provenance::acquired(origin, AT).with_licence(Licence::spdx(licence));
    if let Some(checksum) = Checksum::new(DigestAlgorithm::Sha256, digest) {
        provenance = provenance.with_integrity(checksum);
    }
    provenance
}

/// Nothing has changed, and *checked and unchanged* is a finding rather than an
/// absence (A1).
#[test]
fn a_repository_that_still_says_what_it_said_is_unchanged() {
    let hub = Says::listing(None, Some("apache-2.0"), Some(&digest_of(WEIGHTS)));
    let provenance = acquired(Some("main"), "apache-2.0", &digest_of(WEIGHTS));

    let observed = look(&hub, &provenance, Some("model.gguf"), AT).expect("an upstream to look at");
    assert_eq!(observed.found, Decay::Unchanged);
    assert!(!observed.found.is_a_change());
}

/// The licence changed under a pin. Nothing refuses; the field is simply
/// different, and only a comparison finds it (F17).
#[test]
fn a_relicensing_is_found_by_comparing() {
    let hub = Says::listing(None, Some("cc-by-nc-4.0"), Some(&digest_of(WEIGHTS)));
    let provenance = acquired(Some("main"), "apache-2.0", &digest_of(WEIGHTS));

    let observed = look(&hub, &provenance, Some("model.gguf"), AT).expect("an upstream");
    assert_eq!(
        observed.found,
        Decay::Relicensed {
            was: "apache-2.0".to_owned(),
            now: "cc-by-nc-4.0".to_owned(),
        }
    );
    assert!(observed.found.is_a_change());
}

/// The file is published under the same name and is not the file MCF has. The
/// local copy is untouched — this is about what somebody else would get.
#[test]
fn a_replaced_file_is_found_by_its_digest() {
    let hub = Says::listing(
        None,
        Some("apache-2.0"),
        Some(&digest_of(b"GGUF different weights")),
    );
    let provenance = acquired(Some("main"), "apache-2.0", &digest_of(WEIGHTS));

    let observed = look(&hub, &provenance, Some("model.gguf"), AT).expect("an upstream");
    match observed.found {
        Decay::Replaced { file, was, now } => {
            assert_eq!(file, "model.gguf");
            assert_eq!(was, digest_of(WEIGHTS));
            assert_eq!(now, digest_of(b"GGUF different weights"));
        }
        other => panic!("a replaced file was not found: {other}"),
    }
}

/// A gate that closed after acquisition — the one decay a hub announces before
/// it bites, because the card is readable while the file is not (F17).
#[test]
fn a_gate_that_closed_is_found_in_the_card() {
    let hub = Says::listing(
        Some("manual"),
        Some("apache-2.0"),
        Some(&digest_of(WEIGHTS)),
    );
    let provenance = acquired(Some("main"), "apache-2.0", &digest_of(WEIGHTS));

    let observed = look(&hub, &provenance, Some("model.gguf"), AT).expect("an upstream");
    assert_eq!(
        observed.found,
        Decay::Gated {
            how: "manual".to_owned()
        }
    );
}

/// An artifact with no upstream has nothing to check, and MCF does not invent
/// one to look at (A7).
#[test]
fn an_artifact_with_no_upstream_is_not_checked() {
    let hub = Says::listing(None, Some("apache-2.0"), Some(&digest_of(WEIGHTS)));
    let local = Provenance::acquired(
        Origin::LocalFile {
            path: std::path::PathBuf::from("/somewhere/model.gguf"),
        },
        AT,
    );
    assert!(look(&hub, &local, Some("model.gguf"), AT).is_none());

    let unattributed = Provenance::acquired(Origin::Unattributed, AT);
    assert!(look(&hub, &unattributed, None, AT).is_none());
}

/// The revision is gone: the one unambiguous answer a hub gives, and only when
/// something was pinned in the first place (F17).
#[test]
fn a_withdrawn_revision_is_named_as_one() {
    let hub = Says::refusing(Category::HubRefNotFound);
    let pinned = acquired(Some("abc123"), "apache-2.0", &digest_of(WEIGHTS));
    assert_eq!(
        look(&hub, &pinned, Some("model.gguf"), AT)
            .expect("an upstream")
            .found,
        Decay::RevisionGone {
            revision: "abc123".to_owned()
        }
    );

    // Nothing pinned means nothing is *gone*: what the hub is saying is that
    // the repository is not there, which is the ambiguous answer.
    let unpinned = acquired(None, "apache-2.0", &digest_of(WEIGHTS));
    match look(&hub, &unpinned, Some("model.gguf"), AT)
        .expect("an upstream")
        .found
    {
        Decay::Unreachable { said } => assert_eq!(said, "hub.ref.not_found"),
        other => panic!("an unpinned reference reported {other}"),
    }
}

/// A hub that will not say is not a decay: it says nothing about whether
/// anything changed, and reporting an absence as an event is what A7 forbids
/// (F17 — private, withdrawn and never-existed are one answer).
#[test]
fn a_hub_that_will_not_say_is_not_reported_as_a_change() {
    for category in [Category::HubAuthRequired, Category::HubAuthRejected] {
        let hub = Says::refusing(category);
        let provenance = acquired(Some("main"), "apache-2.0", &digest_of(WEIGHTS));
        let observed = look(&hub, &provenance, Some("model.gguf"), AT).expect("an upstream");
        assert!(
            !observed.found.is_a_change(),
            "{category:?} was reported as a change"
        );
        assert_eq!(
            observed.found,
            Decay::Unreachable {
                said: category.code().to_owned()
            }
        );
    }
}
