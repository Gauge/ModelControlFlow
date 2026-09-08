use core::fmt;
use core::marker::PhantomData;

use super::clock::ClockKind;
use super::duration::Duration;

#[derive(Debug)]
pub struct Instant<K: ClockKind> {
    nanos: u64,
    kind: PhantomData<K>,
}

#[allow(clippy::expl_impl_clone_on_copy)]
impl<K: ClockKind> Clone for Instant<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: ClockKind> Copy for Instant<K> {}

impl<K: ClockKind> PartialEq for Instant<K> {
    fn eq(&self, other: &Self) -> bool {
        self.nanos == other.nanos
    }
}

impl<K: ClockKind> Eq for Instant<K> {}

impl<K: ClockKind> PartialOrd for Instant<K> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: ClockKind> Ord for Instant<K> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.nanos.cmp(&other.nanos)
    }
}

impl<K: ClockKind> Instant<K> {
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            nanos,
            kind: PhantomData,
        }
    }

    #[must_use]
    pub const fn saturating_duration_since(self, earlier: Self) -> Duration<K> {
        Duration::from_nanos(self.nanos.saturating_sub(earlier.nanos))
    }

    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }
}

impl<K: ClockKind> fmt::Display for Instant<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+{} ns ({})", self.nanos, K::NAME)
    }
}
