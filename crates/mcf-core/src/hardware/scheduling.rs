//! Whether *this reading* was affected, rather than whether the machine is
//! busy.
//!
//! B24 makes **unattributable** a verdict: MCF knows the difference between
//! "this model is slow" and "this machine was busy", and says so when it
//! cannot tell. B35 holds that a timing taken under contention measures the
//! contention. D27 applies both to MCF's own budgets, and D30 settles *how* the
//! question is asked.
//!
//! **The question is about the measurement, not about the machine.** The
//! kernel accounts, per task, how long it was *runnable but not running* —
//! waiting for a processor. Read before and after a measurement, the difference
//! is how much of that measurement was MCF queuing rather than working. That is
//! the contamination, measured directly, on the same time scale as the
//! measurement.
//!
//! **Why not the load average**, which is what MCF tried first: F3 in
//! `doc/findings.md` records the experiment. A hundred cold starts on a quiet
//! machine and the same hundred under thirty-two spinning processes moved the
//! p99 by a factor of thirteen — and the one-minute load average read **0.29 in
//! both cases**, because a one-minute average cannot answer a question about a
//! 140-millisecond measurement. It is not a coarse signal; it is a signal on
//! the wrong time scale. The scheduling delay moved from 0.019 % to 11.1 % over
//! the same two runs.
//!
//! The load average is still read and still recorded as a condition (§3.4) —
//! it says something true about the machine — but it decides nothing.
//!
//! **The second signal, and the blind spot it closes.** A thread's scheduling
//! answers *was this thread queuing*, and for a measurement whose work happens
//! in a child process the answer is always no: the measuring thread is blocked
//! in `wait`, not runnable. F5 records what that missed — a cold-start figure
//! two and a half times its ceiling, judged clean, because the time was spent
//! faulting the artifact's pages in from a slow filesystem. So a measurement is
//! also judged by [`children_major_faults`]: a major fault is the kernel going
//! to a device, a warm local artifact takes none, and a reading that took any
//! is a reading of the device (B-193).
//!
//! [`children_major_faults`]: super::children_major_faults

use core::fmt;

use crate::attested::Attested;
use crate::time::{Clock as _, Duration, Instant, Monotonic, SystemClock};

/// What the kernel has accounted to this task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheduling {
    /// Nanoseconds spent running on a processor.
    pub on_cpu: u64,
    /// Nanoseconds spent runnable and waiting for one.
    pub waiting: u64,
}

/// Reads the kernel's accounting for the calling **thread**.
///
/// `Unknown` where the platform does not publish it (A7). D29 makes that a
/// per-platform state rather than a failure: a platform that cannot answer this
/// question cannot assert a timing budget, and says which capability it lacks.
///
/// **The thread, not the process, and the difference is not a detail.** A
/// process's accounting is its *main thread's*, so a measurement taken on any
/// other thread — which is every measurement taken under a test harness — reads
/// as perfectly clean for ever. F3 records the experiment: over the same
/// 300 ms of work on a worker thread, the process's accounting said 0.0000 % on
/// a quiet machine and 0.0000 % under thirty-two spinning processes, while the
/// thread's said 0.0207 % and 50.46 %. A signal that cannot fail is not a
/// signal, and this one silently could not.
#[must_use]
pub fn scheduling() -> Attested<Scheduling> {
    // Linux publishes three numbers: time on the processor, time waiting for
    // one, and timeslices run. The second is the one this module is about.
    let Ok(text) = std::fs::read_to_string("/proc/thread-self/schedstat") else {
        return Attested::Unknown;
    };
    let mut fields = text.split_whitespace();
    let (Some(on_cpu), Some(waiting)) = (fields.next(), fields.next()) else {
        return Attested::Unknown;
    };
    match (on_cpu.parse(), waiting.parse()) {
        (Ok(on_cpu), Ok(waiting)) => Attested::Known(Scheduling { on_cpu, waiting }),
        _ => Attested::Unknown,
    }
}

/// How much of a measurement may be MCF waiting for a processor before the
/// reading stops being about MCF.
///
/// One part in a hundred, in parts per million so the arithmetic stays
/// integral. The number is a judgement and is stated rather than hidden, and
/// F3 is the evidence it was chosen against: a quiet machine measured 190 ppm
/// and a loaded one 111 000 ppm over the same work — three orders of magnitude
/// apart, with this threshold two orders above the first and one below the
/// second. Below it, at most one hundredth of a measured interval is queuing,
/// which cannot move a p99 by the factors F1 and F2 observed; above it, the
/// contention is in the reading and the reading is about the contention.
pub const TOLERATED_DELAY_PPM: u64 = 10_000;

/// Whether a reading can be attributed to what it was measuring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attributability {
    /// MCF was not waiting for a processor to any degree that matters.
    Attributable {
        /// What fraction of the measurement was queuing, in parts per million.
        delay_ppm: u64,
    },
    /// It was, and this is how much.
    ///
    /// B24: a verdict, not a gap. D27 makes a budget run marked this way
    /// neither a pass nor a failure.
    Unattributable {
        /// What fraction of the measurement was queuing, in parts per million.
        delay_ppm: u64,
        /// What it would have had to be below.
        tolerated_ppm: u64,
    },
    /// The measurement went to a device for bytes, so what it measured includes
    /// the storage those bytes are on.
    ///
    /// A separate verdict from [`Attributability::Unattributable`] because it
    /// is a different fact with a different remedy: the machine was not busy,
    /// the artifact was not resident, and the number is about the filesystem
    /// (F5, B-193). Neither is a pass.
    Storage {
        /// How many major page faults the measured work took.
        major_faults: u64,
        /// What fraction of the measurement was queuing, for completeness.
        delay_ppm: u64,
    },
    /// The platform does not account for it, so MCF cannot say either way (A7).
    Unknown,
}

impl Attributability {
    /// Whether a figure measured under this may be asserted against a ceiling.
    ///
    /// Only [`Attributability::Attributable`]. Unknown is not permission: A7
    /// forbids reading an absent value as a favourable one.
    #[must_use]
    pub const fn permits_assertion(&self) -> bool {
        matches!(self, Self::Attributable { .. })
    }

    /// The fraction of the measurement spent queuing, if it is known.
    #[must_use]
    pub const fn delay_ppm(&self) -> Option<u64> {
        match self {
            Self::Attributable { delay_ppm }
            | Self::Unattributable { delay_ppm, .. }
            | Self::Storage { delay_ppm, .. } => Some(*delay_ppm),
            Self::Unknown => None,
        }
    }

    /// How many major page faults the measured work took, where that is known.
    #[must_use]
    pub const fn major_faults(&self) -> Option<u64> {
        match self {
            Self::Storage { major_faults, .. } => Some(*major_faults),
            Self::Attributable { .. } | Self::Unattributable { .. } | Self::Unknown => None,
        }
    }
}

impl fmt::Display for Attributability {
    // Integer division is the conversion to a percentage for display; both
    // operands are bounded and neither can overflow or lose a sign.
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attributable { delay_ppm } => write!(
                f,
                "attributable ({}.{:04} % of the measurement was queuing)",
                delay_ppm / 10_000,
                delay_ppm % 10_000
            ),
            Self::Unattributable {
                delay_ppm,
                tolerated_ppm,
            } => write!(
                f,
                "UNATTRIBUTABLE ({}.{:04} % of the measurement was queuing, tolerated \
                 below {}.{:04} %)",
                delay_ppm / 10_000,
                delay_ppm % 10_000,
                tolerated_ppm / 10_000,
                tolerated_ppm % 10_000
            ),
            Self::Storage {
                major_faults,
                delay_ppm,
            } => write!(
                f,
                "UNATTRIBUTABLE (the measured work took {major_faults} major page faults, so \
                 this reading is of the storage; {}.{:04} % of it was queuing)",
                delay_ppm / 10_000,
                delay_ppm % 10_000
            ),
            Self::Unknown => {
                f.write_str("attributability unknown — this platform does not account for it")
            }
        }
    }
}

/// Watches a measurement, so that afterwards MCF can say whether it was about
/// MCF.
///
/// A guard rather than two loose readings: the two have to bracket the *same*
/// interval, and a pair taken at unrelated moments is two facts that cannot be
/// compared. Starting one and never finishing it costs nothing.
#[derive(Debug)]
pub struct Watch {
    started: Attested<Scheduling>,
    faults_at_start: Attested<u64>,
    at: Instant<Monotonic>,
}

impl Default for Watch {
    fn default() -> Self {
        Self::start()
    }
}

impl Watch {
    /// Begins watching.
    #[must_use]
    pub fn start() -> Self {
        Self {
            started: scheduling(),
            faults_at_start: super::children_major_faults(),
            at: SystemClock.now(),
        }
    }

    /// Stops, and says whether what happened in between was about MCF.
    #[must_use]
    pub fn finish(self) -> Attributability {
        let elapsed = SystemClock.now().saturating_duration_since(self.at);
        let faults = match (self.faults_at_start, super::children_major_faults()) {
            (Attested::Known(before), Attested::Known(after)) => {
                Attested::Known(after.saturating_sub(before))
            }
            _ => Attested::Unknown,
        };
        let (Attested::Known(started), Attested::Known(finished)) = (self.started, scheduling())
        else {
            return Attributability::Unknown;
        };
        Self::judge(started, finished, elapsed, faults)
    }

    /// The judgement itself, separated so a scenario can supply the readings
    /// rather than having to produce contention (D26).
    #[must_use]
    pub fn judge(
        started: Scheduling,
        finished: Scheduling,
        elapsed: Duration<Monotonic>,
        major_faults: Attested<u64>,
    ) -> Attributability {
        let waited = finished.waiting.saturating_sub(started.waiting);
        let elapsed = elapsed.as_nanos();
        if elapsed == 0 {
            // Nothing measurably happened, so nothing can be attributed to it.
            return Attributability::Unknown;
        }
        // Parts per million, computed without floating point: this crate has
        // none by design (see `Quantity`), and a fraction is not a reason to
        // acquire one. The division is the conversion, and `elapsed` is
        // non-zero by the guard above.
        #[allow(clippy::integer_division)]
        let delay_ppm = waited.saturating_mul(1_000_000) / elapsed;

        // Storage first. Both verdicts mean *not a pass*, and where both hold
        // the device is the more specific and more actionable fact: a busy
        // machine is somebody else's compile finishing, and an artifact that
        // was not resident is a property of where it lives.
        if let Attested::Known(major_faults) = major_faults
            && major_faults > 0
        {
            return Attributability::Storage {
                major_faults,
                delay_ppm,
            };
        }

        if delay_ppm <= TOLERATED_DELAY_PPM {
            Attributability::Attributable { delay_ppm }
        } else {
            Attributability::Unattributable {
                delay_ppm,
                tolerated_ppm: TOLERATED_DELAY_PPM,
            }
        }
    }
}

#[cfg(test)]
mod tests;
