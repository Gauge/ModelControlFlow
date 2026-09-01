//! Nothing MCF looks for is found by matching its name exactly (B-417, B16).
//!
//! **Nine times.** A guard, a discovery or a tripwire written against what a
//! thing is *called* rather than what it *is* has failed in this repository
//! nine separate times, and the failures are the same shape every time: the
//! code works, the tests pass, and something real is invisible.
//!
//! - [findings.md](../../doc/findings.md) F129 — the surface tripwire named
//!   four guessed crate names; the surface arrived called something else and
//!   the guard never fired.
//! - F130 — engine discovery matched `component == "llama.cpp"`, so the CUDA
//!   build MCF had provisioned an hour earlier was reported as *no engines*.
//! - F133 — the *generation* path had its own copy of that discovery. F130
//!   fixed one and not the other, so every generation and every depth reading
//!   MCF took went through the processor build while the interface said
//!   *`NVIDIA GeForce RTX 5080`*. Between 4.9× and 5.7× of speed, and a device
//!   label that was false.
//! - And provisioning once asserted `-DGGML_NATIVE=OFF` on every component,
//!   which was llama.cpp's flags spelled as a rule; SDL3 failed for not being
//!   llama.cpp.
//!
//! **The remedy is not vigilance.** B16 prefers a machine-checked rule to a
//! remembered one, and eight rounds of remembering is the evidence. What this
//! check holds is narrow and mechanical: no shipped source compares anything
//! to the literal name of a component MCF provisions. Matching on a
//! *property* — a prefix that holds a server, a crate that reaches the control
//! plane — is what discovery is for, and none of those look like this.
//!
//! **Nothing is exempt, and that is the finding.** The first version of this
//! check carried an allowance for `provision.rs`, on the reasoning that the
//! table which *defines* the names must mention them. It does — as struct
//! fields, not as comparisons — and the lookup that turns what an operator
//! typed into one of its rows compares a field to a variable rather than to a
//! literal. So the allowance was holding a door nobody was using, and the
//! staleness test that would have caught that later caught it immediately.
//!
//! **The names are read from the catalogue, not listed here.** A list of
//! names in a check is the same mistake one level up: it goes stale the week a
//! component is added, and the component it then misses is the new one.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// The component names MCF provisions, from the table that defines them.
///
/// The table moved to `mcf-core` when the daemon had to read it too: what MCF
/// can build is asked for by a surface as well as built by a command, and a
/// second copy of a pinned digest is a second thing to forget to change. The
/// names are still read rather than listed, for the reason in the note above.
fn component_names(root: &Path) -> Vec<String> {
    let source = read(&root.join("crates/mcf-core/src/component.rs"));
    let mut found = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("name: \"") else {
            continue;
        };
        if let Some(name) = rest.split('"').next()
            && !name.is_empty()
        {
            found.push(name.to_owned());
        }
    }
    assert!(
        found.len() >= 2,
        "only {} component name(s) were found, so this check is reading provision.rs wrong: \
         {found:?}",
        found.len()
    );
    found
}

/// Every `.rs` file that ships.
fn shipped_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut looking = vec![root.join("crates")];
    while let Some(directory) = looking.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                looking.push(path);
                continue;
            }
            let named = path.display().to_string();
            // A test may compare against a name: it is asserting about one.
            let is_rust = path
                .extension()
                .is_some_and(|held| held.eq_ignore_ascii_case("rs"));
            if is_rust && !named.contains("tests") {
                found.push(path);
            }
        }
    }
    found
}

/// Nothing is discovered by comparing against a component's name.
#[test]
fn no_discovery_matches_a_component_by_its_name() {
    let root = mcf_checks::workspace::root();
    let names = component_names(&root);
    let mut matching = Vec::new();

    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        let source = read(&file);
        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            // A comment saying what the defect *was* is how this repository
            // records one; it is not the defect.
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            for name in &names {
                for shape in [format!("== \"{name}\""), format!("!= \"{name}\"")] {
                    if trimmed.contains(&shape) {
                        matching.push(format!("{relative}:{}: {trimmed}", number + 1));
                    }
                }
            }
        }
    }

    assert!(
        matching.is_empty(),
        "something is looked for by its name rather than by what it is. That has been wrong \
         nine times here — F129, F130, F133 — and the shape is always the same: the code \
         works, the tests pass, and a thing that is really there is invisible. Match on a \
         property instead: a prefix that holds a server, a build that answers when asked what \
         it can compute on (B-417, B16): {matching:#?}"
    );
}
