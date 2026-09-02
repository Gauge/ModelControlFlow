//! The record stays integral because nothing writes anything else into it
//! (A6, §3.3, F16).
//!
//! **What changed and why this check exists now.** MCF's JSON reader used to
//! refuse fractions and exponents outright, which kept floating point out of
//! the record by making it unspellable. That was the right property and the
//! wrong mechanism: the same reader reads documents MCF did not write, and a
//! repository's `config.json` legitimately contains `1e-06` — the reference
//! model's does, and refusing it meant MCF could not plan for that model at all
//! ([findings.md](../../doc/findings.md) F16).
//!
//! So the reader now keeps such a number as [`Value::ForeignNumber`], byte for
//! byte, unusable as a quantity. The property A6 wants — *no rounded value ever
//! travels through the record* — is no longer given by the parser, so it is
//! given here: **nothing outside the parser constructs one**, and every encoder
//! in the workspace produces integers, text, lists and maps.

// Every item in this file is test code; see the note in checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

use mcf_record::json::Value;

/// The one file allowed to construct the variant: the reader that reads
/// somebody else's JSON.
const THE_READER: &str = "crates/mcf-record/src/json.rs";

/// The one file allowed to write a decimal at all, and it writes one to an
/// engine and not to a record: `llama.cpp`'s API takes a temperature as a
/// JSON number, and a caller who stated `0.7` is owed exactly that on the
/// wire (B-431, A1). Rendered from thousandths, so nothing is rounded; kept
/// to this file, so nothing else discovers the constructor and puts one in a
/// record.
const THE_ENGINE_REQUEST: &str = "crates/mcf-serve/src/served.rs";

/// Nothing but the reader makes a number this format does not carry.
///
/// A source check rather than a type-level one, because the variant has to be
/// public for a reader to match on: what keeps it out of the record is that no
/// encoder writes one, and that is a property of the code as written.
#[test]
fn nothing_but_the_reader_constructs_a_number_the_record_cannot_carry() {
    let root = mcf_checks::workspace::root();
    let mut found = Vec::new();
    for file in sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        // The reader, and the tests beside it that assert what the reader
        // produces: a test that could not name the value it expects could not
        // check it (A19).
        if relative == THE_READER || relative.starts_with("crates/mcf-record/src/json/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            // Constructing one, rather than matching on one: a `match` arm that
            // handles the variant is a reader doing its job, and every decoder
            // in the tree has to have one.
            if code.contains("ForeignNumber(") && !code.contains("Value::ForeignNumber(_)") {
                found.push(format!("{relative}:{}", number.saturating_add(1)));
            }
            // The constructor the reader offers for an engine's request is
            // called from that request and nowhere else.
            if code.contains("exact_thousandths(") && relative != THE_ENGINE_REQUEST {
                found.push(format!("{relative}:{}", number.saturating_add(1)));
            }
        }
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "something outside {THE_READER} constructs a number the record cannot carry. \
         A6 makes every recorded quantity `Ord`, and a value nobody can order is a value \
         that must not reach the record"
    );
}

/// And it is not a quantity to anybody who asks.
///
/// The other half: even if one reached a record, nothing could read it *as a
/// number*, so it cannot become a measurement by accident.
#[test]
fn a_number_the_record_cannot_carry_is_not_a_quantity() {
    let value = mcf_record::json::parse("1e-06").expect("a number this format does not carry");
    assert_eq!(value.as_integer(), None);
    assert_eq!(value.as_text(), None);
    assert_eq!(value.as_list(), None);
    assert_eq!(value.to_line(), "1e-06", "it is not kept as written");
}

/// What MCF encodes is integral, all the way through a real record.
///
/// The property stated end to end rather than in pieces: build the entries MCF
/// actually writes, render them, read them back, and walk every value.
#[test]
fn a_record_mcf_wrote_holds_no_number_it_cannot_carry() {
    let machine = mcf_core::hardware::Machine::read();
    let entries = [
        mcf_record::encode::machine(&machine),
        mcf_record::encode::failure(&mcf_core::failure::Failure::new(
            mcf_core::failure::Category::ConfigInvalid,
            mcf_core::failure::Attribution::User,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("a check"),
            "something went wrong",
        )),
    ];
    for entry in entries {
        let line = entry.to_line();
        let read = mcf_record::json::parse(&line).expect("MCF's own encoding reads back");
        assert!(
            integral(&read),
            "an encoder produced a number the record cannot carry: {line}"
        );
    }
}

fn integral(value: &Value) -> bool {
    match value {
        Value::ForeignNumber(_) => false,
        Value::List(values) => values.iter().all(integral),
        Value::Map(pairs) => pairs.values().all(integral),
        Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Text(_) => true,
    }
}

fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&root.join("crates"), &mut found);
    found
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
