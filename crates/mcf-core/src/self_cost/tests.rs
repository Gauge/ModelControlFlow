//! Tests for MCF's self-measurement.
//!
//! B19 keeps these hermetic: no network, no accelerator, no model. The one
//! subprocess measured is a program the test names itself.

use super::{
    Budget, COLD_START, CORE_BINARY, RESIDENT_IDLE, Verdict, artifact_bytes, cold_start,
    resident_bytes,
};
use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::measurement::{Bytes, Conditions, Floor};

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

/// The reading is of this process, and a process that is running occupies
/// memory. A19: checked against something independently true rather than
/// against itself.
#[test]
fn the_resident_reading_is_of_a_running_process() {
    if let Attested::Known(rss) = resident_bytes() {
        assert!(rss > Bytes(0), "a running process reports no memory");
        assert!(
            rss < Bytes(64 * 1024 * 1024 * 1024),
            "a test binary reports {rss}, which is not plausible"
        );
    }
}

/// A7: an artifact that is not there is unknown, not zero. Zero would read as a
/// binary of no size and pass every budget.
#[test]
fn an_absent_artifact_is_unknown_and_not_zero() {
    let absent = artifact_bytes(std::path::Path::new("/nonexistent/mcf-not-here"));
    assert_eq!(absent, Attested::Unknown);
    assert_eq!(CORE_BINARY.read(absent), Verdict::NotMeasured);
}

/// Not measured is not passing. A budget with no reading has no verdict, and
/// saying so is the difference between a report and a reassurance.
#[test]
fn not_measured_is_not_within() {
    assert_eq!(RESIDENT_IDLE.read(Attested::Unknown), Verdict::NotMeasured);
    assert_ne!(RESIDENT_IDLE.read(Attested::Unknown), Verdict::Within);
    assert_eq!(Verdict::NotMeasured.to_string(), "not measured");
}

/// The ceiling is inclusive, and one byte past it is over. Stated because
/// which side of a boundary a reading falls on is the whole content of a
/// verdict.
#[test]
fn the_ceiling_is_inclusive_and_one_past_it_is_over() {
    let budget = Budget {
        name: "a ceiling",
        ceiling: Bytes(100),
        prohibition: false,
    };
    assert_eq!(budget.read(Attested::Known(Bytes(99))), Verdict::Within);
    assert_eq!(budget.read(Attested::Known(Bytes(100))), Verdict::Within);
    assert_eq!(budget.read(Attested::Known(Bytes(101))), Verdict::Over);
}

/// D24's figures, checked against what D24 says. A19: the document is the
/// independently known value.
#[test]
fn the_budgets_are_the_figures_d24_states() {
    assert_eq!(RESIDENT_IDLE.ceiling, Bytes(20 * 1024 * 1024));
    assert_eq!(CORE_BINARY.ceiling, Bytes(40 * 1024 * 1024));
    assert_eq!(COLD_START.ceiling.as_nanos(), 100_000_000);
    for budget in [RESIDENT_IDLE, CORE_BINARY] {
        assert!(!budget.prohibition, "{} is not a prohibition", budget.name);
    }
}

/// §3.4: a single trial is an anecdote, and there is no way to hold one as a
/// measurement — including here, where the program did not run at all.
#[test]
fn a_command_that_cannot_run_yields_no_measurement() {
    let nothing = cold_start(
        std::path::Path::new("/nonexistent/mcf-not-here"),
        &[],
        5,
        conditions(),
    );
    assert!(nothing.is_none());
}

/// A command that does run yields a measurement with its trials kept.
#[test]
fn a_command_that_runs_yields_a_measurement_with_its_trials() {
    let program = std::path::Path::new("/bin/true");
    if !program.exists() {
        return;
    }
    let measured = cold_start(program, &[], 3, conditions()).expect("three trials ran");
    assert_eq!(measured.n(), 3);
    assert!(measured.spread().minimum <= measured.spread().median);
    assert!(!measured.spread().median.is_simulated());
}
