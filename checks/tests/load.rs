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

fn workers() -> usize {
    thread::available_parallelism()
        .map_or(8, |count| count.get() * 2)
        .min(64)
}

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
                    for _entry in 0..200 {
                        journal
                            .append(&Entry::new(
                                EntryKind::Trials,
                                Timestamp::from_utc_nanos(
                                    1_700_000_000_000_000_000,
                                    Attested::Unknown,
                                ),
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

#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn two_processes_writing_one_record_leave_it_readable() {
    let scratch = Scratch::new("load-two-writers");
    let journal = scratch.journal();
    drop(Journal::open(&journal).expect("a journal opens"));

    let writers = 4;
    let each = 500;
    thread::scope(|scope| {
        for worker in 0..writers {
            let journal = journal.clone();
            let _writing = scope.spawn(move || {
                let mut writing = Journal::open(&journal).expect("a journal opens");
                for _entry in 0..each {
                    writing
                        .append(&Entry::new(
                            EntryKind::SelfCost,
                            Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
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
    let mut identifiers = std::collections::BTreeSet::new();
    for entry in &replayed.entries {
        let id = entry
            .id()
            .map(|id| id.as_str().to_owned())
            .expect("an entry read from a record has the identifier its writer gave it");
        assert!(
            identifiers.insert(id.clone()),
            "two entries from concurrent writers were given one identifier: {id}"
        );
    }

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
