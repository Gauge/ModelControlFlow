use core::fmt;

use super::Quantity;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Basis {
    LocalHistory,
    Corpus { reports: usize },
    VendorModel,
    Declared,
}

impl fmt::Display for Basis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalHistory => f.write_str("local history"),
            Self::Corpus { reports } => write!(f, "corpus, n={reports}"),
            Self::VendorModel => f.write_str("vendor model"),
            Self::Declared => f.write_str("declared"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate<Q: Quantity> {
    low: Q,
    high: Q,
    basis: Basis,
}

impl<Q: Quantity> Estimate<Q> {
    #[must_use]
    pub fn band(first: Q, second: Q, basis: Basis) -> Self {
        Self {
            low: first.min(second),
            high: first.max(second),
            basis,
        }
    }

    #[must_use]
    pub fn point(value: Q, basis: Basis) -> Self {
        Self {
            low: value,
            high: value,
            basis,
        }
    }

    #[must_use]
    pub const fn low(&self) -> Q {
        self.low
    }

    #[must_use]
    pub const fn high(&self) -> Q {
        self.high
    }

    #[must_use]
    pub const fn basis(&self) -> &Basis {
        &self.basis
    }

    #[must_use]
    pub fn is_point(&self) -> bool {
        self.low == self.high
    }
}

impl<Q: Quantity> fmt::Display for Estimate<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_point() {
            write!(f, "~{} (estimate: {})", self.low, self.basis)
        } else {
            write!(f, "{}–{} (estimate: {})", self.low, self.high, self.basis)
        }
    }
}
