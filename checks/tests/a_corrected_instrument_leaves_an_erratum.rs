//! An instrument found to be wrong leaves a record of having been wrong
//! (F93, A1, A2, §3.4, §6.16).
//!
//! **The defect this closes.** §3.4 makes MCF's own version part of every
//! measurement's conditions, on the premise that a reader who knows what took
//! a number can judge it. A version string does not change when an instrument
//! does: three measuring instruments changed in this repository in one working
//! day and every record on either side of all three said `0.1.0-m0`. The
//! condition that was supposed to identify the instrument identified nothing,
//! and `MCF_BUILD_COMMIT` — which exists for exactly this — is set in one
//! script and never in the binary an operator builds.
//!
//! Two things follow, and this file holds both:
//!
//! 1. a measurement carries the **digest of the binary that took it**, which
//!    no build environment can forget to supply;
//! 2. an instrument corrected after measurements were taken leaves an
//!    **erratum**, so that those measurements say what was wrong with them.
//!
//! **Why the erratum cannot be optional.** Correcting an instrument is the
//! moment at which the record silently splits into a before and an after. If
//! that split lives only in a commit message, a reader of the measurement will
//! never find it — which is A2's silent failure with a longer fuse.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_core::errata::{KNOWN, affecting};

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// Every measurement records the binary that took it.
#[test]
fn a_measurement_carries_the_digest_of_its_instrument() {
    let encoded = read("crates/mcf-record/src/encode.rs");
    assert!(
        encoded.contains("\"instrument\"") && encoded.contains("build_identity::instrument()"),
        "the condition set must carry what took the measurement, and a version string does not \
         change when an instrument does (F93, §3.4)"
    );
    assert!(
        encoded.contains("Attested::Unknown => Value::Null"),
        "and a platform that will not let MCF read its own executable yields nothing rather \
         than a placeholder (A7)"
    );

    let identity = read("crates/mcf-core/src/build_identity.rs");
    assert!(
        identity.contains("current_exe()"),
        "the digest must be of the running binary itself: no build cooperation, no git, and \
         it works for a binary shipped in a tarball"
    );
    assert!(
        identity.contains("OnceLock"),
        "and computed once, because hashing the executable per record write would make MCF \
         pay for its own honesty on every line"
    );
}

/// Every finding that corrects an instrument is in the errata list.
///
/// The list is what a reader of an old measurement meets. A correction that
/// never reaches it leaves those measurements looking sound.
#[test]
fn a_corrected_instrument_is_listed() {
    let findings = read("doc/findings.md");
    for held in KNOWN {
        assert!(
            findings.contains(held.finding),
            "{} is cited by an erratum and does not exist in the findings",
            held.finding
        );
        assert!(
            !held.instrument.is_empty(),
            "{} names no instrument, so a reader cannot tell what it applies to",
            held.finding
        );
    }
    assert!(
        KNOWN.len() >= 3,
        "three instrument defects were found in one day and all three are listed; a shorter \
         list means one was corrected without leaving an erratum"
    );
}

/// The list applies backwards and stops at the correction.
#[test]
fn an_erratum_applies_to_everything_before_it_and_nothing_after() {
    for held in KNOWN {
        assert!(
            affecting(held.corrected_at_utc_nanos.saturating_sub(1))
                .iter()
                .any(|found| found.finding == held.finding),
            "{} must apply to a measurement taken before it was corrected",
            held.finding
        );
        assert!(
            !affecting(held.corrected_at_utc_nanos)
                .iter()
                .any(|found| found.finding == held.finding),
            "{} must not apply to one taken after",
            held.finding
        );
    }
}

/// The record is never rewritten to make itself look better.
#[test]
fn nothing_edits_the_measurements_it_corrects() {
    let errata = read("crates/mcf-core/src/errata.rs");
    for rewriting in ["fn amend", "fn correct_entry", "fn rewrite", "fn overwrite"] {
        assert!(
            !errata.contains(rewriting),
            "`{rewriting}` would edit history to look better, and a record that does that is \
             not a record (A1)"
        );
    }
    let log = read("crates/mcf-cli/src/log.rs");
    assert!(
        log.contains("errata_for(entry.recorded_at())"),
        "an erratum must be rendered beside the entry it concerns: a reader meeting a \
         measurement is the person who needs to know, and they will not go looking (A2)"
    );
}
