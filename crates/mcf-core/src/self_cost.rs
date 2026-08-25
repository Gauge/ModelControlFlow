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
use crate::measurement::{Bytes, Conditions, Measurement, Quantity};
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

/// A D24 figure, and how a reading of it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget<Q: Quantity> {
    /// What D24 calls it.
    pub name: &'static str,
    /// The ceiling.
    pub ceiling: Q,
    /// Whether D24 states this as a prohibition rather than a threshold.
    ///
    /// Two of its figures are: zero timer wakeups while idle, zero external
    /// requests from the interface. A prohibition is not a budget that happens
    /// to be zero — it is a statement that the thing may not happen at all,
    /// and rendering it as "0 ≤ 0" invites somebody to argue about the margin.
    pub prohibition: bool,
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
}

impl core::fmt::Display for Verdict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Within => f.write_str("within"),
            Self::Over => f.write_str("OVER"),
            Self::NotMeasured => f.write_str("not measured"),
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
    /// Which statistic of a measurement this should read is §7.50, and it is
    /// open — so this takes a value the caller has already chosen and the
    /// choice is visible at the call site rather than hidden here.
    #[must_use]
    pub fn read(&self, measured: Attested<Q>) -> Verdict {
        match measured {
            Attested::Unknown => Verdict::NotMeasured,
            Attested::Known(value) if value <= self.ceiling => Verdict::Within,
            Attested::Known(_) => Verdict::Over,
        }
    }
}

/// D24's figure for resident memory with nothing loaded.
pub const RESIDENT_IDLE: Budget<Bytes> = Budget {
    name: "resident memory, nothing loaded",
    ceiling: Bytes(20 * 1024 * 1024),
    prohibition: false,
};

/// D24's figure for the core binary's installed footprint.
pub const CORE_BINARY: Budget<Bytes> = Budget {
    name: "core binary, no engines",
    ceiling: Bytes(40 * 1024 * 1024),
    prohibition: false,
};

/// D24's figure for cold start to first command response.
///
/// Which statistic it names is §7.50 and is open; until it is answered, a
/// caller states which one it is reading and says so on the surface.
pub const COLD_START: Budget<Duration<Monotonic>> = Budget {
    name: "cold start to first command response",
    ceiling: Duration::from_nanos(100_000_000),
    prohibition: false,
};

#[cfg(test)]
mod tests;
