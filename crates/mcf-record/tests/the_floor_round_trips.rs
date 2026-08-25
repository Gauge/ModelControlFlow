//! The §3.4 floor, captured from a live machine, survives the record.
//!
//! B-007's condition. What makes it worth a test of its own rather than a
//! property of the encoder is A7: a floor whose unknowns came back as the
//! *word* `unknown` would compare equal to a floor that had read something, and
//! every downstream comparison would be quietly wrong. The round trip is where
//! that shows.
//!
//! B19 keeps it hermetic: a temporary directory, removed when the test ends.

#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::capture;
use mcf_core::hardware::Machine;
use mcf_core::measurement::Conditions;
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;
use mcf_record::{decode, encode};

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-floor-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        Self(path)
    }

    fn journal(&self) -> PathBuf {
        self.0.join("record.jsonl")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// Captured from this machine, written, replayed, and identical.
#[test]
fn a_captured_floor_survives_the_journal_unchanged() {
    let scratch = Scratch::new("live");
    let machine = Machine::read();
    let captured = capture::conditions(&machine, None, "the round-trip test", "full");

    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        journal
            .append(&Entry::new(
                EntryKind::MachineProfile,
                AT,
                0,
                Value::map([("conditions", encode::conditions(&captured))]),
            ))
            .expect("the entry appends");
    }

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert!(replayed.is_complete(), "{}", replayed.statement());
    let recorded = replayed
        .entries
        .first()
        .expect("one entry")
        .body()
        .get("conditions")
        .expect("the conditions are recorded");

    let rebuilt = decode::conditions(recorded, BuildIdentity::current())
        .expect("the conditions are readable");
    assert_eq!(&rebuilt, &captured, "the floor did not survive the record");
}

/// The same, for the machine B19 actually requires the suite to run on: one
/// with no accelerator. Produced through the seam so the case is checked rather
/// than waited for.
#[test]
fn a_floor_full_of_unknowns_survives_as_unknowns() {
    let scratch = Scratch::new("bare");
    let machine = Machine::read_through(&[]);
    let captured = capture::conditions(&machine, None, "the round-trip test", "full");
    assert!(!captured.floor().thermal_state.is_known());

    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        journal
            .append(&Entry::new(
                EntryKind::MachineProfile,
                AT,
                0,
                Value::map([("conditions", encode::conditions(&captured))]),
            ))
            .expect("the entry appends");
    }

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    let recorded = replayed
        .entries
        .first()
        .expect("one entry")
        .body()
        .get("conditions")
        .expect("the conditions are recorded");
    let rebuilt = decode::conditions(recorded, BuildIdentity::current())
        .expect("the conditions are readable");

    assert_eq!(&rebuilt, &captured);
    // The half that matters: an unknown came back unknown and not as a word
    // that happens to read like one (A7).
    assert!(!rebuilt.floor().thermal_state.is_known());
    assert_ne!(
        rebuilt.floor().thermal_state,
        Attested::Known(mcf_core::measurement::ConditionValue::text("unknown"))
    );
}

/// A record that does not ask the same questions is not decoded into a guess.
/// §7.30: a line written by a version this one cannot fully interpret is the
/// reader's problem to name, not the parser's to paper over.
#[test]
fn a_floor_missing_a_question_is_not_decoded() {
    let partial = Value::map([("hardware_state", Value::text("a machine"))]);
    assert!(decode::floor(&partial).is_none());
}

/// Nor is a condition written in a shape this version does not use.
#[test]
fn a_condition_of_an_unexpected_shape_is_not_coerced() {
    let machine = Machine::read_through(&[]);
    let captured = capture::conditions(&machine, None, "the round-trip test", "full");
    let Value::Map(mut fields) = encode::conditions(&captured) else {
        panic!("conditions encode as an object");
    };
    fields.insert("thermal_state".to_owned(), Value::Bool(true));
    assert!(decode::floor(&Value::Map(fields)).is_none());
}

/// An integral condition stays integral, in the record and on the way back.
///
/// The fixture for the first defect the property tier found (B-191): every
/// known condition was rendered through `Display`, so a context length of 4096
/// was written as `"4096"` and read back as text. Nine of the ten questions
/// never noticed, because nine of them are naturally strings. §3.3 asks the
/// record be machine-readable first, and a number a reader has to re-parse from
/// a string is not that; B-007's *round-trips losslessly* is the claim it
/// falsified.
#[test]
fn an_integral_condition_is_recorded_as_a_number() {
    use mcf_core::measurement::{ConditionValue, Floor};

    let floor = Floor {
        context_length: Attested::Known(ConditionValue::integer(4096)),
        quantization: Attested::Known(ConditionValue::text("Q4_K_M")),
        ..Floor::nothing_known()
    };
    let conditions = Conditions::new(BuildIdentity::current(), floor);
    let encoded = encode::conditions(&conditions);

    assert_eq!(
        encoded.get("context_length"),
        Some(&Value::Integer(4096)),
        "an integral condition is written as a number, not as its rendering"
    );
    assert_eq!(
        encoded.get("quantization"),
        Some(&Value::text("Q4_K_M")),
        "a textual condition is unchanged by the fix that made numbers numbers"
    );

    let rebuilt = decode::conditions(&encoded, BuildIdentity::current())
        .expect("the conditions are readable");
    assert_eq!(&rebuilt, &conditions);
}
