//! A model held is a model let go, in the record (B-210, A26, A1).
//!
//! **The record keeps its lifecycle events in pairs**, and every pair exists
//! for the same reason: after the second one the thing is gone, and the entry
//! is the only evidence it was ever there. `ArtifactAcquired` with
//! `ArtifactRemoved`, `ComponentProvisioned` with `ComponentRemoved`,
//! `DaemonStarted` with `DaemonStopped`.
//!
//! **`ModelHosted` arrived without its pair being written on every path.**
//! Stopping the daemon while it held a model did stop the engine — dropping
//! what owns it is what stops it — and wrote nothing. The record then read
//! *hosted* with no answer, and anybody reading it later would conclude a
//! model was still being served on a port. The engine had gone and the record
//! said otherwise, which is worse than either alone (A1).
//!
//! What this holds is that every place which lets go writes it down. It is a
//! structural check rather than a run, because the failing case is a *path*
//! nobody took rather than a value nobody checked.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// Every place that drops what is held writes down that it did.
///
/// The one function that takes the held model out of its slot is the one
/// place that records it, so a second path cannot let go quietly: there is
/// nowhere else to let go from.
#[test]
fn letting_go_happens_in_one_place_and_that_place_records_it() {
    let source = read("crates/mcf-serve/src/daemon.rs");

    // `holding.take()` is how a held model stops being held. Every occurrence
    // must be inside the function that records it.
    let mut taking = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains("holding.take()") {
            taking.push((number + 1, trimmed.to_owned()));
        }
    }
    assert_eq!(
        taking.len(),
        1,
        "a held model is taken out of its slot in more than one place, so one of them can let \
         go without writing it down (B-210, A26): {taking:#?}"
    );

    // And that place records it.
    let (_, body) = source
        .split_once("fn let_go(")
        .expect("there is one place that lets go, and it is called let_go");
    let body = body.split_once("\n    }\n").map_or(body, |(held, _)| held);
    assert!(
        body.contains("holding.take()"),
        "the function named for letting go is not the one that lets go"
    );
    assert!(
        body.contains("EntryKind::ModelUnhosted"),
        "letting go does not write anything down, so the record can say a model is hosted \
         after it has stopped (A1, A26)"
    );
}

/// Stopping the daemon lets go before it records its own stop.
///
/// The order matters for reading the record back: a `daemon_stopped` between
/// a hosting and its release would read as a daemon that stopped while still
/// serving.
#[test]
fn a_daemon_lets_go_before_it_records_its_own_stop() {
    let source = read("crates/mcf-serve/src/daemon.rs");
    let (before, after) = source
        .split_once("EntryKind::DaemonStopped")
        .expect("the daemon records its own stop");
    let _ = after;
    let released = before
        .rfind("self.let_go(")
        .expect("the daemon lets go of a held model before recording that it stopped (B-210)");
    let answered = before
        .rfind("self.answer_one(")
        .expect("the daemon answers the request that stops it");
    assert!(
        released > answered,
        "the daemon records its stop without letting go of what it was holding first"
    );
}

/// Both halves of the pair exist in the record's own vocabulary.
#[test]
fn the_pair_is_a_pair() {
    let kinds = read("crates/mcf-record/src/journal/entry.rs");
    for half in ["ModelHosted", "ModelUnhosted"] {
        assert!(
            kinds.contains(half),
            "{half} is not a kind the record knows, so the pair is not a pair"
        );
    }
    let path: &Path = &mcf_checks::workspace::root().join("crates/mcf-serve/src/daemon.rs");
    assert!(
        path.is_file(),
        "this check looks at a file that is not there"
    );
}
