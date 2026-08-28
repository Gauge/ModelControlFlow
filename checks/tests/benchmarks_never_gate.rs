//! Tests gate correctness; benchmarks produce measurements; neither is the
//! other (A18, §6.7, B-080).
//!
//! A18's violation is *a throughput assertion in the test suite, which is how
//! suites become flaky and then ignored*. Both halves are checked here,
//! because both are the kind of thing that arrives one line at a time in a
//! change that looked reasonable:
//!
//! * a benchmark that can fail — most easily by an exit status that depends on
//!   which verdict came back, at which point *not decided* becomes a red build
//!   and the ceiling becomes a threshold;
//! * a test that asserts on a wall-clock reading, at which point the suite's
//!   greenness depends on what else the machine was doing.
//!
//! **The measured tiers are exempt, and say so.** B-011's budget tier exists to
//! measure MCF's own cost against D24's ceilings and therefore *does* assert on
//! timings — under D30's attributability rule, in release, in an exclusive
//! window, and **scheduled rather than gating**. That is the third thing A18
//! names: a regression detector, whose thresholds are statistical judgments.
//! It is exempt by name below, and the naming is the point: an exemption
//! written down is one somebody can argue with.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// The tiers that are allowed to assert on a timing, and why.
///
/// Each is scheduled rather than gating, so a machine that was busy makes a
/// scheduled run say so rather than making a change unmergeable.
const MEASURED: [&str; 4] = [
    // B-011: MCF's own cost against D24's ceilings, in release, in a window.
    "budget.rs",
    // B-191: drift over a long run — descriptors, directories, memory.
    "soak.rs",
    // B-191: MCF's claims under many callers at once.
    "load.rs",
    // The prototypes are not MCF and are not run by any tier.
    "prototypes",
];

/// **The benchmark runner has no verdict that fails.**
///
/// Its exit status is `served`, and every verdict must set it true. The two
/// ways this breaks are a `served: false` reached from a verdict, and a
/// verdict consulted at all when the status is decided — so the check is that
/// the one place the status is set for a finished run says so in a line a
/// reader will see.
#[test]
fn no_verdict_makes_the_benchmark_fail() {
    let source = read("crates/mcf-cli/src/bench.rs");
    let Some((_, body)) = source.split_once("let held = running.finish();") else {
        panic!("the runner finishes its comparison somewhere");
    };
    let finished = body
        .split_once("#[cfg(test)]")
        .map_or(body, |(before, _)| before);
    assert!(
        finished.contains("served: true"),
        "a finished benchmark must be served whatever it found (A18)"
    );
    assert!(
        !finished.contains("served: false"),
        "no verdict may set a failing exit status: that is a pass condition wearing an exit \
         status (A18, §6.7)"
    );
}

/// The refusals a benchmark *does* have are about MCF being unable to measure,
/// never about what it measured.
///
/// Named so that a fifth one added later has to be argued for here rather than
/// slipped in beside the four.
#[test]
fn the_only_refusals_are_about_being_unable_to_measure() {
    let source = read("crates/mcf-cli/src/bench.rs");
    for refusal in [
        "there is no model at",
        "none is listening",
        // Split across a line by the formatter, so the check looks for the
        // half that cannot be reflowed away.
        "can never be a",
        "did not run",
    ] {
        assert!(
            source.contains(refusal),
            "the refusal `{refusal}` is one of the four this runner has, and it is gone"
        );
    }
}

/// **No gating test asserts on a wall-clock reading.**
///
/// What is caught is an *assertion whose truth depends on how fast the machine
/// was*: an `assert!` that both names an elapsed interval and bounds it. The
/// bound is the part that matters — an equality on a simulated duration is
/// deterministic and is not this, which is why the laboratory's own
/// `assert_eq!(elapsed.as_nanos(), 1_000_000_000)` is not flagged and should
/// not be. A11 already keeps the two kinds of duration in different types; this
/// catches the case where a real one is compared against a number somebody
/// chose.
#[test]
fn no_gating_test_asserts_on_a_timing() {
    /// The words an elapsed interval arrives under.
    const NAMED: [&str; 6] = [
        "elapsed",
        "took",
        "duration",
        "as_secs",
        "as_millis",
        "as_nanos",
    ];
    let mut offenders = Vec::new();
    for path in gating_tests() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        // Whitespace inside an assertion is a formatter's choice, so it is
        // removed before the shapes are looked for.
        let squeezed: String = source
            .chars()
            .filter(|held| !held.is_whitespace())
            .collect();
        for assertion in squeezed.split("assert!(").skip(1) {
            // The condition, up to whatever follows it. A message may name a
            // duration innocently; a condition may not bound one.
            let condition = assertion
                .split_once(",\"")
                .map_or(assertion, |(before, _)| before);
            let bounded = condition.contains('<') || condition.contains('>');
            if !bounded {
                continue;
            }
            if let Some(named) = NAMED.iter().find(|named| condition.contains(**named)) {
                offenders.push(format!("{} → {named} bounded", relative(&path)));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a timing assertion in the gating suite is how a suite becomes flaky and then gets \
         ignored (A18, §6.7). The measured tiers may assert on timings and are named in this \
         file: {offenders:#?}"
    );
}

/// **A correctness test cannot emit a measurement into the record.**
///
/// The other half of B-080's done-when. A `Measurement` built in a test is
/// fine — it is how the type is tested — but a *gating* test that appends one
/// to the real record would be a correctness run producing a measurement, and
/// the record would then hold figures taken in debug, under a suite, on
/// whatever machine ran it (§3.4, A11).
#[test]
fn no_gating_test_writes_to_the_real_record() {
    let mut offenders = Vec::new();
    for path in gating_tests() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        if source.contains("journal::default_path()") {
            offenders.push(relative(&path));
        }
    }
    assert!(
        offenders.is_empty(),
        "a gating test reaches for the machine's own record; a test writes to a scratch \
         directory it removes (B19, A18): {offenders:#?}"
    );
}

/// The benchmark runner is not run by the gating tier.
///
/// A benchmark in the gate is a benchmark that gates, whatever its exit status
/// says — it takes minutes, and a gate people skip does not gate (B38).
#[test]
fn the_gating_tier_does_not_run_the_benchmark() {
    let script = read("scripts/ci.sh");
    let gating = script
        .split_once("=== the scheduled tiers")
        .map_or(script.as_str(), |(before, _)| before);
    assert!(
        !gating.contains("mcf bench") && !gating.contains(" bench "),
        "the gating tier runs the benchmark runner, which makes a measurement a gate (A18, B38)"
    );
}

/// Every test file in the gating tier: the workspace's `tests` directories and
/// the checks, minus the tiers that are allowed to measure.
fn gating_tests() -> Vec<PathBuf> {
    let root = mcf_checks::workspace::root();
    let mut found = rust_sources(&root.join("checks/tests"));
    let Ok(crates) = std::fs::read_dir(root.join("crates")) else {
        return found;
    };
    for entry in crates.flatten() {
        found.extend(rust_sources(&entry.path().join("tests")));
    }
    found.retain(|path| {
        let shown = path.display().to_string();
        // This file names the shapes it forbids, so it matches itself.
        !shown.ends_with("benchmarks_never_gate.rs")
            && !MEASURED.iter().any(|exempt| shown.contains(exempt))
    });
    found.sort();
    assert!(
        found.len() > 10,
        "only {} gating test files were found, so this check is reading the wrong tree",
        found.len()
    );
    found
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|held| held == "rs") {
            found.push(path);
        }
    }
    found
}

fn relative(path: &Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
