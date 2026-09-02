//! Every command MCF has is in `mcf --help`, and every command it lists is one
//! MCF has (A22, §3.15, B-072).
//!
//! **The console is the headless surface, and a surface nobody can find is not
//! one.** A22 says the headless path can do everything the window can. It can:
//! `mcf host`, `mcf hosted`, `mcf unhost` and `mcf settings` were all parsed,
//! dispatched and working. None of the four was in `mcf --help`, so the only
//! way to reach them was to already know they were there — and `mcf status`
//! and `mcf explain` both printed instructions to run two of them (F139).
//!
//! **The half that made it worse.** Each wanted an argument, so the bare form
//! matched no pattern and fell through to the catch-all: `mcf host` answered
//! *no such command: host*. A command that denies its own existence is not a
//! discoverability problem, it is a false statement — and the operator who
//! typed it had been sent there by MCF's own output.
//!
//! **What this holds.** The two lists agree. It reads the console's argument
//! patterns for the command each matches, reads the usage text for the
//! commands it names, and compares. That is a source scan and it is coarse: it
//! proves a name appears on both sides, not that the help describes the
//! command correctly. It is enough for the defect it exists to prevent, which
//! was a name on one side and not the other.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Commands deliberately absent from the usage text, and why.
struct Unlisted {
    command: &'static str,
    because: &'static str,
}

/// The declared omissions.
const UNLISTED: &[Unlisted] = &[Unlisted {
    command: "log",
    because: "the record's own reader, listed under `mcf show` and `mcf export` rather than on \
              its own line; it takes no argument a reader has to be told about",
}];

/// The workspace root, from this file's location.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("the checks crate sits in the workspace"))
        .to_path_buf()
}

/// The console's source.
fn console() -> String {
    std::fs::read_to_string(root().join("crates/mcf-cli/src/main.rs"))
        .unwrap_or_else(|error| panic!("the console is readable: {error}"))
}

/// A command name: lowercase letters, digits and dashes.
fn a_command(from: &str) -> Option<&str> {
    let end = from
        .find(|held: char| !held.is_ascii_lowercase() && !held.is_ascii_digit() && held != '-')
        .unwrap_or(from.len());
    let name = from.get(..end)?;
    let first = name.chars().next()?;
    (first.is_ascii_lowercase() && name.len() > 1).then_some(name)
}

/// Every command the argument patterns match on.
///
/// The patterns are slices of string literals, so the command is the first
/// literal after a `["`. A pattern beginning with a flag is not a command.
fn commands_parsed(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    // Before the tests, because a test's own argument literals are not the
    // console's patterns: one of them held `["measure"]` as an example of an
    // unknown command, which read here as `measure` having a bare form when it
    // had none (F139).
    let end = source.find("#[cfg(test)]").unwrap_or(source.len());
    let patterns = source.get(..end).unwrap_or_default();
    for piece in patterns.split("[\"").skip(1) {
        if let Some(name) = a_command(piece) {
            // The binary's own name appears in argument lists in the tests.
            // It is what commands are typed after, not one of them.
            if name != "mcf" {
                found.insert(name.to_owned());
            }
        }
    }
    found
}

/// The usage text itself, rather than the whole file.
///
/// Scanning the source for `mcf <name>` picked up a *test* asserting that
/// `mcf lab` and `mcf recommend` are not offered, and reported them as
/// commands the help invents. The usage text is what a reader sees, so the
/// usage text is what is read.
///
/// The text is the `COMMANDS` table, one constant the parser reads too
/// (B-426); the block is the string literal it is set to.
fn usage_text(source: &str) -> &str {
    let Some(from) = source.find("const COMMANDS: &str = \"") else {
        return "";
    };
    let rest = source.get(from..).unwrap_or_default();
    // The block ends where the literal does.
    let end = rest.find("\";").unwrap_or(rest.len());
    rest.get(..end).unwrap_or_default()
}

/// Every command the usage text names, which it does as `mcf <name>`.
fn commands_listed(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for piece in usage_text(source).split("mcf ").skip(1) {
        if let Some(name) = a_command(piece) {
            found.insert(name.to_owned());
        }
    }
    found
}

#[test]
fn every_command_mcf_has_is_in_the_help() {
    let source = console();
    let parsed = commands_parsed(&source);
    let listed = commands_listed(&source);
    assert!(!parsed.is_empty(), "no commands were found to check");

    let missing: Vec<&String> = parsed
        .iter()
        .filter(|held| !listed.contains(*held))
        .filter(|held| !UNLISTED.iter().any(|allowed| allowed.command == **held))
        .collect();
    // An omission whose reason no longer holds is a command that should be
    // listed: the reason names where a reader is told about it instead.
    for allowed in UNLISTED {
        assert!(
            allowed.because.contains("listed under") && parsed.contains(allowed.command),
            "`{}` is declared unlisted for a reason that does not say where it is told about \
             instead, or it is no longer a command: {}",
            allowed.command,
            allowed.because
        );
    }
    assert!(
        missing.is_empty(),
        "MCF answers these and `mcf --help` does not mention them, so the only way to reach \
         them is to already know they are there (A22, §3.15, F139): {missing:?}\n\nEither add \
         a usage line, or declare it in UNLISTED with the reason."
    );
}

#[test]
fn every_command_the_help_lists_is_one_mcf_has() {
    // The other direction, which is the worse one: a help that names a command
    // MCF does not answer sends the operator to a dead end and costs them
    // their trust in the rest of the page.
    let source = console();
    let parsed = commands_parsed(&source);
    let listed = commands_listed(&source);
    let phantom: Vec<&String> = listed
        .iter()
        .filter(|held| !parsed.contains(*held))
        .collect();
    assert!(
        phantom.is_empty(),
        "the help names these and the console does not answer them (§3.15): {phantom:?}"
    );
}

#[test]
fn a_command_that_needs_an_argument_does_not_deny_existing() {
    // The specific falsehood F139 was. Every command taking an argument needs
    // a bare-form pattern, or the catch-all answers *no such command* for a
    // command MCF has.
    let source = console();
    let mut denies = Vec::new();
    for command in commands_parsed(&source) {
        let takes_argument = source.contains(&format!("[\"{command}\", "));
        // Two shapes reach the bare form. An explicit `["x"]` pattern, and a
        // rest-pattern `["x", rest @ ..]`, which matches an empty rest and
        // hands the missing argument to that command's own option parser.
        let has_bare = source.contains(&format!("[\"{command}\"]"))
            || source.contains(&format!("[\"{command}\", rest @ ..]"));
        if takes_argument && !has_bare {
            denies.push(command);
        }
    }
    assert!(
        denies.is_empty(),
        "typed bare, each of these falls through to the catch-all and answers `no such \
         command`, denying a command MCF has (A2, F139): {denies:?}\n\nGive each a bare \
         pattern returning `MissingArgument` with what it needs."
    );
}
