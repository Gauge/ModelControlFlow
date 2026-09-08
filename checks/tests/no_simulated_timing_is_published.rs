#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

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

#[test]
fn the_comparison_encoder_refuses_a_simulated_clock() {
    let source = code_only(&read("crates/mcf-bench/src/record.rs"));
    assert!(
        source.contains("pub fn comparison<K: ClockKind + Measurable>"),
        "the encoder that writes a comparison's raw durations must not accept the laboratory's \
         clock (A11, B-082)"
    );
}

#[test]
fn no_writing_crate_names_the_laboratory_clock() {
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

#[test]
fn the_laboratory_cannot_reach_the_comparison_encoder() {
    let manifest = read("crates/mcf-lab/Cargo.toml");
    assert!(
        !manifest.contains("mcf-bench"),
        "the laboratory has acquired a path to the benchmark crate; A11 then depends on nobody \
         using it (B-082)"
    );
}

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
