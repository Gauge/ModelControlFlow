#![allow(clippy::panic)]

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
