//! There is no mean in MCF, and no summary is written in place of its trials.
//!
//! B56's violation is *a stored mean, which is a question nobody can ask
//! again*, and B-270's condition is that one **does not compile**. The strongest
//! available form of that is not a rule about writing: it is that MCF has no
//! mean to write. `Quantity` requires only `Ord`, `Measurement` reports order
//! statistics, and nothing in the shipped crates averages anything.
//!
//! This check is what keeps it true. The failure mode is somebody adding a
//! `fn mean` for one call site — at which point the arithmetic exists, a float
//! creeps in with it, and B56 becomes a convention again.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// Nothing computes a mean, an average or a sum of samples.
///
/// The words rather than the operation, because the operation is `+` and `/`
/// and those have honest uses — a byte count, a percentile's rank. What is
/// being caught is a *summary statistic that discards its inputs*, and in
/// practice those arrive under a name.
#[test]
fn nothing_computes_a_mean() {
    // The names carry their opening parenthesis, because `fn meaning` is a
    // perfectly good function and `fn mean` is not a prefix worth banning.
    const NAMES: [&str; 6] = [
        "fn mean(",
        "fn average(",
        "fn avg(",
        "fn arithmetic_mean(",
        ".sum::<",
        "fn variance(",
    ];
    let mut offenders = Vec::new();
    for path in shipped_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (number, line) in production_lines(&source) {
            for name in NAMES {
                if line.contains(name) {
                    offenders.push(format!("{}:{number} → {name}", relative(&path)));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a summary statistic that discards its inputs has appeared (B56, B-270): {offenders:#?}"
    );
}

/// No floating point reaches a shipped crate.
///
/// The reason is A6's, and it is why there is no mean: `Quantity` is `Ord`
/// because every summary MCF reports is an order statistic, and an `f64` field
/// would be the first step back toward arithmetic — with a NaN one division
/// away from a record.
#[test]
fn no_shipped_type_holds_a_float() {
    let mut offenders = Vec::new();
    for path in shipped_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (number, line) in production_lines(&source) {
            let declares_a_float = line.contains(": f64")
                || line.contains(": f32")
                || line.contains("-> f64")
                || line.contains("-> f32")
                || line.contains("as f64")
                || line.contains("as f32");
            if declares_a_float {
                offenders.push(format!("{}:{number}", relative(&path)));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "floating point has reached a shipped crate, which is how a NaN reaches a \
         record (A6, A1): {offenders:#?}"
    );
}

/// The record's encoders write trials, and every summary they write travels
/// beside the trials it came from rather than instead of them.
#[test]
fn a_summary_is_never_written_without_its_trials() {
    let source = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-record/src/encode.rs"),
    )
    .expect("the encoder is readable");
    // The measurement encoder is the only one that writes a summary at all, and
    // it writes the samples in the same object.
    let measurement = source
        .split_once("pub fn measurement")
        .map(|(_, rest)| rest)
        .expect("the measurement encoder is where this check looks");
    let body = measurement
        .split_once("\n}")
        .map(|(body, _)| body)
        .expect("the encoder is closed");
    assert!(
        body.contains("\"trials\""),
        "the measurement encoder writes a summary without the trials it came from (B56)"
    );
    assert!(
        body.contains("\"spread\""),
        "this check is reading the wrong function"
    );
}

/// A series is never encoded without its thinning factor (B-271).
#[test]
fn a_series_is_never_written_without_its_resolution() {
    let source = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-record/src/encode.rs"),
    )
    .expect("the encoder is readable");
    let series = source
        .split_once("pub fn series")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("the series encoder is where this check looks");
    assert!(
        series.contains("thinning_factor"),
        "a series is encoded without what was done to it (B-271, D16)"
    );
}

/// The crates that ship. The prototype is explicitly not MCF, and the checks
/// crate is not shipped either.
fn shipped_sources() -> Vec<PathBuf> {
    rust_sources(&mcf_checks::workspace::root().join("crates"))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Lines that are neither documentation nor test code; see the note in
/// `fault_catalogue.rs` on why the cut at `#[cfg(test)]` is exact here.
fn production_lines(source: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        lines.push((index + 1, line.to_owned()));
    }
    lines
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
        } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}
