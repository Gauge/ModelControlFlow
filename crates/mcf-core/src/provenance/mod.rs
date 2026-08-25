//! Where an artifact came from, and everything that happened to it since.
//!
//! §3.6: an artifact's provenance travels with it. A7: what is not known is
//! never filled with a plausible value. §3.16 and B16 say to make that
//! structural rather than remembered, so [`Artifact`] has no constructor that
//! omits a [`Provenance`], no `Default`, and no way to replace one after the
//! fact — the field is private and there is no setter, because an artifact
//! whose origin could be rewritten is an artifact whose origin is a claim
//! rather than a record.
//!
//! **The chain is the hard part, and it is why this is a linked structure
//! rather than a flat record.** §XII names the reference model's third-party
//! requantization — a GGUF conversion of somebody else's weights — as the hard
//! provenance case rather than the easy one (B-019). A flat record can say
//! *this file came from that repository*; it cannot say *these bytes are a
//! quantization, performed by that tool, of weights that came from a different
//! repository under a different licence*. [`Provenance::derived_from`] carries
//! the upstream artifact's provenance whole, so the chain traverses to its
//! source or stops at an honest [`Attested::Unknown`].
//!
//! ```
//! use mcf_core::provenance::{Artifact, ArtifactName, Origin, Provenance, Repository};
//! use mcf_core::time::Timestamp;
//!
//! let provenance = Provenance::acquired(
//!     Origin::hub(Repository::new("unsloth/example-GGUF"), None),
//!     Timestamp::now(),
//! );
//! let artifact = Artifact::new(ArtifactName::new("example.gguf"), provenance);
//!
//! // Nothing was read, so nothing is claimed.
//! assert!(artifact.provenance().licence().known().is_none());
//! assert!(artifact.provenance().integrity().known().is_none());
//! ```

mod artifact;
mod checksum;
mod licence;
mod origin;
mod transformation;

pub use artifact::{Artifact, ArtifactName};
pub use checksum::{Checksum, DigestAlgorithm};
pub use licence::Licence;
pub use origin::{Origin, Repository, Revision};
pub use transformation::{ToolIdentity, Transformation, TransformationKind};

use crate::attested::Attested;
use crate::time::Timestamp;

/// Where an artifact came from, and everything that happened to it since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    origin: Origin,
    retrieved_at: Timestamp,
    integrity: Attested<Checksum>,
    licence: Attested<Licence>,
    /// Oldest first, so reading the vector is reading the history forwards.
    transformations: Vec<Transformation>,
    derived_from: Option<Box<Provenance>>,
}

impl Provenance {
    /// The provenance of something MCF acquired.
    ///
    /// Everything that has to be *read* — the checksum, the licence — starts
    /// unknown, because at the moment of acquisition MCF has not read it. A7
    /// makes that a state rather than a gap: the fields are filled by the code
    /// that verified them, and stay unknown otherwise.
    #[must_use]
    pub fn acquired(origin: Origin, retrieved_at: Timestamp) -> Self {
        Self {
            origin,
            retrieved_at,
            integrity: Attested::Unknown,
            licence: Attested::Unknown,
            transformations: Vec::new(),
            derived_from: None,
        }
    }

    /// Records a checksum MCF verified.
    ///
    /// §7.49 and B-301 re-verify before a long run rather than only at
    /// acquisition, so this is set by whatever last checked the bytes, not
    /// once at the start.
    #[must_use]
    pub fn with_integrity(mut self, checksum: Checksum) -> Self {
        self.integrity = Attested::Known(checksum);
        self
    }

    /// Records the licence MCF read.
    ///
    /// §III requires the licence be surfaced before use and B-023 requires it
    /// be reported as `Unknown` rather than as a plausible default. Note that
    /// [`Licence::Stated`] — text present, terms unmatchable — is a *known*
    /// licence with an unmatched body, which is a different answer from
    /// nothing being there, and `hub.licence.unparseable` is the category for
    /// it.
    #[must_use]
    pub fn with_licence(mut self, licence: Licence) -> Self {
        self.licence = Attested::Known(licence);
        self
    }

    /// Records something that was done to the artifact.
    #[must_use]
    pub fn transformed(mut self, transformation: Transformation) -> Self {
        self.transformations.push(transformation);
        self
    }

    /// Records the provenance of the artifact these bytes were derived from.
    ///
    /// The upstream provenance is kept whole rather than summarized: A1 says
    /// never lose information, and a summary of a licence or a revision is the
    /// part of the chain that later turns out to matter.
    #[must_use]
    pub fn derived_from(mut self, source: Self) -> Self {
        self.derived_from = Some(Box::new(source));
        self
    }

    /// Where these bytes came from.
    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    /// When MCF obtained them.
    #[must_use]
    pub const fn retrieved_at(&self) -> Timestamp {
        self.retrieved_at
    }

    /// The checksum, if one has been verified.
    #[must_use]
    pub const fn integrity(&self) -> &Attested<Checksum> {
        &self.integrity
    }

    /// The licence, if one has been read.
    #[must_use]
    pub const fn licence(&self) -> &Attested<Licence> {
        &self.licence
    }

    /// What was done to the artifact, oldest first.
    #[must_use]
    pub fn transformations(&self) -> &[Transformation] {
        &self.transformations
    }

    /// The provenance of the artifact these bytes were derived from.
    #[must_use]
    pub fn source(&self) -> Option<&Self> {
        self.derived_from.as_deref()
    }

    /// This provenance and every one it derives from, nearest first.
    pub fn chain(&self) -> impl Iterator<Item = &Self> {
        core::iter::successors(Some(self), |provenance| provenance.source())
    }

    /// Whether the chain reaches an origin that names a repository and a
    /// revision.
    ///
    /// A *finding*, not a verdict: A9 makes "the chain stops here" a result
    /// worth storing and surfacing, and B-019's condition is that every field
    /// is either recorded or `Unknown` — not that every field is known.
    #[must_use]
    pub fn traces_to_a_pinned_source(&self) -> bool {
        self.chain().any(|provenance| match provenance.origin() {
            Origin::Hub { revision, .. } => revision.is_known(),
            Origin::LocalFile { .. } | Origin::Unattributed => false,
        })
    }

    /// How many artifacts deep the chain goes, counting this one.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.chain().count()
    }
}

#[cfg(test)]
mod tests;
