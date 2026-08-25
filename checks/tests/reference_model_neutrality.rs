//! No code path knows which model is the reference model.
//!
//! §XII names `unsloth/Qwen3.8-27B-GGUF` as the subject of early benchmarking,
//! probing and validation. B28 makes it *a fixture, never a case in the code*:
//! no code path behaves differently because an artifact is the reference model,
//! and the test suite never depends on it. B28's check is mechanical, and B-018
//! is the item: **a grep-level check that no identifier names the reference
//! model outside fixtures and documentation.**
//!
//! B28's violation is *a special case for GGUF-from-unsloth that makes the
//! reference model work and quietly breaks the next artifact*, and B29 says why
//! it matters beyond tidiness: results from one model characterize the
//! instrument and characterize models in general not at all, so a code path
//! that recognizes it is a code path that has stopped measuring.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// The names that would give it away.
///
/// The publisher, the family and the exact reference are all listed: a check
/// for the full reference alone would miss `if family == "qwen"`, which is the
/// same defect with less typing.
const NAMES: [&str; 4] = ["unsloth", "Qwen", "qwen", "QWEN"];

/// No shipped source names the reference model.
#[test]
fn no_shipped_source_names_the_reference_model() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (number, line) in source.lines().enumerate() {
            // Documentation may name it — §XII does, and a doc comment
            // explaining why the reference model is the hard provenance case is
            // exactly the fixture-and-documentation exemption B28 states.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in NAMES {
                if line.contains(name) {
                    offenders.push(format!("{}:{} → {name}", relative(&path), number + 1));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a code path names the reference model (B28, B-018): {offenders:#?}\n\
         §XII makes it a fixture. A path that recognizes it is a path that has \
         stopped measuring (B29)."
    );
}

/// Nor does the laboratory, the prototype, or the checks.
///
/// B28 says *the test suite never depends on it*, which is the half that
/// erodes quietly: a fixture named after the reference model becomes a fixture
/// that only works for it.
#[test]
fn no_test_depends_on_the_reference_model() {
    let root = mcf_checks::workspace::root();
    let mut offenders = Vec::new();
    for directory in ["crates", "checks", "prototypes"] {
        for path in rust_sources(&root.join(directory)) {
            // This file names them in order to forbid them, which is the one
            // place the names have to appear.
            if path.ends_with("reference_model_neutrality.rs") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (number, line) in source.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for name in NAMES {
                    if line.contains(name) {
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

/// The documents may name it, and one of them must — otherwise this check is
/// passing because the reference model has quietly stopped existing.
#[test]
fn the_documents_still_name_it() {
    let intent =
        std::fs::read_to_string(mcf_checks::workspace::root().join("doc/document-of-intent.md"))
            .expect("the intent document is readable");
    assert!(
        NAMES.iter().any(|name| intent.contains(name)),
        "no document names the reference model, so this check is checking nothing"
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
