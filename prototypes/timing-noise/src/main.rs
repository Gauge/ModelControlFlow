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

use mcf_core::time::Monotonic;
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
    // Two commands separated by `vs` is a comparison; one is a noise floor.
    let all: Vec<String> = std::env::args().skip(1).collect();
    if let Some(at) = all.iter().position(|held| held == "vs") {
        return compare(&all, at);
    }
    let mut arguments = all.into_iter();
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
    let mut speeds = Vec::with_capacity(repeats);
    for index in 0..repeats {
        loads.push(load_average());
        let before = mean_frequency();
        let began = Instant::now();
        let ran = Command::new(program)
            .args(rest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let took = began.elapsed().as_secs_f64();
        speeds.push(f64::midpoint(before, mean_frequency()));
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

    report(&timings, &loads, &speeds);
    std::process::ExitCode::SUCCESS
}

/// Two commands, interleaved, until the stopping condition decides.
///
/// The shape F51 requires and B-250 now makes structural: the two arms are run
/// alternately rather than one after the other, so that anything drifting
/// under the comparison lands on both and cancels — and **which one goes first
/// is drawn per pair**, so that going first is not an advantage. Neither is
/// this function's discipline any more. `mcf_bench::compare::Interleaving` is
/// the only way to build a comparison at all, and a caller that wanted to run
/// thirty of one and then thirty of the other could not express it.
///
/// The count is not chosen either — it stops when this run's own resampling
/// separates the difference from its own noise, which is what F53 established
/// a count cannot do.
fn compare(all: &[String], at: usize) -> std::process::ExitCode {
    let Some(resolving) = all.first().and_then(|held| held.parse::<f64>().ok()) else {
        eprintln!("usage: timing-noise <resolving-fraction> <command…> vs <command…>");
        return std::process::ExitCode::FAILURE;
    };
    let (Some(one), Some(other)) = (all.get(1..at), all.get(at.saturating_add(1)..)) else {
        eprintln!("usage: timing-noise <resolving-fraction> <command…> vs <command…>");
        return std::process::ExitCode::FAILURE;
    };
    let ceiling: usize = 120;
    let resolving_ppm = mcf_core::measurement::PartsPerMillion(whole(resolving * 1e6));

    println!(
        "  comparing, alternately and in a drawn order, looking for a {:.0}% difference",
        resolving * 100.0
    );

    // The seed is stated rather than taken from the clock, so that the order
    // this run drew is the order a re-run draws (§3.12). It is printed for the
    // same reason.
    let seed = 0x5DEE_CE66_D125_u64;
    println!("  order seed: {seed:#x}");
    // The arms are named for the reader; which command each names is kept
    // here rather than parsed back out of the name, because a command holds
    // spaces and an arm's name is not a place to encode one.
    let left_arm = mcf_core::trial::Arm::new(one.join(" "));
    let right_arm = mcf_core::trial::Arm::new(other.join(" "));
    let mut running = mcf_bench::compare::Interleaving::<Monotonic>::new(
        as_configuration(&left_arm),
        as_configuration(&right_arm),
        mcf_core::trial::SessionId::new(format!("timing-noise-{}", std::process::id())),
        seed,
        // A timing run: the seed is held still and the generation length is
        // pinned by whatever command the operator named (D19, B-290). The
        // prototype cannot pin a length it does not control, so it records
        // zero — *nothing pinned* — which is what a comparison of two opaque
        // commands honestly is.
        mcf_bench::compare::Discipline::Timing { seed, tokens: 0 },
    );

    let mut failed = false;
    for round in 0..ceiling {
        let _ran = running.round(|arm, _drew| {
            let command = if *arm == left_arm { one } else { other };
            // Nanoseconds, because the crate counts in integers — a shipped
            // type there may not hold a float, since that is how a NaN reaches
            // a record. A prototype is not shipped and may; the conversion is
            // the boundary (F54).
            if let Some(seconds) = timed(command) {
                mcf_core::time::Duration::from_nanos(whole(seconds * 1e9))
            } else {
                failed = true;
                mcf_core::time::Duration::from_nanos(0)
            }
        });
        if failed {
            return std::process::ExitCode::FAILURE;
        }
        let said = running.finding(resolving_ppm);
        if !matches!(
            said.verdict(),
            None | Some(mcf_bench::enough::Verdict::NotYet { .. })
        ) {
            report_comparison(&said, running.comparison());
            return std::process::ExitCode::SUCCESS;
        }
        if round.saturating_add(1) % 10 == 0 {
            println!("    {said}");
        }
    }
    println!(
        "    still undecided after {ceiling} paired trials — which is a result about this \
         machine and not about the two commands (A7)"
    );
    std::process::ExitCode::SUCCESS
}

/// An arm as the configuration it is, which for a shell command is one
/// condition MCF can state and ten it cannot.
///
/// The command line goes in `mcf_configuration`, because that is what actually
/// differs between the arms here. Everything else is `Unknown` rather than
/// filled in with something plausible (A7), so the comparison reports its
/// isolation as *undetermined* — which is the truth about a prototype timing
/// two opaque commands, and is what B-085 exists to make visible.
fn as_configuration(arm: &mcf_core::trial::Arm) -> mcf_bench::compare::UnderTest {
    let mut floor = mcf_core::measurement::Floor::nothing_known();
    floor.mcf_configuration = mcf_core::attested::Attested::Known(
        mcf_core::measurement::ConditionValue::text(arm.as_str()),
    );
    mcf_bench::compare::UnderTest::new(
        arm.clone(),
        mcf_core::measurement::Conditions::new(
            mcf_core::build_identity::BuildIdentity::current(),
            floor,
        ),
    )
}

/// What a decided comparison has to say, including how it was constructed.
fn report_comparison(
    said: &mcf_bench::compare::Finding,
    held: &mcf_bench::compare::Comparison<Monotonic>,
) {
    println!("    {said}");
    // A duration as a person reads it. The prototype may hold a float where a
    // shipped crate may not; the milliseconds are taken with an integer
    // division first so that the conversion cannot lose a nanosecond it was
    // never going to print.
    let seconds = |held: mcf_core::time::Duration<Monotonic>| {
        let millis = held.as_nanos().wrapping_div(1_000_000);
        f64::from(u32::try_from(millis).unwrap_or(u32::MAX)) / 1000.0
    };
    println!(
        "    medians: {:.3} s and {:.3} s",
        middle(
            &held
                .pairs()
                .iter()
                .map(|p| seconds(p.left()))
                .collect::<Vec<f64>>()
        ),
        middle(
            &held
                .pairs()
                .iter()
                .map(|p| seconds(p.right()))
                .collect::<Vec<f64>>()
        )
    );
    let (left_first, right_first) = held.order_balance();
    println!("    order: {left_first} pair(s) ran the left arm first, {right_first} the right");
    // The paired difference *distribution* is the reported quantity (B53), so
    // it is printed rather than summarized away.
    if let Some(differences) = held.paired_differences() {
        let mut left_ahead = 0_usize;
        for held in &differences {
            if matches!(
                held,
                mcf_bench::compare::Difference::Quicker {
                    side: mcf_bench::compare::Side::Left,
                    ..
                }
            ) {
                left_ahead = left_ahead.saturating_add(1);
            }
        }
        println!(
            "    pairs: the left arm was quicker in {left_ahead} of {}",
            differences.len()
        );
        // The raw trials as well as the differences: D16 keeps every trial
        // because a summary is a question nobody can ask again, and the
        // blocked arrangement of these same timings is exactly such a
        // question.
        println!("    pairs, in interleaving order (left, right, first, difference):");
        for (at, (pair, held)) in held.pairs().iter().zip(&differences).enumerate() {
            println!(
                "      #{at}: {:.3} s  {:.3} s  {} first  {held}",
                seconds(pair.left()),
                seconds(pair.right()),
                pair.first()
            );
        }
    }
}

/// A non-negative float as the nearest whole number, saturating.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a duration in seconds, bounded below by zero and above by the run itself"
)]
fn whole(held: f64) -> u64 {
    if held.is_finite() && held > 0.0 {
        held.round() as u64
    } else {
        0
    }
}

/// One run, timed, or nothing if it failed.
fn timed(command: &[String]) -> Option<f64> {
    let (program, rest) = command.split_first()?;
    let began = Instant::now();
    let ran = Command::new(program)
        .args(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    ran.success().then(|| began.elapsed().as_secs_f64())
}

/// The median of an unsorted slice.
fn middle(held: &[f64]) -> f64 {
    let mut out = held.to_vec();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    quantile(&out, 0.5)
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
fn report(timings: &[f64], loads: &[f64], speeds: &[f64]) {
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

    // Whether the timings moved *with* the machine. This is the criterion the
    // operator set — stability against this machine's own baseline — asked in
    // the one way that needs no threshold: if a run's duration tracks the load
    // at its start, the measurement is of a machine that changed, and no
    // repeat count fixes that (F51's level shift, seen from inside).
    let together = correlation(loads, timings);
    let by_chance = correlation_by_chance(loads, timings);
    println!(
        "    duration tracked that load at {together:+.2}  (chance alone gives one that big \
         {:.0}% of the time here)",
        by_chance * 100.0
    );
    let contaminated = by_chance <= FALSE_ALARMS_ALLOWED;
    if contaminated {
        println!();
        println!("  THIS MEASUREMENT IS OF A MACHINE THAT CHANGED, NOT OF THE COMMAND.");
        println!("    The runs got slower as the machine got busier, so the spread below is the");
        println!("    machine's movement wearing the command's clothes. A repeat count derived");
        println!("    from it would be derived from the wrong thing. Take it again when the");
        println!("    machine is in one state — any state — rather than passing through several.");
    }

    // Whether the runs tracked the *frequency* the processor was actually
    // running at, which is a different question from whether they tracked the
    // load. On this machine the governor is already `performance` and the only
    // alternative is `powersave`, so there is nothing a governor could pin —
    // and the frequency still spans nearly nine to one, because boost and idle
    // states move it whatever the governor says. If duration tracks it, that
    // is a noise source no privilege can remove.
    let (slowest, fastest) = speeds
        .iter()
        .fold((f64::MAX, f64::MIN), |(low, high), one| {
            (low.min(*one), high.max(*one))
        });
    println!("    processor ran at {slowest:.2} to {fastest:.2} GHz across the runs");
    println!(
        "    duration tracked that frequency at {:+.2}  (chance alone gives one that big \
         {:.0}% of the time here)",
        correlation(speeds, timings),
        correlation_by_chance(speeds, timings) * 100.0
    );

    warm_up(timings);

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

/// How often chance alone pairs these two series as tightly as they are
/// actually paired.
///
/// A correlation is not evidence until it is compared against what shuffling
/// produces. At twenty samples, coefficients of four-tenths arise readily from
/// unrelated series — which is roughly the size of every correlation this
/// instrument has reported, and is why a fixed threshold on the coefficient
/// was barely above chance (F53). The pairing is broken and remade many times;
/// the answer is how often the shuffled version is at least as tight.
fn correlation_by_chance(one: &[f64], other: &[f64]) -> f64 {
    let n = one.len().min(other.len());
    if n < 4 {
        return 1.0;
    }
    let observed = correlation(one, other).abs();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut shuffled: Vec<f64> = other.iter().take(n).copied().collect();
    let mut at_least = 0_usize;
    for _ in 0..RESAMPLINGS {
        // Fisher-Yates, so every ordering is equally likely.
        for index in (1..n).rev() {
            let swap = usize::try_from(next() % (index as u64 + 1)).unwrap_or(0);
            shuffled.swap(index, swap);
        }
        if correlation(one, &shuffled).abs() >= observed {
            at_least = at_least.saturating_add(1);
        }
    }
    f64::from(u32::try_from(at_least).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(RESAMPLINGS).unwrap_or(u32::MAX))
}

/// What the processor was actually running at, averaged over every core it
/// reports.
///
/// In gigahertz, and it is a *mean over cores* rather than the frequency of
/// the core that did the work — which is not knowable from outside without
/// following the thread. It is enough for the question being asked: whether
/// the machine's clock moved while the runs did.
fn mean_frequency() -> f64 {
    let Ok(cores) = std::fs::read_dir("/sys/devices/system/cpu") else {
        return f64::NAN;
    };
    let mut total = 0.0;
    let mut seen = 0_u32;
    for core in cores.flatten() {
        let at = core.path().join("cpufreq/scaling_cur_freq");
        let Ok(held) = std::fs::read_to_string(at) else {
            continue;
        };
        let Ok(kilohertz) = held.trim().parse::<f64>() else {
            continue;
        };
        total += kilohertz / 1_000_000.0;
        seen = seen.saturating_add(1);
    }
    if seen == 0 {
        f64::NAN
    } else {
        total / f64::from(seen)
    }
}

/// How strongly two series move together, between -1 and 1.
///
/// Ordinary linear correlation. It is used here for one narrow purpose: to ask
/// whether a run's duration tracked the machine's load, which is the operator's
/// stability criterion asked without choosing a threshold for *how much load is
/// too much*. Half is where this calls it contaminated, and that half is the
/// one number here that was chosen rather than measured — stated so, and worth
/// replacing when there is a measurement to replace it with.
fn correlation(one: &[f64], other: &[f64]) -> f64 {
    let n = one.len().min(other.len());
    if n < 3 {
        return 0.0;
    }
    let count = f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    let mean = |held: &[f64]| held.iter().take(n).sum::<f64>() / count;
    let (mean_one, mean_other) = (mean(one), mean(other));
    let mut top = 0.0;
    let mut left = 0.0;
    let mut right = 0.0;
    for at in 0..n {
        let (Some(a), Some(b)) = (one.get(at), other.get(at)) else {
            continue;
        };
        let (da, db) = (a - mean_one, b - mean_other);
        top += da * db;
        left += da * da;
        right += db * db;
    }
    if left <= 0.0 || right <= 0.0 {
        return 0.0;
    }
    top / (left * right).sqrt()
}

/// Whether the runs got faster as they went, by more than this noise
/// produces.
///
/// The other half of what DEC-007 leaves open. A machine, a cache, an engine
/// holding a model — any of them can make the first runs slower than the rest,
/// and a benchmark that averages over a warm-up reports something that happened
/// once as though it happens always.
///
/// It is asked *against the measured noise* rather than against a threshold:
/// the first quarter and last quarter are compared, and the same resampling
/// says how often a gap that size appears between two groups drawn from the
/// same timings. A gap the noise produces routinely is not a warm-up.
fn warm_up(timings: &[f64]) {
    let quarter = timings.len().wrapping_div(4).max(1);
    let Some(first) = timings.get(..quarter) else {
        return;
    };
    let Some(last) = timings.get(timings.len().saturating_sub(quarter)..) else {
        return;
    };
    let sort = |held: &[f64]| {
        let mut held = held.to_vec();
        held.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        held
    };
    let early = quantile(&sort(first), 0.5);
    let late = quantile(&sort(last), 0.5);
    let base = early.min(late);
    let gap = if base > 0.0 {
        (early - late).abs() / base
    } else {
        0.0
    };
    // How often the noise alone produces a gap this size between two groups of
    // this size. If that is common, the gap says nothing.
    let by_chance = false_alarm_rate(timings, quarter, gap.max(f64::EPSILON));

    println!();
    println!("  did the runs settle as they went");
    println!("    first {quarter} runs, median   {early:.3} s");
    println!("    last  {quarter} runs, median   {late:.3} s");
    if by_chance > FALSE_ALARMS_ALLOWED {
        println!(
            "    a gap of {:.1}% — the noise produces one that big {:.0}% of the time, so this \
             is not a warm-up",
            gap * 100.0,
            by_chance * 100.0
        );
    } else {
        println!(
            "    a gap of {:.1}% — the noise produces one that big only {:.1}% of the time, so \
             the early runs really were {}",
            gap * 100.0,
            by_chance * 100.0,
            if early > late { "slower" } else { "faster" }
        );
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
