use core::fmt;
use core::marker::PhantomData;

use crate::measurement::Quantity;

use super::clock::ClockKind;

#[derive(Debug)]
pub struct Duration<K: ClockKind> {
    nanos: u64,
    kind: PhantomData<K>,
}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<K: ClockKind> Clone for Duration<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: ClockKind> Copy for Duration<K> {}

impl<K: ClockKind> PartialEq for Duration<K> {
    fn eq(&self, other: &Self) -> bool {
        self.nanos == other.nanos
    }
}

impl<K: ClockKind> Eq for Duration<K> {}

impl<K: ClockKind> PartialOrd for Duration<K> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: ClockKind> Ord for Duration<K> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.nanos.cmp(&other.nanos)
    }
}

impl<K: ClockKind> core::hash::Hash for Duration<K> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.nanos.hash(state);
    }
}

impl<K: ClockKind> Duration<K> {
    pub const ZERO: Self = Self::from_nanos(0);

    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            nanos,
            kind: PhantomData,
        }
    }

    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    #[must_use]
    pub const fn is_simulated(self) -> bool {
        K::IS_SIMULATED
    }

    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self::from_nanos(self.nanos.saturating_add(other.nanos))
    }
}

impl<K: ClockKind> Quantity for Duration<K> {
    const UNIT: &'static str = "ns";
}

impl<K: ClockKind> fmt::Display for Duration<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} ({})", self.nanos, Self::UNIT, K::NAME)
    }
}
