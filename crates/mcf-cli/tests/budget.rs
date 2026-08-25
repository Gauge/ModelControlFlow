//! The performance budget tier (B-011).
//!
//! B20: *idle CPU, resident memory, disk footprint, cold start and the latency
//! MCF interposes are budgeted, measured and regression-tested.* D24 gives the
//! numbers and D27 says which reading each one is about.
//!
//! **Why this tier is `#[ignore]`d by default.** An event-class figure needs a
//! hundred trials (D27), each of which is a process spawn, and B38 requires the
//! gating tier stay fast because a gate people skip does not gate. This is one
//! of B38's scheduled tiers: `scripts/ci.sh --with-budget` runs it, and it runs
//! before a release.
//!
//! **Why it asserts only on a release build.** D24's ceilings are for the
//! artifact MCF ships, and a debug binary is a different artifact — larger,
//! slower, and with different code. The profile is a condition (§3.4), and a
//! figure measured under the wrong one is reported rather than asserted. A
//! debug run of this tier is therefore informative and never green-by-luck.
//!
//! **Why a busy machine does not fail it.** B35: a timing taken under
//! contention measures the contention. D27 makes such a run *unattributable* —
//! neither a pass nor a failure — and B38's staleness discipline is what stops
//! that becoming a hiding place: an unattributable run does not refresh the
//! tier's age.

// Every item in this file is test code; see the note in checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::hardware::{Attributability, Machine, attributability};
use mcf_core::measurement::{Bytes, ConditionValue, Conditions, Floor, Measurement, Quantity};
use mcf_core::self_cost::{
    self, Budget, COLD_START, CORE_BINARY, EVENT_TRIALS, RESIDENT_IDLE, Verdict,
};
use mcf_core::time::{Duration, Monotonic};

/// The binary under test: the one cargo built for this test run.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

/// The conditions every figure below is taken under.
///
/// A6: no number without them. The load average is in here because D27 makes
/// attributability part of what a budget reading means, and a reader who
/// disagrees with the threshold needs the reading it was applied to.
fn conditions(machine: &Machine, attributability: &Attributability) -> Conditions {
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: Attested::Known(ConditionValue::text(format!(
                "{} · {}",
                machine.processor.model, attributability
            ))),
            mcf_configuration: Attested::Known(ConditionValue::text(
                "the budget tier, release profile",
            )),
            ..Floor::nothing_known()
        },
    )
}

/// Whether this run is against the artifact D24's ceilings are about.
fn is_release() -> bool {
    BuildIdentity::current().profile == "release"
}

/// Reports a figure, and asserts it only where D27 permits.
fn judge<Q: Quantity>(budget: &Budget<Q>, verdict: Verdict, reading: &str) {
    println!("  {:<40} {reading} — {verdict}", budget.name);
    match verdict {
        Verdict::Over => {
            assert!(
                !is_release(),
                "{} is over its ceiling of {} (D24, B20): {reading}",
                budget.name,
                budget.ceiling
            );
            println!(
                "    not a failure: this is a {} build, and D24's ceilings are for the \
                 release artifact",
                BuildIdentity::current().profile
            );
        }
        // Each of these is its own outcome and none of them is a pass. Printing
        // them is the point: a tier that reported only failures would let a
        // figure quietly stop being measured (A2, applied to the suite).
        Verdict::Within
        | Verdict::NotMeasured
        | Verdict::Unattributable
        | Verdict::TooFewTrials { .. } => {}
    }
}

/// D24's installed-footprint figure for the core binary. State-class: read at
/// the maximum, which for one file is its size.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_core_binary_is_within_its_footprint() {
    let measured = self_cost::artifact_bytes(&binary());
    judge(
        &CORE_BINARY,
        CORE_BINARY.read(measured),
        &match measured {
            Attested::Known(size) => size.to_string(),
            Attested::Unknown => "unknown".to_owned(),
        },
    );
}

/// D24's resident-memory figure. Measured from inside the process MCF actually
/// runs, through the surface a user runs — `mcf doctor` reports its own
/// resident set, so this reads the product rather than a stand-in for it.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn resident_memory_is_within_its_ceiling() {
    let machine = Machine::read();
    let attributable = attributability(machine.processor.cores);
    let Some(resident) = reported_resident(&binary()) else {
        judge(&RESIDENT_IDLE, Verdict::NotMeasured, "unknown");
        return;
    };
    // Two readings so the figure is a measurement rather than an anecdote
    // (§3.4). Resident memory of a fresh process is near-deterministic, which
    // is why two suffice for a maximum where a percentile would need a hundred.
    let second = reported_resident(&binary()).unwrap_or(resident);
    let measured =
        Measurement::from_samples([resident, second], conditions(&machine, &attributable))
            .expect("two readings");
    judge(
        &RESIDENT_IDLE,
        RESIDENT_IDLE.read_measurement(&measured, &attributable),
        &measured.maximum().to_string(),
    );
}

/// D24's cold-start figure. Event-class: a hundred trials, read at the 99th
/// percentile, and only on a machine quiet enough to attribute the reading to
/// MCF rather than to whatever else was running (D27).
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn cold_start_is_within_its_ceiling() {
    let machine = Machine::read();
    let attributable = attributability(machine.processor.cores);
    let conditions = conditions(&machine, &attributable);
    let Some(measured) = self_cost::cold_start(&binary(), &["--version"], EVENT_TRIALS, conditions)
    else {
        judge(&COLD_START, Verdict::NotMeasured, "did not run");
        return;
    };
    let spread = measured.spread();
    judge(
        &COLD_START,
        COLD_START.read_measurement(&measured, &attributable),
        &format!(
            "p99 {} (median {}, n={})",
            COLD_START.statistic(&measured),
            spread.median,
            measured.n()
        ),
    );
}

/// The tier says what it measured under, always. B20 requires a performance
/// change carry a before-and-after *under stated conditions*, and conditions
/// that are only in the record are conditions nobody reads while looking at the
/// number.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_tier_states_its_conditions() {
    let machine = Machine::read();
    let attributable = attributability(machine.processor.cores);
    println!("\nbudget tier conditions");
    println!("  {}", BuildIdentity::current());
    println!("  {attributable}");
    println!("  {machine}");
    if !is_release() {
        println!(
            "\n  NOTHING IS ASSERTED in this profile. D24's ceilings are for the release\n\
             \x20 artifact, and a debug binary is a different one (§3.4)."
        );
    }
    if !attributable.permits_assertion() {
        println!(
            "\n  NOTHING IS ASSERTED on this machine right now: a timing taken under\n\
             \x20 contention measures the contention (B35), so this run is unattributable\n\
             \x20 rather than passing or failing (B24, D27). It does not refresh the\n\
             \x20 tier's age (B38)."
        );
    }
    assert!(!machine.to_string().is_empty());
}

/// Runs `mcf doctor --json --no-record` and reads back the resident figure it
/// reports about itself.
///
/// Reading the product's own report rather than measuring from outside is what
/// makes this the figure D24 is about: the resident set of a process at the
/// moment it has finished starting, which nothing outside the process can
/// observe without racing it.
fn reported_resident(binary: &Path) -> Option<Bytes> {
    let output = std::process::Command::new(binary)
        .args(["doctor", "--json", "--no-record"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let value = mcf_record::json::parse(&text).ok()?;
    let bytes = value
        .get("self_cost")?
        .get("resident_bytes")?
        .as_integer()?;
    u64::try_from(bytes).ok().map(Bytes)
}

/// A regression is reported with a before and an after, or it is not reported.
///
/// B20: *a performance change without a before-and-after under stated
/// conditions is not a performance change; it is a guess that also increased
/// complexity.* There is no baseline to compare against yet — that is B-185's
/// tier ages and a stored history — so what this asserts is the shape: every
/// figure this tier produces is a `Measurement` carrying its conditions, so a
/// baseline can be compared against it when one exists.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn every_figure_carries_what_a_comparison_would_need() {
    let machine = Machine::read();
    let attributable = attributability(machine.processor.cores);
    let conditions = conditions(&machine, &attributable);
    let measured: Measurement<Duration<Monotonic>> =
        self_cost::cold_start(&binary(), &["--version"], 2, conditions).expect("two trials ran");
    // The conditions travel with it, which is what a later comparison needs in
    // order to know whether the two are comparable at all (A8).
    let rendered = measured.conditions().to_string();
    assert!(rendered.contains("hardware_state="), "{rendered}");
    assert!(
        rendered.contains(BuildIdentity::current().profile),
        "{rendered}"
    );
    assert!(measured.samples().count() >= 2);
}
