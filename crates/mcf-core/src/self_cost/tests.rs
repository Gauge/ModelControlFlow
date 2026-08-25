//! Tests for MCF's self-measurement.
//!
//! B19 keeps these hermetic: no network, no accelerator, no model. The one
//! subprocess measured is a program the test names itself.

use super::{
    Budget, COLD_START, CORE_BINARY, EVENT_TRIALS, Kind, RECORD_WRITE, RESIDENT_IDLE, Verdict,
    artifact_bytes, cold_start, resident_bytes,
};
use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::hardware::Attributability;
use crate::measurement::{Bytes, Conditions, Floor, Measurement};

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
        kind: Kind::CeilingOnState,
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
    assert_eq!(RECORD_WRITE.ceiling.as_nanos(), 2_000_000);
    for budget in [RESIDENT_IDLE, CORE_BINARY] {
        assert_eq!(budget.kind, Kind::CeilingOnState, "{}", budget.name);
    }
    for budget in [COLD_START, RECORD_WRITE] {
        assert_eq!(budget.kind, Kind::CeilingOnEvent, "{}", budget.name);
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

// ---------------------------------------------------------------------------
// D27 — which reading a budget is about, and when it may be asserted at all.
// ---------------------------------------------------------------------------

fn quiet() -> Attributability {
    Attributability::Attributable { delay_ppm: 189 }
}

fn busy() -> Attributability {
    Attributability::Unattributable {
        delay_ppm: 111_248,
        tolerated_ppm: 10_000,
    }
}

fn samples(values: impl IntoIterator<Item = u64>) -> Measurement<Bytes> {
    Measurement::from_samples(values.into_iter().map(Bytes), conditions())
        .expect("at least two samples")
}

/// B35: a timing taken under contention measures the contention, so
/// attributability is asked first. A reading from a busy machine is not a
/// reading of MCF, and asking whether it passed is asking the wrong question.
#[test]
fn a_busy_machine_makes_a_run_neither_a_pass_nor_a_failure() {
    let over = samples([1_000, 999_999_999]);
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Unattributable
    );
    assert_ne!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Within
    );
    assert_ne!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Over
    );
}

/// A7: an unreadable load is not permission. Unknown attributability blocks an
/// assertion exactly as a busy machine does, because reading an absent value as
/// a favourable one is the substitution A7 forbids.
#[test]
fn unknown_attributability_is_not_permission() {
    let within = samples([1_000, 2_000]);
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&within, &Attributability::Unknown),
        Verdict::Unattributable
    );
    assert!(!Attributability::Unknown.permits_assertion());
    assert!(quiet().permits_assertion());
}

/// D27: an event-class figure needs a hundred trials, because a p99 of twenty
/// is the maximum wearing a percentile's name. Too few is *not asserted*, which
/// is a third thing from passing and from failing.
#[test]
fn an_event_class_figure_refuses_too_few_trials() {
    let twenty = Measurement::from_samples(
        (0..20).map(|i| crate::time::Duration::from_nanos(i * 1000)),
        conditions(),
    )
    .expect("twenty samples");
    assert_eq!(
        COLD_START.read_measurement(&twenty, &quiet()),
        Verdict::TooFewTrials {
            had: 20,
            needs: EVENT_TRIALS
        }
    );
}

/// A state-class figure is read at the maximum: memory that exceeded the
/// ceiling once exceeded it, and a percentile would be a way of not noticing.
#[test]
fn a_state_class_figure_is_read_at_the_maximum() {
    // Ninety-nine readings under the ceiling and one far over it. A p99 would
    // pass this; a maximum does not, and the maximum is the right question.
    let mut values: Vec<u64> = (0..99).map(|_| 1_000).collect();
    values.push(999_999_999);
    let measured = samples(values);
    assert_eq!(RESIDENT_IDLE.statistic(&measured), Bytes(999_999_999));
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&measured, &quiet()),
        Verdict::Over
    );
}

/// An event-class figure is read at the 99th percentile: one outlier in a
/// hundred is the tail a user occasionally feels, and the ceiling is about
/// that rather than about the worst thing that ever happened.
#[test]
fn an_event_class_figure_is_read_at_the_ninety_ninth_percentile() {
    use crate::time::Duration;
    let mut values: Vec<Duration<crate::time::Monotonic>> =
        (0..99).map(|_| Duration::from_nanos(1_000_000)).collect();
    values.push(Duration::from_nanos(9_000_000_000));
    let measured = Measurement::from_samples(values, conditions()).expect("a hundred samples");
    assert_eq!(COLD_START.statistic(&measured).as_nanos(), 1_000_000);
    assert_eq!(
        COLD_START.read_measurement(&measured, &quiet()),
        Verdict::Within
    );
}

/// A prohibition is read at the maximum and only zero passes. Rendering it as a
/// budget that happens to be zero invites an argument about the margin, and
/// there is no margin.
#[test]
fn a_prohibition_admits_no_margin() {
    use crate::measurement::Count;
    let prohibition = Budget {
        name: "timer wakeups while idle",
        ceiling: Count(0),
        kind: Kind::Prohibition,
    };
    let none = Measurement::from_samples([Count(0), Count(0)], conditions()).expect("two");
    let one = Measurement::from_samples([Count(0), Count(1)], conditions()).expect("two");
    assert_eq!(
        prohibition.read_measurement(&none, &quiet()),
        Verdict::Within
    );
    assert_eq!(prohibition.read_measurement(&one, &quiet()), Verdict::Over);
}

/// Every verdict says what it is, and none of them reads as a pass by
/// accident.
#[test]
fn every_verdict_renders_distinctly() {
    let rendered = [
        Verdict::Within.to_string(),
        Verdict::Over.to_string(),
        Verdict::NotMeasured.to_string(),
        Verdict::Unattributable.to_string(),
        Verdict::TooFewTrials {
            had: 20,
            needs: 100,
        }
        .to_string(),
    ];
    for text in &rendered {
        assert!(!text.is_empty());
    }
    let mut unique: Vec<String> = rendered.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), rendered.len(), "two verdicts read the same");
}
