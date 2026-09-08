#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

struct Ambient {
    call: &'static str,
    instead: &'static str,
    why: &'static str,
}

const AMBIENT: &[Ambient] = &[Ambient {
    call: "run(",
    instead: "run_where(",
    why: "it looks up whether a daemon is listening, and which of MCF and a daemon answers \
          changes the refusal a caller sees — so a test calling it reports on the machine \
          (F46, B-378)",
}];

const WATCHED: &[&str] = &["crates/mcf-cli/src/run/tests.rs"];

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
