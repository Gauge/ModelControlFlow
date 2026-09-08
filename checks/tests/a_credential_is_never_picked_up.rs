#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const THE_ONE_PLACE: &str = "crates/mcf-hub/src/credentials.rs";

fn forbidden_names() -> Vec<String> {
    ["HF", "HUGGING_FACE_HUB", "HUGGINGFACE"]
        .iter()
        .map(|prefix| format!("{prefix}_TOKEN"))
        .collect()
}

#[test]
fn nothing_but_the_credential_module_names_a_token_variable() {
    let root = mcf_checks::workspace::root();
    let the_one_place = root.join(THE_ONE_PLACE);
    let names = forbidden_names();
    let mut looked_at = 0_usize;

    for file in rust_sources(&root) {
        if file == the_one_place {
            continue;
        }
        let source = code_only(
            &std::fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("{} is readable: {error}", file.display())),
        );
        looked_at += 1;
        for name in &names {
            assert!(
                !source.contains(name.as_str()),
                "{} names {name}: a credential MCF was not handed is a condition nobody \
                 recorded (B-024, §3.4). If an operator asked for it, pass a lookup to \
                 `mcf_hub::credentials::sightings` from where the asking happened.",
                file.display()
            );
        }
    }

    assert!(
        looked_at > 20,
        "only {looked_at} sources were read, so this check is looking at the wrong tree"
    );
}

#[test]
fn the_credential_module_names_every_variable_an_operator_might_use() {
    let path = mcf_checks::workspace::root().join(THE_ONE_PLACE);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    for name in forbidden_names() {
        assert!(
            source.contains(name.as_str()),
            "{THE_ONE_PLACE} does not know about {name}, so an operator with it set is told              there is no credential anywhere (B-024, A7)"
        );
    }
}

#[test]
fn the_credential_module_reads_no_environment() {
    let path = mcf_checks::workspace::root().join(THE_ONE_PLACE);
    let source = code_only(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display())),
    );
    for reach in ["env::var", "env::vars", "std::env", "var_os"] {
        assert!(
            !source.contains(reach),
            "`{reach}` appears in {THE_ONE_PLACE}: the module that knows the names must not \
             also be the one that looks (B-024)"
        );
    }
    assert!(
        source.contains("look_up: &dyn Fn"),
        "`sightings` no longer takes the way to look as an argument, so nothing keeps the \
         deciding at the call site (B-024)"
    );
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for tree in ["crates", "checks", "prototypes"] {
        collect(&root.join(tree), &mut found);
    }
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
