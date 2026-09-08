#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_record::journal::EntryKind;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

#[test]
fn an_observation_is_not_the_act_it_might_lead_to() {
    assert_eq!(
        EntryKind::ModelProbed.as_str(),
        "model_probed",
        "a kind's written name is stable for life (C5): once written it travels between \
         machines and versions"
    );
    assert_ne!(EntryKind::ModelProbed, EntryKind::ModelConfigured);
    assert_eq!(
        EntryKind::parse("model_probed"),
        Some(EntryKind::ModelProbed),
        "and a record written by this build must be readable by it"
    );
    assert!(
        EntryKind::ALL.contains(&EntryKind::ModelProbed),
        "a kind absent from ALL is invisible to the index, which stores a kind as its position \
         in that list"
    );
}

#[test]
fn every_probe_that_renders_an_observation_records_it() {
    let source = read("crates/mcf-serve/src/probes/run.rs");
    let rendering = renderers(&source);
    assert!(
        rendering.len() >= 2,
        "no probe renderer was found, so this check is reading the wrong file"
    );

    let mut silent = Vec::new();
    for (name, body) in &rendering {
        if !body.contains("Outcome::Observed") {
            continue;
        }
        if !body.contains("record_probed") {
            silent.push(name.clone());
            continue;
        }
        let wrote = body.find("record_probed");
        for verdict in ["DIVERGENCE", "VERIFIED"] {
            if let (Some(wrote), Some(said)) = (wrote, body.find(verdict)) {
                assert!(
                    wrote < said,
                    "{name}: the observation is recorded after the {verdict} branch, so what \
                     is kept depends on how the observation was read (A9)"
                );
            }
        }
    }
    assert!(
        silent.is_empty(),
        "a probe that prints and does not write leaves a measurement nobody can find later \
         (A1, B-386, F106): {silent:#?}"
    );
}

#[test]
fn every_probe_method_has_a_renderer() {
    let mut methods = Vec::new();
    for file in probe_sources() {
        for line in read(&file).lines() {
            if let Some((held, _)) = line.split_once(": Method = Method {")
                && let Some(name) = held.split_whitespace().last()
            {
                methods.push(name.to_owned());
            }
        }
    }
    assert!(
        methods.len() >= 4,
        "the probes define fewer methods than MCF has probes, so this is reading the wrong \
         files: {methods:#?}"
    );
    let source = read("crates/mcf-serve/src/probes/run.rs");
    let rendered = renderers(&source)
        .into_iter()
        .filter(|(_, body)| body.contains("Outcome::Observed"))
        .count();
    assert_eq!(
        rendered,
        methods.len(),
        "there are {} probe method(s) — {methods:#?} — and {rendered} function(s) in probes/run.rs \
         that render one. A probe with no renderer cannot be run; a renderer with no method is \
         not a probe (D42)",
        methods.len()
    );
}

fn probe_sources() -> Vec<String> {
    let root = mcf_checks::workspace::root();
    let mut found = vec!["crates/mcf-serve/src/probes.rs".to_owned()];
    let directory = root.join("crates/mcf-serve/src/probes");
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|kind| kind == "rs")
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
        {
            found.push(format!("crates/mcf-serve/src/probes/{name}"));
        }
    }
    found.sort();
    found
}

fn renderers(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("_lines(") {
        let (before, after) = rest.split_at(at);
        let name = before
            .rsplit_once("fn ")
            .map(|(_, held)| format!("{held}_lines"));
        let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
        if let Some(name) = name
            && !name.contains(' ')
            && !found
                .iter()
                .any(|(held, _): &(String, String)| held == &name)
            && before.ends_with(&format!("fn {}", name.trim_end_matches("_lines")))
        {
            found.push((name, body.to_owned()));
        }
        let Some(next) = after.get("_lines(".len()..) else {
            break;
        };
        rest = next;
    }
    found
}

#[test]
fn a_measurement_that_could_not_be_kept_does_not_read_as_kept() {
    let source = read("crates/mcf-serve/src/probes/run.rs");
    assert!(
        source.contains("BUT NOT RECORDED"),
        "A2: a probe whose result could not be written must say so rather than printing the \
         figure as though it had been kept"
    );
}

#[test]
fn a_context_measured_for_one_file_is_not_a_fact_about_another() {
    let source = read("crates/mcf-cli/src/history.rs");
    let (_, body) = source
        .split_once("fn probed_context(")
        .expect("`probed_context` is what reads it back");
    assert!(
        body.contains("model.display().to_string()"),
        "the path must match exactly: a context measured for one file is not a fact about a \
         differently quantized sibling, and the conditions §3.4 requires include which \
         artifact was asked"
    );
}
