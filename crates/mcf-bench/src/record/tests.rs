//! A null result reaches the record as a result (A9, B-086).
//!
//! **The clock here is [`Monotonic`], and it has to be.** B-082 bounds the
//! encoder by `Measurable`, which the laboratory's clock does not implement, so
//! `record::comparison` of a simulated comparison does not compile. The
//! durations below are stated rather than read — `Duration::from_nanos` is how
//! a test states a known interval — and that is the documented seam: the type
//! stops a *simulated* timing being written, and a test that fabricates a
//! monotonic one is fabricating it on purpose and in one place.
//!
//! [`Monotonic`]: mcf_core::time::Monotonic

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{ConditionValue, Conditions, Floor, PartsPerMillion};
use mcf_core::time::{Duration, Monotonic};
use mcf_core::trial::{Arm, SessionId};
use mcf_record::json::Value;

use crate::compare::{Comparison, Discipline, Interleaving, Method, UnderTest};
use crate::warmth::Warmth;

const FIVE: PartsPerMillion = PartsPerMillion(50_000);
const SECOND: u64 = 1_000_000_000;

/// What these comparisons were asked to do.
///
/// Stated rather than defaulted: §II asks that somebody else be able to repeat
/// a measurement, and a `Method` a fixture left blank would be a fixture
/// asserting that a blank one is enough (PR2, B30).
fn asked() -> Method {
    Method {
        prompt: "Once upon a time".to_owned(),
        resolving: FIVE,
        ceiling: 200,
        engine: Some("provisioned".to_owned()),
        cold: true,
    }
}

fn known(value: &str) -> Attested<ConditionValue> {
    Attested::Known(ConditionValue::text(value))
}

fn everything_known() -> Floor {
    Floor {
        hardware_state: known("this machine"),
        thermal_state: known("steady"),
        driver_versions: known("none"),
        runtime_versions: known("stand-in 0.1"),
        quantization: known("q8_0"),
        context_length: Attested::Known(ConditionValue::integer(2048)),
        batch_shape: Attested::Known(ConditionValue::integer(1)),
        mcf_configuration: known("default"),
        realized_placement: known("host"),
        instrumentation: known("recording"),
        artifact_storage: known("tmpfs"),
        seed_set: known("none: seed 0 held still, 128 token(s) pinned"),
        reuse: known("warm: the model was already resident for every trial"),
    }
}

fn under_test(name: &str, differing: &[(&str, &str)]) -> UnderTest {
    let mut floor = everything_known();
    for (question, value) in differing {
        match *question {
            "quantization" => floor.quantization = known(value),
            "thermal_state" => floor.thermal_state = known(value),
            other => panic!("this helper does not set {other}"),
        }
    }
    UnderTest::new(
        Arm::new(name),
        Conditions::new(BuildIdentity::current(), floor),
    )
}

/// The partiality reaches the record, so a reader six weeks later can tell a
/// short run that was interrupted from a short run that decided quickly
/// (A4, B-087, B-086).
#[test]
fn an_interruption_reaches_the_record() {
    let mut running = Interleaving::<Monotonic>::new(
        under_test("q8_0", &[]),
        under_test("q2_k", &[("quantization", "q2_k")]),
        SessionId::new("s"),
        7,
        Discipline::Timing {
            seed: 0,
            tokens: 128,
        },
    );
    for _ in 0..3 {
        let _ran = running.round(|_arm, _drew| Some((Duration::from_nanos(SECOND), Warmth::Warm)));
    }
    running.stopped_short("the daemon stopped answering");
    let held = running.finish();

    let body = super::comparison(&held, &held.finding(FIVE), &asked());
    assert_eq!(
        body.get("cut_short").and_then(Value::as_text),
        Some("the daemon stopped answering")
    );
    assert_eq!(
        body.get("pairs")
            .and_then(Value::as_list)
            .map(<[Value]>::len),
        Some(3),
        "and the pairs it did take are all there: nine of ten is nine data points (A4)"
    );
}

/// A run that finished writes `null` there, which is *it finished* rather than
/// an interruption nobody recorded.
#[test]
fn a_finished_run_records_no_interruption() {
    let held = run(
        under_test("q8_0", &[]),
        under_test("q2_k", &[("quantization", "q2_k")]),
        SECOND,
        SECOND,
        4,
    );
    assert_eq!(
        super::comparison(&held, &held.finding(FIVE), &asked()).get("cut_short"),
        Some(&Value::Null)
    );
}

/// Runs `rounds` pairs where the left arm takes `left_ns` and the right
/// `right_ns`.
fn run(
    left: UnderTest,
    right: UnderTest,
    left_ns: u64,
    right_ns: u64,
    rounds: usize,
) -> Comparison<Monotonic> {
    let named = left.arm().clone();
    let mut running = Interleaving::<Monotonic>::new(
        left,
        right,
        SessionId::new("s"),
        5,
        Discipline::Timing {
            seed: 0,
            tokens: 128,
        },
    );
    for _ in 0..rounds {
        let _ran = running.round(|arm, _drew| {
            Some((
                Duration::from_nanos(if *arm == named { left_ns } else { right_ns }),
                // Stated rather than defaulted: §6.13 makes what a trial reused
                // a condition, and a fixture that let it be inferred would be
                // testing the inference.
                Warmth::Warm,
            ))
        });
    }
    running.finish()
}

fn text(body: &Value, path: &[&str]) -> String {
    let mut held = body;
    for step in path {
        held = held.get(step).unwrap_or_else(|| {
            panic!("{path:?} is present in {body:?}");
        });
    }
    held.as_text()
        .unwrap_or_else(|| panic!("{path:?} is text in {body:?}"))
        .to_owned()
}

/// **A9's condition.** A comparison that found no difference is written as an
/// outcome named `same`, carrying the resolution it would have seen — not as a
/// failure, and not as an absence.
#[test]
fn a_null_result_is_written_as_a_result() {
    let held = run(
        under_test("q8_0", &[]),
        under_test("q2_k", &[("quantization", "q2_k")]),
        SECOND,
        SECOND,
        40,
    );
    let finding = held.finding(FIVE);
    let body = super::comparison(&held, &finding, &asked());

    assert_eq!(text(&body, &["outcome", "kind"]), "same");
    assert_eq!(
        body.get("outcome").and_then(|held| held.get("resolution")),
        Some(&Value::Integer(50_000)),
        "the size that would have shown travels with the null result (B-086)"
    );
    assert_eq!(text(&body, &["isolation", "kind"]), "isolated");
}

/// The paired differences are written out, because a comparison whose
/// distribution was thrown away is a question nobody can re-ask (B56, D16).
#[test]
fn the_distribution_is_written_and_not_only_the_verdict() {
    let held = run(
        under_test("q8_0", &[]),
        under_test("q2_k", &[("quantization", "q2_k")]),
        SECOND.saturating_mul(2),
        SECOND,
        12,
    );
    let body = super::comparison(&held, &held.finding(FIVE), &asked());
    let pairs = body
        .get("pairs")
        .and_then(Value::as_list)
        .expect("the pairs are a list");
    assert_eq!(pairs.len(), 12);
    for pair in pairs {
        assert_eq!(text(pair, &["difference", "kind"]), "quicker");
        assert_eq!(text(pair, &["difference", "quicker"]), "right");
        assert!(
            matches!(
                pair.get("left_ns"),
                Some(&Value::Integer(n)) if n == 2_000_000_000
            ),
            "the raw duration is kept, not only the ratio: {pair:?}"
        );
        assert!(
            pair.get("first").is_some(),
            "which arm ran first is a fact about the pair"
        );
    }
}

/// A confounded comparison is written as the outcome it is, with no delta and
/// with every variable that differed (A8, A9).
#[test]
fn a_refused_comparison_is_written_as_an_outcome() {
    let held = run(
        under_test("q8_0", &[]),
        under_test(
            "q2_k",
            &[("quantization", "q2_k"), ("thermal_state", "hot")],
        ),
        SECOND.saturating_mul(2),
        SECOND,
        12,
    );
    let body = super::comparison(&held, &held.finding(FIVE), &asked());
    assert_eq!(text(&body, &["outcome", "kind"]), "not_comparable");
    assert_eq!(
        body.get("outcome").and_then(|held| held.get("difference")),
        Some(&Value::Null),
        "a confounded comparison has no delta, in the record as in the type"
    );
    assert_eq!(text(&body, &["isolation", "kind"]), "confounded");
    let differ = body
        .get("isolation")
        .and_then(|held| held.get("differ"))
        .and_then(Value::as_list)
        .expect("the variables that differed are a list");
    assert_eq!(differ.len(), 2);
}

/// A declared confound travels with the reason it was declared (A8).
#[test]
fn a_declared_confound_is_written_with_its_reason() {
    let held = run(
        under_test("q8_0", &[]),
        under_test(
            "q2_k",
            &[("quantization", "q2_k"), ("thermal_state", "hot")],
        ),
        SECOND.saturating_mul(2),
        SECOND,
        12,
    )
    .declaring("the second arm could only be run once the machine was warm");
    let body = super::comparison(&held, &held.finding(FIVE), &asked());
    assert_eq!(text(&body, &["outcome", "kind"]), "differ");
    assert_eq!(
        text(&body, &["declared_confound"]),
        "the second arm could only be run once the machine was warm"
    );
}

/// The conditions of both arms are written, so that a later reader can ask the
/// isolation question again rather than trusting this run's answer to it.
#[test]
fn both_arms_conditions_are_written() {
    let held = run(
        under_test("q8_0", &[]),
        under_test("q2_k", &[("quantization", "q2_k")]),
        SECOND,
        SECOND,
        4,
    );
    let body = super::comparison(&held, &held.finding(FIVE), &asked());
    assert_eq!(text(&body, &["left", "conditions", "quantization"]), "q8_0");
    assert_eq!(
        text(&body, &["right", "conditions", "quantization"]),
        "q2_k"
    );
    assert_eq!(text(&body, &["left", "arm"]), "q8_0");
}
