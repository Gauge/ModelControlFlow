use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attested<T> {
    Known(T),
    Unknown,
}

impl<T> Attested<T> {
    pub const fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown => None,
        }
    }

    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Attested<U> {
        match self {
            Self::Known(value) => Attested::Known(f(value)),
            Self::Unknown => Attested::Unknown,
        }
    }
}

#[allow(
    clippy::derivable_impls,
    reason = "the manual implementation is kept deliberately"
)]
impl<T> Default for Attested<T> {
    fn default() -> Self {
        Self::Unknown
    }
}

impl<T: fmt::Display> fmt::Display for Attested<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(value) => value.fmt(f),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}
