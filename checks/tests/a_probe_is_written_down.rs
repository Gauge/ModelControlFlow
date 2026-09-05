//! What a probe measured is kept, not printed and forgotten (B-386, A1, A2,
//! B-055, D42).
//!
//! **A1's plainest case.** *A measurement nobody can find later is the same as
//! one not taken.* `mcf probe` established the usable context of a model
//! against its declared one (F42) and printed it; the terminal scrolled. Every
//! surface wanting a measured figure rather than a declared one was blocked
//! behind that, which is how B-382 found it.
//!
//! **Kept apart from the act it might lead to** (D42, D43). `ModelProbed` is
//! an observation; `ModelConfigured` is somebody deciding to address a model
//! differently. A probe that changes nothing still measured something (A9),
//! and collapsing the two would make *MCF looked* and *MCF changed* the same
//! entry.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_record::journal::EntryKind;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// The kind exists, is named stably, and is distinct from the act.
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

/// Every probe that renders an observation records it.
///
/// **This named one probe, and three were built after it** (F106). It read
/// `fn context_lines(` by name and asked whether *that* function wrote to the
/// record — so B-386's row said *a probe's outcome is written to the record*
/// while the tool-calling probe printed its result and kept nothing, and a
/// question answered yesterday could not be read back today. It is the shape
/// F103 and F105 both have: a guard covers the place it was written for.
///
/// So it is bound to the shape instead. Any function in `probes/run.rs` that renders
/// an `Outcome::Observed` is rendering a probe's result, and must record it —
/// whichever way it came out (A9), which is checked by requiring the write to
/// sit outside the branch that reports a divergence.
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
        // **Recorded from the observation, before the verdict.** *Agrees* is
        // as much a measurement as *diverges* (A9), and the way that goes
        // wrong is a write that sits inside the branch which decides what the
        // observation means — a record of MCF's interpretation rather than of
        // what happened. So the write must come *before* any line that renders
        // a verdict, which is a position rather than a promise.
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

/// Every probe MCF has is rendered by one of those functions.
///
/// The list of probes is read from where they are defined — a `Method` is what
/// a probe is — so a new one cannot be added, run and printed without this
/// check seeing it. A `Method` with no renderer is a probe nobody can run; a
/// renderer with no `Method` is not a probe.
#[test]
fn every_probe_method_has_a_renderer() {
    let mut methods = Vec::new();
    // Read from wherever a probe lives rather than from a list of three files.
    // The list was three files, and the day two probes arrived in two new ones
    // it was still three (B-057) — which is the shape F103, F105 and F106 each
    // paid for: a guard covers the place it was written for.
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
    // Renderers *of an observation*: a `*_lines` function that renders no
    // `Outcome` is a section of the report rather than a probe's result, and
    // counting it would demand a `Method` for the list of modalities MCF
    // declines to probe — which is the opposite of a probe (B-057).
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

/// Every file a probe can be defined in.
///
/// The directory rather than a list: a probe in a file nobody added to a list
/// is a probe this check does not see, and that is exactly how B-057's two new
/// ones arrived.
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

/// The functions in `probes/run.rs` that render a probe's result, by name and body.
///
/// Named `*_lines` by convention, which is the convention this check makes
/// load-bearing: it is how a renderer is told from a helper, and a probe
/// renderer that broke it would be a probe this check stopped watching.
fn renderers(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("_lines(") {
        let (before, after) = rest.split_at(at);
        let name = before
            .rsplit_once("fn ")
            .map(|(_, held)| format!("{held}_lines"));
        // To the end of the function, which is the first line that starts a
        // new top-level item. A split that silently found nothing would make
        // this check assert about the whole rest of the file.
        let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
        if let Some(name) = name
            && !name.contains(' ')
            && !found.iter().any(|(held, _): &(String, String)| held == &name)
            // A *call* to a renderer is not the renderer: the declaration is
            // the one preceded by `fn `, and the body between it and the next
            // top-level item.
            && before.ends_with(&format!("fn {}", name.trim_end_matches("_lines")))
        {
            found.push((name, body.to_owned()));
        }
        // Past this occurrence, or the next search finds it again and this
        // loop never ends — which is how a check becomes a hang rather than a
        // failure.
        let Some(next) = after.get("_lines(".len()..) else {
            break;
        };
        rest = next;
    }
    found
}

/// A write that failed says so.
#[test]
fn a_measurement_that_could_not_be_kept_does_not_read_as_kept() {
    let source = read("crates/mcf-serve/src/probes/run.rs");
    assert!(
        source.contains("BUT NOT RECORDED"),
        "A2: a probe whose result could not be written must say so rather than printing the \
         figure as though it had been kept"
    );
}

/// The reader will not answer about a different file.
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
