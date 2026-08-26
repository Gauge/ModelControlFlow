//! Every mutation the catalogue declares can still be placed (B-186, B-191).
//!
//! **The failure this exists to prevent, which happened.** The mutation
//! catalogue names a line of source for each mutant and refuses to apply one it
//! cannot place unambiguously — correctly, since a mutation applied somewhere
//! other than where it was meant is a mutant nobody declared. But that refusal
//! arrives when the *scheduled* tier runs, which on a shared machine can be
//! half a day after the refactor that moved the line, and it stops the whole
//! tier: `cannot check` rather than a score.
//!
//! B-300's rewrite of the journal replay moved one such line, and the tier
//! reported it thirteen hours later. So the placement is checked here, in the
//! tier that gates every change, where it costs eighteen file reads and tells
//! whoever moved the line while they still remember why.
//!
//! **What this does not check** is that the mutants are still *killed* — that
//! is the mutation tier's whole job and it costs a suite run each. This checks
//! only that the catalogue still describes the code it is about, which is the
//! part that goes stale silently.
//!
//! **A subtlety this check learned the hard way.** The mutation tier runs the
//! whole suite — including this file — against a *mutated copy* of the tree, so
//! for one entry per run the original line is not there: the mutant is. As
//! first written, this check failed inside that copy, which the runner read as
//! *the suite kills the equivalent-mutant control*, and the tier refused to
//! produce a score at all. So what is asserted is that each entry describes the
//! file **either** as written **or** as mutated: exactly one of the two, which
//! is true in a clean tree and true in a mutated one, and false when somebody
//! moves the line.

// Every item in this file is test code; see the note in checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]

/// The script that holds the catalogue.
const CATALOGUE: &str = "scripts/check-mutants.sh";

/// Every mutation, as the script declares it.
#[test]
fn every_declared_mutation_can_still_be_placed() {
    let root = mcf_checks::workspace::root();
    let script = std::fs::read_to_string(root.join(CATALOGUE))
        .unwrap_or_else(|error| panic!("{CATALOGUE} is readable: {error}"));

    let files = array(&script, "files");
    let finds = array(&script, "finds");
    let replaces = array(&script, "replaces");
    assert!(!files.is_empty(), "the catalogue is empty");
    assert_eq!(
        files.len(),
        finds.len(),
        "the catalogue names {} files and {} things to find",
        files.len(),
        finds.len()
    );
    assert_eq!(
        finds.len(),
        replaces.len(),
        "the catalogue has {} things to find and {} replacements",
        finds.len(),
        replaces.len()
    );

    // The control mutation lands in a file the catalogue also names, so while
    // the runner is testing *it* that entry's line is neither as written nor as
    // its own mutant. Accepting it here is not a hole: the control has its own
    // check below, and what this one is about is whether the catalogue still
    // describes the code.
    let control = scalar(&script, "CONTROL_REPLACE");

    let mut lost = Vec::new();
    for ((file, find), replace) in files.iter().zip(&finds).zip(&replaces) {
        let source = match std::fs::read_to_string(root.join(file)) {
            Ok(source) => source,
            Err(error) => {
                lost.push(format!("{file}: cannot be read ({error})"));
                continue;
            }
        };
        if source.contains(control.as_str()) {
            continue;
        }
        if let Err(why) = placeable(&source, find, replace) {
            lost.push(format!("{file}: {why}"));
        }
    }
    assert_eq!(
        lost,
        Vec::<String>::new(),
        "the mutation catalogue no longer describes the code it is about. A mutation that \
         cannot be placed unambiguously stops the whole scheduled tier with `cannot check` \
         rather than producing a score (B-186), and the person who can fix it is whoever \
         moved the line"
    );
}

/// The control mutation, which must also still be placeable.
///
/// It is declared separately in the script because it is not part of the score:
/// a change with no semantic effect that the suite must *not* kill. If it
/// cannot be placed the tier has no way to tell a killed mutant from a broken
/// copy, and every other result it reports means nothing.
#[test]
fn the_equivalent_mutant_control_can_still_be_placed() {
    let root = mcf_checks::workspace::root();
    let script = std::fs::read_to_string(root.join(CATALOGUE)).expect("the catalogue is readable");
    let file = scalar(&script, "CONTROL_FILE");
    let find = scalar(&script, "CONTROL_FIND");
    let replace = scalar(&script, "CONTROL_REPLACE");
    let source = std::fs::read_to_string(root.join(&file))
        .unwrap_or_else(|error| panic!("{file} is readable: {error}"));
    if let Err(why) = placeable(&source, &find, &replace) {
        panic!("the control mutation cannot be placed in {file}: {why}");
    }
}

/// Whether a mutation still describes the file it names.
///
/// True when the source holds the original line exactly once — a clean tree —
/// **or** the mutated one exactly once, which is what the mutation runner's own
/// copy looks like while it is being tested. Anything else means the catalogue
/// and the code have parted company.
fn placeable(source: &str, find: &str, replace: &str) -> Result<(), String> {
    let as_written = source.matches(find).count();
    let as_mutated = source.matches(replace).count();
    match (as_written, as_mutated) {
        (1, _) | (0, 1) => Ok(()),
        (0, 0) => Err(format!(
            "no line matches `{find}`, and none is mutated to `{replace}`"
        )),
        (many, _) => Err(format!("{many} lines match `{find}`")),
    }
}

/// The strings of one `declare -a name=( … )` array, in order.
///
/// A small reader rather than a shell: what is needed is the quoted strings,
/// and running the script to find out what is in it would be running a script
/// to check the script.
fn array(script: &str, name: &str) -> Vec<String> {
    let opening = format!("declare -a {name}=(");
    let start = script
        .find(&opening)
        .unwrap_or_else(|| panic!("{CATALOGUE} declares no array called {name}"))
        + opening.len();
    let rest = &script[start..];
    let end = rest
        .find("\n)")
        .unwrap_or_else(|| panic!("the {name} array does not end"));
    rest[..end].lines().filter_map(quoted).collect()
}

/// A `readonly NAME="…"` string.
fn scalar(script: &str, name: &str) -> String {
    let opening = format!("readonly {name}=");
    let start = script
        .find(&opening)
        .unwrap_or_else(|| panic!("{CATALOGUE} declares no {name}"))
        + opening.len();
    quoted(script[start..].lines().next().unwrap_or_default())
        .unwrap_or_else(|| panic!("{name} is not a quoted string"))
}

/// One line's quoted string, unescaped, or `None` for a comment or a blank.
///
/// The escapes are the shell's inside double quotes: a backslash before a
/// quote, a backslash, a dollar or a backtick stands for the character itself,
/// and everything else — `\n` in a Rust string literal, for instance — is two
/// characters that must survive as two.
fn quoted(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let inner = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some(next @ ('"' | '\\' | '$' | '`')) => out.push(next),
            Some(next) => {
                out.push('\\');
                out.push(next);
            }
            None => out.push('\\'),
        }
    }
    Some(out)
}
