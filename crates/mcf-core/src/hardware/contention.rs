//! What was competing for this machine (B-216, PR5, §3.8, B24, B4).
//!
//! **B24's refusal, upgraded to a diagnosis.** §3.8: *MCF knows the difference
//! between "this model is slow" and "this machine was busy". When it cannot
//! tell the difference, it says so rather than attributing the result.* Having
//! been told a measurement is unattributable, the operator's next question is
//! always **by what?** — and MCF is the only thing positioned to answer,
//! because it was there when it happened and nothing else was.
//!
//! **On demand, never a monitor** (B4, D5). This samples when something asks
//! it to and at no other time: there is no timer here, no background thread,
//! and nothing that runs while MCF is idle. It costs a stated interval of
//! wall-clock — two readings are needed to turn accumulated processor time
//! into a rate — and that cost is the caller's to spend, after the measurement
//! rather than during it.
//!
//! **What it can read, and what it says it cannot.** D25 draws the boundary at
//! a capability of the observer rather than a vendor list, and the same
//! honesty applies here: the processes competing for the processor are
//! readable without privilege and are read; the accelerator's per-process
//! occupancy needs a vendor library MCF may not have and is reported *attempted
//! and uncharacterized* rather than as zero; the thermal state is whatever the
//! machine exposes, which F53 measured as a single sensor reading sixteen
//! degrees on this one — not a processor temperature, and said to be nothing
//! rather than reported as cold.
//!
//! **It names names.** A snapshot whose answer is *the machine was 94 % busy*
//! is a number; one whose answer is *these four processes took 380 % of a core
//! between them, and here is what they are* is a diagnosis. The command lines
//! are read from `/proc` and are the operator's own machine's — nothing leaves,
//! and §3.20's gate is on whatever sends a record rather than on reading one.
//!
//! **Cross-checked by test:** `contention_agrees_with_the_kernel` — the kernel's
//! own `/proc/stat` accounting, a different file and a different accounting
//! path, which cannot exceed the core count.

use core::fmt;

use crate::attested::Attested;
use crate::hardware::LoadAverage;

/// How long two readings are separated by, to turn accumulated processor time
/// into a rate.
///
/// A fifth of a second: long enough that a busy process shows and short enough
/// that a caller taking one after a run is not made to wait. Stated here
/// because it is the one number in this module that was chosen.
pub const OVER: core::time::Duration = core::time::Duration::from_millis(200);

/// How many competitors are named.
///
/// Also chosen. A list of everything is a list nobody reads; the question is
/// *what was competing*, and the answer is the few that were.
pub const NAMED: usize = 5;

/// One process, and what it was taking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Competitor {
    /// Its process identifier.
    pub pid: u32,
    /// Its command, as the machine records it.
    pub command: String,
    /// Processor time it took over the interval, in thousandths of a core.
    ///
    /// Thousandths rather than a float, for the reason `Quantity` gives: a
    /// condition recorded beside a measurement is measured, and nothing
    /// measured in MCF is floating point.
    pub cores_taken: u64,
    /// Whether this is MCF's own process.
    ///
    /// Named rather than filtered out: MCF competing with itself is a true and
    /// useful thing to see, and a snapshot that hid it would be a snapshot
    /// hiding the one process the reader can do something about.
    pub is_mcf: bool,
}

impl fmt::Display for Competitor {
    // Integer division is the conversion; both operands are bounded.
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

/// What the machine was doing at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// How long the reading actually took, in milliseconds.
    ///
    /// A condition of the rate rather than a detail (§3.4, F90): the window is
    /// [`OVER`] plus whatever walking `/proc` cost, which is not constant and
    /// grows with the number of processes and with how busy the machine is.
    /// Carried so that two snapshots can be told apart when one was taken over
    /// twice the window of the other.
    pub over_millis: u64,
    /// The processes taking the most processor time, most first.
    pub competitors: Vec<Competitor>,
    /// How much of a core everything took between them, in thousandths.
    pub cores_taken: u64,
    /// How long tasks were stalled on the processor, in parts per million of
    /// the last ten seconds — the kernel's own pressure accounting.
    pub processor_pressure: Attested<u64>,
    /// The same for memory.
    pub memory_pressure: Attested<u64>,
    /// The same for input and output.
    pub storage_pressure: Attested<u64>,
    /// The one-minute load average, which decides nothing and is true (F3).
    pub load: Attested<LoadAverage>,
    /// Per-process accelerator occupancy.
    ///
    /// `Unknown` where MCF cannot read it, which D25 makes a capability of the
    /// observer rather than a fact about the machine — and which is *not* the
    /// same as no accelerator contention.
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

/// How much this machine's own load moved while it was watched (B-217, D8,
/// §3.8, DEC-007).
///
/// **Steady against its own baseline, not quiet against a number.** The
/// operator's answer of 2026-08-27 is explicit: a user's machine may idle at
/// thirty or forty percent and that is *its* normal, and refusing to measure
/// below an absolute quiet would deny most people a result while telling them
/// nothing. So what is asked here is whether the machine held still, wherever
/// it was sitting — the spread of what was competing, against the middle of it.
///
/// **This measures; it does not judge.** What spread is too much is the band
/// DEC-007 leaves open, and a threshold invented here would be exactly the
/// figure that decision exists to derive from measurement. So the number is
/// reported and carried as a condition, and the refusal B-217 eventually wants
/// waits for somebody to have measured what it should be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Steadiness {
    /// The middle of what was competing, in thousandths of a core.
    pub middle: u64,
    /// How far the readings spread, in parts per million of that middle.
    ///
    /// `None` where the middle is zero — a machine with nothing competing has
    /// no baseline to be steady against, and dividing by it would be an
    /// infinity where A7 wants a state (§3.4).
    pub spread: Option<u64>,
    /// How many readings it rests on.
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

/// Watches this machine hold still, or not.
///
/// Blocks for `readings` times [`OVER`]. Two is the fewest that can show a
/// spread at all; a caller wanting a firmer answer pays for more readings, and
/// that cost is theirs to spend.
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

/// Samples what is competing for this machine, now.
///
/// Blocks for [`OVER`], because a rate needs two readings. Callers take this
/// *after* a measurement rather than during one: sampling while measuring would
/// make MCF one of the competitors it is reporting (§3.8, B3).
#[must_use]
pub fn sample() -> Snapshot {
    use crate::time::Clock as _;

    let ours = std::process::id();
    let clock = crate::time::SystemClock;
    // **The interval is measured, not assumed** (F90). Walking `/proc` costs
    // real time — it reads a file per process — and that time falls *inside*
    // the window the rate is computed over. Dividing by `OVER` when the window
    // was longer inflates every figure by exactly the ratio, and it inflates
    // most when the machine is busiest, because that is when the walk is
    // slowest and when the reading matters. Measured against `/proc/stat`,
    // assuming `OVER` reported 35.2 cores on a 32-thread machine where the
    // truth was 28.9; dividing by the elapsed time instead agreed to under one
    // percent.
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
            // Thousandths of a core: ticks over the interval, against the
            // ticks one core would have spent in it.
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
    // Busiest first, so a reader stops at the top.
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
        // D25: a capability of the observer. MCF has no vendor library for
        // per-process occupancy here, and *unknown* is not *none*.
        accelerator: Attested::Unknown,
    }
}

/// The kernel's stall accounting for one resource, in parts per million of the
/// last ten seconds.
///
/// `Unknown` on a kernel that does not keep it, which is a capability of the
/// machine and not a quiet one (A7).
fn pressure(resource: &str) -> Attested<u64> {
    let Ok(text) = std::fs::read_to_string(format!("/proc/pressure/{resource}")) else {
        return Attested::Unknown;
    };
    // `some avg10=0.00 avg60=… ` — the ten-second window, which is the one on
    // the scale of a measurement rather than of an afternoon (F3's lesson).
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
    // A percentage to two places, as parts per million.
    Attested::Known(
        whole
            .saturating_mul(10_000)
            .saturating_add(hundredths.saturating_mul(100)),
    )
}

/// Every process's accumulated processor time, in ticks.
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
        // The command is parenthesized and may contain spaces, so the fields
        // after it are counted from the last `)` rather than from the start.
        let Some(after) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
            continue;
        };
        let fields: Vec<&str> = after.split_whitespace().collect();
        // `utime` and `stime` are the eleventh and twelfth fields after the
        // state, which is the first thing after the command.
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

/// A process's command line, or its short name where the line is not readable.
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

/// How many scheduler ticks a second, as the platform counts them.
///
/// A hundred everywhere Linux is configured the usual way. Read from the
/// environment where the platform states it, and otherwise the usual value —
/// which is a stated assumption rather than a silent one, and is wrong only on
/// a kernel built to disagree with every tool that reads `/proc`.
fn ticks_per_second() -> u64 {
    std::env::var("MCF_CLOCK_TICKS")
        .ok()
        .and_then(|held| held.parse::<u64>().ok())
        .filter(|held| *held > 0)
        .unwrap_or(100)
}

#[cfg(test)]
mod tests;
