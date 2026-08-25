//! MCF ships the licence it declares.
//!
//! D28: GPL-3.0-only. B-330's condition is that *the licence is stated in the
//! artifact and surfaced to a redistributor*, and the failure mode is
//! mundane — a `LICENSE` that drifts from the manifest, or a manifest that
//! declares terms the repository does not contain. Either leaves a
//! redistributor with an obligation nobody can read.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

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
