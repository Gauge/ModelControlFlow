#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const OUTCOME: &str = "crates/mcf-core/src/probe.rs";

const SILENT: [&str; 4] = [
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "or_default()",
];

const ASSERTING: [&str; 2] = ["expect(", "unwrap()"];

#[test]
fn the_outcome_type_has_no_fallback_accessor() {
    let path = mcf_checks::workspace::root().join(OUTCOME);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} could not be read: {error}", path.display()));

    let mut offenders = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        if SILENT
            .iter()
            .chain(ASSERTING.iter())
            .any(|fallback| code.contains(fallback))
        {
            offenders.push(format!("{}:{}: {}", OUTCOME, number + 1, line.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "the outcome type must not offer a value where it has said there is \
         none — an inconclusive probe leaves the capability unknown (A7, D42, \
         B-060):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn no_caller_defaults_an_inconclusive_probe() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let is_a_test = relative(&path).contains("tests");
        for (number, line) in source.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains(".observed()") {
                continue;
            }
            let watched: Vec<&str> = if is_a_test {
                SILENT.to_vec()
            } else {
                SILENT.iter().chain(ASSERTING.iter()).copied().collect()
            };
            if watched.iter().any(|fallback| code.contains(fallback)) {
                offenders.push(format!(
                    "{}:{}: {}",
                    relative(&path),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a probe that could not tell must leave the capability unknown, and \
         these lines supply a value instead (A7, D42, B-060):\n{}",
        offenders.join("\n")
    );
}

fn relative(path: &Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
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
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}
