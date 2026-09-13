#![allow(clippy::panic, clippy::expect_used)]

use mcf_serve::hosting::Hosting;

fn settings() -> Hosting {
    Hosting::recommended("llama.cpp", "a card", true, 32_768, Some(8), true, None)
}

/// Every name the Configure tab puts in front of a person, in the order it draws them.
const SHOWN: &[&str] = &[
    "Device",
    "Context window",
    "Cache width",
    "Split mode",
    "Experts",
    "Main device",
    "Devices",
    "Dense layers on the processor",
    "Tensors placed by hand",
    "Cache in system memory",
    "Memory lock",
    "How it loads",
    "Large tensors",
    "Prompt batch",
    "Micro-batch",
    "Threads",
    "Threads for reading a prompt",
    "Flash attention",
    "Thinking budget",
    "Draft depth",
    "Prompt cache",
    "Prompt cache memory",
    "Prefix reuse",
    "Keep idle slots",
    "Checkpoints",
    "Checkpoint spacing",
    "Context shift",
    "Tokens kept in front",
    "Name callers use",
    "Conversations at once",
    "Port",
    "Reachable from the network",
    "Answer kind",
    "Pooling",
    "API key",
];

fn because_of(name: &str) -> Option<&'static str> {
    let held = settings();
    held.listed(&held)
        .into_iter()
        .find(|setting| setting.name.eq_ignore_ascii_case(name))
        .map(|setting| setting.because)
}

#[test]
fn every_setting_the_window_shows_has_something_to_say_about_itself() {
    let mut silent = Vec::new();
    for name in SHOWN {
        match because_of(name) {
            Some(because) if !because.trim().is_empty() => {}
            _ => silent.push(*name),
        }
    }
    assert!(
        silent.is_empty(),
        "a setting a person can change and nothing explains is a setting they have to guess \
         at: {silent:?}"
    );
}

#[test]
fn a_description_is_a_sentence_rather_than_a_restatement_of_the_name() {
    for name in SHOWN {
        let because = because_of(name).unwrap_or_default();
        assert!(
            because.split_whitespace().count() >= 6,
            "{name:?} says only {because:?}"
        );
        assert!(
            !because.eq_ignore_ascii_case(name),
            "{name:?} explains itself with its own name"
        );
    }
}

#[test]
fn the_names_are_short_enough_to_read_at_a_glance() {
    for name in SHOWN {
        assert!(
            name.split_whitespace().count() <= 5,
            "{name:?} is a sentence, not a label"
        );
        assert!(!name.ends_with('.'), "{name:?}");
    }
}

#[test]
fn nothing_is_named_twice() {
    let mut seen: Vec<String> = SHOWN.iter().map(|name| name.to_lowercase()).collect();
    seen.sort();
    let all = seen.len();
    seen.dedup();
    assert_eq!(all, seen.len(), "two rows share a name");
}

#[test]
fn the_thinking_budget_is_among_the_settings_a_person_can_set() {
    assert!(SHOWN.contains(&"Thinking budget"));
    let because = because_of("Thinking budget").unwrap_or_default();
    assert!(
        because.contains("engine"),
        "the budget is counted by the engine, and saying so is the point of it: {because}"
    );
}

#[test]
fn a_setting_named_in_the_window_matches_the_one_the_record_names() {
    let held = settings();
    let named: Vec<&str> = held.listed(&held).into_iter().map(|one| one.name).collect();
    for name in SHOWN {
        assert!(
            named.iter().any(|held| held.eq_ignore_ascii_case(name)),
            "{name:?} is drawn by the window and known to nothing else"
        );
    }
}
