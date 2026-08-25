//! An elapsed interval, and the clock it came from.
//!
//! A11 forbids a performance number originating in simulation. The way to make
//! that a compiler check rather than a review comment is for the clock to be
//! part of the *type*: `Duration<Monotonic>` and `Duration<Simulated>` are
//! different types, so a simulated interval cannot be compared with a real
//! one, cannot be stored where one is expected, and cannot be averaged into a
//! set of them (A8).

use core::fmt;
use core::marker::PhantomData;

use crate::measurement::Quantity;

use super::clock::ClockKind;

/// An elapsed interval in nanoseconds, tagged with the clock that produced it.
#[derive(Debug)]
pub struct Duration<K: ClockKind> {
    nanos: u64,
    kind: PhantomData<K>,
}

// Written out rather than derived. A derive would put a `K: Clone + Ord + …`
// bound on every use site, and the marker is never instantiated — comparing
// two durations compares two nanosecond counts that the type has already
// established came from the same clock.
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
    /// A zero interval.
    pub const ZERO: Self = Self::from_nanos(0);

    /// An interval of a stated number of nanoseconds.
    ///
    /// Available so that the laboratory can state a simulated interval and so
    /// that a test can state a known one. It is not a way around the clock: a
    /// `Duration<Monotonic>` built this way still says it came from the
    /// monotonic clock, and what makes that true is that nothing else can name
    /// `Monotonic` without having taken a reading.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            nanos,
            kind: PhantomData,
        }
    }

    /// The interval in nanoseconds.
    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// Whether this interval came from a simulated clock.
    ///
    /// Carried on the value rather than remembered by its caller, so that a
    /// result knows which clock produced it (D9) without anybody having to
    /// keep track.
    #[must_use]
    pub const fn is_simulated(self) -> bool {
        K::IS_SIMULATED
    }

    /// The sum of two intervals from the same clock.
    ///
    /// Saturating rather than wrapping: an interval that wrapped would be a
    /// small number where a huge one belonged, which is a wrong reading
    /// presented confidently (P1). `u64::MAX` nanoseconds is about 584 years,
    /// so reaching the saturation is itself a finding.
    #[must_use]
    pub const fn saturating_add(self, other: Self) -> Self {
        Self::from_nanos(self.nanos.saturating_add(other.nanos))
    }
}

impl<K: ClockKind> Quantity for Duration<K> {
    const UNIT: &'static str = "ns";
}

impl<K: ClockKind> fmt::Display for Duration<K> {
    /// Nanoseconds, and the clock, always. A6's habit: a bare interval is a
    /// number whose conditions were dropped, and *which clock* is the
    /// condition A11 turns on.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} ({})", self.nanos, Self::UNIT, K::NAME)
    }
}
