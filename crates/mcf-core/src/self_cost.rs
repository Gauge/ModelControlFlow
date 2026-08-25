//! What MCF costs, measured on the machine it is running on.
//!
//! §3.8: *MCF is part of the apparatus it measures. Its own resource
//! consumption contaminates its own benchmarks.* That is the scientific
//! argument for §VII and it is stronger than the ergonomic one — a heavy
//! instrument does not merely annoy the user, it corrupts the readings. So MCF
//! measures itself, and D24 gives the ceilings those measurements are read
//! against.
//!
//! Everything here is a reading of *this* process on *this* machine, taken now.
//! What the platform does not publish is [`Attested::Unknown`] rather than
//! estimated (A7), and what needs a daemon to measure is absent rather than
//! approximated: idle CPU, timer wakeups and memory growth over thirty
//! simulated days are D24 figures about a long-lived process, and there is no
//! long-lived process until M2.
//!
//! B-011 turns these into assertions that fail a build; B-012 characterizes the
//! cost of the observation itself. This module is what both read from.

use crate::attested::Attested;
use crate::hardware::Attributability;
use crate::measurement::{Bytes, Conditions, Measurement, Percentile, Quantity};
use crate::time::{Clock as _, Duration, Monotonic, SystemClock};

/// This process's resident set.
///
/// Read from the kernel's own accounting rather than computed: a number MCF
/// derived about its own memory is a number MCF would then have to defend.
#[must_use]
pub fn resident_bytes() -> Attested<Bytes> {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return Attested::Unknown;
    };
    let kibibytes = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok());
    match kibibytes {
        Some(kib) => Attested::Known(Bytes(kib.saturating_mul(1024))),
        None => Attested::Unknown,
    }
}

/// The size of an artifact on disk.
#[must_use]
pub fn artifact_bytes(path: &std::path::Path) -> Attested<Bytes> {
    match std::fs::metadata(path) {
        Ok(metadata) => Attested::Known(Bytes(metadata.len())),
        Err(_) => Attested::Unknown,
    }
}

/// How long a command takes to answer, over repeated runs.
///
/// D24's *cold start to first command response*. The whole round trip a shell
/// sees — spawn, load, answer, exit — measured with the monotonic clock (B37).
///
/// A trial that could not run is excluded rather than averaged in as a large
/// number: it is not a slow trial, it is not a trial, and the sample count says
/// so (B24's shape).
///
/// Returns `None` when fewer than two trials ran, because §3.4 makes a
/// single-shot timing an anecdote and [`Measurement`] has no way to hold one.
#[must_use]
pub fn cold_start(
    program: &std::path::Path,
    arguments: &[&str],
    trials: usize,
    conditions: Conditions,
) -> Option<Measurement<Duration<Monotonic>>> {
    let clock = SystemClock;
    let mut samples = Vec::with_capacity(trials);
    for _ in 0..trials {
        let started = clock.now();
        let outcome = std::process::Command::new(program)
            .args(arguments)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let elapsed = clock.now().saturating_duration_since(started);
        if outcome.is_ok() {
            samples.push(elapsed);
        }
    }
    Measurement::from_samples(samples, conditions)
}

/// What kind of figure a budget is, which decides the statistic (D27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A thing that may not happen at all. The statistic is the maximum and it
    /// must be zero.
    ///
    /// D24 states two of its figures this way, and the distinction is not
    /// pedantry: a prohibition rendered as "0 ≤ 0" invites an argument about
    /// the margin, and there is no margin.
    Prohibition,
    /// A ceiling on a quantity that *is*. The statistic is the maximum.
    ///
    /// Memory that exceeded the ceiling once exceeded it; a percentile would be
    /// a way of not noticing.
    CeilingOnState,
    /// A ceiling on a quantity that *happens*. The statistic is the 99th
    /// percentile, over at least [`EVENT_TRIALS`] trials.
    ///
    /// D24 names this statistic for added latency and gives the reason — *the
    /// tail is what a user feels* — and D27 generalizes it. The sample floor is
    /// the cost of doing so: a p99 of twenty trials is the maximum wearing a
    /// percentile's name.
    CeilingOnEvent,
}

/// The trials an event-class figure needs before its p99 means anything (D27).
pub const EVENT_TRIALS: usize = 100;

/// A D24 figure, and how a reading of it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget<Q: Quantity> {
    /// What D24 calls it.
    pub name: &'static str,
    /// The ceiling.
    pub ceiling: Q,
    /// Which statistic is compared against it (D27).
    pub kind: Kind,
}

/// Which side of a ceiling a reading falls on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Under the ceiling.
    Within,
    /// Over it.
    Over,
    /// Nothing was read, so there is no verdict — which is not the same as
    /// passing (A7).
    NotMeasured,
    /// The machine was too busy for the reading to be about MCF (B24, B35).
    ///
    /// Neither a pass nor a failure. D27 makes this its own outcome so that a
    /// budget cannot be violated by somebody else's compile — and so that a
    /// regression cannot hide behind a permanently busy machine, because an
    /// unattributable run does not count as a pass either and B38's staleness
    /// discipline applies to what is left.
    Unattributable,
    /// The reading was taken from too few trials for its statistic to mean
    /// anything (D27).
    TooFewTrials {
        /// How many there were.
        had: usize,
        /// How many the statistic needs.
        needs: usize,
    },
}

impl core::fmt::Display for Verdict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Within => f.write_str("within"),
            Self::Over => f.write_str("OVER"),
            Self::NotMeasured => f.write_str("not measured"),
            Self::Unattributable => f.write_str("not attributable — the machine was busy"),
            Self::TooFewTrials { had, needs } => {
                write!(
                    f,
                    "not asserted — {had} trials, and this figure needs {needs}"
                )
            }
        }
    }
}

impl<Q: Quantity> Budget<Q> {
    /// Reads a measurement against this ceiling.
    ///
    /// **Deliberately no percentage of the budget.** D24 calls every figure a
    /// ceiling rather than a target, and *a budget that is merely met has not
    /// been optimized*; a percentage is an invitation to optimize toward the
    /// ceiling instead of away from it.
    ///
    /// For a state-class or prohibition figure, whose statistic is the maximum
    /// and whose reading is therefore a single value.
    #[must_use]
    pub fn read(&self, measured: Attested<Q>) -> Verdict {
        match measured {
            Attested::Unknown => Verdict::NotMeasured,
            Attested::Known(value) if value <= self.ceiling => Verdict::Within,
            Attested::Known(_) => Verdict::Over,
        }
    }

    /// Reads a measurement against this ceiling, applying D27's statistic and
    /// D27's attributability rule.
    ///
    /// The order of the checks is the point. Attributability comes first,
    /// because a reading taken on a busy machine is not a reading of MCF at all
    /// and asking whether it passed is asking the wrong question (B35). The
    /// trial count comes next, because a statistic computed from too few is a
    /// different statistic (D27). Only then is the ceiling consulted.
    #[must_use]
    pub fn read_measurement(
        &self,
        measured: &Measurement<Q>,
        attributability: &Attributability,
    ) -> Verdict {
        if !attributability.permits_assertion() {
            return Verdict::Unattributable;
        }
        let needs = match self.kind {
            Kind::CeilingOnEvent => EVENT_TRIALS,
            // A maximum is a maximum however many readings it is over, but two
            // is still the floor: `Measurement` cannot hold fewer (§3.4).
            Kind::Prohibition | Kind::CeilingOnState => 2,
        };
        if measured.n() < needs {
            return Verdict::TooFewTrials {
                had: measured.n(),
                needs,
            };
        }
        self.read(Attested::Known(self.statistic(measured)))
    }

    /// The value D27 says this figure is about.
    #[must_use]
    pub fn statistic(&self, measured: &Measurement<Q>) -> Q {
        match self.kind {
            Kind::Prohibition | Kind::CeilingOnState => measured.maximum(),
            Kind::CeilingOnEvent => measured.at(Percentile::P99),
        }
    }
}

/// D24's figure for resident memory with nothing loaded.
pub const RESIDENT_IDLE: Budget<Bytes> = Budget {
    name: "resident memory, nothing loaded",
    ceiling: Bytes(20 * 1024 * 1024),
    kind: Kind::CeilingOnState,
};

/// D24's figure for the core binary's installed footprint.
pub const CORE_BINARY: Budget<Bytes> = Budget {
    name: "core binary, no engines",
    ceiling: Bytes(40 * 1024 * 1024),
    kind: Kind::CeilingOnState,
};

/// D24's figure for cold start to first command response.
///
/// Event-class: it is a thing that happens, and D27 makes the statistic the
/// 99th percentile because the tail is what a user feels.
pub const COLD_START: Budget<Duration<Monotonic>> = Budget {
    name: "cold start to first command response",
    ceiling: Duration::from_nanos(100_000_000),
    kind: Kind::CeilingOnEvent,
};

/// D24's figure for the cost of writing one record.
pub const RECORD_WRITE: Budget<Duration<Monotonic>> = Budget {
    name: "record write, per event",
    ceiling: Duration::from_nanos(2_000_000),
    kind: Kind::CeilingOnEvent,
};

#[cfg(test)]
mod tests;
