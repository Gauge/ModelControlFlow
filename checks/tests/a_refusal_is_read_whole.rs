//! A refusal the daemon sends is read by the client that receives it (A2).
//!
//! **What was found.** `mcf settings <model>` printed *refused — MCF did not
//! say why*. The daemon had said why, at length: the category, the detail,
//! what it wanted and what it found, and the cause beneath. The client read
//! a key named `what` from the body, the encoding writes no such key, and the
//! fallback text was a false statement about the daemon. Six readers across
//! the console and the window had each copied the same line (F148).
//!
//! **What this holds.** No shipped client source carries that fallback, and
//! no shipped client prints a refusal body as a raw line. Reading a failure
//! is [`mcf_record::decode::failure_said`]'s job, and a reader that wants
//! something else from a refusal goes through it.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// The crates that talk to the daemon on a person's behalf.
const CLIENTS: &[&str] = &["crates/mcf-cli/src", "crates/mcf-desk/src"];

/// Text a client must never fall back to over a body that said why.
const NEVER: &[&str] = &["did not say why", ".get(\"what\")"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("the checks crate sits in the workspace"))
        .to_path_buf()
}

fn sources(under: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(under) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|held| held == "rs")
            && !path.to_string_lossy().contains("tests")
        {
            into.push(path);
        }
    }
}

#[test]
fn no_client_says_the_daemon_did_not_say_why() {
    let mut found = Vec::new();
    for client in CLIENTS {
        sources(&root().join(client), &mut found);
    }
    assert!(!found.is_empty(), "the clients have source");
    let mut offending = Vec::new();
    for path in found {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} reads: {error}", path.display()));
        for (number, line) in text.lines().enumerate() {
            if NEVER.iter().any(|never| line.contains(never)) {
                offending.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a client reads a refusal with something other than failure_said:\n{}",
        offending.join("\n")
    );
}
