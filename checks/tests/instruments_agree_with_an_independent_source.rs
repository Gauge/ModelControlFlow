//! Every measuring instrument, against something that is not itself
//! (A19, §6.16, F94).
//!
//! **The gap this closes.** A19 — *anything reported is tested against an
//! independently known value* — is cited in fifty-eight files: the digest
//! against published vectors, the tokenizer against reference output,
//! dequantization against `llama.cpp`, time arithmetic against known dates.
//! It was cited in **none** of the modules that measure the machine.
//!
//! MCF ships `mcf cross-check`, which compares its own inference engine
//! against an independently provisioned reference across a hundred and twenty
//! positions, because F40 found two engines parting at step four. That
//! discipline was applied to the engine and to nothing that measures.
//!
//! Four instrument defects followed, and in every case an independent source
//! existed and was never consulted: `/proc/stat` for contention (F90), a
//! second sensor directory for temperature (F91), an independent computation
//! of a textbook interval for the effect size (F92), and the record itself for
//! the instrument's identity (F93).
//!
//! **Scheduled rather than gating**, because these need the real machine and
//! two of them need load. `scripts/ci.sh --with-instruments` runs them.
//!
//! **A disagreement is a finding, not a crash.** Each check reports both
//! values and their difference in full, because *the instrument is wrong* is
//! not actionable and *the instrument reads 35.2 where the kernel reads 28.9*
//! is. MCF keeps measuring: an instrument that disagrees with its reference
//! has told you something about itself, which is what A9 makes a result.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::process::Command;

/// Whether this tier was asked for.
///
/// Absent, every check here reports what it would have done and passes: a
/// scheduled tier that fails when it is not scheduled is a gating tier
/// wearing the wrong name.
fn scheduled() -> bool {
    std::env::var("MCF_WITH_INSTRUMENTS").is_ok()
}

/// The kernel's own processor accounting: busy jiffies over the interval.
///
/// Independent of MCF's per-process summation in every way that matters — a
/// different file, a different accounting path, and a quantity the kernel
/// maintains for its own purposes. And it cannot exceed the core count, which
/// is what made F90's 35.2 cores on a 32-thread machine visible at all.
fn kernel_busy() -> (u64, u64) {
    let text = std::fs::read_to_string("/proc/stat").expect("/proc/stat is readable");
    let line = text.lines().next().expect("/proc/stat has a first line");
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|held| held.parse().ok())
        .collect();
    let total: u64 = fields.iter().sum();
    let idle = fields.get(3).copied().unwrap_or(0) + fields.get(4).copied().unwrap_or(0);
    (total, idle)
}

/// **F90's check.** MCF's contention reading against the kernel's own.
#[test]
fn contention_agrees_with_the_kernel() {
    if !scheduled() {
        println!("not scheduled: `scripts/ci.sh --with-instruments` runs this");
        return;
    }
    let (before, idle_before) = kernel_busy();
    let held = mcf_core::hardware::contention();
    let (after, idle_after) = kernel_busy();

    let ticks = 100_u64;
    let busy =
        (after.saturating_sub(before)).saturating_sub(idle_after.saturating_sub(idle_before));
    let kernel = busy
        .saturating_mul(1_000)
        .saturating_mul(1_000)
        .wrapping_div(ticks.saturating_mul(held.over_millis.max(1)));

    // The physical ceiling first, because a reading above it is not a busy
    // machine but a broken instrument — and that is exactly what F90 was.
    let cores = u64::try_from(std::thread::available_parallelism().map_or(1, Into::into))
        .unwrap_or(1)
        .saturating_mul(1_000);
    assert!(
        held.cores_taken <= cores.saturating_add(cores.wrapping_div(10)),
        "MCF reports {} thousandths of a core on a machine with {cores}; the kernel says \
         {kernel}. A reading above the ceiling is a broken instrument (F90)",
        held.cores_taken
    );

    // Then agreement. MCF sums processes present in both readings and so
    // undercounts process churn and kernel time that belongs to no process;
    // the kernel counts everything. A quarter is generous for that and far
    // tighter than the 22% by which F90's defect over-read.
    let (low, high) = (held.cores_taken.min(kernel), held.cores_taken.max(kernel));
    let apart = high
        .saturating_sub(low)
        .saturating_mul(100)
        .wrapping_div(high.max(1));
    assert!(
        apart <= 25,
        "MCF reads {} thousandths of a core over {} ms and the kernel reads {kernel} — {apart}% \
         apart. Both are counting the same processor over the same window (A19, F90)",
        held.cores_taken,
        held.over_millis
    );
}

/// **F91's check.** A processor die gets hotter when the processor works.
///
/// Independent of the sensor's own claims about itself: it is a physical
/// prediction that a wrong sensor, a board zone read as a die, or a stale
/// value will all fail. This is what would have caught an ACPI zone reading
/// 16.8 °C being taken for a processor at 70 °C.
#[test]
fn a_processor_sensor_responds_to_the_processor() {
    if !scheduled() {
        println!("not scheduled: `scripts/ci.sh --with-instruments` runs this");
        return;
    }
    let sensors = mcf_core::hardware::thermal::sensors();
    let Some(before) = mcf_core::hardware::thermal::processor(&sensors) else {
        println!("this machine publishes no processor sensor; nothing to cross-check (A7)");
        return;
    };
    let cool = before.millidegrees;

    // Enough work to move a die, on every core, without lasting long enough
    // to matter to anybody using the machine.
    let mut hands = Vec::new();
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    for _ in 0..std::thread::available_parallelism().map_or(1, Into::into) {
        let flag = std::sync::Arc::clone(&running);
        hands.push(std::thread::spawn(move || {
            let mut held: u64 = 1;
            while flag.load(std::sync::atomic::Ordering::Relaxed) {
                held = held.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            }
            held
        }));
    }
    std::thread::sleep(std::time::Duration::from_secs(8));
    let warmed = mcf_core::hardware::thermal::sensors();
    let hot = mcf_core::hardware::thermal::processor(&warmed).map(|held| held.millidegrees);
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    for hand in hands {
        drop(hand.join());
    }

    let Some(hot) = hot else {
        panic!("a sensor that was there before the load is not there after it");
    };
    assert!(
        hot > cool,
        "the processor sensor read {cool} m°C idle and {hot} m°C after eight seconds of work on \
         every core. A die that does not warm when the die works is not a die sensor — which is \
         the mistake F91 corrected, an ACPI zone at 16.8 °C taken for a processor at 70 °C"
    );
}

/// **F92's check.** The interval's coverage, against brute-force enumeration.
///
/// The implementation computes coverage from a binomial tail in closed form.
/// This computes it by enumerating every one of the `2^n` sign patterns and
/// counting how many put the true median inside the bracket — the definition
/// itself, with no algebra in it. An off-by-one in the tail index is exactly
/// what this catches, and exactly what the first implementation had.
///
/// Needs no machine and no load, so it runs here rather than in the tier.
#[test]
fn the_intervals_coverage_matches_a_brute_force_count() {
    for n in 6_u32..=16 {
        // The widest k whose coverage reaches the standard, by enumeration.
        let total = 1_u64 << n;
        let mut expected = None;
        for k in 1..=n.wrapping_div(2) {
            let mut inside = 0_u64;
            for pattern in 0..total {
                let ahead = pattern.count_ones();
                // The bracket [d_(k), d_(n+1-k)] misses the median exactly
                // when fewer than k, or more than n-k, of the differences are
                // above it.
                if ahead >= k && ahead <= n.saturating_sub(k) {
                    inside += 1;
                }
            }
            let coverage = inside.saturating_mul(1_000_000).wrapping_div(total);
            if coverage >= 950_000 {
                expected = Some(coverage);
            }
        }
        let differences: Vec<i64> = (0..n).map(|at| 100_000 + i64::from(at)).collect();
        let held = mcf_bench::enough::spread_of(&differences);
        match (expected, held) {
            (Some(want), Some(got)) => assert_eq!(
                got.coverage.0, want,
                "at {n} pairs MCF states {} coverage and enumerating all {total} sign patterns \
                 gives {want} (A19, F92)",
                got.coverage.0
            ),
            (None, None) => {}
            (want, got) => panic!(
                "at {n} pairs enumeration says {want:?} and MCF says {:?}",
                got.map(|held| held.coverage.0)
            ),
        }
    }
}

/// **The instrument identity, against the record.** F93's check.
#[test]
fn the_running_binary_identifies_itself() {
    let held = mcf_core::build_identity::instrument();
    assert!(
        matches!(held, mcf_core::attested::Attested::Known(_)),
        "MCF could not read its own executable, so every measurement it records is attributed \
         to an instrument nobody can identify — which is the whole of F93"
    );
}

/// Occupancy against the vendor's own tool, where one is installed.
#[test]
fn accelerator_occupancy_agrees_with_the_vendor() {
    if !scheduled() {
        println!("not scheduled: `scripts/ci.sh --with-instruments` runs this");
        return;
    }
    let cards = mcf_core::hardware::utilisation::accelerators();
    let Ok(shown) = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        println!("no nvidia-smi here; nothing to cross-check against (A7)");
        return;
    };
    if !shown.status.success() {
        println!("nvidia-smi is present and did not answer; that is its own finding");
        return;
    }
    // MCF reads NVIDIA occupancy through NVML rather than sysfs, and reports
    // `unknown` in this list by design. What is checked is that it does not
    // claim a figure sysfs cannot give it.
    for card in cards.iter().filter(|held| held.driver == "nvidia") {
        assert!(
            matches!(card.percent, mcf_core::attested::Attested::Unknown),
            "{card} claims an occupancy sysfs does not publish for this driver"
        );
        assert!(
            card.because.is_some(),
            "{card} is unknown for no stated reason (A7)"
        );
    }
}

/// The laboratory's reading agrees with counting the attempts by hand.
///
/// **The independent source is arithmetic a reader can do.** Everything else in
/// this file cross-checks against the kernel or a vendor tool, because what it
/// measures is the machine. A laboratory measures a model, and there is no
/// second instrument to ask — so what is checked is that the reading is the one
/// the definition gives, over cases whose answer is stated here rather than
/// computed by the code under test (A19).
///
/// What would go wrong without it is B40's failure with the sign flipped: a
/// reading that quietly counted *cases* rather than *whole answers* would give
/// a model partial credit nobody defined, and it would sort above one that got
/// fewer things entirely right.
/// On a machine with a Radeon, the profiler's temperature for it is the
/// thermal module's, read a second way: through the hwmon class rather than
/// the card's own directory. On a machine without one there is nothing to
/// compare, and that is said rather than passed silently (A7, A19).
#[test]
fn a_radeons_temperature_agrees_with_the_thermal_module() {
    let machine = mcf_core::hardware::Machine::read();
    let Some(card) = machine
        .accelerators
        .iter()
        .find(|held| held.routes().contains(&"amdgpu-files"))
    else {
        println!("no card the amdgpu driver drives is here; nothing to cross-check against (A7)");
        return;
    };
    let mcf_core::attested::Attested::Known(degrees) = card.reading().temperature_c else {
        panic!("a card here whose temperature the driver does not publish: {card}");
    };
    let sensors = mcf_core::hardware::thermal::sensors();
    let Some(theirs) = sensors.iter().find(|sensor| sensor.chip == "amdgpu") else {
        panic!("the thermal module does not read the chip the profiler reads: {sensors:?}");
    };
    let difference = (i64::from(degrees) * 1000 - theirs.millidegrees).abs();
    assert!(
        difference <= 1000,
        "the profiler read {degrees} °C and the thermal module {} m°C for the same chip",
        theirs.millidegrees
    );
}

#[test]
fn a_laboratory_reading_is_the_share_of_attempts_that_were_whole() {
    use mcf_bench::eval::{Ran, Trials};

    // Three attempts, one of them entirely right. Counted by hand: one in
    // three, which is 333,333 parts per million after the division truncates.
    let held = Trials {
        task: "a-task",
        attempts: vec![
            Ran::Checked { passed: 3, of: 3 },
            Ran::Checked { passed: 2, of: 3 },
            Ran::Refused {
                because: "timed out".to_owned(),
                wrote_something: true,
            },
        ],
    };
    assert_eq!(held.ran(), 2, "two attempts ran");
    assert_eq!(held.whole(), 1, "one of them satisfied every case");
    let graded = held.graded();
    let score = graded
        .score()
        .expect("something ran, so there is a reading");
    assert_eq!(
        score.parts_per_million(),
        333_333,
        "the reading is whole answers over attempts made — not cases over cases, which \
         would be 5 of 9 and would give partial credit nobody defined (B40)"
    );

    // And the denominator is attempts made rather than attempts that ran: a
    // model whose answers often fail to run is a model that often fails, and
    // dividing by the ones that ran would hide exactly that.
    let ran_only = Trials {
        task: "a-task",
        attempts: vec![Ran::Checked { passed: 3, of: 3 }],
    };
    assert_eq!(
        ran_only
            .graded()
            .score()
            .expect("it ran")
            .parts_per_million(),
        1_000_000,
        "one attempt, whole, is the whole of the scale"
    );
}
