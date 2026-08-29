//! No performance figure originates in simulation (A11, B-082, D9).
//!
//! **Two halves, and only the first was already held.** `ClockKind` puts the
//! clock in the *type*, so a simulated interval cannot be compared with a real
//! one, stored where one is expected, or averaged into a set of them — that is
//! `time_model.rs`'s subject and it holds. What it did not stop was a
//! simulated timing being **written down**: an encoder generic over the clock
//! will happily put the laboratory's arithmetic into the record, where it
//! becomes a measurement with nothing on it to say otherwise.
//!
//! `Measurable` closes it. It is implemented for `Monotonic` and for nothing
//! else, every path that puts a duration into the record is bounded by it, and
//! encoding a laboratory comparison is therefore a compile error. This file is
//! what keeps that true: the bound is one word, and a word is easy to delete
//! while making something else compile.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// `Measurable` is implemented for the monotonic clock and nothing else.
///
/// Adding an implementation is a decision about what MCF is willing to call a
/// measurement. It is allowed — a second real clock could arrive — and it is
/// not allowed to happen by accident, which is what this asserts.
#[test]
fn only_the_monotonic_clock_may_be_published() {
    let source = code_only(&read("crates/mcf-core/src/time/clock.rs"));
    let implementations: Vec<&str> = source
        .lines()
        .filter(|line| line.trim_start().starts_with("impl Measurable for "))
        .collect();
    assert_eq!(
        implementations,
        ["impl Measurable for Monotonic {}"],
        "a clock other than the monotonic one has been declared publishable (A11, B-082)"
    );
    assert!(
        source.contains("pub trait Measurable: ClockKind {}"),
        "this check is reading the wrong file"
    );
}

/// The comparison encoder — the one path that writes timings as measurements —
/// is bounded by it.
#[test]
fn the_comparison_encoder_refuses_a_simulated_clock() {
    let source = code_only(&read("crates/mcf-bench/src/record.rs"));
    assert!(
        source.contains("pub fn comparison<K: ClockKind + Measurable>"),
        "the encoder that writes a comparison's raw durations must not accept the laboratory's \
         clock (A11, B-082)"
    );
}

/// No crate that writes to the record names the laboratory's clock at all.
///
/// The strongest available form of the second half, and it is checkable
/// because it is true: the laboratory is one crate, and every other shipped
/// crate has no business naming `Simulated`. A crate that starts to is one
/// where a simulated duration has become reachable from a writing path, and
/// that is the moment to ask why rather than three months later.
///
/// Documentation may name it — this file does, and so does the encoder's own
/// explanation of why it will not take one — so comments are removed first.
#[test]
fn no_writing_crate_names_the_laboratory_clock() {
    /// The crates that may. `mcf-lab` *is* the laboratory; `mcf-core` defines
    /// both clocks.
    const ALLOWED: [&str; 2] = ["mcf-lab", "mcf-core"];

    let mut offenders = Vec::new();
    for path in shipped_sources() {
        let shown = relative(&path);
        if ALLOWED.iter().any(|allowed| shown.contains(allowed)) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (number, line) in production_lines(&code_only(&source)) {
            if line.contains("Simulated") {
                offenders.push(format!("{shown}:{number}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a crate that writes to the record names the laboratory's clock, which is how a \
         simulated timing gets published (A11, B-082): {offenders:#?}"
    );
}

/// And the laboratory does not depend on the crate that encodes comparisons,
/// so it could not write one even if it held one.
///
/// A dependency check rather than a source one: it is the edge that would have
/// to exist first, and it does not.
#[test]
fn the_laboratory_cannot_reach_the_comparison_encoder() {
    let manifest = read("crates/mcf-lab/Cargo.toml");
    assert!(
        !manifest.contains("mcf-bench"),
        "the laboratory has acquired a path to the benchmark crate; A11 then depends on nobody \
         using it (B-082)"
    );
}

/// Every shipped source file, tests excluded.
fn shipped_sources() -> Vec<PathBuf> {
    rust_sources(&mcf_checks::workspace::root().join("crates"))
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|held| held == "rs") && !path.ends_with("tests.rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Lines before the test module, which is where a source check's subject ends.
fn production_lines(source: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        lines.push((index.saturating_add(1), line.to_owned()));
    }
    lines
}

/// The source with its documentation comments removed, so that a sentence
/// naming a clock is not read as the code naming it.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
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
