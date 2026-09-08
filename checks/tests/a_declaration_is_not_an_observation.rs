#![allow(clippy::panic)]

use std::path::Path;

#[test]
fn there_is_no_way_to_take_the_value_without_its_kind() {
    let source = code_only(&ships(&read(&capability())));
    for convenience in [
        "unwrap_or",
        "unwrap_or_else",
        "unwrap_or_default",
        "pub fn value",
        "pub fn get",
        "pub fn into_inner",
        "pub fn take",
        "pub const fn value",
    ] {
        assert!(
            !source.contains(convenience),
            "`{convenience}` on `Capability` would let a declaration be read as a fact \
             (B-050, §3.18)"
        );
    }
}

#[test]
fn the_two_halves_are_named_for_what_they_are() {
    let source = code_only(&ships(&read(&capability())));
    assert!(
        source.contains("pub const fn declaration(&self) -> Option<&T>"),
        "the declared half is no longer read as a declaration"
    );
    assert!(
        source.contains("pub const fn observation(&self) -> Option<&T>"),
        "the observed half is no longer read as an observation"
    );
    assert!(
        source.contains("pub fn is_established(&self)"),
        "the question *may MCF act on this* is no longer asked in one place"
    );
}

#[test]
fn established_means_verified_and_nothing_else() {
    let source = code_only(&ships(&read(&capability())));
    assert!(
        source.contains("matches!(self.state(), State::Verified)"),
        "`is_established` no longer means *MCF observed it*, which is the whole of §3.18"
    );
}

#[test]
fn nothing_feeds_a_declaration_into_the_observed_half() {
    let root = mcf_checks::workspace::root();
    let mut found = Vec::new();
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        for line in code_only(&ships(&read(&file))).lines() {
            let trimmed = line.trim();
            if trimmed.contains("and_verified(") && trimmed.contains("declaration()") {
                found.push(format!("{relative}: {trimmed}"));
            }
            if trimmed.contains("Capability::verified(") && trimmed.contains("declar") {
                found.push(format!("{relative}: {trimmed}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a declaration is being passed as an observation (§3.18, A21): {found:#?}"
    );
}

fn capability() -> std::path::PathBuf {
    mcf_checks::workspace::root().join("crates/mcf-core/src/capability.rs")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn ships(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(at) => source.get(..at).unwrap_or(source).to_owned(),
        None => source.to_owned(),
    }
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn shipped_sources(root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    collect(&root.join("crates"), &mut found);
    found.retain(|path| {
        let text = path.display().to_string();
        !text.contains("/tests/") && !text.ends_with("tests.rs")
    });
    found.sort();
    found
}

fn collect(directory: &Path, into: &mut Vec<std::path::PathBuf>) {
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
