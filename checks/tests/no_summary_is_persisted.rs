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

/// No floating point reaches a shipped crate — with two exceptions, each
/// stated here and paid for below.
///
/// The reason is A6's, and it is why there is no mean: `Quantity` is `Ord`
/// because every summary MCF reports is an order statistic, and an `f64` field
/// would be the first step back toward arithmetic — with a NaN one division
/// away from a record.
///
/// **`mcf-standin` is exempt, because inference *is* floating-point
/// arithmetic.** D31 has MCF write a second implementation of it; a
/// dequantizer that could not hold an `f32` could not decode a weight. What
/// the rule is actually protecting is the *record*, and that protection is
/// structural rather than lexical: the record's format has no floating-point
/// representation at all (`mcf_record::json::Value`), and `mcf-standin` is not
/// a dependency of `mcf-record`. Both are asserted in
/// `a_float_cannot_reach_the_record` below, so the exemption costs a check
/// rather than a promise.
///
/// **The laboratory's model builder is exempt for the same reason and pays the
/// same price.** `mcf-lab/src/fixture.rs` writes GGUF files, and a GGUF holds
/// weights: a builder that could not hold an `f32` could not write one. It has
/// held `Vec<f32>` since it existed — B-366's dense-weight model only made the
/// float visible to this check by needing a function that returns one. The
/// exemption is the file, not the crate, and the same two facts pay for it:
/// the record's format has no floating-point variant, and `mcf-record` does not
/// depend on `mcf-lab` either.
#[test]
fn no_shipped_type_holds_a_float() {
    let mut offenders = Vec::new();
    for path in shipped_sources() {
        if exempt(&path) {
            continue;
        }
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

/// The two places a float is admitted, because the format they read or write
/// is made of them.
///
/// Named as paths rather than as crates so that admitting one file does not
/// admit the crate around it: `mcf-lab` is a large crate and only its model
/// builder writes weights.
fn exempt(path: &Path) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str() == "mcf-standin")
    {
        return true;
    }
    relative(path) == "crates/mcf-lab/src/fixture.rs"
}

/// The exemptions above are paid for: a float has no way into the record.
///
/// Two facts, and either alone would be enough. The record's own format has no
/// floating-point representation — a `Value` cannot hold one, so there is
/// nothing to write even for a caller that wanted to. And `mcf-record` does not
/// depend on `mcf-standin`, so the crate where floats live cannot reach the
/// crate that persists anything.
#[test]
fn a_float_cannot_reach_the_record() {
    let codec = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-record/src/json.rs"),
    )
    .expect("the record's codec is in the tree");
    for float in ["F32(", "F64(", "Float("] {
        assert!(
            !codec.contains(float),
            "the record's format has grown a floating-point variant ({float}), which is \
             what `no_shipped_type_holds_a_float` was protecting (A6, A1)"
        );
    }

    let record = mcf_checks::workspace::MEMBERS
        .iter()
        .find(|member| member.name == "mcf-record")
        .expect("mcf-record is a member");
    for float_holder in ["mcf-standin", "mcf-lab"] {
        assert!(
            !record.depends_on.contains(&float_holder),
            "mcf-record depends on {float_holder}, so a crate where floats live can reach \
             the crate that persists things"
        );
    }
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
