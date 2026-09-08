#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

#[test]
fn nothing_computes_a_mean() {
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

const EXEMPT: [&str; 2] = ["mcf-standin", "mcf-desk"];

fn exempt(path: &Path) -> bool {
    if path.components().any(|component| {
        EXEMPT
            .iter()
            .any(|crate_name| component.as_os_str() == *crate_name)
    }) {
        return true;
    }
    relative(path) == "crates/mcf-lab/src/fixture.rs"
}

#[test]
fn a_float_cannot_reach_the_record() {
    let codec = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-record/src/json.rs"),
    )
    .expect("the record's codec is in the tree");
    let manifest =
        std::fs::read_to_string(mcf_checks::workspace::root().join("crates/mcf-record/Cargo.toml"))
            .expect("the record's manifest is in the tree");
    for crate_name in EXEMPT {
        assert!(
            !manifest.contains(crate_name),
            "mcf-record depends on {crate_name}, where floats live — the exemption is no \
             longer paid for"
        );
    }
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

#[test]
fn a_summary_is_never_written_without_its_trials() {
    let source = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-record/src/encode.rs"),
    )
    .expect("the encoder is readable");
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

fn shipped_sources() -> Vec<PathBuf> {
    rust_sources(&mcf_checks::workspace::root().join("crates"))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
}

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
