//! The daemon runs unprivileged, and the privileged surface is the helper's
//! (B-190, D35, §6.32, A26).
//!
//! **What B-190's condition asks.** *The daemon runs unprivileged in every
//! scenario; the helper's surface is enumerated.* The second half is a list in
//! code, checked against D35 below. The first half cannot be checked by running
//! the daemon — a daemon that holds no rights looks exactly like one whose
//! rights nobody exercised — so what is checked is the thing that would make it
//! possible: no shipped code outside the helper reaches for elevation, and
//! nothing links the helper into a process that stays up.
//!
//! **Why a static check is the right shape here.** §6.32's rule is structural:
//! privilege lives in one short executable that does one thing and exits. A
//! test that watched a daemon behave would confirm today's behaviour; this
//! confirms the property that makes tomorrow's behaviour possible, which is
//! what A26 is about.

// Every item in this file is test code; see the note in checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

/// The helper, which is allowed to be about privilege because it is the thing
/// that has it.
const HELPER: &str = "crates/mcf-helper";

/// Ways a program reaches for rights it was not started with.
const ELEVATES: &[&str] = &[
    "setuid",
    "seteuid",
    "setresuid",
    "setgid",
    "setegid",
    "capset",
    "prctl",
    "sudo",
    "pkexec",
    "doas",
    "polkit",
];

/// Nothing but the helper reaches for elevation, anywhere in the shipped tree.
///
/// Including the helper itself in spirit: it does not *elevate*, it is either
/// started with rights or refuses. What it does contain is the word `root`,
/// which is why the check is about reaching rather than about mentioning.
#[test]
fn nothing_shipped_reaches_for_elevation() {
    let root = mcf_checks::workspace::root();
    let mut found = Vec::new();
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            // A line of prose about elevation is not an act of elevation, and
            // this tree is mostly prose (C1). What is looked for is a call.
            let code = line.split("//").next().unwrap_or_default();
            for reach in ELEVATES {
                if code.contains(reach) {
                    found.push(format!("{relative}:{} → {reach}", number.saturating_add(1)));
                }
            }
        }
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "shipped code reaches for elevation (A26, §6.32). Privilege belongs in {HELPER}, \
         which is started with rights or refuses, and never asks for more than it was given"
    );
}

/// Nothing links the helper except the laboratory that reproduces its failures.
///
/// A daemon that *linked* the privileged code would be a daemon holding it,
/// whether or not it ran it. The split is the point: the helper is a program
/// the daemon starts, not a module it calls (§6.32).
#[test]
fn nothing_links_the_helper_but_the_laboratory() {
    let root = mcf_checks::workspace::root();
    let mut linked = Vec::new();
    for member in mcf_checks::workspace::MEMBERS {
        if member.name == "mcf-helper" {
            continue;
        }
        let manifest = std::fs::read_to_string(root.join(member.path).join("Cargo.toml"))
            .expect("a member manifest is readable");
        if manifest.contains("mcf-helper") {
            linked.push(member.name);
        }
    }
    assert_eq!(
        linked,
        // The checks link it to read `OPERATIONS` below, which is the surface
        // this file exists to hold to D35; nothing in the checks ships.
        vec!["mcf-lab", "mcf-checks"],
        "the privileged helper is linked by something other than the laboratory (§6.32). \
         The laboratory links it because A13 requires a scenario for every category MCF's \
         code constructs, and a scenario about a mock is not one (D26)"
    );
}

/// The helper's surface is D35's list, and D35's list is the helper's surface.
///
/// Both directions, because either drift is the same defect: a program that
/// does something the decision does not name, or a decision that names
/// something no program does (A19, §6.32).
#[test]
fn the_helpers_surface_is_the_one_the_decision_names() {
    let intent =
        std::fs::read_to_string(mcf_checks::workspace::root().join("doc/document-of-intent.md"))
            .expect("the intent document is readable");
    let d35 = intent
        .split("### D35")
        .nth(1)
        .expect("D35 is in the intent document")
        .split("\n### ")
        .next()
        .expect("D35 ends somewhere");

    for (operation, what) in mcf_helper::OPERATIONS {
        assert!(
            d35.contains(operation),
            "the helper performs `{operation}` ({what}) and D35 does not name it"
        );
    }
    // And the other way: the three things D35 counts.
    assert!(
        d35.contains("three things"),
        "D35 no longer says how many operations there are"
    );
    assert_eq!(
        mcf_helper::OPERATIONS.len(),
        3,
        "the helper's surface grew without D35 growing with it"
    );
}

/// Every shipped `.rs` file: the crates, not the checks, the prototypes or the
/// tests.
fn shipped_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&root.join("crates"), &mut found);
    found.retain(|path| {
        let text = path.display().to_string();
        !text.contains("/tests/") && !text.ends_with("tests.rs")
    });
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
