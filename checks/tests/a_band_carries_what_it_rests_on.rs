//! A projection says what it was read between (B-385, §3.4, A6, A7, B34).
//!
//! **The defect this closes, found rather than predicted.** F74: an unrelated
//! test suite held twenty-six cores of this machine while four comparisons
//! were taken, and a generation that takes 400 ms on a quiet machine took
//! eighteen seconds. Those entries are true and stay in the record (A1). But
//! `project::band` read the history without reading the conditions the history
//! was taken under, so every surface rendering a band — the expected duration,
//! `mcf explain`, `mcf doctor`'s score — rested on them in silence. A number
//! whose conditions do not travel with it is exactly what §3.4 and A6 exist to
//! prevent, one layer below where MCF was enforcing them.
//!
//! **Carried, not filtered.** Filtering contended history needs a threshold,
//! and what counts as too busy is DEC-007's open band. Saying what the band
//! rested on needs no threshold at all.
//!
//! **And unknown is not quiet** (A7). Entries written before B-217 recorded
//! nothing about the machine. That is `None`, it renders as *unknown*, and a
//! zero would be a claim MCF cannot support.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use mcf_bench::project::{Point, band};

fn point(bytes: u64, competing: Option<u64>) -> Point {
    Point {
        bytes,
        tokens: 128,
        fastest: bytes.saturating_mul(10),
        slowest: bytes.saturating_mul(12),
        competing,
    }
}

/// The band and its conditions cannot be separated by a caller.
#[test]
fn the_conditions_travel_with_the_band() {
    let held = band(&[point(100, Some(33_050)), point(200, Some(150))], 150, 128)
        .expect("150 sits between 100 and 200");
    let shown = held.rested_on().to_string();
    assert!(
        shown.contains("33.05 core(s)") && shown.contains("0.15 core(s)"),
        "both of the two runs the band was read between must be named: {shown}"
    );
    assert_eq!(
        held.rested_on().busiest(),
        Some(33_050),
        "and the busier of them is available to a caller that wants one figure"
    );
}

/// A history that recorded nothing about the machine says *unknown*.
#[test]
fn nothing_recorded_is_never_rendered_as_quiet() {
    let held = band(&[point(100, None), point(200, None)], 150, 128)
        .expect("150 sits between 100 and 200");
    assert_eq!(held.rested_on().busiest(), None);
    let shown = held.rested_on().to_string();
    assert!(
        shown.contains("unknown and not quiet"),
        "A7: an absent reading is unknown, and a zero would be a claim MCF cannot support: \
         {shown}"
    );
    for wrong in ["0.00 core(s)", "quiet machine", "nothing competing"] {
        assert!(!shown.contains(wrong), "{wrong} in {shown}");
    }
}

/// One known and one unknown is neither of the two simple sentences.
#[test]
fn a_half_known_pair_says_which_half() {
    let held = band(&[point(100, Some(2_500)), point(200, None)], 150, 128)
        .expect("150 sits between 100 and 200");
    let shown = held.rested_on().to_string();
    assert!(shown.contains("2.50 core(s)"), "{shown}");
    assert!(
        shown.contains("recorded nothing about the machine"),
        "the unknown half must not be quietly dropped (A1): {shown}"
    );
}

/// No point is removed from the history for being contended.
#[test]
fn contended_history_is_carried_and_never_filtered() {
    let source = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-bench/src/project.rs"),
    )
    .expect("project.rs is readable");
    for filtering in ["competing >", "competing <", "competing.is_some_and"] {
        assert!(
            !source.contains(filtering),
            "the projection must not filter on how busy the machine was: what counts as too \
             busy is DEC-007's open band, and a threshold invented here is exactly the figure \
             that decision exists to derive from measurement"
        );
    }
}

/// Every surface that renders a band renders what it rested on.
#[test]
fn the_surfaces_render_it() {
    for (file, why) in [
        (
            "crates/mcf-cli/src/explain.rs",
            "the page that answers *how fast would this be here*",
        ),
        (
            "crates/mcf-cli/src/bench.rs",
            "the expected duration a run declares before it starts",
        ),
    ] {
        let source = std::fs::read_to_string(mcf_checks::workspace::root().join(file))
            .unwrap_or_else(|error| panic!("{file} is readable: {error}"));
        assert!(
            source.contains("rested_on()"),
            "{file} renders a band and must render what it rested on — {why} (B-385)"
        );
    }
}
