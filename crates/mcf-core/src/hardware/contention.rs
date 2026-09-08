use core::fmt;

use crate::attested::Attested;
use crate::hardware::LoadAverage;

pub const OVER: core::time::Duration = core::time::Duration::from_millis(200);

pub const NAMED: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Competitor {
    pub pid: u32,
    pub command: String,
    pub cores_taken: u64,
    pub is_mcf: bool,
}

impl fmt::Display for Competitor {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{:02} core(s)  pid {}  {}{}",
            self.cores_taken / 1000,
            (self.cores_taken % 1000) / 10,
            self.pid,
            self.command,
            if self.is_mcf { "  (this is MCF)" } else { "" }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub over_millis: u64,
    pub competitors: Vec<Competitor>,
    pub cores_taken: u64,
    pub processor_pressure: Attested<u64>,
    pub memory_pressure: Attested<u64>,
    pub storage_pressure: Attested<u64>,
    pub load: Attested<LoadAverage>,
    pub accelerator: Attested<String>,
}

impl fmt::Display for Snapshot {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{:02} core(s) taken over {} ms by {} process(es)",
            self.cores_taken / 1000,
            (self.cores_taken % 1000) / 10,
            self.over_millis,
            self.competitors.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Steadiness {
    pub middle: u64,
    pub spread: Option<u64>,
    pub readings: usize,
}

impl fmt::Display for Steadiness {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.spread {
            Some(spread) => write!(
                form,
                "{}.{:02} core(s) competing, holding to {}.{}% over {} reading(s)",
                self.middle / 1000,
                (self.middle % 1000) / 10,
                spread / 10_000,
                (spread / 1_000) % 10,
                self.readings
            ),
            None => write!(
                form,
                "nothing measurable competing over {} reading(s), so there is no baseline to be \
                 steady against",
                self.readings
            ),
        }
    }
}

#[must_use]
pub fn steadiness(readings: usize) -> Steadiness {
    let mut seen: Vec<u64> = (0..readings.max(2)).map(|_| sample().cores_taken).collect();
    seen.sort_unstable();
    let middle = seen.get(seen.len().wrapping_div(2)).copied().unwrap_or(0);
    let spread = seen
        .last()
        .zip(seen.first())
        .map(|(most, least)| most.saturating_sub(*least))
        .filter(|_| middle > 0)
        .map(|held| {
            u64::try_from(
                u128::from(held)
                    .saturating_mul(1_000_000)
                    .wrapping_div(u128::from(middle)),
            )
            .unwrap_or(u64::MAX)
        });
    Steadiness {
        middle,
        spread,
        readings: seen.len(),
    }
}

#[must_use]
pub fn sample() -> Snapshot {
    use crate::time::Clock as _;

    let ours = std::process::id();
    let clock = crate::time::SystemClock;
    let opened = clock.now();
    let before = processor_time();
    std::thread::sleep(OVER);
    let after = processor_time();
    let elapsed_millis = clock
        .now()
        .saturating_duration_since(opened)
        .as_nanos()
        .wrapping_div(1_000_000)
        .max(1);

    let ticks = ticks_per_second();
    let mut competitors: Vec<Competitor> = after
        .iter()
        .filter_map(|(pid, command, later)| {
            let earlier = before
                .iter()
                .find(|(was, _, _)| was == pid)
                .map_or(*later, |(_, _, held)| *held);
            let took = later.saturating_sub(earlier);
            if took == 0 {
                return None;
            }
            let whole = took
                .saturating_mul(1_000)
                .saturating_mul(1_000)
                .wrapping_div(ticks.saturating_mul(elapsed_millis));
            Some(Competitor {
                pid: *pid,
                command: command.clone(),
                cores_taken: whole,
                is_mcf: *pid == ours,
            })
        })
        .collect();
    competitors.sort_by_key(|one| core::cmp::Reverse(one.cores_taken));
    let cores_taken = competitors
        .iter()
        .fold(0_u64, |held, one| held.saturating_add(one.cores_taken));
    competitors.truncate(NAMED);

    Snapshot {
        over_millis: elapsed_millis,
        competitors,
        cores_taken,
        processor_pressure: pressure("cpu"),
        memory_pressure: pressure("memory"),
        storage_pressure: pressure("io"),
        load: crate::hardware::load_average(),
        accelerator: Attested::Unknown,
    }
}

fn pressure(resource: &str) -> Attested<u64> {
    let Ok(text) = std::fs::read_to_string(format!("/proc/pressure/{resource}")) else {
        return Attested::Unknown;
    };
    let Some(field) = text
        .lines()
        .find(|line| line.starts_with("some "))
        .and_then(|line| {
            line.split_whitespace()
                .find_map(|held| held.strip_prefix("avg10="))
        })
    else {
        return Attested::Unknown;
    };
    let (whole, rest) = field.split_once('.').unwrap_or((field, "0"));
    let (Ok(whole), Ok(hundredths)) = (whole.parse::<u64>(), rest.parse::<u64>()) else {
        return Attested::Unknown;
    };
    Attested::Known(
        whole
            .saturating_mul(10_000)
            .saturating_add(hundredths.saturating_mul(100)),
    )
}

fn processor_time() -> Vec<(u32, String, u64)> {
    let mut held = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return held;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|held| held.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let Some(after) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
            continue;
        };
        let fields: Vec<&str> = after.split_whitespace().collect();
        let (Some(user), Some(system)) = (
            fields.get(11).and_then(|held| held.parse::<u64>().ok()),
            fields.get(12).and_then(|held| held.parse::<u64>().ok()),
        ) else {
            continue;
        };
        held.push((pid, command_of(pid), user.saturating_add(system)));
    }
    held
}

fn command_of(pid: u32) -> String {
    if let Ok(line) = std::fs::read_to_string(format!("/proc/{pid}/cmdline")) {
        let joined: Vec<&str> = line
            .split('\0')
            .filter(|held| !held.is_empty())
            .take(4)
            .collect();
        if !joined.is_empty() {
            let held = joined.join(" ");
            return held.chars().take(80).collect();
        }
    }
    std::fs::read_to_string(format!("/proc/{pid}/comm")).map_or_else(
        |_| "a process that ended before it could be named".to_owned(),
        |held| held.trim().to_owned(),
    )
}

fn ticks_per_second() -> u64 {
    std::env::var("MCF_CLOCK_TICKS")
        .ok()
        .and_then(|held| held.parse::<u64>().ok())
        .filter(|held| *held > 0)
        .unwrap_or(100)
}

#[cfg(test)]
mod tests;
