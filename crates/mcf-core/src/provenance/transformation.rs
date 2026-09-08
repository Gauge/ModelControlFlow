use core::fmt;

use crate::attested::Attested;
use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum TransformationKind {
    Quantization,
    Requantization,
    FormatConversion,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolIdentity {
    name: String,
    version: Attested<String>,
}

impl ToolIdentity {
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

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transformation {
    kind: TransformationKind,
    detail: Attested<String>,
    performed_by: Attested<ToolIdentity>,
    performed_at: Attested<Timestamp>,
}

impl Transformation {
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

    #[must_use]
    pub const fn kind(&self) -> &TransformationKind {
        &self.kind
    }

    #[must_use]
    pub const fn detail(&self) -> &Attested<String> {
        &self.detail
    }

    #[must_use]
    pub const fn performed_by(&self) -> &Attested<ToolIdentity> {
        &self.performed_by
    }

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
