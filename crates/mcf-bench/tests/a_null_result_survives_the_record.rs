#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

use mcf_bench::compare::{Discipline, Interleaving, MachineHeld, Method, UnderTest};
use mcf_bench::record;
use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{ConditionValue, Conditions, Floor, PartsPerMillion};
use mcf_core::time::{Duration, Monotonic, Timestamp};
use mcf_core::trial::{Arm, SessionId};
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::Value;

const AT: Timestamp = Timestamp::from_utc_nanos(1_756_058_651_442_000_000, Attested::Unknown);
const FIVE: PartsPerMillion = PartsPerMillion(50_000);
const SECOND: u64 = 1_000_000_000;

fn watched() -> MachineHeld {
    MachineHeld {
        before: 4_000,
        after: 4_200,
        steady_before: Some(20_000),
        steady_after: Some(30_000),
    }
}

fn asked() -> Method {
    Method {
        workload: mcf_core::contribution::Workload::Declared,
        prompt: "Once upon a time".to_owned(),
        resolving: FIVE,
        ceiling: 200,
        engine: Some("provisioned".to_owned()),
        cold: true,
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-null-{name}-{}", std::process::id()));
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

fn under_test(name: &str, quantization: &str) -> UnderTest {
    let floor = Floor {
        hardware_state: Attested::Known(ConditionValue::text("this machine")),
        thermal_state: Attested::Known(ConditionValue::text("steady")),
        driver_versions: Attested::Known(ConditionValue::text("none")),
        runtime_versions: Attested::Known(ConditionValue::text("stand-in 0.1")),
        quantization: Attested::Known(ConditionValue::text(quantization)),
        context_length: Attested::Known(ConditionValue::integer(2048)),
        batch_shape: Attested::Known(ConditionValue::integer(1)),
        mcf_configuration: Attested::Known(ConditionValue::text("default")),
        realized_placement: Attested::Known(ConditionValue::text("host")),
        instrumentation: Attested::Known(ConditionValue::text("recording")),
        artifact_storage: Attested::Known(ConditionValue::text("tmpfs")),
        seed_set: Attested::Unknown,
        reuse: Attested::Unknown,
    };
    UnderTest::new(
        Arm::new(name),
        Conditions::new(BuildIdentity::current(), floor),
    )
}

fn a_null_comparison() -> (Value, usize) {
    let named = Arm::new("q8_0");
    let mut running = Interleaving::<Monotonic>::new(
        under_test("q8_0", "q8_0"),
        under_test("q2_k", "q2_k"),
        SessionId::new("2026-08-27T09-00-00Z"),
        21,
        Discipline::Timing {
            seed: 0,
            tokens: 128,
        },
    );
    for round in 0..40_u64 {
        let _ran = running.round(|arm, _drew| {
            let wobble = round.wrapping_rem(7).wrapping_mul(3_000_000);
            let slower = (*arm == named) == round.is_multiple_of(2);
            let own = if slower { 1_000_000 } else { 0 };
            Some((
                Duration::from_nanos(SECOND.saturating_add(wobble).saturating_add(own)),
                mcf_bench::warmth::Warmth::Warm,
            ))
        });
    }
    let held = running.finish();
    let finding = held.finding(FIVE);
    let body = record::comparison(&held, &finding, &asked(), Some(&watched()));
    (body, held.pairs().len())
}

#[test]
fn a_null_result_is_written_and_read_back_as_a_result() {
    let scratch = Scratch::new("same");
    let (body, pairs) = a_null_comparison();
    assert_eq!(pairs, 40);

    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        journal
            .append(&Entry::new(EntryKind::Comparison, AT, body))
            .expect("the comparison appends");
    }

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    assert!(replayed.is_complete(), "{}", replayed.statement());
    let entry = replayed.entries.first().expect("one entry");

    assert_eq!(entry.kind(), EntryKind::Comparison);
    assert_ne!(entry.kind(), EntryKind::Failure);

    let read = entry.body();
    let outcome = read.get("outcome").expect("the outcome is recorded");
    assert_eq!(
        outcome.get("kind").and_then(Value::as_text),
        Some("same"),
        "the outcome names itself: {read:?}"
    );
    assert_eq!(
        outcome.get("resolution").and_then(Value::as_integer),
        Some(50_000),
        "and carries the size that would have shown, so *no difference* never \
         reads as *we stopped looking*"
    );
    assert_eq!(
        outcome.get("pairs").and_then(Value::as_integer),
        Some(40),
        "and what it cost"
    );
}

#[test]
fn the_distribution_survives_so_the_verdict_can_be_re_asked() {
    let scratch = Scratch::new("pairs");
    let (body, _) = a_null_comparison();

    {
        let mut journal = Journal::open(&scratch.journal()).expect("a journal opens");
        journal
            .append(&Entry::new(EntryKind::Comparison, AT, body))
            .expect("the comparison appends");
    }

    let replayed = replay(&scratch.journal()).expect("the journal replays");
    let read = replayed.entries.first().expect("one entry").body();
    let pairs = read
        .get("pairs")
        .and_then(Value::as_list)
        .expect("the pairs are on the disk");
    assert_eq!(pairs.len(), 40);
    for pair in pairs {
        assert!(
            pair.get("left_ns").and_then(Value::as_integer).is_some()
                && pair.get("right_ns").and_then(Value::as_integer).is_some(),
            "both raw durations are kept, not only the ratio: {pair:?}"
        );
        assert!(
            pair.get("first").and_then(Value::as_text).is_some(),
            "and which arm ran first, which is what makes an order effect visible"
        );
    }

    for side in ["left", "right"] {
        assert!(
            read.get(side)
                .and_then(|held| held.get("conditions"))
                .and_then(|held| held.get("quantization"))
                .is_some(),
            "{side} arm's conditions are on the disk"
        );
    }
}
