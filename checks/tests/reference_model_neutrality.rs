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
//!
//! **Two things wear the same words, and only one of them is forbidden**
//! (DEC-053, F20). Recognizing an *artifact* — a publisher, a repository, a
//! file name, a digest — is what B28 forbids. Reading a *field the file itself
//! states* — `general.architecture`, `tokenizer.ggml.pre` — is what GGUF is
//! for, and an engine that refused to would be an engine that runs one family.
//!
//! The publisher is forbidden everywhere and unconditionally: it can only ever
//! be an artifact check. A family name is forbidden everywhere **except one
//! declared module**, and that module is then held to three things which
//! together mean it cannot be hiding an artifact check: it names no publisher,
//! it names no repository or file, and every table in it is **total** — each
//! has a default arm or is checked against a listed set, so an unnamed family
//! gets an answer MCF decided in advance rather than falling through to one
//! that happens to suit the reference model.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// The names that would give it away.
///
/// The publisher, the family and the exact reference are all listed: a check
/// for the full reference alone would miss `if family == "qwen"`, which is the
/// same defect with less typing.
const NAMES: [&str; 4] = ["unsloth", "Qwen", "qwen", "QWEN"];

/// The publisher, which no code path may name anywhere at all.
///
/// A family name can be a field GGUF states. A publisher cannot: nothing in a
/// model file that MCF reads is keyed on who published it, so a mention of this
/// in code is an artifact check and nothing else.
const PUBLISHER: &str = "unsloth";

/// The one module where a family may be named, and what it must be true of.
///
/// One place, so that the exemption is a structure rather than a habit: adding
/// a family means editing this file, and this check reads it.
const TABLES: &str = "crates/mcf-standin/src/architecture.rs";

/// What the reference model is, other than its family.
///
/// Its repository and its file: the spellings that can only mean *this
/// artifact*, which the exempt module may not contain either.
const THE_ARTIFACT: [&str; 2] = ["Qwen3.8-27B", "UD-Q4_K_M"];

/// No shipped source names the reference model.
#[test]
fn no_shipped_source_names_the_reference_model() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let exempt = path.ends_with("architecture.rs");
        for (number, line) in source.lines().enumerate() {
            // Documentation may name it — §XII does, and a doc comment
            // explaining why the reference model is the hard provenance case is
            // exactly the fixture-and-documentation exemption B28 states.
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

/// The exempt module names no artifact, and every table in it is total.
///
/// This is the whole of what the exemption rests on. Without it, `architecture.rs`
/// would be a place where B28 does not apply; with it, it is a place where a
/// family may be *named* and an artifact still may not be *recognized*.
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

    // Totality is *not* checked here, and the reason is worth stating: a
    // `match` on a `&str` does not compile without a catch-all, so the compiler
    // already guarantees that an unlisted family gets an answer. A check for it
    // would be a check that cannot fail (F19's lesson about tests that pass
    // vacuously).
    //
    // What is left to check is the shape of what this module does, and the
    // sharpest version of that is a question of types. A function here takes a
    // name the file stated and returns a *decision* — a rotation, a
    // pre-tokenizer. A function that took a name and returned a **bool** would
    // be answering a different question: *is this the special one*. That is the
    // shape B28 forbids, and it is the shape this module would take if it ever
    // stopped being a table.
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

    // And it must not be able to see an artifact at all: identity lives in
    // provenance, and a table that reached for a digest or a repository would
    // be recognizing rather than reading (B28).
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
