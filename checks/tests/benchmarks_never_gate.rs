#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const MEASURED: [&str; 4] = ["budget.rs", "soak.rs", "load.rs", "prototypes"];

#[test]
fn no_verdict_makes_the_benchmark_fail() {
    let source = read("crates/mcf-cli/src/bench.rs");
    assert_eq!(
        source
            .matches("let finding = held.finding(resolving);")
            .count(),
        1,
        "the command takes its verdict in exactly one place, and this check reads from there"
    );
    let Some((_, body)) = source.split_once("let finding = held.finding(resolving);") else {
        panic!("the command takes a verdict somewhere");
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

#[test]
fn the_only_refusals_are_about_being_unable_to_measure() {
    let source = read("crates/mcf-cli/src/bench.rs");
    for refusal in [
        "there is no model at",
        "none is listening",
        "can never be a",
        "did not run",
    ] {
        assert!(
            source.contains(refusal),
            "the refusal `{refusal}` is one of the four this runner has, and it is gone"
        );
    }
}

#[test]
fn no_gating_test_asserts_on_a_timing() {
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
        let squeezed: String = source
            .chars()
            .filter(|held| !held.is_whitespace())
            .collect();
        for assertion in squeezed.split("assert!(").skip(1) {
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
