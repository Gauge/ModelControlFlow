//! The processor, host memory, and the performance profile in force.
//!
//! Read from what the kernel publishes, every time. Nothing here is cached:
//! §3.8 makes available memory and the governor time-varying conditions, and a
//! reading taken at install is not a reading taken at measurement time.
//!
//! Every field is [`Attested`], including the ones that seem certain. A machine
//! whose `/proc` is not mounted — a minimal container, a platform that is not
//! Linux — produces a profile that says so rather than one that guesses (A7),
//! and B19 requires the suite pass on such a machine.

use core::fmt;

use crate::attested::Attested;
use crate::measurement::Bytes;

/// What the machine computes with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Processor {
    /// The model, as the processor names itself.
    pub model: Attested<String>,
    /// Physical cores.
    pub cores: Attested<u32>,
    /// Hardware threads.
    pub threads: Attested<u32>,
}

impl fmt::Display for Processor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "processor: {} · {} cores · {} threads",
            self.model, self.cores, self.threads
        )
    }
}

/// Host memory, total and available.
///
/// *Available* rather than *free*: the kernel's own estimate of what a new
/// allocation could obtain, which is the quantity that decides whether a model
/// fits. Free memory on a machine with a large page cache reads as almost
/// nothing and would refuse loads that would in fact succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    /// Total installed.
    pub total: Attested<Bytes>,
    /// What the kernel estimates a new allocation could obtain.
    pub available: Attested<Bytes>,
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "memory: {} total · {} available",
            self.total, self.available
        )
    }
}

/// The performance profile the host is running under.
///
/// Kept as the platform's own word rather than mapped onto a scale MCF
/// invented: `performance` and `powersave` are the governor's vocabulary, and
/// an ordering between them is a claim about what they do that MCF has not
/// measured.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PowerProfile(String);

impl PowerProfile {
    /// The profile, as the platform names it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PowerProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reads the processor.
pub(super) fn read_processor() -> Processor {
    let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") else {
        return Processor {
            model: Attested::Unknown,
            cores: Attested::Unknown,
            threads: Attested::Unknown,
        };
    };

    let model = cpuinfo
        .lines()
        .find_map(|line| line.strip_prefix("model name"))
        .and_then(|rest| rest.split_once(':'))
        .map(|(_, value)| value.trim().to_owned());

    // A hardware thread is a `processor:` line. Physical cores are the number
    // of distinct (physical id, core id) pairs; where the kernel does not
    // publish those the count is unknown rather than assumed equal to the
    // thread count, because they differ on every machine with simultaneous
    // multithreading — which is most of them.
    let threads = u32::try_from(
        cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count(),
    )
    .ok();

    let mut pairs: Vec<(&str, &str)> = Vec::new();
    let mut physical: Option<&str> = None;
    for line in cpuinfo.lines() {
        if let Some(value) = field(line, "physical id") {
            physical = Some(value);
        }
        if let (Some(core), Some(package)) = (field(line, "core id"), physical) {
            pairs.push((package, core));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    let cores = u32::try_from(pairs.len()).ok().filter(|count| *count > 0);

    Processor {
        model: attest(model),
        cores: attest(cores),
        threads: attest(threads),
    }
}

/// Reads host memory.
pub(super) fn read_memory() -> Memory {
    let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") else {
        return Memory {
            total: Attested::Unknown,
            available: Attested::Unknown,
        };
    };
    Memory {
        total: attest(kibibytes(&meminfo, "MemTotal:")),
        available: attest(kibibytes(&meminfo, "MemAvailable:")),
    }
}

/// Reads the performance profile in force.
///
/// The first processor's governor. Machines can in principle run different
/// governors per core; where they do, this reading is not the whole truth, and
/// that is a limitation to state rather than a reason to average two words
/// together.
pub(super) fn read_power_profile() -> Attested<PowerProfile> {
    const PATH: &str = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";
    match std::fs::read_to_string(PATH) {
        Ok(governor) if !governor.trim().is_empty() => {
            Attested::Known(PowerProfile(governor.trim().to_owned()))
        }
        Ok(_) | Err(_) => Attested::Unknown,
    }
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?;
    let (_, value) = rest.split_once(':')?;
    Some(value.trim())
}

fn kibibytes(meminfo: &str, key: &str) -> Option<Bytes> {
    meminfo
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kib| Bytes(kib.saturating_mul(1024)))
}

fn attest<T>(value: Option<T>) -> Attested<T> {
    match value {
        Some(value) => Attested::Known(value),
        None => Attested::Unknown,
    }
}
