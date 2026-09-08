use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Licence {
    Spdx(String),
    Stated,
}

impl Licence {
    #[must_use]
    pub fn spdx(identifier: impl Into<String>) -> Self {
        Self::Spdx(identifier.into())
    }

    #[must_use]
    pub const fn is_identified(&self) -> bool {
        matches!(self, Self::Spdx(_))
    }
}

impl fmt::Display for Licence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spdx(identifier) => f.write_str(identifier),
            Self::Stated => f.write_str("stated, unmatched"),
        }
    }
}
