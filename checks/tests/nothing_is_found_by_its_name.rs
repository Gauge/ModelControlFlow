#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

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
