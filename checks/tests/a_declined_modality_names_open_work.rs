#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

fn read(relative: &str) -> String {
    let at = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&at).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", at.display());
    })
}

fn register() -> String {
    read("doc/backlog.md")
}

struct Declined {
    modality: String,
    until: String,
}

fn declined() -> Vec<Declined> {
    let source = read("crates/mcf-serve/src/probes/declined.rs");
    let mut found = Vec::new();
    let mut modality: Option<String> = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("modality: \"")
            && let Some(name) = rest.split('"').next()
        {
            modality = Some(name.to_owned());
        }
        if let Some(rest) = trimmed.strip_prefix("until: \"")
            && let Some(item) = rest.split('"').next()
            && let Some(named) = modality.take()
        {
            found.push(Declined {
                modality: named,
                until: item.to_owned(),
            });
        }
    }
    assert!(
        !found.is_empty(),
        "no declined modality was read, so this check is reading the table wrong"
    );
    found
}

#[test]
fn every_declined_modality_names_an_item_the_register_holds() {
    let register = register();
    for held in declined() {
        assert!(
            !held.until.is_empty(),
            "{} declines and names no item, so nothing would ever change it (C5)",
            held.modality
        );
        assert!(
            register
                .lines()
                .any(|line| line.starts_with(&format!("| {} |", held.until))),
            "{} waits on {} and the register has no such item (C5)",
            held.modality,
            held.until
        );
    }
}

#[test]
fn no_declined_modality_outlives_the_work_that_would_answer_it() {
    let register = register();
    for held in declined() {
        let row = register
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", held.until)))
            .unwrap_or_else(|| {
                panic!(
                    "{} waits on {}, which is not in the register",
                    held.modality, held.until
                )
            });
        let status = row
            .rsplit_once(" | ")
            .map(|(_, held)| held.to_lowercase())
            .unwrap_or_default();
        assert!(
            !status.starts_with("done") && !status.starts_with("**done"),
            "{} is done, so {:?} is not waiting on it any more — probe the modality, or point \
             the entry at the open work that is really in the way. A refusal aimed at finished \
             work reads as *not yet* and means *never* (B-380, A7)",
            held.until,
            held.modality
        );
    }
}

#[test]
fn every_declined_modality_says_what_it_looked_for_and_why() {
    for held in declined() {
        for (what, text) in [
            ("modality", held.modality.as_str()),
            ("until", held.until.as_str()),
        ] {
            assert!(
                !text.trim().is_empty(),
                "{} declines with no {what}, which is an omission wearing a refusal's clothes (A7)",
                held.modality
            );
        }
    }
}
