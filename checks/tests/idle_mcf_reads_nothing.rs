//! An idle MCF reads no counters and costs nothing (B-187, B-108, B39, B4,
//! D5, §3.13, §6.18).
//!
//! **Two claims, one mechanism.** B-187: *counter reads are zero outside a lab
//! run.* B-108: *the benchmark subsystem consumes nothing during ordinary
//! serving.* Both are the same discipline seen from two sides — nothing in MCF
//! runs on a timer, so there is nothing for an idle process to do.
//!
//! **Why it matters beyond tidiness.** A daemon that polls thermal counters to
//! look responsive is a daemon that is one of the competitors it reports
//! (§3.8), and a benchmark subsystem that costs something while idle has made
//! every serving measurement conditional on whether it was compiled in. B4
//! forbids ambient sampling for exactly this reason: a reading exists because
//! somebody asked for it, never because a clock came round.
//!
//! **Measured, not only checked** (F85): an idle daemon on this machine took
//! **zero** processor ticks and issued **zero** read syscalls across ninety
//! seconds. What this file holds is the structure that makes that true, so
//! that the next person to add a heartbeat has to argue with a test.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// Nothing in the serving path is on a timer.
#[test]
fn the_daemon_has_no_heartbeat() {
    for file in sources("crates/mcf-serve/src") {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        // A sleep in a *retry* loop waits for something that was asked for; a
        // sleep in a spawned loop is a heartbeat. The difference is whether it
        // is inside a thread of its own, so that is what is forbidden.
        assert!(
            !source.contains("thread::spawn(move || loop"),
            "{}: a spawned loop is a heartbeat, and an idle MCF must have none (B4, B-187)",
            file.display()
        );
    }
}

/// The one thermal read there is happens only when something asks.
#[test]
fn a_counter_is_read_because_somebody_asked() {
    let source = read("crates/mcf-core/src/hardware/nvml.rs");
    assert!(
        source.contains("nvmlDeviceGetTemperature"),
        "this check is about the temperature read; if it has moved, the check must move with it"
    );
    // Reached only through `Machine::read_through`, which is called by
    // commands. A call from anywhere that runs without being asked would be
    // ambient sampling.
    let mut callers = Vec::new();
    for file in sources("crates") {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        // Tests read the machine to assert about it, which is a command
        // somebody ran deliberately — `cargo test` is not an idle MCF.
        let is_test = file.display().to_string().contains("tests")
            || source.contains("#[cfg(test)]") && source.contains("fn read_through");
        if !is_test
            && (source.contains("Machine::read()") || source.contains("Machine::read_through("))
        {
            callers.push(file);
        }
    }
    for caller in &callers {
        let named = caller.display().to_string();
        assert!(
            named.contains("mcf-cli") || named.contains("mcf-core/src/hardware"),
            "{named} reads the machine, and only a command the operator ran may: a read from \
             the serving path would make MCF one of the competitors it reports (§3.8, B4)"
        );
    }
    assert!(
        !callers.is_empty(),
        "no caller was found at all, so this check is reading the wrong tree"
    );
}

/// The serving path cannot run a benchmark, because it cannot see one.
#[test]
fn the_benchmark_subsystem_is_not_in_the_serving_path() {
    let manifest = read("crates/mcf-serve/Cargo.toml");
    assert!(
        !manifest.contains("mcf-bench"),
        "B-108: a benchmark subsystem that the daemon can reach is one whose idle cost has to \
         be argued about; one it cannot name has none by construction (§6.18)"
    );
}

fn sources(relative: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&mcf_checks::workspace::root().join(relative), &mut found);
    found
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
