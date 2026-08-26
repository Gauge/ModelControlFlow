//! The four gated categories are where they say they are (B-039, §6.14).
//!
//! §6.14 draws the line at category rather than frequency: untrusted execution,
//! large irrecoverable resource use, network exposure and destruction are asked
//! about **every time**, and everything else flows.
//! `mcf_core::authorization` enumerates them and says, for each, where MCF asks
//! — or that no path exists to ask about. This reads that enumeration and
//! checks it against the tree.
//!
//! **Why the *absences* need checking most.** Two of the four are claimed as
//! *no path exists*: MCF runs nothing it acquires, and listens on no network.
//! Those are the strongest statements in the file and the easiest to falsify by
//! accident — a `TcpListener` added for a convenience, a subprocess spawned for
//! a diagnostic. A gate nobody built is fine while the capability is absent and
//! a hole the moment it is not, so what this asserts is the absence itself.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

use mcf_core::authorization::{Asking, GATED, Gated};

/// Every gate is either a command that exists or an absence that holds.
#[test]
fn every_gate_is_where_it_says_it_is() {
    for gate in GATED {
        match gate.asking() {
            Asking::ByCommand { command, .. } => {
                let surface =
                    read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/main.rs"));
                let verb = command.strip_prefix("mcf ").unwrap_or(command);
                assert!(
                    surface.contains(&format!("\"{verb}\"")),
                    "{gate} says it is asked by `{command}` and the surface has no such command"
                );
            }
            Asking::NoPathExists { .. } => {}
            // `Asking` is non-exhaustive: a shape added later is a way of
            // asking this check has not been taught, and saying so is better
            // than passing over it.
            other => panic!("{gate} is asked in a way this check does not know: {other:?}"),
        }
    }
}

/// Destruction: `mcf rm` will not destroy anything without a stated reason, and
/// nothing is deleted at all without `--purge`.
#[test]
fn destruction_needs_a_reason_and_a_second_word_to_delete() {
    let Asking::ByCommand { and, .. } = Gated::Destruction.asking() else {
        panic!("destruction stopped being asked by a command without this check changing");
    };
    assert!(and.contains("--purge"), "{and}");

    let removal = read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/models.rs"));
    assert!(
        removal.contains("let Some(reason) = reason else"),
        "`rm` no longer previews when no reason is given (B-027, §3.11)"
    );
    assert!(
        removal.contains("nothing was deleted"),
        "`rm` no longer says that it deleted nothing"
    );

    let store = read(&mcf_checks::workspace::root().join("crates/mcf-hub/src/store.rs"));
    assert!(
        store.contains(
            "pub fn purge(removed: &Removed, authorization: &Authorization, plan: &Plan)"
        ),
        "purging no longer takes the authorization it was given for"
    );
}

/// Large irrecoverable use: `mcf pull` acquires what was named and nothing
/// else. A repository asked for without a file is answered, not fetched.
#[test]
fn a_large_download_happens_only_when_it_is_named() {
    let Asking::ByCommand { and, .. } = Gated::LargeIrrecoverableUse.asking() else {
        panic!("acquisition stopped being asked by a command without this check changing");
    };
    assert!(and.contains("named"), "{and}");

    let pull = read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/pull.rs"));
    assert!(
        pull.contains("let Some(wanted) = reference.file.clone() else"),
        "`pull` no longer requires a file to be named before it acquires one (§6.14)"
    );
    assert!(
        pull.contains("nothing was acquired"),
        "`pull` no longer says that it acquired nothing"
    );
}

/// Network exposure: nothing in the shipped tree listens where another machine
/// could reach it.
///
/// The claim `mcf_core::authorization` makes is that exposure is not something
/// a mistake can do because it is not something MCF can do. A listener is the
/// act, so this finds every one and holds it to one of two shapes: a Unix
/// socket, which has no address another machine can name, or a loopback port
/// bound by the laboratory to talk to itself — declared below, with what it is
/// for.
///
/// Anything else is MCF having learned to be reachable, which is a capability
/// §6.12 gates and B-036 would have to build the gate for.
#[test]
fn nothing_listens_where_another_machine_could_reach_it() {
    let Asking::NoPathExists { why } = Gated::NetworkExposure.asking() else {
        panic!(
            "MCF has learned to expose itself to a network, and the gate §6.12 asks for is \
             now owed rather than unnecessary (B-036)"
        );
    };
    assert!(why.contains("Unix socket"), "{why}");

    // The laboratory listens on the loopback address to drive MCF's own client
    // against something that answers (B-028, D26). It is in the shipped binary
    // because A22 and B19 put the laboratory in the product, and it is declared
    // here for the same reason every deletion is: so that a second listener
    // cannot appear without somebody writing down what it is for.
    let declared: &[(&str, &str)] = &[(
        "crates/mcf-lab/src/serving.rs",
        "the laboratory's hub on a socket: a port the kernel chooses on 127.0.0.1, so that \
         code which talks to an operating system can be tested against something that answers",
    )];

    let root = mcf_checks::workspace::root();
    let mut reachable = Vec::new();
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        for line in code_only(&ships(&read(&file))).lines() {
            let trimmed = line.trim();
            if !trimmed.contains("bind(") {
                continue;
            }
            // A Unix socket has no address another machine can name.
            if trimmed.contains("UnixListener") {
                continue;
            }
            let loopback = trimmed.contains("127.0.0.1") || trimmed.contains("[::1]");
            let allowed = declared.iter().any(|(file, _)| *file == relative);
            if !(loopback && allowed) {
                reachable.push(format!("{relative}: {trimmed}"));
            }
        }
    }

    assert!(
        reachable.is_empty(),
        "MCF binds something that is not a Unix socket or a declared loopback port, so \
         network exposure is now a capability and needs the gate §6.12 asks for (B-036, \
         B-039): {reachable:#?}"
    );

    // And nothing declared has stopped listening, which would leave a licence
    // to listen that nobody is using.
    for (file, what) in declared {
        let source = code_only(&ships(&read(&root.join(file))));
        assert!(
            source.contains("bind("),
            "{file} is declared as a listener and no longer binds anything: {what}"
        );
    }
}

/// Untrusted execution: the absence is asserted next door, and this checks that
/// the two files still agree about which absence it is.
#[test]
fn untrusted_execution_is_the_absence_the_other_check_holds() {
    let Asking::NoPathExists { why } = Gated::UntrustedExecution.asking() else {
        panic!(
            "MCF has learned to run something it acquired, and §6.4's gate is now owed \
             rather than unnecessary (B-025)"
        );
    };
    assert!(why.contains("B-025"), "{why}");
    assert!(
        mcf_checks::workspace::root()
            .join("checks/tests/nothing_acquired_is_ever_run.rs")
            .exists(),
        "the check that holds this absence is gone and the claim is now unheld"
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// A file with its inline test module cut off.
fn ships(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(at) => source.get(..at).unwrap_or(source).to_owned(),
        None => source.to_owned(),
    }
}

/// A file with its documentation removed: the prose names what it forbids in
/// order to say it is absent.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn shipped_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(&root.join("crates"), &mut found);
    found.retain(|path| {
        let text = path.display().to_string();
        !text.contains("/tests/") && !text.ends_with("tests.rs")
    });
    found.sort();
    found
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            collect(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
