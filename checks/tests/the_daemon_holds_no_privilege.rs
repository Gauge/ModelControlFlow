#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

const HELPER: &str = "crates/mcf-helper";

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
        vec!["mcf-lab", "mcf-checks"],
        "the privileged helper is linked by something other than the laboratory (§6.32). \
         The laboratory links it because A13 requires a scenario for every category MCF's \
         code constructs, and a scenario about a mock is not one (D26)"
    );
}

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
