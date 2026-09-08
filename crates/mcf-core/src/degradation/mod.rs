use core::fmt;

use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degradation {
    causes: Vec<Failure>,
}

impl Degradation {
    #[must_use]
    pub fn because(
        category: Category,
        attribution: Attribution,
        subsystem: Subsystem,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            causes: vec![Failure::new(
                category,
                attribution,
                Disposition::Degraded,
                subsystem,
                detail,
            )],
        }
    }

    #[must_use]
    pub fn from_failure(failure: Failure) -> Option<Self> {
        if failure.disposition() == Disposition::Degraded {
            Some(Self {
                causes: vec![failure],
            })
        } else {
            None
        }
    }

    #[must_use]
    pub fn causes(&self) -> &[Failure] {
        &self.causes
    }

    #[must_use]
    pub fn and(mut self, other: Self) -> Self {
        self.causes.extend(other.causes);
        self
    }
}

impl fmt::Display for Degradation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DEGRADED")?;
        for cause in &self.causes {
            write!(f, " — {cause}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degraded<T> {
    value: T,
    degradation: Degradation,
}

impl<T> Degraded<T> {
    #[must_use]
    pub const fn new(value: T, degradation: Degradation) -> Self {
        Self { value, degradation }
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    #[must_use]
    pub const fn degradation(&self) -> &Degradation {
        &self.degradation
    }

    #[must_use]
    pub fn into_parts(self) -> (T, Degradation) {
        (self.value, self.degradation)
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Degraded<U> {
        Degraded {
            value: f(self.value),
            degradation: self.degradation,
        }
    }

    #[must_use]
    pub fn zip<U>(self, other: Degraded<U>) -> Degraded<(T, U)> {
        Degraded {
            value: (self.value, other.value),
            degradation: self.degradation.and(other.degradation),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Degraded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.value, self.degradation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeDegraded<T> {
    Full(T),
    Reduced(Degraded<T>),
}

impl<T> MaybeDegraded<T> {
    #[must_use]
    pub const fn is_degraded(&self) -> bool {
        matches!(self, Self::Reduced(_))
    }

    #[must_use]
    pub const fn degradation(&self) -> Option<&Degradation> {
        match self {
            Self::Full(_) => None,
            Self::Reduced(degraded) => Some(degraded.degradation()),
        }
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        match self {
            Self::Full(value) => value,
            Self::Reduced(degraded) => degraded.value(),
        }
    }

    #[must_use]
    pub fn degrade(self, degradation: Degradation) -> Degraded<T> {
        match self {
            Self::Full(value) => Degraded::new(value, degradation),
            Self::Reduced(degraded) => {
                let (value, existing) = degraded.into_parts();
                Degraded::new(value, existing.and(degradation))
            }
        }
    }
}

impl<T: fmt::Display> fmt::Display for MaybeDegraded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(value) => value.fmt(f),
            Self::Reduced(degraded) => degraded.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests;
