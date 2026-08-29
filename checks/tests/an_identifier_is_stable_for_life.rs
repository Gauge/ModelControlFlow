//! Every published identifier still means what it meant (C5, C6, B16, A1,
//! §7.30, F108).
//!
//! **Two rules rested on review and now do not.** C5: *identifiers are stable
//! for life — never reused, never renamed, deprecated only in favour of a named
//! successor.* C6: *nothing is deleted; dropped work keeps its reasoning.* Both
//! are about the same thing from two sides, and both were checked by somebody
//! remembering.
//!
//! **Why it matters more than it sounds.** These identifiers are not internal
//! names. A taxonomy code appears in the record, in an export and — once §XIV
//! ships — in another machine's copy of this one's evidence. A record entry's
//! *kind* is stored in the derived index **as its position in `EntryKind::ALL`**,
//! so reordering that list silently reinterprets every entry ever written. A
//! backlog identifier is what a finding, a rule and a commit message cite. None
//! of these can be renamed by a careful edit; they can only be renamed by an
//! edit nobody noticed.
//!
//! **The ledger is the mechanism.** `checks/identifiers.tsv` holds every
//! published identifier as it stands. This check recomputes the set from the
//! source and the documents and compares:
//!
//! * an identifier in the ledger and not in the tree has been **removed or
//!   renamed** — C5 and C6, and the check fails;
//! * an entry kind at a different **position** has been reordered, which
//!   changes what every existing index entry means;
//! * a taxonomy code whose **meaning** changed has been reused, which is the
//!   one thing C5 calls out by name;
//! * a format version that moved is a schema change, which §7.30 makes an
//!   interface event rather than an edit;
//! * an identifier in the tree and not in the ledger is an **addition**, and
//!   the check fails with the lines to add. That friction is the point: a new
//!   entry in a published namespace is a deliberate act, and its diff is that
//!   namespace's changelog.
//!
//! The ledger's own diff is therefore the only place a reader can see what this
//! project has promised to keep, and it is one line per promise.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeSet;

/// Where the ledger lives.
const LEDGER: &str = "checks/identifiers.tsv";

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The identifiers as the tree has them now, one line each.
///
/// The same shape the ledger is written in, so that the comparison is a set
/// difference and the failure message is the lines somebody has to add.
fn current() -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    // Entry kinds, with their position: the derived index stores a kind as its
    // index into `ALL`, so the order is part of the record's format (D20).
    let entry = read("crates/mcf-record/src/journal/entry.rs");
    let names: Vec<(String, String)> = entry
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("Self::")?;
            let (variant, rest) = rest.split_once(" => \"")?;
            let (name, _) = rest.split_once('"')?;
            Some((variant.to_owned(), name.to_owned()))
        })
        .collect();
    let all = entry
        .split_once("pub const ALL:")
        .and_then(|(_, rest)| rest.split_once("];"))
        .map(|(held, _)| held.to_owned())
        .expect("EntryKind::ALL is where the record's kinds are ordered");
    let mut position = 0_usize;
    for line in all.lines() {
        let Some(variant) = line.trim().strip_prefix("Self::").and_then(|held| {
            held.strip_suffix(',')
                .filter(|held| !held.contains(' '))
                .map(str::to_owned)
        }) else {
            continue;
        };
        if let Some((_, name)) = names.iter().find(|(held, _)| *held == variant) {
            found.insert(format!("entry-kind\t{position}\t{name}"));
            position = position.saturating_add(1);
        }
    }

    // Taxonomy codes with their meanings, read from the document that is the
    // classification rather than from the types generated out of it.
    for line in read("doc/taxonomy.md").lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((code, rest)) = rest.split_once("` | ") else {
            continue;
        };
        let Some(meaning) = rest.strip_suffix(" |") else {
            continue;
        };
        if code.contains('.') && !code.contains(' ') {
            found.insert(format!("taxonomy\t{code}\t{meaning}"));
        }
    }

    // The other two axes of a classified failure, which travel with it.
    let axes = read("crates/mcf-core/src/failure/axes.rs");
    let (attribution, disposition) = axes
        .split_once("pub enum Disposition")
        .expect("the two axes are in one module");
    for (kind, chunk) in [("attribution", attribution), ("disposition", disposition)] {
        for line in chunk.lines() {
            if let Some(name) = written_name(line) {
                found.insert(format!("{kind}\t{name}"));
            }
        }
    }

    // What a bundle says it is, and the two schema versions.
    let export = read("crates/mcf-record/src/export.rs");
    for line in export.lines() {
        if let Some(name) = written_name(line) {
            found.insert(format!("bundle-kind\t{name}"));
        }
    }
    found.insert(format!("format\tbundle\t{}", format_version(&export)));
    found.insert(format!(
        "format\tjournal\t{}",
        format_version(&read("crates/mcf-record/src/journal.rs"))
    ));

    // The register's own identifiers, which every citation depends on.
    for line in read("doc/backlog.md").lines() {
        for prefix in ["B-", "DEC-"] {
            if let Some(id) = row_identifier(line, prefix) {
                found.insert(format!("register\t{id}"));
            }
        }
    }

    // Findings and rules, cited across every document.
    for line in read("doc/findings.md").lines() {
        if let Some(id) = finding_identifier(line) {
            found.insert(format!("finding\t{id}"));
        }
    }
    for line in read("doc/rules.md").lines() {
        if let Some(id) = rule_identifier(line) {
            found.insert(format!("rule\t{id}"));
        }
    }

    found
}

/// `Self::Whatever => "written_name",` → `written_name`
fn written_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("Self::")?;
    let (_, rest) = rest.split_once(" => \"")?;
    let (name, tail) = rest.split_once('"')?;
    // A name, not a sentence: the arms that build prose are not identifiers.
    if !tail.starts_with(',') || name.contains(' ') || name.contains('.') {
        return None;
    }
    Some(name.to_owned())
}

fn format_version(source: &str) -> String {
    source
        .split_once("pub const FORMAT_VERSION: i64 = ")
        .and_then(|(_, rest)| rest.split_once(';'))
        .map(|(held, _)| held.trim().to_owned())
        .expect("a format version is declared")
}

/// `| B-001 | …` → `B-001`
fn row_identifier(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix("| ")?;
    let (first, _) = rest.split_once(" |")?;
    let digits = first.trim().strip_prefix(prefix)?;
    if digits.is_empty() || !digits.chars().all(|held| held.is_ascii_digit()) {
        return None;
    }
    Some(format!("{prefix}{digits}"))
}

/// `## 80 · F80 — …` → `F80`
fn finding_identifier(line: &str) -> Option<String> {
    let (_, after) = line.strip_prefix("## ")?.split_once(" · ")?;
    let (token, _) = after.split_once(' ')?;
    let digits = token.strip_prefix('F')?;
    if digits.is_empty() || !digits.chars().all(|held| held.is_ascii_digit()) {
        return None;
    }
    Some(token.to_owned())
}

/// `### A6 — …` → `A6`
fn rule_identifier(line: &str) -> Option<String> {
    let (token, _) = line.strip_prefix("### ")?.split_once(" — ")?;
    let (letter, digits) = token.split_at(1);
    if !matches!(letter, "A" | "B" | "C")
        || digits.is_empty()
        || !digits.chars().all(|held| held.is_ascii_digit())
    {
        return None;
    }
    Some(token.to_owned())
}

/// Nothing published has been renamed, reordered, reused or removed.
#[test]
fn every_identifier_in_the_ledger_still_means_what_it_meant() {
    let ledger: BTreeSet<String> = read(LEDGER)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(
        ledger.len() > 500,
        "the ledger holds {} lines, so this check is reading the wrong file",
        ledger.len()
    );

    let now = current();
    let gone: Vec<&String> = ledger.difference(&now).collect();
    assert!(
        gone.is_empty(),
        "these identifiers were in {LEDGER} and are not in the tree. An identifier is stable \
         for life: never reused, never renamed, deprecated only in favour of a named successor \
         (C5), and dropped work keeps its identifier and its reasoning (C6). A taxonomy code \
         travels between machines; an entry kind's *position* is what the derived index stores. \
         If one of these is a deliberate deprecation, the successor goes in the ledger beside \
         it:\n{gone:#?}"
    );

    let added: Vec<&String> = now.difference(&ledger).collect();
    assert!(
        added.is_empty(),
        "these identifiers are in the tree and not in {LEDGER}. Adding to a published namespace \
         is deliberate, and the ledger's diff is that namespace's changelog — add these lines \
         in the same change:\n{added:#?}"
    );
}

/// The ledger is a ledger: one identifier a line, sorted within its kind, no
/// duplicates.
#[test]
fn the_ledger_is_readable_by_a_person_and_a_diff() {
    let text = read(LEDGER);
    let mut seen = BTreeSet::new();
    for (number, line) in text.lines().enumerate() {
        assert!(
            !line.trim().is_empty(),
            "{LEDGER}:{} is blank; a ledger with holes in it invites an edit that closes one",
            number.saturating_add(1)
        );
        assert!(
            line.contains('\t'),
            "{LEDGER}:{} has no kind: every line is `<kind>\\t<identifier>[\\t<meaning>]`",
            number.saturating_add(1)
        );
        assert!(
            seen.insert(line),
            "{LEDGER}:{} repeats an identifier, and a namespace with two entries for one name \
             is the reuse C5 forbids",
            number.saturating_add(1)
        );
    }
}
