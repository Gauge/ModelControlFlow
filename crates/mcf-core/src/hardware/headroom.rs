//! Whether this machine had room to be measured on (B-217, DEC-007, F95,
//! §3.8, A21, B34).
//!
//! **The band, and where it came from.** DEC-007 settled that quiet is
//! relative — *a machine steady throughout a run is measurable wherever its
//! baseline sits* — and left the band open, insisting it be measured rather
//! than chosen. F95 measured it: the same paired comparison at rising
//! fractions of a thirty-two-thread machine, reading out the *width* of the
//! effect-size interval at a fixed six pairs. Width stayed inside the
//! machine's own quiet variability up to thirty percent of capacity and was
//! six to thirty times wider by fifty-one.
//!
//! **Half a machine free is not enough.** With sixteen of thirty-two threads
//! idle the interval was already six to thirty times wider than baseline.
//! Degradation does not wait for saturation, which is why this is a fraction
//! of capacity rather than a count of free cores.
//!
//! **The default is somebody else's measurement, and says so** (A21, B34).
//! Thirty percent was measured on one machine, one model pair, one token
//! budget. That makes it a *declared* figure here in exactly the sense F42's
//! declared context length is declared: better than a number chosen out of the
//! air, and not the same as a local measurement. A machine that runs
//! `prototypes/contention-band` for itself replaces it, which is A20's shape —
//! an estimate is replaced by a measurement, never promoted into one.
//!
//! **It does not refuse.** The operator's decision (2026-08-28): a run outside
//! the band happens, is recorded in full, and is marked as not fit to
//! contribute. A4 keeps what it produced and A1 keeps the record of it; what
//! it loses is the right to travel. Refusing to start would deny a result to
//! anyone whose machine is simply busy, and this project's whole answer to
//! that is to measure and mark rather than to gate.
//!
//! **Not an instrument:** it measures nothing. The competing load comes from
//! `contention` and the capacity from the platform; this divides one by the
//! other.

use core::fmt;

/// The fraction of a machine's capacity beyond which measurements widen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    /// The fraction, in parts per million of the machine's capacity.
    pub fraction: u64,
    /// Where the figure came from, so a reader can weigh it.
    pub measured_on: &'static str,
}

/// What F95 measured, which is the best figure MCF has and not a local one.
pub const MEASURED: Band = Band {
    fraction: 300_000,
    measured_on: "one 32-thread machine, one model pair, 32 tokens (F95); interval width stayed \
                  inside that machine's own quiet variability to 30% of capacity and was 6× to \
                  30× wider by 51%",
};

/// How much of this machine was already busy when a run happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headroom {
    /// What was competing, in thousandths of a processor.
    pub competing: u64,
    /// What the machine has, in thousandths of a processor.
    pub capacity: u64,
    /// The band this was judged against.
    pub band: Band,
}

impl Headroom {
    /// What was left, as this machine's own capacity was measured.
    #[must_use]
    pub fn taken(competing: u64) -> Self {
        let capacity = u64::try_from(std::thread::available_parallelism().map_or(1, Into::into))
            .unwrap_or(1)
            .saturating_mul(1_000);
        Self {
            competing,
            capacity,
            band: MEASURED,
        }
    }

    /// How much of the machine was taken, in parts per million.
    ///
    /// A fraction rather than a count, because F95 measured the departure at a
    /// *fraction* of capacity: sixteen busy threads is half of this machine
    /// and a sixteenth of a large one, and the two do not behave alike.
    #[must_use]
    pub fn fraction(&self) -> u64 {
        self.competing
            .saturating_mul(1_000_000)
            .checked_div(self.capacity.max(1))
            .unwrap_or(0)
    }

    /// Whether the machine had room for this to be a measurement worth
    /// contributing.
    #[must_use]
    pub fn within_band(&self) -> bool {
        self.fraction() <= self.band.fraction
    }
}

impl fmt::Display for Headroom {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let per_cent = |held: u64| format!("{}.{}%", held / 10_000, (held % 10_000) / 1_000);
        write!(
            form,
            "{} of this machine was already busy ({}.{:02} of {} core(s))",
            per_cent(self.fraction()),
            self.competing / 1_000,
            (self.competing % 1_000) / 10,
            self.capacity / 1_000
        )?;
        if self.within_band() {
            write!(
                form,
                " — inside the band of {}, so the machine had room",
                per_cent(self.band.fraction)
            )
        } else {
            write!(
                form,
                " — OUTSIDE the band of {}, so this is a real measurement that is not fit to \
                 contribute (B-217, DEC-007). The band is DECLARED from {}; \
                 `prototypes/contention-band` measures it here (A21, A20)",
                per_cent(self.band.fraction),
                self.band.measured_on
            )
        }
    }
}

#[cfg(test)]
mod tests;
