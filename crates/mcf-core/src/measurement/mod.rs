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

/// A statistic that cannot be rendered without its sample count and its spread
/// (A6, B-073, §3.4).
///
/// **The hole this closes.** [`Measurement`] has no rendering that drops its
/// conditions — but a surface never had to use it. It could ask for
/// [`Measurement::at`] or [`Measurement::maximum`], get a bare `Q`, and print
/// that: `mcf doctor` did exactly this, reporting a p99 with its sample count
/// and no spread at all. A6's *no number without its conditions, its sample
/// count and its spread* was held by the type for the whole measurement and by
/// nobody for the statistic taken out of it.
///
/// So a statistic leaves a measurement in this, which carries the rest with it.
/// The bare value is reachable — arithmetic needs it — through
/// [`Stated::value`], named for what calling it does, and a surface that
/// formats *that* into a line is a surface a check can find (B-073).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stated<Q: Quantity> {
    /// Which statistic this is, in the word a reader knows it by.
    what: &'static str,
    value: Q,
    n: usize,
    spread: Spread<Q>,
}

impl<Q: Quantity> Stated<Q> {
    /// States a statistic with everything A6 requires beside it.
    #[must_use]
    pub fn new(what: &'static str, value: Q, measured: &Measurement<Q>) -> Self {
        Self {
            what,
            value,
            n: measured.n(),
            spread: measured.spread(),
        }
    }

    /// The bare number, for arithmetic.
    ///
    /// Named for what it does, the way [`crate::touchstone::Touchstone::bare`]
    /// and `Content::disclose` are: comparing a statistic with a ceiling needs
    /// the number and nothing else, and that is a legitimate call. Formatting
    /// it into a line is not, and is what `checks/tests/a_number_carries_its_conditions.rs`
    /// looks for.
    #[must_use]
    pub const fn value(&self) -> Q {
        self.value
    }

    /// How many samples it was taken from.
    #[must_use]
    pub const fn n(&self) -> usize {
        self.n
    }

    /// The spread it came out of.
    #[must_use]
    pub const fn spread(&self) -> &Spread<Q> {
        &self.spread
    }
}

impl<Q: Quantity> fmt::Display for Stated<Q> {
    /// The statistic, what it is, how many samples, and the spread — never
    /// fewer. The conditions belong to the [`Measurement`] and are rendered
    /// with it; what this closes is the *statistic* leaving without its own
    /// evidence.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} (n={}, median {}, p5–p95 {}–{}, min {}, max {})",
            self.what,
            self.value,
            self.n,
            // The median beside the statistic, because D27's reason for
            // reading an event-class figure at the 99th percentile is that the
            // *gap* between the two is what a busy machine looks like. Dropping
            // it hides the reason a reading may not be usable — which is what
            // the first version of this rendering did, and what
            // `an_event_class_figure_is_reported_at_the_percentile_d27_names`
            // caught.
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
