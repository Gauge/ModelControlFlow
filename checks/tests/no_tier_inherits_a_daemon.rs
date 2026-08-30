//! A shell tier answers from the binary it built, never from a daemon somebody
//! left running (F104, F103, F102, F46, §3.12, A19).
//!
//! **The third guard on the same defect, one level further out.** F46 found a
//! Rust test whose answer depended on whether a daemon was up, and F47 guarded
//! Rust sources against the calls that reach for ambient state. F102 found a
//! shell tier reporting on the machine rather than on the models, and F103
//! found the same shape in the oracle — where `mine` was `mcf run` with no
//! engine named, so a listening daemon serving the reference implementation
//! would have made the oracle compare the reference **with itself**. Every
//! `mcf run` in a tier names its engine now, which
//! `no_tier_inherits_an_engine.rs` holds.
//!
//! **And naming the engine is not enough.** The daemon honours `--engine
//! stand-in` — that was read in `mcf_serve::generation::choose_engine` and then
//! measured — but the daemon's stand-in is *its own binary's*, built from
//! whatever source that process was started with. Measured on this machine
//! (F104): a client built with a marked account, asked to run a model with a
//! daemon listening from an unmarked build, printed the **unmarked** engine
//! line. The tier had built a binary and then asked a different one.
//!
//! So a tier that drives a subcommand which can reach a daemon brings a runtime
//! directory of its own. The Rust tier has done this since it was written —
//! `crates/mcf-cli/tests/whole_system.rs` gives every process a machine of its
//! own, socket included — and the shell tiers handed it the operator's.
//!
//! **Why the subcommand list is derived rather than typed.** A guard covers the
//! shape of the place it was written for (F103's own lesson), so the set of
//! subcommands that can reach a daemon is read out of the CLI: every module
//! that calls `crate::serve::socket_path()`. A new one arrives with a name this
//! file does not know, and the check fails until somebody writes down which
//! subcommand it is — which is the ratchet, not an inconvenience.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

/// A module of the CLI that looks for the control socket, and the subcommand a
/// person types to reach it.
struct Reaches {
    /// The file, relative to the workspace root.
    module: &'static str,
    /// How it is spelled on a command line. `None` where the module is the
    /// daemon itself rather than a way of asking one something.
    subcommand: Option<&'static str>,
}

/// Every module that consults the control socket, and what it is called.
///
/// `serve.rs` is where the socket's location is decided and where the daemon
/// itself lives; a tier that runs `mcf serve` is starting one rather than
/// inheriting one, so it is not on the list of things to isolate.
const REACHES: &[Reaches] = &[
    Reaches {
        module: "crates/mcf-cli/src/serve.rs",
        subcommand: None,
    },
    Reaches {
        module: "crates/mcf-cli/src/run.rs",
        subcommand: Some("run"),
    },
    Reaches {
        module: "crates/mcf-cli/src/bench.rs",
        subcommand: Some("bench"),
    },
    Reaches {
        module: "crates/mcf-cli/src/probe.rs",
        subcommand: Some("probe"),
    },
    Reaches {
        module: "crates/mcf-cli/src/measure.rs",
        subcommand: Some("measure"),
    },
    // One module, two subcommands: they send the two halves of the same act
    // — what is published, and fetch one of it — and splitting them into two
    // files to satisfy a table would be arranging the code around the check.
    Reaches {
        module: "crates/mcf-cli/src/acquire.rs",
        subcommand: Some("offered"),
    },
    Reaches {
        module: "crates/mcf-cli/src/acquire.rs",
        subcommand: Some("acquire"),
    },
    Reaches {
        module: "crates/mcf-cli/src/crosscheck.rs",
        subcommand: Some("cross-check"),
    },
    // The terminal application is a client of the same socket. It runs until
    // the operator quits and no tier drives it, but being written down is what
    // keeps that true (F104).
    Reaches {
        module: "crates/mcf-cli/src/tui.rs",
        subcommand: Some("tui"),
    },
    // The window is a client of the same socket, for the same reason the
    // console is. It runs until it is closed and no tier drives it, but being
    // written down is what keeps that true (F104).
    Reaches {
        module: "crates/mcf-cli/src/desk.rs",
        subcommand: Some("desk"),
    },
];

/// How a tier says it brought its own: the helper in `scripts/lib-tiers.sh`.
const HELPER: &str = "tier_private_runtime_dir";

/// The scheduled tiers, which are the scripts named `check-*.sh`.
fn tier_scripts() -> Vec<PathBuf> {
    let scripts = mcf_checks::workspace::root().join("scripts");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&scripts)
        .expect("the scripts directory is in the tree")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|kind| kind == "sh")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("check-"))
        })
        .collect();
    found.sort();
    found
}

/// Lines that are not comments, which is where a rule can be broken.
fn instructions(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with('#'))
}

/// The set of modules that call `socket_path`, read from the source.
fn modules_that_reach() -> Vec<String> {
    let root = mcf_checks::workspace::root();
    let source = root.join("crates/mcf-cli/src");
    let mut found = Vec::new();
    let mut stack = vec![source];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|kind| kind != "rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            // The definition itself is in `serve.rs`; a *call* is what makes a
            // module able to reach a daemon.
            if instructions(&text).any(|line| {
                line.contains("socket_path()") && !line.contains("pub(crate) fn socket_path")
            }) {
                let relative = path
                    .strip_prefix(mcf_checks::workspace::root())
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

/// The table above still names every module that can reach a daemon.
#[test]
fn every_module_that_reaches_a_daemon_is_written_down() {
    let found = modules_that_reach();
    let mut unknown: Vec<&String> = found
        .iter()
        .filter(|module| !REACHES.iter().any(|known| known.module == module.as_str()))
        .collect();
    unknown.sort();
    assert!(
        unknown.is_empty(),
        "a CLI module consults the control socket and this check does not know which \
         subcommand it is, so a tier could drive it and inherit a daemon unnoticed (F104): \
         {unknown:#?}"
    );

    let mut stale: Vec<&str> = REACHES
        .iter()
        .map(|known| known.module)
        .filter(|module| !found.iter().any(|seen| seen == module))
        .collect();
    stale.sort_unstable();
    assert!(
        stale.is_empty(),
        "this check watches modules that no longer consult the socket; a stale list is a \
         list nobody trusts: {stale:#?}"
    );
}

/// A tier that drives a daemon-reaching subcommand brings its own socket.
#[test]
fn no_tier_lets_a_daemon_answer_for_the_binary_it_built() {
    let mut offenders = Vec::new();
    for path in tier_scripts() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        // Whether this tier runs the binary MCF builds at all.
        let invokes = |subcommand: &str| {
            instructions(&source).any(|line| {
                (line.contains("$mcf\"") || line.contains("${mcf}"))
                    && line.contains(&format!(" {subcommand} "))
            })
        };
        let driven: Vec<&str> = REACHES
            .iter()
            .filter_map(|reaches| reaches.subcommand)
            .filter(|subcommand| invokes(subcommand))
            .collect();
        if driven.is_empty() {
            continue;
        }
        let isolates = instructions(&source).any(|line| line.contains(HELPER))
            && instructions(&source).any(|line| line.contains("export XDG_RUNTIME_DIR="));
        if !isolates {
            offenders.push(format!(
                "{name}: drives {driven:?} and does not export a runtime directory of its own \
                 via {HELPER}"
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "a scheduled tier drives a subcommand that consults the control socket without \
         bringing a socket of its own, so a daemon built from other source can answer for the \
         binary the tier just built (F104, §3.12): {offenders:#?}"
    );
}

/// The helper the tiers rely on is still there and still short by construction.
///
/// A Unix socket path has to fit in `sun_path`, and the first attempt at F104's
/// experiment put the directory under a path 96 bytes long: the daemon refused
/// to start. That refusal was honest, and a tier that isolates itself only
/// where the path happens to be short isolates itself on some machines and not
/// others.
#[test]
fn the_helper_bounds_the_socket_path() {
    let library =
        std::fs::read_to_string(mcf_checks::workspace::root().join("scripts/lib-tiers.sh"))
            .expect("the tier library is in the tree");
    assert!(
        library.contains(&format!("{HELPER}()")),
        "the helper the tiers call no longer exists (F104)"
    );
    assert!(
        library.contains("108"),
        "the helper no longer bounds the socket path against `sun_path`, so a machine with a \
         long TMPDIR would silently fail to isolate (F104)"
    );
}

/// What a person compares the reported build against.
fn shell(command: &str) -> Option<String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The build a run reports is the binary that ran it, checked against a digest
/// this repository did not compute (A19, F93, F104).
#[test]
fn the_reported_build_is_the_binary_that_ran() {
    let identifier = mcf_core::build_identity::identifier();
    let Some((version, reported)) = identifier.split_once('+') else {
        panic!("the build identifier no longer names an instrument: {identifier}");
    };
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "the build identifier no longer starts with the version"
    );
    if reported == "unknown" {
        // A7: the platform would not let the binary read itself. That is a
        // state, and it is not this check's business to invent one.
        return;
    }

    let Ok(binary) = std::env::current_exe() else {
        return;
    };
    let Some(independent) = shell(&format!(
        "sha256sum {} 2>/dev/null | cut -c1-12",
        Path::new(&binary).display()
    )) else {
        return;
    };
    if independent.is_empty() {
        // No `sha256sum` here. Said rather than silently passing: a check that
        // could not run has not run (B38).
        eprintln!("no sha256sum on this machine; the digest was not cross-checked");
        return;
    }
    assert_eq!(
        reported, independent,
        "the build MCF reports is not the digest of the binary that reported it"
    );
}
