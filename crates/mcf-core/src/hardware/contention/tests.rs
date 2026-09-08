use crate::attested::Attested;

use super::{NAMED, Snapshot, sample};

#[test]
fn it_names_the_busiest_and_no_more_than_it_says() {
    let held: Snapshot = sample();
    assert!(
        held.competitors.len() <= NAMED,
        "a list of everything is a list nobody reads: {}",
        held.competitors.len()
    );
    for pair in held.competitors.windows(2) {
        let (Some(one), Some(other)) = (pair.first(), pair.last()) else {
            continue;
        };
        assert!(
            one.cores_taken >= other.cores_taken,
            "the busiest first, so a reader stops at the top"
        );
    }
}

#[test]
fn mcfs_own_process_is_named_as_its_own() {
    let mut spinning = 0_u64;
    let until = std::time::Instant::now() + super::OVER;
    let held = std::thread::spawn(move || {
        while std::time::Instant::now() < until {
            spinning = spinning.wrapping_add(1);
        }
        spinning
    });
    let snapshot = sample();
    let _spun = held.join();

    let ours = std::process::id();
    let mine = snapshot.competitors.iter().find(|held| held.pid == ours);
    if let Some(mine) = mine {
        assert!(
            mine.is_mcf,
            "MCF's own process must be marked as its own, not left to be guessed at"
        );
        assert!(
            format!("{mine}").contains("this is MCF"),
            "and say so where it is rendered: {mine}"
        );
    }
}

#[test]
fn the_total_counts_what_is_not_named() {
    let held = sample();
    let named: u64 = held
        .competitors
        .iter()
        .fold(0, |sum, one| sum.saturating_add(one.cores_taken));
    assert!(
        held.cores_taken >= named,
        "the total ({}) must count processes the list did not name ({named})",
        held.cores_taken
    );
}

#[test]
fn an_unreadable_accelerator_is_unknown_and_not_zero() {
    assert_eq!(
        sample().accelerator,
        Attested::Unknown,
        "MCF has no per-process accelerator reading, and says so rather than reporting none"
    );
}

#[test]
fn pressure_is_read_or_reported_absent() {
    let held = sample();
    for (named, value) in [
        ("processor", held.processor_pressure),
        ("memory", held.memory_pressure),
        ("storage", held.storage_pressure),
    ] {
        match value {
            Attested::Known(held) => assert!(
                held <= 1_000_000,
                "{named} pressure is a fraction of the window: {held} ppm"
            ),
            Attested::Unknown => {}
        }
    }
    if std::path::Path::new("/proc/pressure/cpu").exists() {
        assert!(
            matches!(held.processor_pressure, Attested::Known(_)),
            "this machine keeps pressure accounting and MCF did not read it"
        );
    }
}

#[test]
fn it_costs_the_interval_it_states_and_no_more() {
    let began = std::time::Instant::now();
    let _held = sample();
    let took = began.elapsed();
    assert!(
        took >= super::OVER,
        "a rate needs two readings an interval apart: {took:?}"
    );
    assert!(
        took < super::OVER.saturating_mul(20),
        "and reading /proc twice must not dominate the interval: {took:?}"
    );
}

#[test]
fn steadiness_is_measured_against_the_machines_own_middle() {
    let held = super::steadiness(2);
    assert_eq!(held.readings, 2, "two is the fewest that can show a spread");
    if held.middle > 0 {
        assert!(
            held.spread.is_some(),
            "a machine with something competing has a baseline to be steady against"
        );
    }
    let text = format!("{held}");
    assert!(
        text.contains("competing") || text.contains("no baseline"),
        "{text}"
    );
}

#[test]
fn nothing_competing_has_no_baseline_rather_than_a_perfect_one() {
    let held = super::Steadiness {
        middle: 0,
        spread: None,
        readings: 3,
    };
    assert!(
        format!("{held}").contains("no baseline"),
        "a zero baseline must not read as perfect steadiness: {held}"
    );
}

#[test]
fn one_reading_is_raised_to_two() {
    assert_eq!(super::steadiness(0).readings, 2);
    assert_eq!(super::steadiness(1).readings, 2);
}

#[test]
fn it_reports_a_number_and_no_verdict() {
    let held = super::steadiness(2);
    let said = format!("{held}");
    for judgement in ["too ", "quiet", "unusable", "refus"] {
        assert!(
            !said.contains(judgement),
            "no rendering here may judge a spread — what is too much is DEC-007's open band: \
             `{judgement}` in {said}"
        );
    }
}

#[test]
fn the_interval_is_measured_rather_than_assumed() {
    let source = include_str!("../contention.rs");
    assert!(
        source.contains("saturating_duration_since(opened)"),
        "the window a rate is divided by must be the one that elapsed, not the one that was \
         intended: `/proc` takes time to walk and that time is inside the window"
    );
    assert!(
        !source.contains("OVER.as_millis()"),
        "dividing by the intended interval is the defect itself, and it reports figures the \
         machine cannot physically produce"
    );
}

#[test]
fn no_snapshot_reports_more_cores_than_the_machine_has() {
    let held = super::sample();
    let ceiling = u64::try_from(std::thread::available_parallelism().map_or(1, Into::into))
        .unwrap_or(u64::MAX)
        .saturating_mul(1_000);
    let slack = ceiling.saturating_add(ceiling.wrapping_div(10));
    assert!(
        held.cores_taken <= slack,
        "reported {} thousandths of a core on a machine with {ceiling} — a reading above the \
         ceiling is a broken instrument, not a busy machine",
        held.cores_taken
    );
}
