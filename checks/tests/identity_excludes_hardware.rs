//! The identity of a configuration names no machine.
//!
//! B57's check is `compiler`: *the identity type excludes hardware fields by
//! construction* (B-272). The compiler holds it as long as no such field is
//! ever added, and that is what a type cannot check about itself — the failure
//! mode is a field added later for the convenience of one query, after which
//! every machine has its own universe of configurations and §XIV's corpus has
//! nothing to aggregate.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// No field, type or accessor in the configuration module names a machine.
///
/// The vocabulary is deliberately broad. A field called `device_id` is the
/// same defect as one called `hardware`, and so is one called `host`.
#[test]
fn the_configuration_module_names_no_machine() {
    const MACHINE_WORDS: [&str; 10] = [
        "hardware",
        "machine",
        "accelerator_",
        "device",
        "gpu",
        "cpu_model",
        "host",
        "driver",
        "thermal",
        "vram",
    ];
    let mut offenders = Vec::new();
    for file in ["mod.rs", "sampling.rs", "placement.rs"] {
        for (number, line) in code_only(&configuration_source(file)) {
            let lowered = line.to_lowercase();
            for word in MACHINE_WORDS {
                if lowered.contains(word) {
                    offenders.push(format!("configuration/{file}:{number} → {word}"));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the identity type names a machine: {offenders:#?} (B57, B-272)"
    );
}

/// The fields of `Configuration` are exactly the ones D17, D18 and intent v16
/// put in the identity.
///
/// Stated as a list so that adding a field is a deliberate act with a citation
/// rather than an edit nobody reviewed. C5 makes identity boundaries expensive
/// to move: a group can be widened and never narrowed.
#[test]
fn the_identity_is_exactly_what_the_intent_puts_in_it() {
    let body = block(
        &configuration_source("mod.rs"),
        "pub struct Configuration {",
    );
    let mut fields: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter_map(|line| line.trim_end_matches(',').split_once(": "))
        .map(|(name, _)| name.trim_start_matches("pub ").to_owned())
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            "context_length",
            "engine",
            "placement",
            "quantization",
            "sampling",
            "weights",
        ],
        "the identity's fields have changed; D17, D18 and intent v16 decide this set"
    );
}

/// The realized layout is a condition rather than identity, and it is present
/// — so the split intent v16 describes exists in both halves rather than only
/// in the half that removes something.
#[test]
fn the_realized_layout_is_a_condition() {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/measurement/conditions.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    assert!(
        source.contains("pub realized_placement: Attested<ConditionValue>"),
        "the realized layout is not a condition, so intent v16's split is half-built"
    );
}

fn configuration_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-core/src/configuration")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// Lines that are not documentation, numbered from one.
///
/// The documentation names the machine words in order to say they are absent,
/// so a check that read it would fail on the paragraph explaining why it
/// passes.
fn code_only(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(index, line)| (index + 1, line.to_owned()))
        .collect()
}

fn block(source: &str, opening: &str) -> String {
    let (_, rest) = source
        .split_once(opening)
        .unwrap_or_else(|| panic!("`{opening}` is no longer where this check looks"));
    let (body, _) = rest
        .split_once("\n}")
        .unwrap_or_else(|| panic!("`{opening}` is never closed"));
    body.to_owned()
}
