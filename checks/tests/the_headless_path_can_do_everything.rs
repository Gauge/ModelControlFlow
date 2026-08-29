//! Every control-plane operation has a command, and no capability is reachable
//! only through a client (A22, B-072, §XI, §6.21).
//!
//! **A22 is absolute and its check named an item that did not exist.** *Every
//! action is available with no display attached; the interface may not be the
//! only way to do anything.* The rule notes that this is close to
//! self-enforcing — a capability reachable only through an interface is one the
//! laboratory cannot test, which A19 already forbids — but *close to* is not a
//! check, and B-072 is the row that was open.
//!
//! **What is enumerable today.** MCF has one surface, the command line, and one
//! wire protocol, the control plane. The direction that can go wrong right now
//! is an operation on that wire with no command to reach it: a daemon that
//! answers a question nobody at a terminal can ask. So every variant of
//! `mcf_serve::control::Request` must be sent by `crates/mcf-cli`.
//!
//! **The direction that cannot go wrong yet, and will.** When §XI's window
//! arrives it becomes a second client of the same wire, and A22's real target
//! is an action *it* has that no command does. The enumeration this file builds
//! is the half that can exist before the window does, and the reason to build
//! it now is the same one that put the privileged helper and reference-model
//! neutrality in M0: a special case is far cheaper to prevent than to find. The
//! window's own actions join this check when there are any.
//!
//! **Not the other direction.** A command with no control operation is
//! ordinary: most of MCF does its work in the calling process and never touches
//! a daemon. A22 constrains what the *interface* may have, never what the
//! headless path may.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// Every `.rs` under a directory.
fn sources(relative: &str) -> Vec<PathBuf> {
    let root = mcf_checks::workspace::root().join(relative);
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The operations the control plane answers, read from the protocol itself.
///
/// From the enum rather than from a list here: a list is a thing that goes
/// stale the week it is written, and the operation this check would then miss
/// is precisely the new one (F79, F110).
fn operations() -> Vec<String> {
    let source = read("crates/mcf-serve/src/control.rs");
    let (_, after) = source
        .split_once("pub enum Request {")
        .expect("the control plane's requests are an enum");
    let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
    let mut found = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        // A variant: `Status,` or `Generate {`. Not a field, which is
        // lowercase, and not documentation.
        let Some(name) = trimmed
            .strip_suffix(',')
            .or_else(|| trimmed.strip_suffix(" {"))
        else {
            continue;
        };
        if name.starts_with(char::is_uppercase)
            && name.chars().all(|held| held.is_ascii_alphanumeric())
            && !found.contains(&name.to_owned())
        {
            found.push(name.to_owned());
        }
    }
    found
}

/// Every control operation is sent by the headless surface.
#[test]
fn every_control_operation_has_a_command() {
    let operations = operations();
    assert!(
        operations.len() >= 4,
        "only {} control operation(s) were found, so this check is reading the protocol wrong: \
         {operations:#?}",
        operations.len()
    );

    let surface: String = sources("crates/mcf-cli/src")
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect();

    let mut orphans = Vec::new();
    for operation in &operations {
        if !surface.contains(&format!("Request::{operation}")) {
            orphans.push(operation.clone());
        }
    }
    assert!(
        orphans.is_empty(),
        "the control plane answers {orphans:#?}, and no command asks. A capability reachable \
         only through a client is what A22 forbids, and one reachable through neither is a \
         capability the laboratory cannot test (A19, B-072)"
    );
}

/// And the surface that sends them is the one a person types at.
///
/// The check above would pass if the only sender were a test or a probe's
/// helper. What A22 wants is a *command*: something in the argument parser that
/// a person reaches without writing Rust.
#[test]
fn the_operations_a_person_can_ask_for_are_commands() {
    let main = read("crates/mcf-cli/src/main.rs");
    for (operation, command) in [("Status", "status"), ("Stop", "stop"), ("Generate", "run")] {
        assert!(
            main.contains(&format!("[\"{command}\"")),
            "`{operation}` is answered by the daemon and `mcf {command}` is not a command a \
             person can type (A22)"
        );
    }
    // `Holding` is deliberately not its own command: it is the second half of
    // one question a person has — *what is running here* — and `mcf status`
    // asks both in one breath. What A22 requires is that it be reachable
    // without a display, which it is.
    assert!(
        read("crates/mcf-cli/src/serve.rs").contains("Request::Holding"),
        "nothing headless asks the daemon what it is holding (A22, B-072)"
    );
}

/// There is no second surface yet, and when there is, it joins this check.
///
/// Stated as an assertion rather than a comment so that the day a window crate
/// appears, this fails and somebody extends the enumeration instead of
/// discovering six months later that A22 was checked against one client.
#[test]
fn a_second_surface_would_have_to_be_enumerated_here() {
    let crates = mcf_checks::workspace::root().join("crates");
    let mut surfaces = Vec::new();
    for entry in std::fs::read_dir(&crates)
        .expect("the crates directory is in the tree")
        .flatten()
    {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        // A surface is a thing a person looks at. The window (§XI) is the one
        // A22 was written for; these are the names it could arrive under.
        if ["mcf-window", "mcf-web", "mcf-ui", "mcf-client"].contains(&name.as_str()) {
            surfaces.push(name);
        }
    }
    assert!(
        surfaces.is_empty(),
        "a second surface exists: {surfaces:#?}. A22 says the interface may not be the only way \
         to do anything, so its actions must be enumerated and matched against the control \
         plane here — which is the half of B-072 that could not be written before it existed"
    );
}
