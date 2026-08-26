//! A paired comparison is reconstructible from the record alone.
//!
//! B-270's second condition, and D16's reason for keeping raw trials: *§6.17
//! needs the shape of a bimodal distribution, §3.27 needs the pairing, and
//! §7.7 has not decided what statistic matters, so a frozen summary is a
//! question that can never be re-asked.*
//!
//! This writes a session's trials to a journal, throws the trials away, reads
//! the journal back, and asks §3.27's question of what came out. If the pairing
//! were not reconstructible the record would be holding numbers rather than
//! evidence.
//!
//! B19 keeps it hermetic: a temporary directory, removed when the test ends.

#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{Conditions, Count, Floor};
use mcf_core::time::Timestamp;
use mcf_core::trial::{Arm, Position, SessionId, Trial, Trials};
use mcf_record::encode;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("mcf-paired-{}", std::process::id()));
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

/// Eight trials, interleaved A, B, A, B — the shape B53 requires.
fn a_session() -> Trials<Count> {
    let session = SessionId::new("2026-08-25T10-00-00Z");
    let (a, b) = (Arm::new("Q4_K_M"), Arm::new("Q5_K_M"));
    Trials::from((0..8).map(|position| {
        let even = position % 2 == 0;
        Trial::new(
            Count(if even { 40 + position } else { 50 + position }),
            if even { a.clone() } else { b.clone() },
            Position(u32::try_from(position).unwrap_or(0)),
            session.clone(),
        )
    }))
}

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

#[test]
fn a_pairing_survives_a_round_trip_through_the_journal() {
    let scratch = Scratch::new();
    let written = a_session();

    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        let body = encode::trials(&written, &conditions(), |Count(n)| {
            i64::try_from(n).unwrap_or(i64::MAX)
        });
        journal
            .append(&Entry::new(EntryKind::Trials, AT, body))
            .expect("the trials append");
    }

    // Everything in memory is gone; what follows uses only the file.
    drop(written);

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert!(replayed.is_complete(), "{}", replayed.statement());
    let entry = replayed.entries.first().expect("one entry");
    assert_eq!(entry.kind(), EntryKind::Trials);

    let rebuilt = read_trials(entry.body());
    assert_eq!(rebuilt.all().len(), 8);
    assert!(rebuilt.is_balanced());

    // §3.27's question, asked of what came out of the file.
    let paired = rebuilt.paired_with(&Arm::new("Q4_K_M"), &Arm::new("Q5_K_M"));
    assert_eq!(paired.len(), 4);
    assert_eq!(paired.unpaired, 0);
    assert_eq!(paired.left_was_smaller(), [true, true, true, true]);

    // And the interleaving order survived, so drift over the session is still
    // visible in the record rather than only in the process that wrote it.
    let positions: Vec<(u32, u32)> = paired
        .pairs
        .iter()
        .map(|(l, r)| (l.position().0, r.position().0))
        .collect();
    assert_eq!(positions, [(0, 1), (2, 3), (4, 5), (6, 7)]);
}

/// The conditions travel with the session, so a reader can tell whether two
/// sessions are comparable at all (A6, A8).
#[test]
fn the_conditions_are_in_the_record_beside_the_trials() {
    let body = encode::trials(&a_session(), &conditions(), |Count(n)| {
        i64::try_from(n).unwrap_or(i64::MAX)
    });
    let recorded = body.get("conditions").expect("conditions are recorded");
    assert!(
        recorded.get("mcf").is_some(),
        "the instrument is not recorded"
    );
    assert_eq!(
        recorded.get("thermal_state"),
        Some(&Value::Null),
        "an unread condition is not null"
    );
}

/// Rebuilds trials from a recorded body, using nothing but the file's contents.
fn read_trials(body: &Value) -> Trials<Count> {
    let rows = body
        .get("trials")
        .and_then(Value::as_list)
        .expect("the body holds trials");
    Trials::from(rows.iter().map(|row| {
        let value = row
            .get("value")
            .and_then(Value::as_integer)
            .expect("a value");
        let arm = row.get("arm").and_then(Value::as_text).expect("an arm");
        let position = row
            .get("position")
            .and_then(Value::as_integer)
            .expect("a position");
        let session = row
            .get("session")
            .and_then(Value::as_text)
            .expect("a session");
        Trial::new(
            Count(u64::try_from(value).unwrap_or(0)),
            Arm::new(arm),
            Position(u32::try_from(position).unwrap_or(0)),
            SessionId::new(session),
        )
    }))
}
