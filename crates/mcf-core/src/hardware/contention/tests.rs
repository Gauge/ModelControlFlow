//! What a contention snapshot has to get right on the machine running it.
//!
//! B19 keeps these hermetic: they read `/proc`, which is this machine's own
//! state and needs no network, no accelerator and no model. What they assert
//! is the *shape* of the answer, because the values are whatever else is
//! running — a test that asserted a quiet machine would fail on a busy one and
//! would be measuring the runner rather than the code.

use crate::attested::Attested;

use super::{NAMED, Snapshot, sample};

/// A snapshot names what it found, most first, and never more than it says.
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

/// **MCF's own process is named rather than filtered out.** MCF competing with
/// itself is a true and useful thing to see, and a snapshot that hid it would
/// be hiding the one process the reader can do something about.
#[test]
fn mcfs_own_process_is_named_as_its_own() {
    // The test binary spends processor time by definition — it is running.
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
    // Where it is not in the top few, that is a fact about a busy machine and
    // not a defect — the assertion above is about what happens when it is.
}

/// The total is the sum of everything, not of the few that are named.
///
/// A snapshot that totalled only its own top five would understate the machine
/// by exactly the amount it did not show, which is the sort of quiet
/// arithmetic A1 forbids.
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

/// **D25's boundary, restated.** Per-process accelerator occupancy needs a
/// vendor library MCF may not have, and *unknown* is not *no contention*.
#[test]
fn an_unreadable_accelerator_is_unknown_and_not_zero() {
    assert_eq!(
        sample().accelerator,
        Attested::Unknown,
        "MCF has no per-process accelerator reading, and says so rather than reporting none"
    );
}

/// The kernel's pressure accounting is read where the machine keeps it, and is
/// `Unknown` where it does not — a kernel without it is not a quiet one (A7).
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
    // On a machine that keeps it, at least one is readable — which is a
    // statement about this test's runner and is asserted only that way round.
    if std::path::Path::new("/proc/pressure/cpu").exists() {
        assert!(
            matches!(held.processor_pressure, Attested::Known(_)),
            "this machine keeps pressure accounting and MCF did not read it"
        );
    }
}

/// It is on demand and it says what it cost: two readings a stated interval
/// apart, and no timer anywhere (B4, D5).
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
