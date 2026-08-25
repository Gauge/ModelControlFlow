//! The soak tier: sustained operation, watching for what only shows up after a
//! long time (B-191, D10, §6.34).
//!
//! D10 asks for *load and soak tests for a daemon that must run for months*.
//! There is no daemon at M0 (B-030), so what can honestly be soaked is the code
//! a daemon will spend those months in: the journal it appends to, the
//! laboratory it runs, and the temporary state both create. What this tier
//! looks for is not a wrong answer — the other tiers find those — but *drift*:
//! a descriptor never closed, a directory never removed, memory that grows with
//! the number of operations rather than with the work in flight.
//!
//! **This is not B-148.** That is the endurance scenario at M8: days of
//! simulated operation with a daemon, state migration, and a machine that
//! changes underneath. This is the part available now, and it says so rather
//! than letting a green run here read as that claim (A19).
//!
//! **Against the simulated laboratory** (§6.34), so it stays cheap enough to
//! run often and deterministic enough to believe. No real weights, no network,
//! no clock the tier has to wait on.
//!
//! **Scheduled, not gating** — `scripts/ci.sh --with-soak`, which runs it on
//! **one thread**. That is not a preference. Resident memory and open
//! descriptors are properties of the *process*, so a second test allocating in
//! parallel is read here as growth — the same mistake B35 names for timings,
//! where a reading taken under contention measures the contention. Run with the
//! harness's default parallelism this tier reported a 70 MB leak that was
//! another test's replay, which is how the constraint was found.
//!
//! **What it asserts, and what it only reports.** A descriptor leak and a
//! directory leak are counted exactly and asserted at zero. Resident memory is
//! *reported* with a generous ceiling rather than asserted tightly: an
//! allocator is free to keep what it has taken, and a tight assertion on
//! somebody else's policy is how a suite becomes flaky and then ignored (A18).
//! D24's memory ceiling is B-011's business, measured on the shipped artifact;
//! this is a leak check, and the number it prints is the evidence.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use mcf_checks::scratch::Scratch;

use mcf_core::attested::Attested;
use mcf_core::measurement::Bytes;
use mcf_core::self_cost::resident_bytes;
use mcf_core::time::{Clock as _, SimulatedClock};
use mcf_lab::{CATALOGUE, run};
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;

/// How much resident growth over a whole soak is reported without comment.
///
/// Generous on purpose: an allocator is free to keep what it has taken, so a
/// tight bound would be an assertion about somebody else's policy. It is still
/// small enough to bite — a leak of even eighty bytes per operation over a
/// hundred thousand operations clears it — and every reading taken under it is
/// printed, so a figure that crept from half a megabyte to seven is visible
/// long before it fails.
const TOLERATED_GROWTH: u64 = 8 * 1024 * 1024;

/// The number of open descriptors this process holds, where the platform says.
///
/// `None` rather than zero where it does not: A7's habit applied to a check —
/// a leak detector that reported "no leak" because it could not count would be
/// the vacuous green this whole tier is written against.
fn open_descriptors() -> Option<usize> {
    Some(std::fs::read_dir("/proc/self/fd").ok()?.count())
}

/// Reports a resident reading, or says it could not take one.
fn resident() -> Option<u64> {
    match resident_bytes() {
        Attested::Known(Bytes(bytes)) => Some(bytes),
        Attested::Unknown => None,
    }
}

fn report_growth(what: &str, before: Option<u64>, after: Option<u64>) {
    match (before, after) {
        (Some(before), Some(after)) => {
            let growth = after.saturating_sub(before);
            println!("  {what}: resident {before} → {after} bytes (+{growth})");
            assert!(
                growth < TOLERATED_GROWTH,
                "{what} grew by {growth} bytes, past the {TOLERATED_GROWTH} this tier tolerates"
            );
        }
        _ => println!(
            "  {what}: resident memory is not readable on this platform, so it is not checked"
        ),
    }
}

/// A hundred thousand entries into one journal, replayed as it goes.
///
/// The property is that a long record stays a complete record: every entry is
/// there, in order, and a replay at any point reads all of it. A journal that
/// lost its ordering or its tail after some number of appends would be the
/// silent shortening B62 forbids, arriving through duration rather than damage.
///
/// No memory claim is made here — the checkpoints hold tens of thousands of
/// entries by design, and what a *writer* costs over a long run is the next
/// test, which does not replay at all. Keeping the two apart is the same
/// discipline B35 states for timings: a reading taken while something else was
/// allocating is a reading about that.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn a_long_record_stays_a_complete_record() {
    const ENTRIES: usize = 100_000;
    const CHECKPOINT: usize = 20_000;

    let scratch = Scratch::new("soak-journal");
    let descriptors_before = open_descriptors();

    let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
    for sequence in 0..ENTRIES {
        journal
            .append(&entry(u64::try_from(sequence).unwrap_or(0)))
            .expect("an append succeeds");

        if sequence > 0 && sequence % CHECKPOINT == 0 {
            let replayed = replay(&scratch.journal()).expect("the journal replays mid-soak");
            assert!(replayed.is_complete(), "{}", replayed.statement());
            assert_eq!(replayed.entries.len(), sequence + 1);
            println!("  checkpoint at {sequence}: complete");
        }
    }
    drop(journal);

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert!(replayed.is_complete(), "{}", replayed.statement());
    assert_eq!(replayed.entries.len(), ENTRIES);
    for (sequence, held) in replayed.entries.iter().enumerate() {
        assert_eq!(
            held.body().get("cycle"),
            Some(&Value::Integer(i64::try_from(sequence).unwrap_or(-1))),
            "entry {sequence} is out of order after a long run"
        );
    }

    if let (Some(before), Some(after)) = (descriptors_before, open_descriptors()) {
        assert!(
            after <= before,
            "the soak leaked {} descriptors",
            after - before
        );
    }
}

/// A hundred thousand appends do not grow the writer.
///
/// A `Journal` holds a file handle, a count and two clock readings; nothing
/// about appending should accumulate. This is the leak check, so it replays
/// nothing: a replay returns every entry it read, which is D20's design and
/// would swamp the reading.
///
/// The replay's own footprint is measured here too, once, and **reported
/// rather than asserted** — it is proportional to the journal by construction,
/// and it is the number D6's derived index exists to stop growing (B-042,
/// B-300). Asserting on it would be asserting that MCF never keeps a record
/// long enough to matter.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn a_long_run_of_appends_does_not_grow_the_writer() {
    const ENTRIES: u64 = 100_000;

    let scratch = Scratch::new("soak-writer");
    let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
    // One append before the baseline, so whatever the first one initializes is
    // not read as growth.
    journal.append(&entry(0)).expect("an append succeeds");
    let before = resident();

    for sequence in 1..ENTRIES {
        journal
            .append(&entry(sequence))
            .expect("an append succeeds");
    }
    let after = resident();
    drop(journal);
    report_growth(
        "a hundred thousand appends, nothing replayed",
        before,
        after,
    );

    let before_replay = resident();
    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert_eq!(
        replayed.entries.len(),
        usize::try_from(ENTRIES).unwrap_or(0)
    );
    if let (Some(before), Some(after)) = (before_replay, resident()) {
        let held = after.saturating_sub(before);
        println!(
            "  a replay of {ENTRIES} entries holds {held} bytes resident, about {} per entry \
             — proportional by construction (D20), and what B-042's derived index is for",
            held.checked_div(ENTRIES).unwrap_or(0)
        );
    }
}

/// Ten thousand journals opened and closed. A descriptor held past the handle
/// that owned it is invisible until the process has done it thousands of times,
/// which is exactly what a daemon does.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn opening_and_closing_a_record_ten_thousand_times_leaks_nothing() {
    const CYCLES: usize = 10_000;

    let scratch = Scratch::new("soak-open");
    // One cycle first, so the baseline includes whatever the first open costs
    // once — a lazily initialized anything would otherwise read as a leak.
    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        journal.append(&entry(0)).expect("an append succeeds");
    }

    let Some(before) = open_descriptors() else {
        println!("  descriptors are not countable on this platform, so this is not checked");
        return;
    };
    let resident_before = resident();

    for cycle in 1..CYCLES {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal reopens");
        journal
            .append(&entry(u64::try_from(cycle).unwrap_or(0)))
            .expect("an append succeeds");
    }

    let after = open_descriptors().expect("descriptors were countable a moment ago");
    println!("  {CYCLES} open-append-close cycles: {before} descriptors → {after}");
    assert_eq!(
        after,
        before,
        "{CYCLES} cycles left {} descriptors behind",
        after.saturating_sub(before)
    );
    report_growth("ten thousand journal cycles", resident_before, resident());

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert_eq!(replayed.entries.len(), CYCLES);
}

fn entry(sequence: u64) -> Entry {
    Entry::new(
        EntryKind::SelfCost,
        mcf_core::time::Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
        sequence,
        Value::map([(
            "cycle",
            Value::Integer(i64::try_from(sequence).unwrap_or(-1)),
        )]),
    )
}

/// The laboratory, run for a long time, leaves nothing behind.
///
/// A27 applies to MCF's own suite: every scenario builds a directory and every
/// `World` removes it. Run once that is invisible; run twenty thousand times it
/// is either invisible or it is a full disk on a machine that runs the tier
/// nightly. The scratch directories are counted rather than trusted.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn twenty_thousand_scenario_runs_leave_nothing_behind() {
    const ROUNDS: usize = 20_000;

    let before = laboratory_directories();
    let descriptors_before = open_descriptors();
    let resident_before = resident();

    let mut produced = 0usize;
    for round in 0..ROUNDS {
        let scenario = &CATALOGUE[round % CATALOGUE.len()];
        if run(scenario).matches(scenario.produces) {
            produced += 1;
        }
    }
    assert_eq!(
        produced, ROUNDS,
        "a scenario stopped producing its category"
    );

    let after = laboratory_directories();
    println!("  {ROUNDS} scenario runs: {before} laboratory directories → {after}");
    assert!(
        after <= before,
        "the laboratory left {} directories behind",
        after.saturating_sub(before)
    );
    if let (Some(before), Some(after)) = (descriptors_before, open_descriptors()) {
        assert!(
            after <= before,
            "the laboratory leaked {} descriptors",
            after - before
        );
    }
    report_growth("twenty thousand scenario runs", resident_before, resident());
}

/// How many laboratory scratch directories are lying about.
fn laboratory_directories() -> usize {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("mcf-lab-"))
        .count()
}

/// Thirty simulated days of the clock a laboratory runs on.
///
/// The lab's clock is supplied rather than waited on (D26), so a month of it
/// costs nothing — which is the whole reason §6.34 puts soak against the
/// simulated laboratory. What is checked is that a clock advanced by a month in
/// small steps arrives where the arithmetic says: a simulated clock that
/// drifted would make every deadline scenario a different scenario after a long
/// run.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn a_month_of_simulated_time_arrives_where_the_arithmetic_says() {
    const STEP_NANOS: u64 = 60 * 1_000_000_000;
    const STEPS: u64 = 30 * 24 * 60;

    let clock = SimulatedClock::new();
    let start = clock.now();
    for _ in 0..STEPS {
        clock.advance(STEP_NANOS);
    }
    let elapsed = clock.now().saturating_duration_since(start);
    assert_eq!(elapsed.as_nanos(), STEPS * STEP_NANOS);
    println!(
        "  {STEPS} one-minute steps: {} simulated days elapsed",
        elapsed
            .as_nanos()
            .checked_div(24 * 60 * 60 * 1_000_000_000)
            .unwrap_or(0)
    );
}
