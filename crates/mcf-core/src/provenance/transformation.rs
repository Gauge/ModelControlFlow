//! What was done to an artifact after it was obtained.
//!
//! §3.6 asks for "every transformation since", and §XII explains why: the
//! reference model is a third-party requantization, so the interesting
//! artifacts are the ones several transformations away from the weights they
//! describe. A quantization is not an annotation on a file — it is a
//! measurement condition (§3.4), part of a configuration's identity (D17), and
//! the thing a reader most wants to know when two figures disagree.

use core::fmt;

use crate::attested::Attested;
use crate::time::Timestamp;

/// What kind of thing was done.
///
/// `Other` carries the operator's own words rather than forcing a fit. A7's
/// spirit: a transformation MCF has no name for is recorded as what it was
/// called, not filed under the nearest known kind — which would make the
/// record say something nobody claimed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TransformationKind {
    /// Weights reduced in precision.
    Quantization,
    /// Already-quantized weights taken to a different quantization — the case
    /// §XII calls the hard one, because the source of the source is where the
    /// chain usually breaks.
    Requantization,
    /// The same weights written in a different container format.
    FormatConversion,
    /// Something else, named by whoever did it.
    Other(String),
}

impl fmt::Display for TransformationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quantization => f.write_str("quantization"),
            Self::Requantization => f.write_str("requantization"),
            Self::FormatConversion => f.write_str("format conversion"),
            Self::Other(name) => f.write_str(name),
        }
    }
}

/// What performed a transformation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolIdentity {
    name: String,
    version: Attested<String>,
}

impl ToolIdentity {
    /// A tool, and its version if that is known.
    #[must_use]
    pub fn new(name: impl Into<String>, version: Option<String>) -> Self {
        Self {
            name: name.into(),
            version: match version {
                Some(version) => Attested::Known(version),
                None => Attested::Unknown,
            },
        }
    }

    /// The tool's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its version, if that is known.
    ///
    /// Frequently it is not, and that is worth stating rather than hiding: a
    /// quantization performed by an unnamed version of a tool is a
    /// reproducibility gap (P3), and a gap nobody can see is a gap nobody
    /// closes.
    #[must_use]
    pub const fn version(&self) -> &Attested<String> {
        &self.version
    }
}

impl fmt::Display for ToolIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}

/// One thing that was done to an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transformation {
    kind: TransformationKind,
    detail: Attested<String>,
    performed_by: Attested<ToolIdentity>,
    performed_at: Attested<Timestamp>,
}

impl Transformation {
    /// Records a transformation.
    ///
    /// Everything except the kind is [`Attested`], because for a third-party
    /// artifact almost none of it is knowable: the publisher's pipeline is not
    /// MCF's to interrogate. B-019's condition is that every field is either
    /// recorded or `Unknown`, and this signature is what makes the second
    /// option a state rather than an omission.
    ///
    /// [`Attested`]: crate::attested::Attested
    #[must_use]
    pub const fn new(
        kind: TransformationKind,
        detail: Attested<String>,
        performed_by: Attested<ToolIdentity>,
        performed_at: Attested<Timestamp>,
    ) -> Self {
        Self {
            kind,
            detail,
            performed_by,
            performed_at,
        }
    }

    /// What kind of thing was done.
    #[must_use]
    pub const fn kind(&self) -> &TransformationKind {
        &self.kind
    }

    /// The specifics — a quantization's scheme, a conversion's target format.
    #[must_use]
    pub const fn detail(&self) -> &Attested<String> {
        &self.detail
    }

    /// What performed it.
    #[must_use]
    pub const fn performed_by(&self) -> &Attested<ToolIdentity> {
        &self.performed_by
    }

    /// When.
    #[must_use]
    pub const fn performed_at(&self) -> &Attested<Timestamp> {
        &self.performed_at
    }
}

impl fmt::Display for Transformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) by {} at {}",
            self.kind, self.detail, self.performed_by, self.performed_at
        )
    }
}
