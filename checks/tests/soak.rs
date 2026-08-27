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

/// An idle daemon costs nothing, measured rather than asserted by design
/// (B-031, B-004, D24, §3.13).
///
/// D24 states two of its figures as prohibitions rather than thresholds, and
/// this is one: **zero timer wakeups while idle**. The daemon's shape is what
/// makes it true — it blocks in `accept` and has no tick, no poll and no
/// watcher — and a shape is a claim until something measures it. This runs a
/// real daemon for a minute with nobody talking to it and reads three things
/// the kernel keeps:
///
/// * the processor time it used, which should be indistinguishable from none;
/// * its context switches, voluntary and involuntary, which count the times it
///   was scheduled at all;
/// * the record, which must be byte-for-byte what it was — B-004's condition is
///   *writes zero records*, and a daemon that logged a heartbeat would fail
///   here rather than in review.
///
/// A minute is D24's own window. It is long, and this is the tier for long.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn an_idle_daemon_costs_nothing_for_a_minute() {
    use std::io::BufRead as _;

    let scratch = Scratch::new("idle-daemon");
    let journal = scratch.path().join("mcf").join("record.jsonl");
    // A record with something in it, so that "unchanged" is a claim about a
    // file with content rather than about an absence.
    {
        let mut writing = Journal::open(&journal).expect("a journal opens");
        writing.append(&entry(0)).expect("it appends");
    }

    // The binary this workspace built, found by path rather than by
    // `CARGO_BIN_EXE_*`: that variable exists only for a crate's own tests, and
    // the tier this belongs to lives in the checks crate. A machine that has
    // not built one reports that it could not measure, which is not a pass.
    let Some(binary) = the_built_binary() else {
        println!("no mcf binary is built; the idle claim stands unmeasured");
        return;
    };
    let mut daemon = std::process::Command::new(&binary)
        .arg("serve")
        .env("XDG_DATA_HOME", scratch.path())
        .env("XDG_RUNTIME_DIR", scratch.path())
        .env_remove("HOME")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the daemon starts");

    // Wait until it says it is up, so the minute is a minute of *idling*
    // rather than of starting.
    {
        let stdout = daemon.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up"), "{line}");
    }

    let pid = daemon.id();
    let before_record = std::fs::read(&journal).expect("the record is readable");
    let before_switches = context_switches(pid);
    std::thread::sleep(std::time::Duration::from_secs(60));
    let after_switches = context_switches(pid);
    let after_record = std::fs::read(&journal).expect("the record is readable");
    let used = processor_time(pid);

    let stop = std::process::Command::new(&binary)
        .args(["stop", "--because", "the idle measurement is done"])
        .env("XDG_DATA_HOME", scratch.path())
        .env("XDG_RUNTIME_DIR", scratch.path())
        .env_remove("HOME")
        .status();
    let _ended = daemon.wait();
    assert!(
        stop.is_ok_and(|status| status.success()),
        "it would not stop"
    );

    assert_eq!(
        before_record, after_record,
        "the record changed while the daemon was idle: B-004's condition is that an idle \
         daemon writes zero records"
    );
    match (before_switches, after_switches) {
        (Some(before), Some(after)) => {
            let woken = after.saturating_sub(before);
            println!("  idle daemon: {woken} context switches over 60 s");
            assert!(
                woken <= 2,
                "the daemon was scheduled {woken} times while idle, which is a timer somewhere \
                 (D24, §3.13)"
            );
        }
        _ => println!("  this platform does not publish context switches; unmeasured"),
    }
    match used {
        Some(ticks) => {
            println!("  idle daemon: {ticks} clock ticks of processor time over 60 s");
            assert!(
                ticks <= 2,
                "an idle daemon used {ticks} ticks of processor time, which is work nobody \
                 asked for"
            );
        }
        None => println!("  this platform does not publish processor time; unmeasured"),
    }
}

/// An idle daemon with a model resident costs nothing for a minute (D41, M2's
/// third exit criterion, §3.13).
///
/// The same measurement as the idle daemon's, taken after one generation has
/// loaded the laboratory's fixture and left it resident. What must be true is
/// that residency is memory and nothing else: no timer to unload, no watcher,
/// no tick — the processor time and context switches over the minute are read
/// as deltas from after the generation, and the record must not move.
#[test]
#[ignore = "the soak tier is scheduled: scripts/ci.sh --with-soak (B38)"]
fn an_idle_daemon_with_a_model_resident_costs_nothing_for_a_minute() {
    use std::io::BufRead as _;

    let scratch = Scratch::new("idle-resident");
    let store = scratch
        .path()
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&store).expect("a store");
    std::fs::write(
        store.join("a-model-that-runs.gguf"),
        mcf_lab::fixture::a_model_that_runs(),
    )
    .expect("the fixture written");
    let journal = scratch.path().join("mcf").join("record.jsonl");

    let Some(binary) = the_built_binary() else {
        println!("no mcf binary is built; the resident-idle claim stands unmeasured");
        return;
    };
    let mut daemon = std::process::Command::new(&binary)
        .arg("serve")
        .env("XDG_DATA_HOME", scratch.path())
        .env("XDG_RUNTIME_DIR", scratch.path())
        .env_remove("HOME")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the daemon starts");
    {
        let stdout = daemon.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up"), "{line}");
    }

    // One generation, so that a model is resident for the minute.
    let ran = std::process::Command::new(&binary)
        .args([
            "run",
            "lab/fixture:a-model-that-runs.gguf",
            "--prompt",
            "yes",
            "--limit",
            "2",
        ])
        .env("XDG_DATA_HOME", scratch.path())
        .env("XDG_RUNTIME_DIR", scratch.path())
        .env_remove("HOME")
        .output()
        .expect("the client runs");
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(said.contains("model loaded loaded"), "{said}");

    let pid = daemon.id();
    let before_record = std::fs::read(&journal).expect("the record is readable");
    let before_switches = context_switches(pid);
    let before_ticks = processor_time(pid);
    std::thread::sleep(std::time::Duration::from_secs(60));
    let after_switches = context_switches(pid);
    let after_ticks = processor_time(pid);
    let after_record = std::fs::read(&journal).expect("the record is readable");

    let stop = std::process::Command::new(&binary)
        .args(["stop", "--because", "the resident-idle measurement is done"])
        .env("XDG_DATA_HOME", scratch.path())
        .env("XDG_RUNTIME_DIR", scratch.path())
        .env_remove("HOME")
        .status();
    let _ended = daemon.wait();
    assert!(
        stop.is_ok_and(|status| status.success()),
        "it would not stop"
    );

    assert_eq!(
        before_record, after_record,
        "the record changed while the daemon idled with a model resident"
    );
    if let (Some(before), Some(after)) = (before_switches, after_switches) {
        let woken = after.saturating_sub(before);
        println!("  idle daemon, model resident: {woken} context switches over 60 s");
        assert!(
            woken <= 2,
            "scheduled {woken} times while idle with a model resident: a timer (D41, §6.9)"
        );
    } else {
        println!("  this platform does not publish context switches; unmeasured");
    }
    if let (Some(before), Some(after)) = (before_ticks, after_ticks) {
        let ticks = after.saturating_sub(before);
        println!("  idle daemon, model resident: {ticks} clock ticks over 60 s");
        assert!(
            ticks <= 2,
            "used {ticks} ticks while idle with a model resident"
        );
    } else {
        println!("  this platform does not publish processor time; unmeasured");
    }
}

/// The `mcf` binary this workspace built, debug or release.
fn the_built_binary() -> Option<std::path::PathBuf> {
    let root = mcf_checks::workspace::root();
    ["debug", "release"]
        .into_iter()
        .map(|profile| root.join("target").join(profile).join("mcf"))
        .find(|path| path.is_file())
}

/// How many times a process has been scheduled, voluntarily or not.
fn context_switches(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut total = 0_u64;
    for line in status.lines() {
        if line.starts_with("voluntary_ctxt_switches:")
            || line.starts_with("nonvoluntary_ctxt_switches:")
        {
            total = total.saturating_add(line.split_whitespace().nth(1)?.parse::<u64>().ok()?);
        }
    }
    Some(total)
}

/// How much processor time a process has used, in the platform's own ticks.
fn processor_time(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name can contain spaces and parentheses, so the fields after
    // it are found from the last `)` rather than by splitting the whole line.
    let after_name = stat.rsplit_once(american_paren())?.1;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    // utime and stime are the 12th and 13th fields after the state.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

const fn american_paren() -> char {
    ')'
}
