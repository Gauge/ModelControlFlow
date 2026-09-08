#![allow(clippy::panic, clippy::expect_used)]

use mcf_core::touchstone::CATALOGUE;

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
