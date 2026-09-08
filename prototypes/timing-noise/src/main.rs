use mcf_core::time::Monotonic;
use std::process::{Command, Stdio};
use std::time::Instant;

const EFFECTS: [f64; 4] = [0.02, 0.05, 0.10, 0.20];

const FALSE_ALARMS_ALLOWED: f64 = 0.05;

const RESAMPLINGS: usize = 4000;

fn main() -> std::process::ExitCode {
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

    let seed = 0x5DEE_CE66_D125_u64;
    println!("  order seed: {seed:#x}");
    let left_arm = mcf_core::trial::Arm::new(one.join(" "));
    let right_arm = mcf_core::trial::Arm::new(other.join(" "));
    let mut running = mcf_bench::compare::Interleaving::<Monotonic>::new(
        as_configuration(&left_arm),
        as_configuration(&right_arm),
        mcf_core::trial::SessionId::new(format!("timing-noise-{}", std::process::id())),
        seed,
        mcf_bench::compare::Discipline::Timing { seed, tokens: 0 },
    );

    let mut failed = false;
    for round in 0..ceiling {
        let _ran = running.round(|arm, _drew| {
            let command = if *arm == left_arm { one } else { other };
            if let Some(seconds) = timed(command) {
                Some((
                    mcf_core::time::Duration::from_nanos(whole(seconds * 1e9)),
                    mcf_bench::warmth::Warmth::Unstated,
                ))
            } else {
                failed = true;
                None
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

fn report_comparison(
    said: &mcf_bench::compare::Finding,
    held: &mcf_bench::compare::Comparison<Monotonic>,
) {
    println!("    {said}");
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

fn middle(held: &[f64]) -> f64 {
    let mut out = held.to_vec();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    quantile(&out, 0.5)
}

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

fn false_alarm_rate(timings: &[f64], each: usize, effect: f64) -> f64 {
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
