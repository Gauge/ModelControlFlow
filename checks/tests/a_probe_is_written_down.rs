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

/// The probe writes whichever way the measurement came out.
#[test]
fn agreement_is_recorded_as_well_as_divergence() {
    let source = read("crates/mcf-cli/src/probe.rs");
    let (_, after) = source
        .split_once("fn context_lines(")
        .expect("the context probe is where the measurement is taken");
    // To the end of the function, which is the first line that starts a new
    // top-level item. `\n/// ` is not reliable — the next item may have no doc
    // comment — and a split that silently found nothing would make this check
    // assert about the whole rest of the file.
    let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
    assert!(
        body.contains("record_probed_context"),
        "a probe that prints and does not write leaves a measurement nobody can find (A1)"
    );
    // The write must not sit inside the divergence branch: *agrees* is as much
    // a measurement as *diverges*, and a record that kept only the surprising
    // half could not answer *what does this machine take* (A9).
    let (_, wrote) = body
        .split_once("record_probed_context")
        .expect("checked just above");
    assert!(
        !wrote.contains("DIVERGENCE"),
        "the write must come after both branches, so that agreement is kept too (A9)"
    );
}

/// A write that failed says so.
#[test]
fn a_measurement_that_could_not_be_kept_does_not_read_as_kept() {
    let source = read("crates/mcf-cli/src/probe.rs");
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
