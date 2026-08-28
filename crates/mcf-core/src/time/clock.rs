//! The clocks, and what distinguishes them.
//!
//! D9: durations come from a monotonic clock, always, and the laboratory uses
//! a simulated one. Both are [`Clock`]s and neither is the wall clock, which
//! measures nothing and only names moments ([`Timestamp`]).
//!
//! [`Timestamp`]: super::Timestamp

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant as StdInstant;

use super::instant::Instant;

/// Which clock a reading came from.
///
/// A sealed-by-convention marker: the two implementors below are the only
/// ones, and adding a third is a decision about what MCF is willing to call a
/// duration rather than a convenience.
pub trait ClockKind: Copy + core::fmt::Debug + 'static {
    /// The name that appears wherever a duration is rendered.
    const NAME: &'static str;
    /// Whether readings are simulated. A11 turns on this.
    const IS_SIMULATED: bool;
}

/// The real, monotonic clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monotonic;

impl ClockKind for Monotonic {
    const NAME: &'static str = "monotonic";
    const IS_SIMULATED: bool = false;
}

/// A clock whose readings may be published as performance figures (A11,
/// B-082).
///
/// **The other half of the clock's being in the type.** [`ClockKind`] already
/// stops a simulated interval being compared with a real one; this stops one
/// being *written down* as a measurement. Every path that puts a duration into
/// the record is bounded by this trait, so encoding a simulated timing does not
/// compile — which is what B-082 asks for in place of a review comment.
///
/// It is implemented for [`Monotonic`] and for nothing else. [`Simulated`]
/// deliberately does not implement it: D9 gives the laboratory a clock so that
/// it can reproduce *behaviour* deterministically, and A11 forbids a
/// performance number originating there. A laboratory that wanted to publish a
/// timing would be a laboratory reporting how fast its own arithmetic is.
///
/// Adding an implementation is a decision about what MCF is willing to call a
/// measurement, which is why it is a trait somebody has to write rather than a
/// boolean somebody can set.
pub trait Measurable: ClockKind {}

impl Measurable for Monotonic {}

/// The laboratory's clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Simulated;

impl ClockKind for Simulated {
    const NAME: &'static str = "simulated";
    const IS_SIMULATED: bool = true;
}

/// Something that can be read for an instant.
pub trait Clock {
    /// Which clock this is.
    type Kind: ClockKind;

    /// Reads it.
    fn now(&self) -> Instant<Self::Kind>;
}

/// The machine's monotonic clock.
///
/// Readings are nanoseconds since the first read in this process, which is
/// deliberate: a monotonic reading has no calendar meaning, and giving it one
/// would invite exactly the subtraction B37 exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemClock;

/// The process's monotonic origin. Set once, on the first reading.
static ORIGIN: OnceLock<StdInstant> = OnceLock::new();

impl Clock for SystemClock {
    type Kind = Monotonic;

    fn now(&self) -> Instant<Monotonic> {
        let elapsed = ORIGIN.get_or_init(StdInstant::now).elapsed().as_nanos();
        // Saturating rather than truncating. `u64::MAX` nanoseconds is roughly
        // 584 years of process uptime; a truncating cast would wrap and report
        // a fresh process as ancient, which is a wrong number stated
        // confidently (P1).
        Instant::from_nanos(u64::try_from(elapsed).unwrap_or(u64::MAX))
    }
}

/// The laboratory's clock: it moves only when a scenario moves it.
///
/// B27 makes determinism a feature of the laboratory rather than of the world.
/// A scenario that waits for a second does not wait for a second; it says so,
/// and the second happens.
#[derive(Debug, Default)]
pub struct SimulatedClock {
    nanos: AtomicU64,
    kind: PhantomData<Simulated>,
}

impl SimulatedClock {
    /// A clock reading zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nanos: AtomicU64::new(0),
            kind: PhantomData,
        }
    }

    /// Moves the clock forward.
    ///
    /// Saturating, and forward only: there is no `rewind`. A backward step is
    /// a *wall-clock* anomaly (`time.jump.backward`) and belongs to the
    /// scenario that injects it, not to a monotonic reading — a monotonic
    /// clock that went backwards would not be one.
    pub fn advance(&self, nanos: u64) {
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }
}

impl Clock for SimulatedClock {
    type Kind = Simulated;

    fn now(&self) -> Instant<Simulated> {
        Instant::from_nanos(self.nanos.load(Ordering::Relaxed))
    }
}
