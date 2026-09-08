#![allow(clippy::panic, clippy::expect_used)]

use mcf_core::touchstone::CATALOGUE;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

#[test]
fn a_touchstone_carries_no_number() {
    for held in CATALOGUE {
        for part in [held.subject(), held.bare(), held.unmeasured()] {
            assert!(
                !part.chars().any(|letter| letter.is_ascii_digit()),
                "a touchstone with a number in it will be read as a measurement: {part:?}"
            );
        }
    }
}

#[test]
fn nothing_in_the_record_knows_what_a_touchstone_is() {
    for file in [
        "crates/mcf-record/src/encode.rs",
        "crates/mcf-record/src/journal.rs",
        "crates/mcf-record/src/journal/entry.rs",
        "crates/mcf-record/src/export.rs",
        "crates/mcf-record/src/contribution.rs",
    ] {
        let path = mcf_checks::workspace::root().join(file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        assert!(
            !source.contains("Touchstone") && !source.contains("touchstone"),
            "{file} names a touchstone: guidance that reaches the record is guidance a later \
             reader meets as data (B-380, A25's shape)"
        );
    }
}

#[test]
fn the_comparison_view_keeps_them_apart_from_its_results() {
    let source = read("crates/mcf-cli/src/bench.rs");
    assert!(
        source.contains("rules of thumb, which are not results"),
        "the comparison view must separate guidance from results with its own heading, because \
         a reader skimming sees the heading and not the sentence (B-380)"
    );
    assert!(
        source.contains("fn rules_of_thumb("),
        "and must choose which apply from what the comparison isolated, rather than printing \
         all of them beside everything"
    );
    assert!(
        !source.contains(".bare()"),
        "the comparison view renders a touchstone without its mark (A5's shape)"
    );
}
