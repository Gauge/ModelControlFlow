use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Attribution {
    Mcf,
    Managed,
    Machine,
    Hub,
    Artifact,
    User,
    ModelUnderTest,
    Unattributable,
}

use super::category::Branch;

impl Attribution {
    pub const ALL: [Self; 8] = [
        Self::Mcf,
        Self::Managed,
        Self::Machine,
        Self::Hub,
        Self::Artifact,
        Self::User,
        Self::ModelUnderTest,
        Self::Unattributable,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mcf => "mcf",
            Self::Managed => "managed",
            Self::Machine => "machine",
            Self::Hub => "hub",
            Self::Artifact => "artifact",
            Self::User => "user",
            Self::ModelUnderTest => "model-under-test",
            Self::Unattributable => "unattributable",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == value)
    }
}

impl fmt::Display for Attribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Disposition {
    Refused,
    Degraded,
    Partial,
    Recovered,
    Aborted,
    Invalidated,
}

impl Disposition {
    pub const ALL: [Self; 6] = [
        Self::Refused,
        Self::Degraded,
        Self::Partial,
        Self::Recovered,
        Self::Aborted,
        Self::Invalidated,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Degraded => "degraded",
            Self::Partial => "partial",
            Self::Recovered => "recovered",
            Self::Aborted => "aborted",
            Self::Invalidated => "invalidated",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.as_str() == value)
    }
}

impl fmt::Display for Disposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Attribution {
    #[must_use]
    pub const fn branch(self) -> Option<Branch> {
        match self {
            Self::ModelUnderTest => Some(Branch::TheModel),
            Self::Machine | Self::Hub => Some(Branch::TheEnvironment),
            Self::Artifact => Some(Branch::TheArtifact),
            Self::Mcf | Self::Managed | Self::User => Some(Branch::McfItself),
            Self::Unattributable => None,
        }
    }
}
