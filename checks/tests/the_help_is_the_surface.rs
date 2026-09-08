#![allow(clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

struct Unlisted {
    command: &'static str,
    because: &'static str,
}

const UNLISTED: &[Unlisted] = &[Unlisted {
    command: "log",
    because: "the record's own reader, listed under `mcf show` and `mcf export` rather than on \
              its own line; it takes no argument a reader has to be told about",
}];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("the checks crate sits in the workspace"))
        .to_path_buf()
}

fn console() -> String {
    std::fs::read_to_string(root().join("crates/mcf-cli/src/main.rs"))
        .unwrap_or_else(|error| panic!("the console is readable: {error}"))
}

fn a_command(from: &str) -> Option<&str> {
    let end = from
        .find(|held: char| !held.is_ascii_lowercase() && !held.is_ascii_digit() && held != '-')
        .unwrap_or(from.len());
    let name = from.get(..end)?;
    let first = name.chars().next()?;
    (first.is_ascii_lowercase() && name.len() > 1).then_some(name)
}

fn commands_parsed(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let end = source.find("#[cfg(test)]").unwrap_or(source.len());
    let patterns = source.get(..end).unwrap_or_default();
    for piece in patterns.split("[\"").skip(1) {
        #[allow(
            clippy::collapsible_if,
            reason = "the nested form is kept deliberately"
        )]
        if let Some(name) = a_command(piece) {
            if name != "mcf" {
                found.insert(name.to_owned());
            }
        }
    }
    found
}

fn usage_text(source: &str) -> &str {
    let Some(from) = source.find("const COMMANDS: &str = \"") else {
        return "";
    };
    let rest = source.get(from..).unwrap_or_default();
    let end = rest.find("\";").unwrap_or(rest.len());
    rest.get(..end).unwrap_or_default()
}

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
    let source = console();
    let mut denies = Vec::new();
    for command in commands_parsed(&source) {
        let takes_argument = source.contains(&format!("[\"{command}\", "));
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
