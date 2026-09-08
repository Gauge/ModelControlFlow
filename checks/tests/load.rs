#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use mcf_checks::scratch::Scratch;

use mcf_core::attested::Attested;
use mcf_core::time::Timestamp;
use mcf_record::export;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;
use mcf_serve::engines::{Choice, Device, Engine, Kind, resolve};
use mcf_serve::hosting::Hosting;

fn workers() -> usize {
    thread::available_parallelism()
        .map_or(8, |count| count.get() * 2)
        .min(64)
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

fn a_card(name: &str, free: u64) -> Device {
    Device {
        kind: Kind::Gpu,
        name: name.to_owned(),
        free: Some(free),
    }
}

fn engines_here() -> Vec<(Engine, Vec<Device>)> {
    let engine = Engine {
        name: "an engine".to_owned(),
        prefix: std::path::PathBuf::from("/nowhere"),
        commit: "abc".to_owned(),
    };
    vec![(
        engine,
        vec![
            Device {
                kind: Kind::Cpu,
                name: "CPU".to_owned(),
                free: Some(64_000_000_000),
            },
            a_card("Card A", 116_000_000_000),
            a_card("Card B", 101_000_000_000),
        ],
    )]
}

const WEIGHTS: [u64; 4] = [
    1_000_000_000,
    30_000_000_000,
    135_000_000_000,
    400_000_000_000,
];

fn planned(weights: u64) -> Result<Choice, String> {
    resolve(&engines_here(), weights, Some(114_688), 40_960).map_err(|refused| refused.says())
}

#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn the_placement_decided_under_load_is_the_one_decided_alone() {
    let workers = workers();
    let rounds = 40;
    for weights in WEIGHTS {
        let alone = planned(weights);
        let divergences = Arc::new(AtomicUsize::new(0));
        let ran = Arc::new(AtomicUsize::new(0));
        thread::scope(|scope| {
            for _ in 0..workers {
                let divergences = Arc::clone(&divergences);
                let ran = Arc::clone(&ran);
                let alone = alone.clone();
                let _worker = scope.spawn(move || {
                    for _ in 0..rounds {
                        ran.fetch_add(1, Ordering::Relaxed);
                        if planned(weights) != alone {
                            divergences.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });
        println!(
            "  {} decisions for {weights} bytes across {workers} workers",
            ran.load(Ordering::Relaxed)
        );
        assert_eq!(ran.load(Ordering::Relaxed), workers * rounds);
        assert_eq!(
            divergences.load(Ordering::Relaxed),
            0,
            "a model of {weights} bytes was placed differently under load than alone"
        );
    }
}

#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn a_model_no_single_card_holds_is_spread_the_same_way_every_time() {
    let workers = workers();
    let alone = planned(135_000_000_000).expect("two cards together hold it");
    assert!(alone.is_spread(), "the fixture stopped being a spread");
    let divergences = Arc::new(AtomicUsize::new(0));
    thread::scope(|scope| {
        for _ in 0..workers {
            let divergences = Arc::clone(&divergences);
            let alone = alone.clone();
            let _worker = scope.spawn(move || {
                for _ in 0..32 {
                    match planned(135_000_000_000) {
                        Ok(held) if held == alone => {}
                        _ => {
                            divergences.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });
    assert_eq!(
        divergences.load(Ordering::Relaxed),
        0,
        "the split across cards was not decided the same way under load"
    );
}

#[test]
#[ignore = "the load tier is scheduled: scripts/ci.sh --with-load (B38)"]
fn every_setting_reads_back_under_load() {
    let workers = workers();
    let choice = planned(30_000_000_000).expect("one card holds it");
    let hosting = Hosting::recommended(
        &choice.engine,
        &choice.device.name,
        true,
        choice.context,
        Some(8),
        true,
        None,
    )
    .spread_over(choice.split());
    let empty = Arc::new(AtomicUsize::new(0));
    thread::scope(|scope| {
        for _ in 0..workers {
            let empty = Arc::clone(&empty);
            let hosting = hosting.clone();
            let _worker = scope.spawn(move || {
                for _ in 0..64 {
                    for setting in hosting.listed(&hosting) {
                        if setting.value.is_empty() || setting.because.is_empty() {
                            empty.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });
    assert_eq!(
        empty.load(Ordering::Relaxed),
        0,
        "a setting rendered empty under load, so a reader would be shown a blank"
    );
}
