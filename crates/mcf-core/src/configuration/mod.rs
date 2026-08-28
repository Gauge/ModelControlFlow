//! What the hosting system needs in order to run a model — and nothing about
//! the machine it runs on.
//!
//! D17: *a configuration is what the hosting system needs in order to run a
//! model. Weights and revision, quantization, context length, runtime,
//! sampling parameters. That set is the identity: two runs share an identity
//! when they share that description.* B57 is the rule, and B-272 is the item:
//! the identity type excludes hardware **by construction**.
//!
//! **Why the exclusion is structural and not a convention.** Hardware in the
//! key would give every machine its own universe of configurations, and §XIV's
//! corpus would have nothing to aggregate. With hardware as a *condition*
//! (§3.4), the same configuration measured on two machines is one thing
//! observed twice, and hardware becomes the axis those observations are
//! analysed *along*. There is no field here to put a machine in, so the
//! mistake has no spelling.
//!
//! **Grouping is a view, never the key.** Nothing in this module collapses two
//! configurations into one. Collapsing — by model family, by quantization
//! class, by whatever a question needs — happens where the question is asked,
//! because D17's test is that a group can be widened and never narrowed.
//!
//! **What is deliberately here and might not look like identity.**
//!
//! * *Sampling parameters* (D18). A model cannot run without them, they change
//!   behaviour profoundly, and §XV cannot reproduce behaviour without them.
//! * *Engine build* (intent v16). An engine that changes silently colours every
//!   measurement taken after it; discovering that a corpus mixed two engine
//!   builds is unrecoverable, whereas grouping-as-a-view recovers what a
//!   question needs.
//! * *Declared placement* (intent v16). The intent belongs to the
//!   configuration; the layout that actually resulted is a condition, and
//!   divergence between them is a finding — it is how a configuration visibly
//!   fails to transfer.
//!
//! **And what is deliberately not.** The seed set is a condition, not identity
//! (D19): sampling parameters change the distribution, and a seed only draws
//! from it.
//!
//! **Every value is exact.** Temperature and top-p are carried as thousandths
//! in [`Thousandths`] rather than as floating point, because identity is an
//! equality question and floating-point equality is the wrong tool for one —
//! `0.7` is not a value a float holds exactly, and two configurations that
//! should be the same must not depend on how each was parsed.

mod calibrated;
mod placement;
mod sampling;

pub use calibrated::{Calibrated, Chosen};
pub use placement::Placement;
pub use sampling::{Sampling, Thousandths};

use core::fmt;

use crate::attested::Attested;
use crate::provenance::{Checksum, Repository, Revision};

/// Which weights, as precisely as MCF can say.
///
/// D17's test is to err toward *more* in the identity, because a group can be
/// widened and never narrowed — so the digest is here alongside the repository
/// and revision, even though the revision usually determines it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weights {
    /// Where the weights came from.
    pub repository: Attested<Repository>,
    /// The revision pinned at acquisition.
    pub revision: Attested<Revision>,
    /// The digest of the bytes that were actually loaded.
    pub digest: Attested<Checksum>,
}

impl fmt::Display for Weights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{} ({})", self.repository, self.revision, self.digest)
    }
}

/// The quantization scheme the weights are in, as the artifact names it.
///
/// Kept as written rather than parsed into a lattice of precisions: the
/// schemes are the publishers' vocabulary, they are not ordered, and inventing
/// an ordering would let MCF compare two of them and mean nothing by it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantization(String);

impl Quantization {
    /// A quantization scheme.
    #[must_use]
    pub fn new(scheme: impl Into<String>) -> Self {
        Self(scheme.into())
    }

    /// The scheme, as written.
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

/// Which engine, at which build.
///
/// The build is separate from the name because that is the part that changes
/// silently. Two runs through "the same engine" at different builds are two
/// configurations (intent v16).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Engine {
    /// The engine's name.
    pub name: String,
    /// The build MCF shipped, as precisely as the build system can say.
    pub build: Attested<String>,
}

impl Engine {
    /// An engine at a build.
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

/// The complete description of a runnable thing.
///
/// There is no hardware field, no machine identifier, no accelerator and no
/// host name — and no constructor that would accept one. That absence is
/// B-272's condition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Configuration {
    /// Which weights.
    pub weights: Weights,
    /// In which quantization.
    pub quantization: Attested<Quantization>,
    /// At what context length, in tokens.
    pub context_length: Attested<u32>,
    /// Through which engine, at which build.
    pub engine: Engine,
    /// Where the operator asked for it to be placed.
    pub placement: Placement,
    /// With which sampling parameters.
    pub sampling: Sampling,
}

impl Configuration {
    /// Whether two configurations are the same runnable thing.
    ///
    /// Exactly `PartialEq`, stated as a method so that call sites read as the
    /// question they are asking. Note what it does *not* consult: nothing here
    /// can see a machine, so this answers the same way wherever it is asked —
    /// which is what makes §XIV's corpus aggregable.
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
