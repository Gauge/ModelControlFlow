//! No code path takes a credential MCF was not given.
//!
//! B-024: *credentials are the user's, held deliberately, never a silent
//! prerequisite.* The failure it names is not a bug anybody writes on purpose —
//! it is a convenience. Somebody adds `std::env::var("HF_TOKEN").ok()` to a
//! client because the tests were annoying, and from then on MCF acquires
//! artifacts under a condition nobody recorded: §3.4 makes the conditions part
//! of the measurement, and an artifact fetched with a token that happened to be
//! in the environment was fetched under different conditions from one fetched
//! without it.
//!
//! A comment cannot hold that line and a review will not, because the change
//! that breaks it is one line long and looks helpful. So it is structural: the
//! names of the credential variables appear in exactly one file, and that file
//! reads no environment at all — it is *handed* a way to look
//! (`mcf_hub::credentials::sightings`), which puts the deciding at the call
//! site where an operator's instruction can reach it.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// The one file allowed to name the variables, relative to the workspace root.
const THE_ONE_PLACE: &str = "crates/mcf-hub/src/credentials.rs";

/// What a token variable is called, wherever anybody would reach for one.
///
/// Written in pieces so this file does not itself contain the strings it
/// forbids — a check that tripped over its own source would have to be
/// exempted, and an exemption is how this kind of check dies.
fn forbidden_names() -> Vec<String> {
    ["HF", "HUGGING_FACE_HUB", "HUGGINGFACE"]
        .iter()
        .map(|prefix| format!("{prefix}_TOKEN"))
        .collect()
}

/// The credential variables are named in one file and nowhere else.
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

/// And it names all of them, which is the claim nothing else is allowed to
/// make.
///
/// Without this the exclusivity above would be satisfied by a workspace that
/// had forgotten a variable entirely — every file passing because no file knows
/// the name an operator's token is actually under.
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

/// And that file reads no environment itself.
///
/// This is the half that makes the first half mean something: a module that
/// held the names *and* read the environment would satisfy an exclusivity check
/// while doing exactly what B-024 forbids.
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

/// A file with its documentation removed.
///
/// The prose names what it forbids in order to say it is absent, and a check
/// that grepped the whole file would fail on the paragraph explaining why it
/// passes.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under the workspace's own crates, checks included.
///
/// Deliberately not the whole root: `target/` holds vendored build output that
/// is nobody's decision here, and walking it would make this check slow enough
/// that somebody moves it out of the gate.
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
