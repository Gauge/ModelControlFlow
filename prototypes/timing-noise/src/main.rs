//! How much an identical run's timing varies here, and how many repeats that
//! implies (DEC-007, §3.4, A19).
//!
//! **Why this exists before any benchmark.** DEC-007 asks what makes a
//! performance result publishable — how many repeats, how much spread is too
//! much. The operator's answer was not to choose those numbers but to *derive*
//! them: measure how much the timing of an identical run actually moves on
//! this machine, and let the acceptance criteria follow from that. Every
//! threshold in this repository that held up was measured rather than assumed,
//! and the one that was reasoned about let a defect through (F27, F32).
//!
//! **What is measured.** One command, run many times, unchanged. The output is
//! deliberately not looked at: the command should be deterministic, so that the
//! only thing varying is how long it took. What comes back is the distribution
//! of wall times and, beside each, what the machine's load average was when the
//! run began — because the operator's second answer was that *quiet* is
//! relative, and a machine that idles at forty percent is a machine whose
//! normal is forty percent.
//!
//! **The number this exists to produce is not the spread.** It is *how many
//! repeats are needed to tell a real difference from this noise*, which is what
//! DEC-007 actually has to answer. That is derived without assuming the timings
//! are normally distributed, because they are not: a wall time has a floor and
//! no ceiling, and the tail is whatever else the machine did. Instead, the
//! samples are resampled against themselves — two groups of `n` drawn from the
//! *same* measured distribution — and the question asked of each candidate `n`
//! is how often two such groups differ by more than the effect being looked
//! for. When that is under one in twenty, `n` is enough to stop the noise
//! manufacturing a difference.
//!
//! A19 in one sentence: this reports the false-alarm rate it measured, not a
//! confidence it asserted.
//!
//! ```text
//! cargo run -p mcf-prototype-timing-noise -- <repeats> <command> [args…]
//! ```

use std::process::{Command, Stdio};
use std::time::Instant;

/// The relative differences a benchmark might want to detect.
///
/// Two percent is the size of a careful optimization; twenty percent is the
/// size of a different quantization. If the machine cannot support the small
/// end at any reasonable repeat count, that is the finding.
const EFFECTS: [f64; 4] = [0.02, 0.05, 0.10, 0.20];

/// How often a difference may be manufactured by noise before `n` is too small.
///
/// One in twenty, which is a convention rather than a measurement and is
/// stated as such. It is the only number here that was chosen.
const FALSE_ALARMS_ALLOWED: f64 = 0.05;

/// How many resamplings decide each rate.
const RESAMPLINGS: usize = 4000;

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(repeats) = arguments.next().and_then(|held| held.parse::<usize>().ok()) else {
        eprintln!("usage: timing-noise <repeats> <command> [args…]");
        return std::process::ExitCode::FAILURE;
    };
    let command: Vec<String> = arguments.collect();
    let Some((program, rest)) = command.split_first() else {
        eprintln!("usage: timing-noise <repeats> <command> [args…]");
        return std::process::ExitCode::FAILURE;
    };
    if repeats < 4 {
        eprintln!("four repeats is the fewest that can say anything about a spread");
        return std::process::ExitCode::FAILURE;
    }

    println!("  running {program} {repeats} times");
    let mut timings = Vec::with_capacity(repeats);
    let mut loads = Vec::with_capacity(repeats);
    for index in 0..repeats {
        loads.push(load_average());
        let began = Instant::now();
        let ran = Command::new(program)
            .args(rest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let took = began.elapsed().as_secs_f64();
        match ran {
            Ok(status) if status.success() => timings.push(took),
            Ok(status) => {
                eprintln!("  run {index} exited {status} — the command must succeed every time");
                return std::process::ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("  run {index} could not start: {error}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }

    report(&timings, &loads);
    std::process::ExitCode::SUCCESS
}

/// What the machine said it was doing, at the moment a run began.
///
/// The one-minute load average: not a measure of *this* run and not meant to
/// be, but the thing that says whether the machine was in the same state
/// throughout — which is the criterion, rather than an absolute quiet.
fn load_average() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|held| {
            held.split_whitespace()
                .next()
                .and_then(|first| first.parse().ok())
        })
        .unwrap_or(f64::NAN)
}

/// The distribution, and what it implies.
fn report(timings: &[f64], loads: &[f64]) {
    let sorted = {
        let mut held = timings.to_vec();
        held.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        held
    };
    let median = quantile(&sorted, 0.5);
    let low = quantile(&sorted, 0.25);
    let high = quantile(&sorted, 0.75);
    let spread = if median > 0.0 {
        (high - low) / median
    } else {
        f64::NAN
    };

    println!();
    println!("  the runs, in seconds");
    println!(
        "    fastest   {:.3}",
        sorted.first().copied().unwrap_or(0.0)
    );
    println!("    quartile  {low:.3}");
    println!("    median    {median:.3}");
    println!("    quartile  {high:.3}");
    println!("    slowest   {:.3}", sorted.last().copied().unwrap_or(0.0));
    println!(
        "    the middle half spans {:.1}% of the median",
        spread * 100.0
    );
    println!(
        "    slowest is {:.2}x the fastest",
        sorted.last().copied().unwrap_or(0.0) / sorted.first().copied().unwrap_or(1.0)
    );

    let (least_load, most_load) = loads
        .iter()
        .fold((f64::MAX, f64::MIN), |(least, most), one| {
            (least.min(*one), most.max(*one))
        });
    println!();
    println!("  the machine, while this ran");
    println!("    load average at the start of a run ranged {least_load:.2} to {most_load:.2}");
    println!("    which is the state to hold steady, not a number to be under");

    println!();
    println!("  how many repeats before noise stops manufacturing a difference");
    println!("    (two groups drawn from these same timings, {RESAMPLINGS} times each)");
    for effect in EFFECTS {
        let mut answer = None;
        for candidate in [3usize, 5, 7, 10, 15, 20, 30, 50, 75, 100] {
            let rate = false_alarm_rate(timings, candidate, effect);
            if rate <= FALSE_ALARMS_ALLOWED {
                answer = Some((candidate, rate));
                break;
            }
        }
        match answer {
            Some((candidate, rate)) => println!(
                "    to see a {:>2.0}% difference: {candidate:>3} repeats  (false alarms {:.1}%)",
                effect * 100.0,
                rate * 100.0
            ),
            None => println!(
                "    to see a {:>2.0}% difference: more than 100 repeats — this machine's noise \
                 is larger than the effect",
                effect * 100.0
            ),
        }
    }
}

/// How often two groups of `n`, drawn from the *same* timings, differ by at
/// least `effect`.
///
/// There is no real difference between the groups by construction, so every
/// difference this finds is the noise pretending to be one. That rate is what
/// a repeat count has to hold down, and measuring it needs no assumption about
/// the shape of the distribution — which matters, because a wall time has a
/// floor at the work itself and a tail made of whatever else the machine did.
fn false_alarm_rate(timings: &[f64], each: usize, effect: f64) -> f64 {
    // A fixed seed, because a threshold that moves between runs is not a
    // threshold (§3.12).
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut alarms = 0_usize;
    let mut left = vec![0.0; each];
    let mut right = vec![0.0; each];
    for _ in 0..RESAMPLINGS {
        let held = u64::try_from(timings.len()).unwrap_or(1).max(1);
        for slot in 0..each {
            let a = usize::try_from(next() % held).unwrap_or(0);
            let b = usize::try_from(next() % held).unwrap_or(0);
            if let (Some(one), Some(other), Some(into_left), Some(into_right)) = (
                timings.get(a),
                timings.get(b),
                left.get_mut(slot),
                right.get_mut(slot),
            ) {
                *into_left = *one;
                *into_right = *other;
            }
        }
        left.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        right.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let one = quantile(&left, 0.5);
        let other = quantile(&right, 0.5);
        let base = one.min(other);
        if base > 0.0 && (one - other).abs() / base >= effect {
            alarms = alarms.saturating_add(1);
        }
    }
    f64::from(u32::try_from(alarms).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(RESAMPLINGS).unwrap_or(u32::MAX))
}

/// The value at a fraction of the way through a sorted slice.
fn quantile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let last = sorted.len().saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "an index into a sample of at most a few hundred timings"
    )]
    let at = (fraction * last as f64).round() as usize;
    sorted.get(at.min(last)).copied().unwrap_or(f64::NAN)
}
