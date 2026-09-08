use core::fmt;

use crate::attested::Attested;
use crate::time::{Clock as _, Duration, Instant, Monotonic, SystemClock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheduling {
    pub on_cpu: u64,
    pub waiting: u64,
}

#[must_use]
pub fn scheduling() -> Attested<Scheduling> {
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

pub const TOLERATED_DELAY_PPM: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attributability {
    Attributable { delay_ppm: u64 },
    Unattributable { delay_ppm: u64, tolerated_ppm: u64 },
    Storage { major_faults: u64, delay_ppm: u64 },
    Unknown,
}

impl Attributability {
    #[must_use]
    pub const fn permits_assertion(&self) -> bool {
        matches!(self, Self::Attributable { .. })
    }

    #[must_use]
    pub const fn delay_ppm(&self) -> Option<u64> {
        match self {
            Self::Attributable { delay_ppm }
            | Self::Unattributable { delay_ppm, .. }
            | Self::Storage { delay_ppm, .. } => Some(*delay_ppm),
            Self::Unknown => None,
        }
    }

    #[must_use]
    pub const fn major_faults(&self) -> Option<u64> {
        match self {
            Self::Storage { major_faults, .. } => Some(*major_faults),
            Self::Attributable { .. } | Self::Unattributable { .. } | Self::Unknown => None,
        }
    }
}

impl fmt::Display for Attributability {
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
    #[must_use]
    pub fn start() -> Self {
        Self {
            started: scheduling(),
            faults_at_start: super::children_major_faults(),
            at: SystemClock.now(),
        }
    }

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
            return Attributability::Unknown;
        }
        #[allow(clippy::integer_division)]
        let delay_ppm = waited.saturating_mul(1_000_000) / elapsed;

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
