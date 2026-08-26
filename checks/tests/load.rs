//! The load tier: MCF's own code under concurrency, against the simulated
//! laboratory (B-191, D10, §6.34).
//!
//! §6.34 settles what load runs against: *the simulated laboratory, not real
//! weights, so it stays cheap enough to run often and deterministic enough to
//! believe*. At M0 that means the two things a machine can do many of at once —
//! reproduce failures, and write records — and the question is whether they
//! still mean what they mean when thirty-two of them are happening.
//!
//! **What this tier asserts is correctness, never speed.** A18 keeps tests and
//! benchmarks apart: a test has a pass condition and a benchmark has none, and
//! a throughput assertion in a suite is how suites become flaky and then
//! ignored. So nothing below times anything. What it asserts is that the
//! laboratory's determinism (B27) and the journal's completeness (B62) are
//! properties of the code rather than of there having been only one caller.
//!
//! **Scheduled, not gating** — `scripts/ci.sh --with-load`. It runs the whole
//! fault catalogue on every core.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use mcf_checks::scratch::Scratch;

use mcf_core::attested::Attested;
use mcf_core::time::Timestamp;
use mcf_lab::{CATALOGUE, Outcome, run};
use mcf_record::export;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;

/// How many workers the tier runs. Twice the reported parallelism, so that
/// threads genuinely contend rather than each getting a core to itself —
/// contention is the condition being tested.
fn workers() -> usize {
    thread::available_parallelism()
        .map_or(8, |count| count.get() * 2)
        .min(64)
}

/// B27: determinism is a property of the laboratory, not of the world. Every
/// scenario, on every worker, at the same time — and each one still produces
/// the category it declares.
///
/// The failure this exists to catch is a scenario that shares something: a
/// fixed path, a static, an environment variable. Run one at a time it looks
/// deterministic; run thirty-two at once and it is not, and every fault-
/// injection result MCF reports would then be a result about the scheduler.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn every_scenario_produces_its_category_on_every_worker_at_once() {
    let workers = workers();
    let rounds = 40;
    let mismatches = Arc::new(AtomicUsize::new(0));
    let ran = Arc::new(AtomicUsize::new(0));

    thread::scope(|scope| {
        for _ in 0..workers {
            let mismatches = Arc::clone(&mismatches);
            let ran = Arc::clone(&ran);
            let _worker = scope.spawn(move || {
                for _ in 0..rounds {
                    for scenario in CATALOGUE {
                        let outcome = run(scenario);
                        ran.fetch_add(1, Ordering::Relaxed);
                        if !outcome.matches(scenario.produces) {
                            mismatches.fetch_add(1, Ordering::Relaxed);
                            println!("  {} produced {outcome} under load", scenario.id);
                        }
                    }
                }
            });
        }
    });

    let ran = ran.load(Ordering::Relaxed);
    println!(
        "  {ran} scenario runs across {workers} workers ({} scenarios × {rounds} rounds)",
        CATALOGUE.len()
    );
    assert_eq!(ran, workers * rounds * CATALOGUE.len());
    assert_eq!(
        mismatches.load(Ordering::Relaxed),
        0,
        "a scenario stopped producing its category under load"
    );
}

/// The same scenario, run concurrently, produces the identical outcome every
/// time — not merely the right category.
///
/// `mcf_lab::repeat` asserts this serially. Under load it is the stronger
/// claim: §3.17's *a failure found once reproduces exactly, forever* has to
/// survive the machine being busy, or a reproduction is a coin toss.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn a_concurrent_reproduction_is_identical_to_a_solitary_one() {
    let workers = workers();
    for scenario in CATALOGUE {
        let alone = run(scenario);
        let divergences = Arc::new(AtomicUsize::new(0));
        thread::scope(|scope| {
            for _ in 0..workers {
                let divergences = Arc::clone(&divergences);
                let alone = alone.clone();
                let _worker = scope.spawn(move || {
                    for _ in 0..16 {
                        if run(scenario) != alone {
                            divergences.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });
        assert_eq!(
            divergences.load(Ordering::Relaxed),
            0,
            "{} diverged under load from what it produces alone: {alone}",
            scenario.id
        );
    }
}

/// Many records at once, each complete. D20 makes the journal the record, and
/// a record that lost entries when the machine was busy would be the silent
/// shortening B62 forbids.
///
/// One journal per worker, because who writes to *one* journal is DEC-037 and
/// it is open (B-332). This tier tests what MCF has decided, and says which
/// question it is not answering rather than inventing an answer.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn every_concurrent_record_replays_complete() {
    let workers = workers();
    let entries = 2_000;

    thread::scope(|scope| {
        for worker in 0..workers {
            let _writer = scope.spawn(move || {
                let scratch = Scratch::new("load-journal");
                {
                    let mut journal =
                        Journal::open(&scratch.journal()).expect("a journal opens under load");
                    for sequence in 0..entries {
                        journal
                            .append(&Entry::new(
                                EntryKind::SelfCost,
                                Timestamp::from_utc_nanos(
                                    1_700_000_000_000_000_000,
                                    Attested::Unknown,
                                ),
                                u64::try_from(sequence).unwrap_or(0),
                                Value::map([
                                    (
                                        "worker",
                                        Value::Integer(i64::try_from(worker).unwrap_or(-1)),
                                    ),
                                    (
                                        "sequence",
                                        Value::Integer(i64::try_from(sequence).unwrap_or(-1)),
                                    ),
                                ]),
                            ))
                            .expect("an append succeeds under load");
                    }
                }

                let replayed = replay(&scratch.journal()).expect("the journal replays");
                assert!(replayed.is_complete(), "{}", replayed.statement());
                assert_eq!(replayed.entries.len(), entries);
                // Each entry is the one this worker wrote, in order: a journal
                // that had picked up another worker's line would still be
                // "complete" and would be wrong.
                for (sequence, entry) in replayed.entries.iter().enumerate() {
                    let body = entry.body();
                    assert_eq!(
                        body.get("worker"),
                        Some(&Value::Integer(i64::try_from(worker).unwrap_or(-1)))
                    );
                    assert_eq!(
                        body.get("sequence"),
                        Some(&Value::Integer(i64::try_from(sequence).unwrap_or(-1)))
                    );
                }
            });
        }
    });
    println!("  {workers} concurrent journals × {entries} entries, all complete and in order");
}

/// A bundle written while the machine is busy is a bundle that reads back.
/// B-302's one mechanism is what §XIV and PR2 will both use, so it is worth
/// knowing it does not depend on being the only thing running.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn every_concurrent_export_reads_back() {
    let workers = workers();
    thread::scope(|scope| {
        for worker in 0..workers {
            let _writer = scope.spawn(move || {
                let scratch = Scratch::new("load-export");
                {
                    let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
                    for sequence in 0..200 {
                        journal
                            .append(&Entry::new(
                                EntryKind::Trials,
                                Timestamp::from_utc_nanos(
                                    1_700_000_000_000_000_000,
                                    Attested::Unknown,
                                ),
                                sequence,
                                Value::map([(
                                    "worker",
                                    Value::Integer(i64::try_from(worker).unwrap_or(-1)),
                                )]),
                            ))
                            .expect("an append succeeds");
                    }
                }
                let bundle = scratch.join("bundle.mcf");
                let manifest = export::write(&scratch.journal(), &bundle, export::Kind::Export)
                    .expect("a bundle is written");
                let (kind, stated, entries) =
                    export::read(&bundle).expect("the bundle reads back under load");
                assert_eq!(kind, export::Kind::Export);
                assert_eq!(stated, manifest);
                assert_eq!(entries.len(), 200);
            });
        }
    });
    println!("  {workers} concurrent bundles, each read back against its own manifest");
}

/// A3, one level up: a scenario that goes wrong under load is reported, and
/// the harness that ran it is still there to report it. Asserted by running
/// the whole catalogue concurrently and requiring every outcome to be a
/// *classified* one — `Unexpected` is a finding, not a crash, and either way
/// the tier finishes and says so.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn the_harness_survives_everything_it_runs() {
    let workers = workers();
    let unexpected = Arc::new(AtomicUsize::new(0));
    thread::scope(|scope| {
        for _ in 0..workers {
            let unexpected = Arc::clone(&unexpected);
            let _worker = scope.spawn(move || {
                for scenario in CATALOGUE {
                    match run(scenario) {
                        Outcome::Produced(failure) => {
                            assert!(
                                !failure.detail().is_empty(),
                                "{} produced a failure with no detail",
                                scenario.id
                            );
                        }
                        Outcome::Unexpected(what) => {
                            println!("  {} reported: {what}", scenario.id);
                            unexpected.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });
    assert_eq!(
        unexpected.load(Ordering::Relaxed),
        0,
        "a scenario stopped reproducing its failure under load"
    );
}

/// Two *processes* appending to one record leave a record that replays whole
/// (DEC-037, D20, B62).
///
/// The test above runs many writers, each with a journal of its own — which is
/// the shape a laboratory has and not the shape a machine has. A machine has
/// one record and, since the daemon exists, more than one thing that writes to
/// it: `mcf pull` records an acquisition, `mcf rm` a removal, and the daemon
/// its own starting and stopping.
///
/// What is asserted here is the property that survives without any coordination
/// at all: **no line is torn**. Every writer opens the journal in append mode
/// and writes one whole line per entry, and
/// [findings.md](../../doc/findings.md) F13 measures what that is worth — at
/// 400 bytes, 8 KiB and 128 KiB a line, on tmpfs and on btrfs, sixteen thousand
/// lines from eight processes arrived intact.
///
/// What is *not* asserted is that the identifiers are unique. Two writers count
/// their own appends, so two entries can carry the same sequence number, and
/// that is DEC-037's question rather than a defect this test hides.
#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn two_processes_writing_one_record_leave_it_readable() {
    let scratch = Scratch::new("load-two-writers");
    let journal = scratch.journal();
    // The header is written once, before anybody races for it: a journal's
    // first line is its format version, and two processes creating one at the
    // same moment is a different question (B-332).
    drop(Journal::open(&journal).expect("a journal opens"));

    let writers = 4;
    let each = 500;
    thread::scope(|scope| {
        for worker in 0..writers {
            let journal = journal.clone();
            let _writing = scope.spawn(move || {
                // A process of its own, so that nothing is shared but the file:
                // two threads in one process share a `Journal` if they are not
                // careful, and what a machine actually has is two programs.
                let mut writing = Journal::open(&journal).expect("a journal opens");
                for sequence in 0..each {
                    writing
                        .append(&Entry::new(
                            EntryKind::SelfCost,
                            Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
                            u64::try_from(sequence).unwrap_or(0),
                            Value::map([
                                ("worker", Value::Integer(worker)),
                                ("filler", Value::text("x".repeat(400))),
                            ]),
                        ))
                        .expect("it appends");
                }
            });
        }
    });

    let replayed = replay(&journal).expect("the record replays");
    assert!(
        replayed.loss.is_none(),
        "a line was torn by concurrent writers: {:?}",
        replayed.loss
    );
    assert_eq!(
        replayed.entries.len(),
        usize::try_from(writers * each).unwrap_or(0),
        "entries were lost between the writers and the record"
    );
    // And every entry is one somebody wrote, whole: a torn line that happened
    // to parse would show up as a body missing its filler.
    for entry in &replayed.entries {
        assert!(
            entry
                .body()
                .get("filler")
                .and_then(Value::as_text)
                .is_some_and(|filler| filler.len() == 400),
            "an entry arrived with its body cut short"
        );
    }
}
