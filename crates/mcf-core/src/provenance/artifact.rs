//! The handle, which cannot exist without its provenance.
//!
//! B-006's condition: *an artifact handle cannot exist without provenance*.
//! The enforcement is the absence of alternatives — one constructor, which
//! takes a [`Provenance`]; a private field with no setter; no `Default`, no
//! `From<ArtifactName>`, and no way to swap the provenance out afterwards.
//!
//! The last of those is the one worth arguing. A mutable provenance would make
//! an artifact's origin a claim that the current holder can restate, and §3.6
//! wants a record. Everything that *adds* to a provenance — a verified
//! checksum, a licence that was read, a transformation — goes through
//! [`Artifact::amend`], which takes the existing provenance and returns a new
//! one, so an amendment is written as what it is rather than as an assignment.

use core::fmt;

use super::Provenance;

/// What an artifact is called on this machine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactName(String);

impl ArtifactName {
    /// Names an artifact.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An artifact, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    name: ArtifactName,
    provenance: Provenance,
}

impl Artifact {
    /// An artifact with its provenance.
    #[must_use]
    pub const fn new(name: ArtifactName, provenance: Provenance) -> Self {
        Self { name, provenance }
    }

    /// What it is called.
    #[must_use]
    pub const fn name(&self) -> &ArtifactName {
        &self.name
    }

    /// Where it came from.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Adds to what is known about where it came from.
    ///
    /// The amendment is a function of the existing provenance, so there is no
    /// way to write one that discards it. A1: never lose information.
    #[must_use]
    pub fn amend(self, amendment: impl FnOnce(Provenance) -> Provenance) -> Self {
        Self {
            provenance: amendment(self.provenance),
            name: self.name,
        }
    }
}

impl fmt::Display for Artifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} from {}", self.name, self.provenance.origin())
    }
}
