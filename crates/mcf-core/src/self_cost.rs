use crate::attested::Attested;
use crate::hardware::Attributability;
use crate::measurement::{Bytes, Conditions, Measurement, Percentile, Quantity, Stated};
use crate::time::{Clock as _, Duration, Monotonic, SystemClock};

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

#[must_use]
pub fn artifact_bytes(path: &std::path::Path) -> Attested<Bytes> {
    match std::fs::metadata(path) {
        Ok(metadata) => Attested::Known(Bytes(metadata.len())),
        Err(_) => Attested::Unknown,
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Prohibition,
    CeilingOnState,
    CeilingOnEvent,
}

pub const EVENT_TRIALS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget<Q: Quantity> {
    pub name: &'static str,
    pub ceiling: Q,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Within,
    Over,
    NotMeasured,
    Unattributable,
    TooFewTrials { had: usize, needs: usize },
}

impl core::fmt::Display for Verdict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Within => f.write_str("within"),
            Self::Over => f.write_str("OVER"),
            Self::NotMeasured => f.write_str("not measured"),
            Self::Unattributable => f.write_str("not attributable"),
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
    #[must_use]
    pub fn read(&self, measured: Attested<Q>) -> Verdict {
        match measured {
            Attested::Unknown => Verdict::NotMeasured,
            Attested::Known(value) if value <= self.ceiling => Verdict::Within,
            Attested::Known(_) => Verdict::Over,
        }
    }

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
            Kind::Prohibition | Kind::CeilingOnState => 2,
        };
        if measured.n() < needs {
            return Verdict::TooFewTrials {
                had: measured.n(),
                needs,
            };
        }
        self.read(Attested::Known(self.statistic(measured).value()))
    }

    #[must_use]
    pub fn statistic(&self, measured: &Measurement<Q>) -> Stated<Q> {
        match self.kind {
            Kind::Prohibition | Kind::CeilingOnState => {
                Stated::new("max", measured.maximum(), measured)
            }
            Kind::CeilingOnEvent => Stated::new("p99", measured.at(Percentile::P99), measured),
        }
    }
}

pub const RESIDENT_IDLE: Budget<Bytes> = Budget {
    name: "resident memory, nothing loaded",
    ceiling: Bytes(20 * 1024 * 1024),
    kind: Kind::CeilingOnState,
};

pub const CORE_BINARY: Budget<Bytes> = Budget {
    name: "core binary, no engines",
    ceiling: Bytes(40 * 1024 * 1024),
    kind: Kind::CeilingOnState,
};

pub const COLD_START: Budget<Duration<Monotonic>> = Budget {
    name: "cold start to first command response",
    ceiling: Duration::from_nanos(100_000_000),
    kind: Kind::CeilingOnEvent,
};

pub const ADDED_LATENCY: Budget<Duration<Monotonic>> = Budget {
    name: "added latency, request to the engine's first token",
    ceiling: Duration::from_nanos(5_000_000),
    kind: Kind::CeilingOnEvent,
};

pub const RECORD_WRITE: Budget<Duration<Monotonic>> = Budget {
    name: "record write, per event",
    ceiling: Duration::from_nanos(2_000_000),
    kind: Kind::CeilingOnEvent,
};

#[cfg(test)]
mod tests;
