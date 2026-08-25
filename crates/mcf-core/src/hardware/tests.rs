//! Tests for the hardware profiler.
//!
//! B19 requires the suite pass on a laptop with no accelerator, so every test
//! here has to hold on a machine with one and on a machine without. What is
//! asserted is therefore never *what* the machine is — that varies — but that
//! whatever MCF says about it is well-formed, honest about what it could not
//! read, and consistent between the routes that answered.

use super::{Characterization, Machine, Missing, Reading, routes};
use crate::attested::Attested;
use crate::measurement::Bytes;

/// A19: the profiler is checked against an independently known value — the
/// kernel's own accounting, read a second way.
#[test]
fn the_thread_count_matches_the_kernel() {
    let machine = Machine::read();
    let Attested::Known(threads) = machine.processor.threads else {
        // No `/proc`. The suite must still pass, and B19 is why.
        return;
    };
    let available = std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get);
    assert!(
        u64::from(threads) >= available as u64,
        "the profile reports {threads} threads and the runtime sees {available}"
    );
}

/// A7: what MCF cannot read stays unknown. Reading a machine never fails and
/// never invents — a machine with no `/proc`, no governor and no accelerator
/// produces a complete profile in which almost everything is `unknown`.
#[test]
fn reading_a_machine_never_fails_and_never_invents() {
    let machine = Machine::read();
    let rendered = machine.to_string();
    assert!(rendered.contains("processor:"), "{rendered}");
    assert!(rendered.contains("memory:"), "{rendered}");
    assert!(rendered.contains("power profile:"), "{rendered}");
    if let Attested::Known(total) = machine.memory.total {
        assert!(total > Bytes(0), "the machine reports no memory at all");
    }
}

/// Available memory never exceeds total. A profile that said otherwise would be
/// a reading nobody could act on, and the two come from different lines of the
/// same file — which is exactly where a parsing error would show.
#[test]
fn available_memory_does_not_exceed_total() {
    let memory = Machine::read().memory;
    if let (Attested::Known(total), Attested::Known(available)) = (memory.total, memory.available) {
        assert!(available <= total, "{available} available of {total} total");
    }
}

/// D25: a device is characterized when all four readings are present, and
/// otherwise names which are missing. Both outcomes are correct; what is
/// asserted is that the verdict and the reading agree.
#[test]
fn the_verdict_agrees_with_the_reading() {
    for device in Machine::read().accelerators {
        let reading = device.reading();
        match device.characterization() {
            Characterization::Characterized => {
                assert!(reading.model.is_known(), "{device}");
                assert!(reading.driver.is_known(), "{device}");
                assert!(reading.memory_available.is_known(), "{device}");
                assert!(reading.temperature_c.is_known(), "{device}");
                assert!(reading.missing().is_empty(), "{device}");
            }
            Characterization::AttemptedUncharacterized { missing } => {
                assert!(!missing.is_empty(), "uncharacterized and missing nothing");
                assert_eq!(missing, reading.missing());
            }
        }
        assert!(!device.routes().is_empty(), "a device nothing reported");
    }
}

/// A8: two routes that both claim to know a field and disagree is a finding
/// about a route, and it is surfaced rather than resolved by preferring one.
#[test]
fn routes_that_disagree_are_reported() {
    for device in Machine::read().accelerators {
        assert!(
            device.disagreements().is_empty(),
            "two routes disagree about {:?} on {device}",
            device.disagreements()
        );
    }
}

/// A9: no accelerator is a result. The profile says so and the machine is
/// still fully described.
#[test]
fn a_machine_with_no_accelerator_says_so() {
    let machine = Machine::read();
    if machine.accelerators.is_empty() {
        assert!(machine.to_string().contains("accelerators: none present"));
        assert!(machine.every_accelerator_is_characterized());
    }
}

/// Every route declares what it can supply, and the declaration is not empty.
/// A21's shape: a route that *should* have supplied a reading and did not is a
/// different thing from one that never claimed to.
#[test]
fn every_route_declares_its_coverage() {
    let routes = routes();
    assert!(!routes.is_empty(), "no route is compiled in");
    for route in &routes {
        assert!(!route.name().is_empty());
        assert!(
            !route.covers().is_empty(),
            "{} claims to supply nothing",
            route.name()
        );
        for reading in route.probe() {
            // A route may fail to supply something it covers — the driver may
            // be loaded and refuse a query — but it may never supply something
            // it does not claim to.
            let supplied = supplied_by(&reading);
            for what in supplied {
                assert!(
                    route.covers().contains(&what),
                    "{} supplied {what}, which it does not claim to cover",
                    route.name()
                );
            }
        }
    }
}

fn supplied_by(reading: &Reading) -> Vec<Missing> {
    Missing::ALL
        .into_iter()
        .filter(|what| !reading.missing().contains(what))
        .collect()
}

/// Merging fills gaps and never overwrites. A route that ran second must not
/// be able to replace a reading the first one took, because that would hide a
/// disagreement instead of reporting it.
#[test]
fn merging_fills_gaps_and_never_overwrites() {
    let first = Reading {
        model: Attested::Known("first".to_owned()),
        ..Reading::nothing_known()
    };
    let second = Reading {
        model: Attested::Known("second".to_owned()),
        temperature_c: Attested::Known(41),
        ..Reading::nothing_known()
    };
    let merged = first.clone().filled_from(&second);
    assert_eq!(merged.model, Attested::Known("first".to_owned()));
    assert_eq!(merged.temperature_c, Attested::Known(41));
    assert_eq!(first.disagreements_with(&second), ["model"]);
}

/// A reading that knows nothing is missing all four, which is what makes a
/// file-only machine report *attempted, uncharacterized* rather than silently
/// passing D25's bar.
#[test]
fn a_reading_that_knows_nothing_is_missing_everything() {
    assert_eq!(Reading::nothing_known().missing(), Missing::ALL.to_vec());
}

/// An unknown field is never a disagreement. Two routes, one of which did not
/// look, have not contradicted each other.
#[test]
fn silence_is_not_disagreement() {
    let known = Reading {
        model: Attested::Known("a device".to_owned()),
        ..Reading::nothing_known()
    };
    assert!(
        known
            .disagreements_with(&Reading::nothing_known())
            .is_empty()
    );
}
