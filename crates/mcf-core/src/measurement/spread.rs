use core::fmt;

use super::Quantity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percentile(u8);

impl Percentile {
    pub const P5: Self = Self(5);
    pub const MEDIAN: Self = Self(50);
    pub const P95: Self = Self(95);
    pub const P99: Self = Self(99);

    #[must_use]
    pub const fn new(rank: u8) -> Option<Self> {
        if rank > 100 { None } else { Some(Self(rank)) }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread<Q: Quantity> {
    pub minimum: Q,
    pub p5: Q,
    pub median: Q,
    pub p95: Q,
    pub maximum: Q,
}

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
