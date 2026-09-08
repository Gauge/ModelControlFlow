mod calibrated;
mod placement;
mod sampling;

pub use calibrated::{Calibrated, Chosen};
pub use placement::Placement;
pub use sampling::{Sampling, Thousandths};

use core::fmt;

use crate::attested::Attested;
use crate::provenance::{Checksum, Repository, Revision};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weights {
    pub repository: Attested<Repository>,
    pub revision: Attested<Revision>,
    pub digest: Attested<Checksum>,
}

impl fmt::Display for Weights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{} ({})", self.repository, self.revision, self.digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantization(String);

impl Quantization {
    #[must_use]
    pub fn new(scheme: impl Into<String>) -> Self {
        Self(scheme.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Quantization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Engine {
    pub name: String,
    pub build: Attested<String>,
}

impl Engine {
    #[must_use]
    pub fn new(name: impl Into<String>, build: Option<String>) -> Self {
        Self {
            name: name.into(),
            build: match build {
                Some(build) => Attested::Known(build),
                None => Attested::Unknown,
            },
        }
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} build {}", self.name, self.build)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Configuration {
    pub weights: Weights,
    pub quantization: Attested<Quantization>,
    pub context_length: Attested<u32>,
    pub engine: Engine,
    pub placement: Placement,
    pub sampling: Sampling,
}

impl Configuration {
    #[must_use]
    pub fn is_the_same_thing_as(&self, other: &Self) -> bool {
        self == other
    }
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} · {} · ctx {} · {} · {} · {}",
            self.weights,
            self.quantization,
            self.context_length,
            self.engine,
            self.placement,
            self.sampling,
        )
    }
}

#[cfg(test)]
mod tests;
