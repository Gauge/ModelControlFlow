mod conditions;
mod estimate;
mod isolation;
mod quantity;
mod spread;

pub use crate::attested::Attested;
pub use conditions::{ConditionValue, Conditions, Floor};
pub use estimate::{Basis, Estimate};
pub use isolation::{INSTRUMENT, Isolation};
pub use quantity::{Bytes, Count, PartsPerMillion, Quantity};
pub use spread::{Percentile, Spread};

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement<Q: Quantity> {
    first: Q,
    second: Q,
    rest: Vec<Q>,
    conditions: Conditions,
}

impl<Q: Quantity> Measurement<Q> {
    #[must_use]
    pub fn of(
        first: Q,
        second: Q,
        rest: impl IntoIterator<Item = Q>,
        conditions: Conditions,
    ) -> Self {
        Self {
            first,
            second,
            rest: rest.into_iter().collect(),
            conditions,
        }
    }

    #[must_use]
    pub fn from_samples(
        samples: impl IntoIterator<Item = Q>,
        conditions: Conditions,
    ) -> Option<Self> {
        let mut samples = samples.into_iter();
        let first = samples.next()?;
        let second = samples.next()?;
        Some(Self::of(first, second, samples, conditions))
    }

    pub fn samples(&self) -> impl Iterator<Item = Q> + '_ {
        [self.first, self.second]
            .into_iter()
            .chain(self.rest.iter().copied())
    }

    #[must_use]
    pub fn n(&self) -> usize {
        self.rest.len().saturating_add(2)
    }

    #[must_use]
    pub fn minimum(&self) -> Q {
        self.samples().fold(self.first, Q::min)
    }

    #[must_use]
    pub fn maximum(&self) -> Q {
        self.samples().fold(self.first, Q::max)
    }

    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }

    #[must_use]
    pub fn sorted(&self) -> Vec<Q> {
        let mut sorted: Vec<Q> = self.samples().collect();
        sorted.sort_unstable();
        sorted
    }

    #[must_use]
    pub fn spread(&self) -> Spread<Q> {
        let sorted = self.sorted();
        let maximum = self.maximum();
        Spread {
            minimum: self.minimum(),
            p5: spread::at_percentile(&sorted, Percentile::P5, maximum),
            median: spread::at_percentile(&sorted, Percentile::MEDIAN, maximum),
            p95: spread::at_percentile(&sorted, Percentile::P95, maximum),
            maximum,
        }
    }

    #[must_use]
    pub fn at(&self, percentile: Percentile) -> Q {
        spread::at_percentile(&self.sorted(), percentile, self.maximum())
    }
}

impl<Q: Quantity> fmt::Display for Measurement<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let spread = self.spread();
        write!(
            f,
            "{} (n={}, p5–p95 {}–{}, min {}, max {}) under [{}]",
            spread.median,
            self.n(),
            spread.p5,
            spread.p95,
            spread.minimum,
            spread.maximum,
            self.conditions,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stated<Q: Quantity> {
    what: &'static str,
    value: Q,
    n: usize,
    spread: Spread<Q>,
}

impl<Q: Quantity> Stated<Q> {
    #[must_use]
    pub fn new(what: &'static str, value: Q, measured: &Measurement<Q>) -> Self {
        Self {
            what,
            value,
            n: measured.n(),
            spread: measured.spread(),
        }
    }

    #[must_use]
    pub const fn value(&self) -> Q {
        self.value
    }

    #[must_use]
    pub const fn n(&self) -> usize {
        self.n
    }

    #[must_use]
    pub const fn spread(&self) -> &Spread<Q> {
        &self.spread
    }
}

impl<Q: Quantity> fmt::Display for Stated<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} (n={}, median {}, p5–p95 {}–{}, min {}, max {})",
            self.what,
            self.value,
            self.n,
            self.spread.median,
            self.spread.p5,
            self.spread.p95,
            self.spread.minimum,
            self.spread.maximum
        )
    }
}

#[cfg(test)]
mod tests;
