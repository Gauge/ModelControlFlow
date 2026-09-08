use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant as StdInstant;

use super::instant::Instant;

pub trait ClockKind: Copy + core::fmt::Debug + 'static {
    const NAME: &'static str;
    const IS_SIMULATED: bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monotonic;

impl ClockKind for Monotonic {
    const NAME: &'static str = "monotonic";
    const IS_SIMULATED: bool = false;
}

pub trait Measurable: ClockKind {}

impl Measurable for Monotonic {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Simulated;

impl ClockKind for Simulated {
    const NAME: &'static str = "simulated";
    const IS_SIMULATED: bool = true;
}

pub trait Clock {
    type Kind: ClockKind;

    fn now(&self) -> Instant<Self::Kind>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SystemClock;

static ORIGIN: OnceLock<StdInstant> = OnceLock::new();

impl Clock for SystemClock {
    type Kind = Monotonic;

    fn now(&self) -> Instant<Monotonic> {
        let elapsed = ORIGIN.get_or_init(StdInstant::now).elapsed().as_nanos();
        Instant::from_nanos(u64::try_from(elapsed).unwrap_or(u64::MAX))
    }
}

#[derive(Debug, Default)]
pub struct SimulatedClock {
    nanos: AtomicU64,
    kind: PhantomData<Simulated>,
}

impl SimulatedClock {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nanos: AtomicU64::new(0),
            kind: PhantomData,
        }
    }

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
