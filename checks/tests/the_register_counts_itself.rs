//! The backlog's headline counts agree with the backlog (B-041, C6).
//!
//! The register opens with a sentence saying how many items it holds and in
//! what states. That sentence is written by hand and the table below it is
//! edited every working day, so it goes stale silently — and a stale one is
//! worse than none, because it is read as a summary of work that has been
//! done. When this check was first written the header said 204 build items of
//! which 54 were done; the table held 217 of which 84 were.
//!
//! B-041's reasoning applies exactly: *none of those were noticed by reading;
//! all three are mechanical.* So this counts the rows and compares.
//!
//! It is also a light format contract on the status column. Every row's status
//! must **begin** with one of the words the register defines, because the
//! counting is the only reader that cannot skim: a status that opens with
//! prose is a row nobody can total. That caught one, too — a row whose status
//! began *the refusal half is done*, which is `in progress` written as an
//! essay.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::collections::BTreeMap;

/// The states a decision may be in, as the register writes them.
const DECISION_STATES: [&str; 6] = [
    "open",
    "drafted",
    "narrowed",
    "partly settled",
    "decided",
    "resolved",
];

/// The states a build item may be in, as the register writes them.
///
/// `blocked` is written `blocked (by ID)`, so the word is the prefix.
const BUILD_STATES: [&str; 5] = ["open", "in progress", "blocked", "done", "dropped"];

/// Every row's status begins with a state the register defines.
///
/// A status that opens with prose cannot be totalled, and a register whose
/// totals cannot be computed is one whose headline is an assertion (A19).
#[test]
fn every_status_begins_with_a_state() {
    let mut wrong = Vec::new();
    for (id, status) in rows() {
        let states: &[&str] = if id.starts_with("DEC-") {
            &DECISION_STATES
        } else {
            &BUILD_STATES
        };
        let opening = status.to_lowercase().replace("**", "");
        if !states.iter().any(|state| opening.starts_with(state)) {
            wrong.push(format!("{id} → {}", &status[..status.len().min(60)]));
        }
    }
    assert!(
        wrong.is_empty(),
        "a status that does not begin with a state cannot be counted (B-041): {wrong:#?}"
    );
}

/// The headline sentence's counts are the counts.
#[test]
fn the_headline_agrees_with_the_table() {
    let mut decisions: BTreeMap<String, usize> = BTreeMap::new();
    let mut builds: BTreeMap<String, usize> = BTreeMap::new();
    for (id, status) in rows() {
        let opening = status.to_lowercase().replace("**", "");
        let (states, into): (&[&str], &mut BTreeMap<String, usize>) = if id.starts_with("DEC-") {
            (&DECISION_STATES, &mut decisions)
        } else {
            (&BUILD_STATES, &mut builds)
        };
        // Longest match first, so that `decided` does not swallow a status the
        // register spells `decided` inside a longer word.
        let mut named: Vec<&&str> = states.iter().collect();
        named.sort_by_key(|state| core::cmp::Reverse(state.len()));
        if let Some(state) = named.into_iter().find(|state| opening.starts_with(**state)) {
            *into.entry((*state).to_owned()).or_default() += 1;
        }
    }

    let headline = headline();
    let (decided, built) = (total(&decisions), total(&builds));
    for (kind, counted) in [("decisions", &decisions), ("build items", &builds)] {
        for (state, count) in counted {
            let expected = format!("{count} {state}");
            assert!(
                headline.contains(&expected),
                "the register's headline does not say `{expected}` about its {kind}; it says:\n\
                 {headline}"
            );
        }
    }
    for expected in [
        format!("{decided} decisions"),
        format!("{built} build items"),
        format!("{} items", decided.saturating_add(built)),
    ] {
        assert!(
            headline.contains(&expected),
            "the register's headline does not say `{expected}`; it says:\n{headline}"
        );
    }
}

/// How many, over every state.
fn total(counted: &BTreeMap<String, usize>) -> usize {
    counted.values().copied().sum()
}

/// The register's opening sentence, with its line breaks removed so that a
/// count split across two lines still reads as one phrase.
fn headline() -> String {
    let source = backlog();
    // The first bold run that counts items. The front matter is bold too, and
    // a check that read `**Type**` would be checking the table's header.
    let mut rest = source.as_str();
    while let Some(from) = rest.find("**") {
        let after = rest.get(from.saturating_add(2)..).unwrap_or_default();
        let Some(to) = after.find("**") else { break };
        let held = after.get(..to).unwrap_or_default();
        if held.contains("items:") {
            return held.split_whitespace().collect::<Vec<&str>>().join(" ");
        }
        rest = after.get(to.saturating_add(2)..).unwrap_or_default();
    }
    panic!("the register has no headline counting its items");
}

/// Every item row, as an identifier and the status cell that ends it.
///
/// The first row for an identifier wins, because an item may be referred to
/// again in a later table and C5 makes the identifier stable for life.
fn rows() -> Vec<(String, String)> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for line in backlog().lines() {
        let is_decision = line.starts_with("| DEC-");
        if !is_decision && !line.starts_with("| B-") {
            continue;
        }
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
        // A decision row is six cells and a build row is five. Anything else
        // is a table with a different shape, and counting it would be
        // counting something else.
        let wanted = if is_decision { 6 } else { 5 };
        if cells.len() != wanted {
            continue;
        }
        let (Some(id), Some(status)) = (cells.first(), cells.last()) else {
            continue;
        };
        let id = id.trim().to_owned();
        if seen.iter().any(|(held, _)| *held == id) {
            continue;
        }
        seen.push((id, status.trim().to_owned()));
    }
    assert!(
        seen.len() > 100,
        "only {} rows were found, so this check is reading the wrong file",
        seen.len()
    );
    seen
}

/// The register.
fn backlog() -> String {
    let path = mcf_checks::workspace::root().join("doc/backlog.md");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
