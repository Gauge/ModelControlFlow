//! A rule of thumb cannot be read as a measurement, cannot be recorded, and
//! cannot outlive the laboratory that would replace it (B-380, DEC-002, A21,
//! A25's shape, §3.15, §3.18).
//!
//! **The hazard is the whole of the design.** A rule of thumb printed beside a
//! measured number in the same typeface *becomes* a measured number to a reader
//! who is not looking for the difference, and the readers touchstones exist for
//! are exactly those readers. So the properties that keep them apart are held
//! here rather than asked for in a style guide.
//!
//! **Why a check and not only the type.** `Touchstone`'s constructor and
//! `Display` hold two of the three properties inside the crate. The third —
//! that a touchstone never reaches the record — is a property *between* crates,
//! which is what a type cannot check about itself; it is the same shape as
//! `content_is_not_the_record.rs`, and it fails the same way: a convenience
//! added later for one call site.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use mcf_core::touchstone::CATALOGUE;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// No number, anywhere in the catalogue.
///
/// The property that makes the separation visible rather than merely stated:
/// every number on a surface came from a measurement, so a sentence with a
/// number in it is a sentence a reader is entitled to check.
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

/// The record has no way to hold one.
///
/// A25 keeps user content out of the record by making the two stores different
/// types with no path between them, and the reasoning transfers exactly: a
/// filter can be misconfigured, and a sentence that was never written cannot be
/// read back as a measurement by somebody who has forgotten where it came from.
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

/// A touchstone is temporary, and the register says when it expires.
///
/// Each names the item whose laboratory would replace it with a measurement.
/// When that item is done, offering a rule of thumb about something MCF can now
/// measure is the defect — so this fails until the touchstone is removed, which
/// is B-380's *the replacement is visible as a change* held by a machine.
#[test]
fn no_touchstone_outlives_the_laboratory_that_would_replace_it() {
    let register = read("doc/backlog.md");
    for held in CATALOGUE {
        let row = register
            .lines()
            .find(|line| line.starts_with(&format!("| {} |", held.until())))
            .unwrap_or_else(|| {
                panic!(
                    "a touchstone names {} and the register has no such item (C5)",
                    held.until()
                )
            });
        let status = row
            .rsplit_once(" | ")
            .map(|(_, held)| held.to_lowercase())
            .unwrap_or_default();
        assert!(
            !status.starts_with("done") && !status.starts_with("**done"),
            "{} is done, so MCF can now measure {:?} and must not offer a rule of thumb about \
             it — remove the touchstone (B-380)",
            held.until(),
            held.subject()
        );
    }
}

/// The surface renders them apart, under their own heading, through the type.
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
    // Rendered through `Display`, which cannot omit the mark or the limits. A
    // surface reaching for `bare()` has written the word and can be found.
    assert!(
        !source.contains(".bare()"),
        "the comparison view renders a touchstone without its mark (A5's shape)"
    );
}
