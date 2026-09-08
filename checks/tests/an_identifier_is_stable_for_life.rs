#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeSet;

const LEDGER: &str = "checks/identifiers.tsv";

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn current() -> BTreeSet<String> {
    let mut found = BTreeSet::new();

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

    for line in read("doc/backlog.md").lines() {
        for prefix in ["B-", "DEC-"] {
            if let Some(id) = row_identifier(line, prefix) {
                found.insert(format!("register\t{id}"));
            }
        }
    }

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

fn written_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("Self::")?;
    let (_, rest) = rest.split_once(" => \"")?;
    let (name, tail) = rest.split_once('"')?;
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

fn row_identifier(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix("| ")?;
    let (first, _) = rest.split_once(" |")?;
    let digits = first.trim().strip_prefix(prefix)?;
    if digits.is_empty() || !digits.chars().all(|held| held.is_ascii_digit()) {
        return None;
    }
    Some(format!("{prefix}{digits}"))
}

fn finding_identifier(line: &str) -> Option<String> {
    let (_, after) = line.strip_prefix("## ")?.split_once(" · ")?;
    let (token, _) = after.split_once(' ')?;
    let digits = token.strip_prefix('F')?;
    if digits.is_empty() || !digits.chars().all(|held| held.is_ascii_digit()) {
        return None;
    }
    Some(token.to_owned())
}

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
