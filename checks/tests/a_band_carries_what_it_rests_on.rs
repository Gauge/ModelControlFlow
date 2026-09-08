#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

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
