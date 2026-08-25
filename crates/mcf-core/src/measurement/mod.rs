//! A number bound to the conditions that produced it.
//!
//! A6 is the rule this module is the enforcement of: *no number without its
//! conditions, its sample count and its spread*. B16 says to prefer the
//! machine-checked form, and §3.16 says to make illegal states unrepresentable
//! rather than to validate against them, so [`Measurement`] has no constructor
//! that omits a condition set and none that accepts fewer than two samples.
//! There is no `Measurement::new(value)`, no `Default`, and no `From<Q>`.
//!
//! Three properties are structural rather than checked:
//!
//! * **Uncertainty is mandatory** (§3.4). [`Measurement::of`] takes two
//!   samples before it takes any others, so a single-shot timing — which §3.4
//!   calls an anecdote — has no way to become a measurement. One trial is
//!   still a real thing and is kept (A4); it is simply not this type.
//! * **Trials are kept; summaries are derived** (B56). The samples are stored
//!   and every summary — minimum, median, percentile, maximum — is computed
//!   when it is asked for. Nothing stores a mean, because a stored mean is a
//!   question nobody can ask again.
//! * **An estimate is not one of these.** A20 keeps [`Estimate`] a separate
//!   type with no conversion in either direction, so a guess cannot be
//!   promoted into a measurement or compared with one — only replaced by one.
//! * **No NaN can enter.** [`Quantity`] requires [`Ord`], which rules out
//!   floating-point sample types; every summary MCF reports is an order
//!   statistic and needs ordering rather than arithmetic. See [`Quantity`].
//!
//! ```
//! use mcf_core::build_identity::BuildIdentity;
//! use mcf_core::measurement::{Bytes, Conditions, Floor, Measurement};
//!
//! let conditions = Conditions::new(BuildIdentity::current(), Floor::nothing_known());
//! let resident = Measurement::of(Bytes(7_900_000), Bytes(8_100_000), [Bytes(8_000_000)], conditions);
//!
//! assert_eq!(resident.n(), 3);
//! assert_eq!(resident.spread().median, Bytes(8_000_000));
//! assert_eq!(resident.spread().minimum, Bytes(7_900_000));
//! ```

mod conditions;
mod estimate;
mod quantity;
mod spread;

pub use crate::attested::Attested;
pub use conditions::{ConditionValue, Conditions, Floor};
pub use estimate::{Basis, Estimate};
pub use quantity::{Bytes, Count, PartsPerMillion, Quantity};
pub use spread::{Percentile, Spread};

use core::fmt;

/// A quantity, its samples, and the conditions they were taken under.
///
/// The first two samples are separate fields rather than the first two
/// elements of a vector. That is what makes "at least two" a property of the
/// type instead of a check: there is no empty measurement to guard against, so
/// every order statistic below is total and no summary needs a sentinel value
/// for a case that cannot arise (§3.16).
///
/// Cloning costs in proportion to the sample count, which is deliberate: B56
/// keeps the trials, and a type that were cheap to copy would be a type that
/// had thrown them away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement<Q: Quantity> {
    first: Q,
    second: Q,
    /// Everything after the first two, in the order it was taken. Order is
    /// information — it is what makes drift over a session visible — so
    /// nothing here is sorted in place.
    rest: Vec<Q>,
    conditions: Conditions,
}

impl<Q: Quantity> Measurement<Q> {
    /// Takes a measurement from at least two samples.
    ///
    /// The signature is the rule: two samples are positional arguments, so
    /// there is no way to reach this type with one. `rest` carries however
    /// many more there were, in the order they were taken.
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

    /// Takes a measurement from a collection of samples, if there are enough.
    ///
    /// Returns `None` for fewer than two. That is not a failure and carries no
    /// taxonomy category: one trial is a trial, it is kept like any other
    /// (A4), and the honest statement is that it is not a measurement — which
    /// is what the absent value says.
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

    /// The samples, in the order they were taken.
    pub fn samples(&self) -> impl Iterator<Item = Q> + '_ {
        [self.first, self.second]
            .into_iter()
            .chain(self.rest.iter().copied())
    }

    /// The sample count. Never below two.
    #[must_use]
    pub fn n(&self) -> usize {
        self.rest.len().saturating_add(2)
    }

    /// The smallest sample. Total: there is always a first one.
    #[must_use]
    pub fn minimum(&self) -> Q {
        self.samples().fold(self.first, Q::min)
    }

    /// The largest sample. Total, for the same reason.
    #[must_use]
    pub fn maximum(&self) -> Q {
        self.samples().fold(self.first, Q::max)
    }

    /// The conditions these samples were taken under.
    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }

    /// The samples, sorted.
    ///
    /// Computed on demand rather than stored, so the order they were taken in
    /// is never lost (B56).
    #[must_use]
    pub fn sorted(&self) -> Vec<Q> {
        let mut sorted: Vec<Q> = self.samples().collect();
        sorted.sort_unstable();
        sorted
    }

    /// The spread: five order statistics.
    ///
    /// A6 requires a spread and does not say which; these are the ones that
    /// survive a skewed distribution, which timing distributions always are.
    /// C4 prefers a number reproducible to ±10 % over one quotable to three
    /// decimals, and order statistics are what that preference looks like in
    /// code — a single slow trial moves the maximum and leaves the median
    /// where it was, which is the truth about what the machine usually does.
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

    /// The sample at a percentile, by the nearest-rank method.
    ///
    /// Nearest-rank rather than an interpolating definition because it returns
    /// a value that was actually observed. An interpolated percentile is a
    /// number no trial produced, and A20 would then have to decide whether it
    /// is a measurement or an estimate — a question the method avoids
    /// entirely.
    #[must_use]
    pub fn at(&self, percentile: Percentile) -> Q {
        spread::at_percentile(&self.sorted(), percentile, self.maximum())
    }
}

impl<Q: Quantity> fmt::Display for Measurement<Q> {
    /// The value, its sample count, its spread and its conditions — in that
    /// order, and never fewer.
    ///
    /// A6's violation is a surface that renders the number and drops the rest,
    /// so this type has no rendering that does. C1 keeps this a *rendering*:
    /// the record holds the samples and the conditions, and a surface that
    /// wants a different shape builds it from those rather than parsing this.
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

#[cfg(test)]
mod tests;
