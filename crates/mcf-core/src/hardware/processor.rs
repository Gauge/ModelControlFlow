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
//!
//! **Cross-check owed (B-390):** core counts and the governor are read from
//! one place and compared against nothing. An independent source exists —
//! `lscpu`, `/proc/cpuinfo` against `sysconf` — and has not been used.

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

/// What the control group this process runs in will still allow it, in bytes.
///
/// **`/proc/meminfo` describes the machine, which is not always the thing MCF
/// is running in.** Under a container or a systemd scope with a memory limit,
/// `MemAvailable` reports the host's free memory — a number about somewhere
/// else. MCF planned a context window against 119 GiB while running under a
/// 40 GiB limit, and the kernel ended the engine sixteen seconds in. That is
/// A21 with the machine itself as the declaration: a figure read honestly,
/// describing something other than what it is used to decide (F144).
///
/// cgroup v2's unified hierarchy names this process's group, and a limit may
/// sit on it or on any ancestor, so the smallest headroom found is the one
/// that binds. `None` where there is no limit anywhere, or where the files
/// cannot be read — an unknown limit is not a limit of zero (A7).
pub(super) fn cgroup_headroom() -> Option<u64> {
    let own = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own.lines().find_map(|line| line.strip_prefix("0::"))?;
    let mut at = std::path::PathBuf::from("/sys/fs/cgroup");
    let mut least: Option<u64> = None;
    let mut consider = |at: &std::path::Path, least: &mut Option<u64>| {
        let read = |name: &str| std::fs::read_to_string(at.join(name)).ok();
        let (Some(max), Some(now)) = (read("memory.max"), read("memory.current")) else {
            return;
        };
        // `max` is the word rather than a number where nothing is limited.
        let (Ok(max), Ok(now)) = (max.trim().parse::<u64>(), now.trim().parse::<u64>()) else {
            return;
        };
        let headroom = max.saturating_sub(now);
        *least = Some(least.map_or(headroom, |held: u64| held.min(headroom)));
    };
    consider(&at, &mut least);
    for part in path.split('/').filter(|part| !part.is_empty()) {
        at.push(part);
        consider(&at, &mut least);
    }
    least
}

/// Reads memory available to this process: the machine's, or its group's
/// where that is smaller.
pub(super) fn read_memory() -> Memory {
    let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") else {
        return Memory {
            total: Attested::Unknown,
            available: Attested::Unknown,
        };
    };
    Memory {
        total: attest(kibibytes(&meminfo, "MemTotal:")),
        available: attest(available_now().map(crate::measurement::Bytes)),
    }
}

/// What this process may take right now, in bytes.
///
/// **The narrow reading, so that the serving path can have it.** B4 keeps
/// hardware sampling out of the daemon — a process that reads the machine
/// becomes one of the competitors it reports (§3.8) — and `Machine::read`
/// samples processors, cards and a thermal counter to answer a question about
/// memory. This is two file reads and the arithmetic between them, which is
/// what the daemon needs and all of what it needs.
///
/// The machine's free memory, or its group's headroom where that is smaller.
#[must_use]
pub(super) fn available_now() -> Option<u64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
    let host = meminfo
        .as_deref()
        .and_then(|held| kibibytes(held, "MemAvailable:"))
        .map(|bytes| bytes.0);
    match (host, cgroup_headroom()) {
        // The smaller of the two, because both are true and only one of them
        // is a limit MCF can be stopped by.
        (Some(host), Some(group)) => Some(host.min(group)),
        (host, None) => host,
        (None, group) => group,
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

#[cfg(test)]
mod cgroup_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::{cgroup_headroom, read_memory};
    use crate::attested::Attested;

    /// Whatever this machine says, the two readings are consistent.
    ///
    /// The property is not a number — the workspace's tests run on machines
    /// with limits and without — but that a limit, where there is one, binds
    /// the figure MCF plans against (F144).
    #[test]
    fn a_limit_binds_what_is_reported_available() {
        let memory = read_memory();
        let Attested::Known(available) = memory.available else {
            // A machine that will not say is a state, not a failure (A7).
            return;
        };
        if let Some(headroom) = cgroup_headroom() {
            assert!(
                available.0 <= headroom,
                "this process runs under a limit with {headroom} bytes of headroom and MCF \
                 reports {} available, which is memory it cannot have (F144)",
                available.0
            );
        }
        if let Attested::Known(total) = memory.total {
            assert!(
                available.0 <= total.0,
                "more is available than the machine has"
            );
        }
    }

    /// The unlimited case is not a limit of zero.
    ///
    /// `memory.max` holds the word `max` where nothing is limited, and a
    /// parse that treated it as a number would make every unlimited group
    /// look full (A7).
    #[test]
    fn no_limit_is_not_a_limit_of_nothing() {
        if let Some(headroom) = cgroup_headroom() {
            assert!(
                headroom > 0,
                "an unlimited or unreadable group reported no headroom at all, which would \
                 refuse every model on a machine with nothing wrong with it"
            );
        }
    }
}
