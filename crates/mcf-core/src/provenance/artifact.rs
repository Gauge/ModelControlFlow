use core::fmt;

use super::Provenance;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactName(String);

impl ArtifactName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    name: ArtifactName,
    provenance: Provenance,
}

impl Artifact {
    #[must_use]
    pub const fn new(name: ArtifactName, provenance: Provenance) -> Self {
        Self { name, provenance }
    }

    #[must_use]
    pub const fn name(&self) -> &ArtifactName {
        &self.name
    }

    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

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
