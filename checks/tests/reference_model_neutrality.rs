#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const NAMES: [&str; 4] = ["unsloth", "Qwen", "qwen", "QWEN"];

const PUBLISHER: &str = "unsloth";

const TABLES: &str = "crates/mcf-standin/src/architecture.rs";

const THE_ARTIFACT: [&str; 2] = ["Qwen3.8-27B", "UD-Q4_K_M"];

#[test]
fn no_shipped_source_names_the_reference_model() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let exempt = path.ends_with("architecture.rs");
        for (number, line) in source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in NAMES {
                if line.contains(name) && !(exempt && name != PUBLISHER) {
                    offenders.push(format!("{}:{} → {name}", relative(&path), number + 1));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a code path names the reference model (B28, B-018): {offenders:#?}\n\
         §XII makes it a fixture. A path that recognizes it is a path that has \
         stopped measuring (B29). A family GGUF states may be named in {TABLES} \
         and nowhere else (DEC-053)."
    );
}

#[test]
fn the_tables_name_families_and_never_an_artifact() {
    let path = mcf_checks::workspace::root().join(TABLES);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{TABLES} is the declared exemption and must exist"));

    let code: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    for line in &code {
        assert!(
            !line.contains(PUBLISHER),
            "the exempt module names the publisher, which can only be an artifact check: {line}"
        );
        for artifact in THE_ARTIFACT {
            assert!(
                !line.contains(artifact),
                "the exempt module names the reference artifact itself: {line}"
            );
        }
    }

    let mut taking_a_name = 0;
    for line in &code {
        let Some(after) = line.split_once("fn ") else {
            continue;
        };
        if !after.1.contains("&str") {
            continue;
        }
        taking_a_name += 1;
        assert!(
            !after.1.contains("-> bool"),
            "a function in {TABLES} takes a name and answers yes or no: {line}\n\
             a table returns a decision; a predicate over a name is `is this the special one`, \
             which is what B28 forbids (DEC-053)"
        );
    }
    assert!(
        taking_a_name > 0,
        "no function in {TABLES} receives a name, so this check is checking nothing"
    );

    for reaching in [
        "fs::",
        "digest",
        "sha256",
        "repository",
        "provenance",
        "Path",
    ] {
        for line in &code {
            assert!(
                !line.contains(reaching),
                "the exempt module reaches for artifact identity (`{reaching}`): {line}"
            );
        }
    }
}

#[test]
fn no_test_depends_on_the_reference_model() {
    let root = mcf_checks::workspace::root();
    let mut offenders = Vec::new();
    for directory in ["crates", "checks", "prototypes"] {
        for path in rust_sources(&root.join(directory)) {
            if path.ends_with("reference_model_neutrality.rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let exempt =
                path.ends_with("architecture.rs") || path.ends_with("architecture/tests.rs");
            for (number, line) in source.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for name in NAMES {
                    if line.contains(name) && !(exempt && name != PUBLISHER) {
                        offenders.push(format!("{}:{} → {name}", relative(&path), number + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the suite has acquired a dependency on the reference model (B28, B19): {offenders:#?}"
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
