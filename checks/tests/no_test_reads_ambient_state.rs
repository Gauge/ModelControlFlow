//! A test says what it was given, never what it happened to find.
//!
//! §3.12 is about reproducibility, and a suite whose answer depends on the
//! state of the machine it ran on is not reproducible — it is a different
//! experiment each time, reported as though it were the same one. The failure
//! is quiet in the worst way: the test passes for the author and fails for
//! somebody else, or passes all week and fails on the afternoon somebody left
//! a daemon running.
//!
//! **This was not hypothetical.** `a_file_that_is_not_a_model_is_refused_legibly`
//! asserted that MCF's refusal names *no vendored engine* — true when MCF
//! answers for itself, false when a daemon is listening and a provisioned
//! engine refuses first. It failed during B-056's work because a daemon had
//! been left running, and passed when it was stopped (F46, B-378). The test
//! was right; the isolation was missing.
//!
//! **What is checked.** Some functions reach for ambient state in the middle
//! of doing their job — where a daemon is, what the platform's directories
//! are. Each has a sibling that takes the same thing as an argument. A test
//! may call the sibling; calling the ambient one from a test is what this
//! forbids, because it is the shape that made the suite report on a machine.
//!
//! It is a small table on purpose. It grows when a function is found to have
//! this shape, and each row is a line somebody has to write down — which is
//! the point: B16 asks for the machine-checked form of a rule, and a rule
//! nobody can see the scope of is not one.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// One entry point a test may not call, and what to call instead.
struct Ambient {
    /// How the call is spelled.
    call: &'static str,
    /// The sibling that takes the state as an argument.
    instead: &'static str,
    /// What it reaches for, and why that makes a test unreproducible.
    why: &'static str,
}

/// The entry points that look up ambient state.
const AMBIENT: &[Ambient] = &[Ambient {
    call: "run(",
    instead: "run_where(",
    why: "it looks up whether a daemon is listening, and which of MCF and a daemon answers \
          changes the refusal a caller sees — so a test calling it reports on the machine \
          (F46, B-378)",
}];

/// Where those entry points live, so that an unrelated `run(` elsewhere is not
/// mistaken for one of them.
const WATCHED: &[&str] = &["crates/mcf-cli/src/run/tests.rs"];

/// No test calls an entry point that reaches for ambient state.
#[test]
fn no_test_calls_an_ambient_entry_point() {
    let root = mcf_checks::workspace::root();
    let mut offenders = Vec::new();
    for watched in WATCHED {
        let path = root.join(watched);
        let Ok(source) = std::fs::read_to_string(&path) else {
            panic!(
                "{watched} is watched by this check and is not there — the check has gone stale"
            );
        };
        for (number, line) in source.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            for ambient in AMBIENT {
                // `run_where(` contains `run(`? It does not — but `_run(` and
                // `rerun(` would, so the call has to start a word.
                if !starts_a_call(code, ambient.call) {
                    continue;
                }
                offenders.push(format!(
                    "{watched}:{}: calls `{}` — use `{}` instead, because {}",
                    number + 1,
                    ambient.call,
                    ambient.instead,
                    ambient.why
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a test must say what it was given rather than report on what it found (§3.12, \
         B-378):\n{}",
        offenders.join("\n")
    );
}

/// Whether the line calls `name` as a call rather than ending some longer one.
fn starts_a_call(line: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(at) = line[from..].find(name) {
        let at = from + at;
        let before = line[..at].chars().next_back();
        if !before.is_some_and(|character| character.is_alphanumeric() || character == '_') {
            return true;
        }
        from = at + name.len();
    }
    false
}

/// The sibling each row names is real.
///
/// A table pointing at a function that does not exist would fail somebody
/// reading the message rather than the check, which is worse than no check.
#[test]
fn every_row_names_a_function_that_exists() {
    let root = mcf_checks::workspace::root();
    let sources = rust_sources(&root.join("crates"));
    for ambient in AMBIENT {
        let wanted = format!("fn {}", ambient.instead.trim_end_matches('('));
        let found = sources
            .iter()
            .any(|path| std::fs::read_to_string(path).is_ok_and(|source| source.contains(&wanted)));
        assert!(
            found,
            "this check tells somebody to call `{}` and no such function exists",
            ambient.instead
        );
    }
}

/// Every `.rs` file under a directory, in a stable order.
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
