//! MCF ships the licence it declares.
//!
//! D28: GPL-3.0-only. B-330's condition is that *the licence is stated in the
//! artifact and surfaced to a redistributor*, and the failure mode is
//! mundane — a `LICENSE` that drifts from the manifest, or a manifest that
//! declares terms the repository does not contain. Either leaves a
//! redistributor with an obligation nobody can read.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

/// The repository contains the licence, verbatim.
///
/// Checked by its own words rather than by a digest: a digest would fail on a
/// trailing newline and say nothing about *which* licence is there, and the
/// question is which one.
#[test]
fn the_licence_text_is_the_gpl_version_three() {
    let text = licence();
    for phrase in [
        "GNU GENERAL PUBLIC LICENSE",
        "Version 3, 29 June 2007",
        "Copyright (C) 2007 Free Software Foundation, Inc.",
        "TERMS AND CONDITIONS",
        "Disclaimer of Warranty",
    ] {
        assert!(text.contains(phrase), "LICENSE does not contain {phrase:?}");
    }
    assert!(
        !text.contains("GNU AFFERO"),
        "LICENSE is the Affero variant, and D28 chose GPL-3.0-only"
    );
    assert!(
        text.lines().count() > 600,
        "LICENSE is {} lines, which is not the whole text",
        text.lines().count()
    );
}

/// The manifest declares what the repository contains.
#[test]
fn the_manifest_declares_the_licence_the_repository_holds() {
    let manifest =
        mcf_checks::workspace::read("Cargo.toml").expect("the workspace manifest is readable");
    assert_eq!(
        manifest.get("workspace.package", "license"),
        Some("\"GPL-3.0-only\""),
        "the manifest declares something other than what LICENSE holds (D28)"
    );
}

/// No vendored component ships without a recorded compatibility finding.
///
/// D28 makes every vendored component's terms MCF's problem: one that is not
/// GPL-3.0-compatible is one MCF cannot ship whatever its merits, and D23's
/// third tier is where it goes. There is nothing vendored yet, so this check
/// has nothing to find — and it says so rather than passing silently, because a
/// check that would pass an empty world is a check that stops being read.
#[test]
fn every_vendored_component_has_a_compatibility_finding() {
    let root = mcf_checks::workspace::root();
    let vendored = root.join("vendor");
    if !vendored.exists() {
        println!(
            "nothing is vendored yet: no component has been admitted, so there is no \
             compatibility finding to require (D23, D28, B-330)"
        );
        return;
    }
    let findings = std::fs::read_to_string(root.join("doc/vendored.md"))
        .expect("something is vendored and doc/vendored.md does not exist (B-330)");
    let Ok(entries) = std::fs::read_dir(&vendored) else {
        panic!("{} cannot be read", vendored.display());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            findings.contains(&name),
            "`{name}` is vendored and has no recorded compatibility finding (B-330)"
        );
    }
}

fn licence() -> String {
    let path = mcf_checks::workspace::root().join("LICENSE");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The artifact states the licence, and the list it states matches the register.
///
/// B-330's second half: *the licence is stated in the artifact and surfaced to a
/// redistributor*. A redistributor has a binary, not a repository, so the
/// question is what `mcf licence` says — and what it says about vendored
/// components has to be what `doc/vendored.md` records, or a component's terms
/// reach the person with the obligation in only one of the two places.
#[test]
fn the_artifact_states_the_licence_and_agrees_with_the_register() {
    let source = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-cli/src/licence.rs"),
    )
    .expect("the licence surface is in the tree");

    assert!(
        source.contains("GPL-3.0-only"),
        "the artifact's licence surface does not name the licence D28 chose"
    );
    assert!(
        source.contains("include_str!(\"../../../LICENSE\")"),
        "the artifact refers to the licence rather than carrying it, so a binary \
         conveyed on its own carries no copy (GPL-3.0 §4)"
    );

    // The two lists, compared name by name and revision by revision. It used
    // to be enough to check that both were empty or both were not, which was
    // written when both were empty; fourteen components later, *agreeing about
    // whether anything is vendored* is not the property B-330 wants. A
    // redistributor's obligations are what `mcf licence` prints, and the
    // compatibility finding for each is in the register — if the two lists
    // differ, one of them is about a binary nobody is shipping.
    let register = std::fs::read_to_string(mcf_checks::workspace::root().join("doc/vendored.md"))
        .expect("doc/vendored.md is readable");

    let in_artifact = compiled_in(&source);
    let in_register = admitted(&register);
    assert!(
        !in_artifact.is_empty(),
        "the artifact names no vendored component, and the tree has a vendor directory"
    );
    assert_eq!(
        in_artifact, in_register,
        "the artifact and doc/vendored.md do not name the same vendored components \
         (B-330). What `mcf licence` prints is what a redistributor is obliged to \
         convey, and the register is where each one's compatibility finding lives"
    );
}

/// Every `name-revision` the artifact says it ships.
fn compiled_in(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let (_, table) = source
        .split_once("pub(crate) const VENDORED: &[Component] = &[")
        .expect("the artifact declares a vendored table");
    let (table, _) = table.split_once("\n];").expect("the table ends");
    let mut name = None;
    for line in table.lines() {
        let line = line.trim();
        if let Some(value) = field(line, "name:") {
            name = Some(value);
        } else if let Some(revision) = field(line, "revision:") {
            let named = name
                .take()
                .expect("a component is named before its revision");
            found.push(format!("{named}-{revision}"));
        }
    }
    found.sort();
    found
}

/// Every `name-revision` the register says is compiled in.
///
/// The first table only: the register's second one is the crates Cargo requires
/// to be present and nothing compiles (F9.4), which a redistributor does not
/// convey and `mcf licence` therefore does not print.
fn admitted(register: &str) -> Vec<String> {
    let (_, rest) = register
        .split_once("**What is compiled into the artifact.**")
        .expect("the register says what is compiled in");
    let (table, _) = rest.split_once("\n\n**").unwrap_or((rest, ""));
    let mut found: Vec<String> = table
        .lines()
        .filter_map(|line| {
            let cell = line.trim().strip_prefix("| `")?;
            let (name, _) = cell.split_once('`')?;
            Some(name.to_owned())
        })
        .collect();
    found.sort();
    found
}

/// The value of a `field: "value",` line, if this is one.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.strip_prefix(name)?
        .trim()
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
}
