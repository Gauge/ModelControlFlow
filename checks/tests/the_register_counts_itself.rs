#![allow(clippy::panic)]

use std::collections::BTreeMap;

const DECISION_STATES: [&str; 6] = [
    "open",
    "drafted",
    "narrowed",
    "partly settled",
    "decided",
    "resolved",
];

const BUILD_STATES: [&str; 5] = ["open", "in progress", "blocked", "done", "dropped"];

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

fn total(counted: &BTreeMap<String, usize>) -> usize {
    counted.values().copied().sum()
}

fn headline() -> String {
    let source = backlog();
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

fn split_cells(row: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut escaped = false;
    for letter in row.chars() {
        match letter {
            '\\' if !escaped => escaped = true,
            '|' if !escaped => cells.push(String::new()),
            other => {
                if escaped
                    && other != '|'
                    && let Some(last) = cells.last_mut()
                {
                    last.push('\\');
                }
                escaped = false;
                if let Some(last) = cells.last_mut() {
                    last.push(other);
                }
            }
        }
    }
    cells
}

fn rows() -> Vec<(String, String)> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for line in backlog().lines() {
        let is_decision = line.starts_with("| DEC-");
        if !is_decision && !line.starts_with("| B-") {
            continue;
        }
        // `\|` is a literal pipe inside a cell and not a separator — which
        // the register uses to write things like a closure's `||`. Splitting
        let cells: Vec<String> = split_cells(line.trim().trim_matches('|'));
        let wanted = if is_decision { 6 } else { 5 };
        assert_eq!(
            cells.len(),
            wanted,
            "the row for {} has {} cell(s) and needs {wanted}: a literal `|` inside a cell \
             splits it, and must be written `\\|`",
            cells.first().map_or("?", |held| held.trim()),
            cells.len()
        );
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

fn backlog() -> String {
    let path = mcf_checks::workspace::root().join("doc/backlog.md");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
