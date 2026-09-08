use core::fmt;

use crate::attested::Attested;
use crate::measurement::Bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Processor {
    pub model: Attested<String>,
    pub cores: Attested<u32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    pub total: Attested<Bytes>,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PowerProfile(String);

impl PowerProfile {
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

pub(super) fn cgroup_headroom() -> Option<u64> {
    let own = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own.lines().find_map(|line| line.strip_prefix("0::"))?;
    let mut at = std::path::PathBuf::from("/sys/fs/cgroup");
    let mut least: Option<u64> = None;
    let consider = |at: &std::path::Path, least: &mut Option<u64>| {
        let read = |name: &str| std::fs::read_to_string(at.join(name)).ok();
        let (Some(max), Some(now)) = (read("memory.max"), read("memory.current")) else {
            return;
        };
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

#[must_use]
pub(super) fn available_now() -> Option<u64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok();
    let host = meminfo
        .as_deref()
        .and_then(|held| kibibytes(held, "MemAvailable:"))
        .map(|bytes| bytes.0);
    match (host, cgroup_headroom()) {
        (Some(host), Some(group)) => Some(host.min(group)),
        (host, None) => host,
        (None, group) => group,
    }
}

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
    #![allow(clippy::panic, clippy::expect_used)]

    use super::{cgroup_headroom, read_memory};
    use crate::attested::Attested;

    #[test]
    fn a_limit_binds_what_is_reported_available() {
        let memory = read_memory();
        let Attested::Known(available) = memory.available else {
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
