mod artifact;
mod checksum;
mod licence;
mod origin;
mod pin;
mod transformation;
mod upstream;

pub use artifact::{Artifact, ArtifactName};
pub use checksum::{Checksum, DigestAlgorithm};
pub use licence::Licence;
pub use origin::{Origin, Repository, Revision};
pub use pin::checked_out;
pub use transformation::{ToolIdentity, Transformation, TransformationKind};
pub use upstream::{Decay, Observation};

use crate::attested::Attested;
use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    origin: Origin,
    retrieved_at: Attested<Timestamp>,
    integrity: Attested<Checksum>,
    licence: Attested<Licence>,
    transformations: Vec<Transformation>,
    observed: Vec<Observation>,
    derived_from: Option<Box<Provenance>>,
}

impl Provenance {
    #[must_use]
    pub fn acquired(origin: Origin, retrieved_at: Timestamp) -> Self {
        Self {
            origin,
            retrieved_at: Attested::Known(retrieved_at),
            integrity: Attested::Unknown,
            licence: Attested::Unknown,
            transformations: Vec::new(),
            observed: Vec::new(),
            derived_from: None,
        }
    }

    #[must_use]
    pub fn known_of(origin: Origin) -> Self {
        Self {
            origin,
            retrieved_at: Attested::Unknown,
            integrity: Attested::Unknown,
            licence: Attested::Unknown,
            transformations: Vec::new(),
            observed: Vec::new(),
            derived_from: None,
        }
    }

    #[must_use]
    pub fn with_integrity(mut self, checksum: Checksum) -> Self {
        self.integrity = Attested::Known(checksum);
        self
    }

    #[must_use]
    pub fn with_licence(mut self, licence: Licence) -> Self {
        self.licence = Attested::Known(licence);
        self
    }

    #[must_use]
    pub fn transformed(mut self, transformation: Transformation) -> Self {
        self.transformations.push(transformation);
        self
    }

    #[must_use]
    pub fn derived_from(mut self, source: Self) -> Self {
        self.derived_from = Some(Box::new(source));
        self
    }

    #[must_use]
    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    #[must_use]
    pub const fn retrieved_at(&self) -> &Attested<Timestamp> {
        &self.retrieved_at
    }

    #[must_use]
    pub const fn integrity(&self) -> &Attested<Checksum> {
        &self.integrity
    }

    #[must_use]
    pub const fn licence(&self) -> &Attested<Licence> {
        &self.licence
    }

    #[must_use]
    pub fn transformations(&self) -> &[Transformation] {
        &self.transformations
    }

    #[must_use]
    pub fn observed(mut self, observation: Observation) -> Self {
        self.observed.push(observation);
        self
    }

    #[must_use]
    pub fn observations(&self) -> &[Observation] {
        &self.observed
    }

    #[must_use]
    pub fn last_observation(&self) -> Option<&Observation> {
        self.observed.last()
    }

    #[must_use]
    pub fn source(&self) -> Option<&Self> {
        self.derived_from.as_deref()
    }

    pub fn chain(&self) -> impl Iterator<Item = &Self> {
        core::iter::successors(Some(self), |provenance| provenance.source())
    }

    #[must_use]
    pub fn traces_to_a_pinned_source(&self) -> bool {
        self.chain().any(|provenance| match provenance.origin() {
            Origin::Hub { revision, .. } => revision.is_known(),
            Origin::LocalFile { .. } | Origin::Unattributed => false,
        })
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.chain().count()
    }
}

#[cfg(test)]
mod tests;
