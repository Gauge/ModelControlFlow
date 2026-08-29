//! A shell tier says which engine it ran, never whatever was listening (F103,
//! F102, F46, F47, §3.12, A19).
//!
//! **The gap this closes was named in writing and left open.** F47 built
//! `no_test_reads_ambient_state.rs` after a Rust test was found reporting on
//! whether a daemon happened to be running, and said plainly what it had not
//! established: *"Whether other tiers have the same dependency."* They did.
//!
//! **Why the existing guard cannot see this.** It watches Rust source for calls
//! to functions that reach for ambient state. A shell tier makes no such call —
//! it runs the *built binary*, and `mcf run` consulting a daemon is correct
//! behaviour for a person at a terminal. The defect is that a **check** invoked
//! it the way a person would, and a check is not a person: §3.12 forbids a
//! suite whose answer depends on the state it found.
//!
//! **What it cost.** `check-corpus.sh` passed all morning with no daemon and
//! failed the moment one was up (F102). Worse, `check-oracle.sh` compared
//! `mine` against `theirs` where `mine` was `mcf run` with no engine named — so
//! with a provisioned daemon listening it compared the reference implementation
//! **with itself** and reported agreement, in the one check A19's discipline
//! rests on (F103).
//!
//! So: every invocation of `mcf run` in a scheduled tier names its engine.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

/// The tiers that drive the built binary.
fn tier_scripts() -> Vec<PathBuf> {
    let scripts = mcf_checks::workspace::root().join("scripts");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&scripts)
        .expect("the scripts directory is in the tree")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("check-") && name.ends_with(".sh"))
        })
        .collect();
    found.sort();
    found
}

/// The whole of a shell command, following `\` continuations.
///
/// A call split across lines is one call, and reading it line by line is how a
/// check comes to believe an engine was not named when the next line names it.
fn commands(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending = String::new();
    for line in source.lines() {
        let trimmed = line.trim_end();
        if let Some(head) = trimmed.strip_suffix('\\') {
            pending.push_str(head);
            pending.push(' ');
            continue;
        }
        pending.push_str(trimmed);
        out.push(std::mem::take(&mut pending));
    }
    if !pending.is_empty() {
        out.push(pending);
    }
    out
}

/// Every `mcf run` in a tier names the engine it is asking about.
#[test]
fn no_tier_runs_a_model_without_naming_the_engine() {
    let mut offenders = Vec::new();
    for path in tier_scripts() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        for command in commands(&source) {
            let trimmed = command.trim_start();
            // Comments explain the rule; they do not break it.
            if trimmed.starts_with('#') {
                continue;
            }
            if !command.contains("run ") {
                continue;
            }
            // The binary is invoked through a variable in every tier.
            if !command.contains("$mcf\"") && !command.contains("${mcf}") {
                continue;
            }
            if !command.contains("--engine") {
                offenders.push(format!("{name}: {}", command.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a scheduled tier runs a model without naming an engine, so its answer depends on \
         whether a daemon happens to be listening (F102, F103, §3.12): {offenders:#?}"
    );
}

/// The oracle names the stand-in specifically, because that is the whole claim.
///
/// A19's mechanical form is *MCF's own engine against an independent one*. An
/// oracle whose `mine` came from the provisioned engine would compare the
/// reference with itself — and would pass, which is the dangerous part.
#[test]
fn the_oracle_compares_mcfs_own_engine_against_the_reference() {
    let oracle =
        std::fs::read_to_string(mcf_checks::workspace::root().join("scripts/check-oracle.sh"))
            .expect("the oracle tier is in the tree");

    assert!(
        oracle.contains("MINE_ENGINE=stand-in"),
        "the oracle no longer pins `mine` to MCF's own engine, so it may be comparing the \
         reference implementation with itself (F103, A19)"
    );
    for command in commands(&oracle) {
        if command.trim_start().starts_with('#') {
            continue;
        }
        if command.contains("mine=$(") && command.contains("run ") {
            assert!(
                command.contains("$MINE_ENGINE"),
                "the oracle's `mine` is produced by a run that does not name MCF's own \
                 engine: {}",
                command.trim()
            );
        }
    }
}

/// The corpus names its engine, and says so where a reader will see it.
#[test]
fn the_corpus_states_which_engine_answered() {
    let corpus =
        std::fs::read_to_string(mcf_checks::workspace::root().join("scripts/check-corpus.sh"))
            .expect("the corpus tier is in the tree");
    assert!(
        corpus.contains("ENGINE=stand-in"),
        "the corpus no longer pins its engine (F102)"
    );
    assert!(
        corpus.contains("named rather than inherited"),
        "the corpus no longer prints which engine answered, so a reader cannot tell what the \
         result is about (§3.4)"
    );
}
