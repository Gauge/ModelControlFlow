//! What a person reads never cites a document (A23's limit, B16).
//!
//! **The rules are for the code, and the record is where a citation is
//! useful.** A19 and A2 make MCF say what went wrong; neither makes it say
//! *which rule says so*. An operator who reads `no inference engine is vendored
//! yet (B-320, D32)` has been handed a reference to a document they have never
//! seen, in place of a sentence they could have acted on — and the sentence
//! that carried it was wrong anyway, printed whether or not an engine was there.
//!
//! **The scope is runtime strings, not comments.** Doc comments cite rules
//! constantly and should: that is the code explaining itself to whoever reads
//! it next. This looks only at string literals on lines that are not comments,
//! in the crates whose output a person sees.
//!
//! **The shapes it knows.** A rule is a letter and digits (`A22`, `B65`,
//! `PR9`); a register item is `B-320`; a clause is `§3.13` or `§VI`; a finding
//! is `F128`. All of them are meaningful in the tree and none of them is
//! meaningful on a screen.

use std::path::PathBuf;

/// Crates whose strings a person reads.
///
/// Every shipped crate but the laboratory: what a library crate says in a
/// failure's detail or a category's description reaches a person through the
/// CLI and the window. The laboratory is left out because its text names
/// quantizations — `Q6` — which have the shape of a citation and are not one,
/// and telling the two apart by shape alone is not possible (F268).
const SURFACES: &[&str] = &[
    "crates/mcf-cli",
    "crates/mcf-tui",
    "crates/mcf-serve",
    "crates/mcf-desk",
    "crates/mcf-core",
    "crates/mcf-bench",
    "crates/mcf-hub",
    "crates/mcf-record",
    "crates/mcf-standin",
    "crates/mcf-helper",
];

/// Lines that may cite, because what they carry is not read by a person.
///
/// The record's own field names and the categories are identifiers by design —
/// `platform.mechanism.unavailable` is what a client matches on. A citation
/// *inside* a category would still be caught, because these are whole-line
/// exemptions for lines that construct a record rather than a sentence.
fn is_exempt(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("#[")
        || trimmed.starts_with("reason =")
        // A test asserting on a citation is a test about this rule.
        || trimmed.contains("assert")
}

fn sources(relative: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![mcf_checks::workspace::root().join(relative)];
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // A crate's `tests/` directory is the same fixtures and
                // assertions as a `tests.rs`, one directory down.
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs")
                // A test's strings are fixtures and assertions, not output. The
                // rule is about what reaches a person.
                && path.file_name().is_none_or(|name| name != "tests.rs")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every string literal on the line, roughly: what sits between unescaped
/// double quotes. Good enough to find prose, which is what this is looking for.
fn literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut escaped = false;
    for character in line.chars() {
        match current.as_mut() {
            Some(held) => {
                if escaped {
                    escaped = false;
                    held.push(character);
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    found.push(current.take().unwrap_or_default());
                } else {
                    held.push(character);
                }
            }
            None if character == '"' => current = Some(String::new()),
            None => {}
        }
    }
    found
}

/// A citation, by shape rather than by a list of the ones that exist today.
fn citation_in(text: &str) -> Option<String> {
    if let Some(at) = text.find('§') {
        return Some(text.get(at..).unwrap_or("§").chars().take(6).collect());
    }
    let words = text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'));
    for word in words {
        // `B-320`: a letter, a hyphen, digits.
        if let Some((head, tail)) = word.split_once('-')
            && !head.is_empty()
            && head.len() <= 2
            && head.chars().all(|c| c.is_ascii_uppercase())
            && !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_digit())
        {
            return Some(word.to_owned());
        }
        // `A22`, `PR9`, `F128`: one or two capitals then digits, and short.
        let letters = word.chars().take_while(char::is_ascii_uppercase).count();
        let digits = word.chars().skip(letters).collect::<String>();
        if (1..=2).contains(&letters)
            && !digits.is_empty()
            && digits.len() <= 3
            && digits.chars().all(|c| c.is_ascii_digit())
            && word.len() == letters + digits.len()
        {
            return Some(word.to_owned());
        }
    }
    None
}

/// No new sentence a person reads carries a rule, a register item or a clause.
#[test]
fn nothing_a_person_reads_cites_a_document() {
    let mut found = Vec::new();
    for surface in SURFACES {
        for path in sources(surface) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (number, line) in text.lines().enumerate() {
                if is_exempt(line) {
                    continue;
                }
                for literal in literals(line) {
                    // Prose, not an identifier: a citation only misleads inside
                    // a sentence, and a short literal is a key or a path.
                    if literal.split_whitespace().count() < 4 {
                        continue;
                    }
                    if let Some(cited) = citation_in(&literal) {
                        let relative = path
                            .strip_prefix(mcf_checks::workspace::root())
                            .unwrap_or(&path)
                            .display();
                        found.push(format!("{relative}:{} → {cited}", number + 1));
                    }
                }
            }
        }
    }
    // **This was a ratchet.** Sixty-four surfaces carried a citation into
    // something a person reads when this check was written, spread across
    // fifteen files, and the count was written down so that adding one
    // failed the build and removing one failed it too, with the instruction
    // to lower the constant. It reached zero with B-403 (F264), and stays
    // there: a citation added now fails with the sentence that carries it.
    assert!(
        found.is_empty(),
        "{} sentence(s) a person reads cite a document. The rule belongs in the code and in \
         the record; the screen gets what to do about it:\n{found:#?}",
        found.len()
    );
}

/// The shapes are recognised, so the check above is not passing by blindness.
#[test]
fn the_check_knows_a_citation_when_it_sees_one() {
    for cited in [
        "no inference engine is vendored yet (B-320, D32)",
        "the interface may not be the only way to do anything (A22)",
        "idle costs nothing because of §3.13 and nothing else",
        "this was found by F128 while building",
    ] {
        assert!(
            citation_in(cited).is_some(),
            "missed a citation in {cited:?}"
        );
    }
    for clean in [
        "no engine is installed yet — MCF can build one for you",
        "this model needs about 20.00 GB and the largest device here has 2.00 GB free",
        "nothing is listening; mcf serve starts a daemon",
        "recovered 5232 record entries and 34 model files",
    ] {
        assert_eq!(citation_in(clean), None, "false alarm on {clean:?}");
    }
}
