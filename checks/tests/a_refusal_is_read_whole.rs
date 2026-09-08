#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const CLIENTS: &[&str] = &["crates/mcf-cli/src", "crates/mcf-desk/src"];

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
