//! Order statistics, and the method that produces them.
//!
//! §3.4 makes uncertainty mandatory and does not name a summary, so the
//! summary is named here and the method is stated rather than assumed: a
//! percentile is meaningless without saying which of the several definitions
//! produced it, and two figures computed by different definitions are not
//! comparable (A8).

use core::fmt;

use super::Quantity;

/// A percentile, between 0 and 100 inclusive.
///
/// A newtype rather than a `u8` so that a rank cannot be passed where a sample
/// count was meant, and so that an out-of-range value has no representation
/// (§3.16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percentile(u8);

impl Percentile {
    /// The 5th percentile — the lower edge of the spread MCF reports.
    pub const P5: Self = Self(5);
    /// The 50th percentile.
    pub const MEDIAN: Self = Self(50);
    /// The 95th percentile — the upper edge.
    pub const P95: Self = Self(95);
    /// The 99th percentile — what every event-class budget is read at (D27).
    pub const P99: Self = Self(99);

    /// A percentile, if the rank is one.
    ///
    /// Returns `None` above 100. A7's habit: the absence is a variant rather
    /// than a clamp, because a caller that asked for the 120th percentile has
    /// a bug and silently answering with the maximum would hide it.
    #[must_use]
    pub const fn new(rank: u8) -> Option<Self> {
        if rank > 100 { None } else { Some(Self(rank)) }
    }

    /// The rank.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0
    }
}

impl fmt::Display for Percentile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

/// The spread of a measurement: five values that were actually observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread<Q: Quantity> {
    /// The smallest sample.
    pub minimum: Q,
    /// The 5th percentile.
    pub p5: Q,
    /// The median.
    pub median: Q,
    /// The 95th percentile.
    pub p95: Q,
    /// The largest sample.
    pub maximum: Q,
}

/// The sample at a percentile, by the nearest-rank method.
///
/// `rank = ceil(p / 100 × n)`, one-based, clamped into `1..=n`. The definition
/// is stated rather than assumed: a percentile computed by an interpolating
/// definition is a different number, and A8 refuses a comparison where more
/// than one thing differed.
///
/// `largest` is passed in rather than read off the end of the slice, which is
/// what makes this function total without a sentinel: [`Measurement`] knows its
/// extremes from its first two samples, and an index past the end of a clamped
/// rank means the largest sample — which is the right answer, not a fallback.
///
/// [`Measurement`]: super::Measurement
pub(super) fn at_percentile<Q: Quantity>(sorted: &[Q], percentile: Percentile, largest: Q) -> Q {
    let n = sorted.len();
    let rank = usize::from(percentile.rank())
        .saturating_mul(n)
        .div_ceil(100)
        .max(1)
        .min(n);
    sorted
        .get(rank.saturating_sub(1))
        .copied()
        .unwrap_or(largest)
}
