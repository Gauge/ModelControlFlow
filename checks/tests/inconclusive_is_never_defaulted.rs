//! An inconclusive probe leaves the capability unknown, and no code path may
//! quietly supply a value instead.
//!
//! B-060 is the item and A7 is the rule: *unknown stays unknown*. D42 gives a
//! probe three outcomes rather than two, and the third one is load-bearing —
//! *the model did not do the thing* and *MCF could not tell* are different
//! facts, and only the first would license a configuration. The failure this
//! guards against is small and natural to write:
//!
//! ```ignore
//! let addressing = probed.outcome.observed().unwrap_or(&RAW);
//! ```
//!
//! which reads as caution and is the opposite of it: MCF's own default handed
//! back as though the probe had confirmed it. F38 is why the check exists
//! rather than the rule being trusted — there, a probe that *did* decide
//! decided backwards, and the only reason it was caught is that the
//! inconclusive path was still saying *inconclusive* honestly enough to be
//! compared against.
//!
//! Two things are checked. The type may not grow a fallback accessor, because
//! then every caller has one; and no caller may build a fallback out of the
//! accessor the type does have.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// Where the outcome type lives.
const OUTCOME: &str = "crates/mcf-core/src/probe.rs";

/// Ways of substituting a value for one the type has said it does not have,
/// silently.
///
/// Listed as spellings rather than as one pattern because the point is to be
/// read: somebody adding a fallback should meet this list and have to argue
/// with it, which is B16's machine-checked form of a rule doing its work.
const SILENT: [&str; 4] = [
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "or_default()",
];

/// Ways of substituting a value *loudly*.
///
/// These are forbidden in shipped code for the same reason as the silent ones
/// — a probe that could not tell is not an error to panic on, it is an
/// observation — but they are how a test says *the probe decided here*, which
/// is an assertion and the thing the test exists to make. So they are checked
/// everywhere except in test files, where the meaning inverts.
const ASSERTING: [&str; 2] = ["expect(", "unwrap()"];

/// The outcome type offers no way to get a value out of an inconclusive
/// result.
///
/// `observed()` returns an `Option` and that is the whole surface. A method
/// that took a default would put the coercion inside the type, where every
/// caller inherits it and no caller is visible in a diff.
#[test]
fn the_outcome_type_has_no_fallback_accessor() {
    let path = mcf_checks::workspace::root().join(OUTCOME);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} could not be read: {error}", path.display()));

    let mut offenders = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let code = line.trim_start();
        // A doc comment may name a fallback in order to forbid it, which is
        // what the one in `probe.rs` does. Only code is checked.
        if code.starts_with("//") {
            continue;
        }
        if SILENT
            .iter()
            .chain(ASSERTING.iter())
            .any(|fallback| code.contains(fallback))
        {
            offenders.push(format!("{}:{}: {}", OUTCOME, number + 1, line.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "the outcome type must not offer a value where it has said there is \
         none — an inconclusive probe leaves the capability unknown (A7, D42, \
         B-060):\n{}",
        offenders.join("\n")
    );
}

/// No caller turns the absence of an observation into a value.
///
/// The accessor returns an `Option` on purpose, and `.observed().unwrap_or(…)`
/// undoes that in eleven characters. Matching is on the two appearing together
/// on one line, which is how it would actually be written.
#[test]
fn no_caller_defaults_an_inconclusive_probe() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        // In a test, `expect` on an outcome the test itself constructed to be
        // conclusive is the assertion — it is how the test states that the
        // probe decided. Only the silent substitutions are wrong there.
        let is_a_test = relative(&path).contains("tests");
        for (number, line) in source.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains(".observed()") {
                continue;
            }
            let watched: Vec<&str> = if is_a_test {
                SILENT.to_vec()
            } else {
                SILENT.iter().chain(ASSERTING.iter()).copied().collect()
            };
            if watched.iter().any(|fallback| code.contains(fallback)) {
                offenders.push(format!(
                    "{}:{}: {}",
                    relative(&path),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a probe that could not tell must leave the capability unknown, and \
         these lines supply a value instead (A7, D42, B-060):\n{}",
        offenders.join("\n")
    );
}

/// The path as it reads in this repository.
fn relative(path: &Path) -> String {
    path.strip_prefix(mcf_checks::workspace::root())
        .unwrap_or(path)
        .display()
        .to_string()
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
