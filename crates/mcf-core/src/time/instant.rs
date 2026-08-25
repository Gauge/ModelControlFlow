//! A monotonic reading: ordered, subtractable, and meaningless as a date.
//!
//! An [`Instant`] is what a monotonic clock returns. It is comparable with
//! other readings from the same kind of clock and with nothing else, and it
//! has no calendar interpretation — which is the point. B37's violation,
//! `end_wall - start_wall`, is impossible here because a wall-clock reading is
//! a [`Timestamp`] and has no subtraction at all.
//!
//! [`Timestamp`]: super::Timestamp

use core::fmt;
use core::marker::PhantomData;

use super::clock::ClockKind;
use super::duration::Duration;

/// A reading from a monotonic or simulated clock.
#[derive(Debug)]
pub struct Instant<K: ClockKind> {
    nanos: u64,
    kind: PhantomData<K>,
}

// Written out rather than derived, for the reason given on `Duration`.
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
    /// A reading at a stated offset from the clock's origin.
    ///
    /// Constructed by the clock, and by scenarios that state a reading
    /// directly. It is not a way to fabricate an interval: the interval still
    /// has to be taken between two readings of the same clock kind.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self {
            nanos,
            kind: PhantomData,
        }
    }

    /// How long after `earlier` this reading is.
    ///
    /// Saturating at zero rather than signed or wrapping. A monotonic clock
    /// does not go backwards, so a negative interval means the two readings
    /// did not come from one clock — and returning `0` is the bounded wrong
    /// answer rather than an enormous one. When the laboratory can inject the
    /// case (B-009), it becomes `time.jump.backward` with a loud invalidation
    /// (D9) rather than a value at all.
    #[must_use]
    pub const fn saturating_duration_since(self, earlier: Self) -> Duration<K> {
        Duration::from_nanos(self.nanos.saturating_sub(earlier.nanos))
    }

    /// The reading, in nanoseconds from the clock's origin.
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
