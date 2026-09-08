#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

fn tier_scripts() -> Vec<PathBuf> {
    let scripts = mcf_checks::workspace::root().join("scripts");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&scripts)
        .expect("the scripts directory is in the tree")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|kind| kind == "sh")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("check-"))
        })
        .collect();
    found.sort();
    found
}

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
            if trimmed.starts_with('#') {
                continue;
            }
            if !command.contains("run ") {
                continue;
            }
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
