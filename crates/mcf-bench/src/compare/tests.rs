//! What the construction has to refuse, and what it has to find.
//!
//! The clock here is [`Simulated`], because these are questions about the
//! *shape* of a comparison rather than about how long anything took, and A11
//! keeps the two kinds of duration apart in the type system. A test that
//! reached for the monotonic clock would be measuring the test machine.

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{ConditionValue, Conditions, Floor, Isolation, PartsPerMillion};
use mcf_core::time::{Duration, Simulated};
use mcf_core::trial::{Arm, Position, SessionId, Trial, Trials};

use mcf_core::trial::{Draw, SeedSet};

use super::{
    Comparison, Difference, Discipline, Interleaving, NotComparable, Side, Strength, UnderTest,
    Withheld,
};
use crate::enough::Verdict;
use crate::warmth::{Reuse, Warmth};

/// Five percent, as this module spells it.
const FIVE: PartsPerMillion = PartsPerMillion(50_000);
/// Twenty percent.
const TWENTY: PartsPerMillion = PartsPerMillion(200_000);

/// A second, and a second and a half, in nanoseconds.
const SECOND: u64 = 1_000_000_000;
const SLOWED: u64 = 1_500_000_000;

fn arms() -> (Arm, Arm) {
    (Arm::new("q8_0"), Arm::new("q2_k"))
}

fn session() -> SessionId {
    SessionId::new("2026-08-27T09-00-00Z")
}

fn ns(nanos: u64) -> Duration<Simulated> {
    Duration::from_nanos(nanos)
}

/// A run of a stated length that found the model already loaded.
///
/// Every fixture here is a warm run, stated rather than defaulted: §6.13 makes
/// what a trial reused a condition, and a test that let it be inferred would
/// be testing the inference. The `Option` is the runner's shape and is always
/// `Some` here — a run that did not happen is a separate fixture, because it
/// is a separate thing (A4).
#[expect(
    clippy::unnecessary_wraps,
    reason = "the runner's closure returns None for a run that did not happen, and this is the \
              one that did"
)]
fn warm(nanos: u64) -> Option<(Duration<Simulated>, Warmth)> {
    Some((ns(nanos), Warmth::Warm))
}

/// A floor in which every condition is known, so that a test changes exactly
/// what it means to change.
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

fn known(value: &str) -> Attested<ConditionValue> {
    Attested::Known(ConditionValue::text(value))
}

/// An arm as a configuration, optionally differing in one named condition.
fn under_test(arm: &Arm, differing: &[(&str, &str)]) -> UnderTest {
    let mut floor = everything_known();
    for (question, value) in differing {
        match *question {
            "quantization" => floor.quantization = known(value),
            "thermal_state" => floor.thermal_state = known(value),
            "context_length" => floor.context_length = known(value),
            other => panic!("this helper does not set {other}"),
        }
    }
    UnderTest::new(
        arm.clone(),
        Conditions::new(BuildIdentity::current(), floor),
    )
}

/// The two arms as configurations differing in exactly one condition, which is
/// the only shape A8 admits a delta from. Most tests here are about the
/// *pairing* rather than about isolation, so they take this and say no more.
fn isolated(left: &Arm, right: &Arm) -> (UnderTest, UnderTest) {
    (
        under_test(left, &[]),
        under_test(right, &[("quantization", "q2_k")]),
    )
}

/// A comparison of two arms read back out of one session's trials, isolated.
fn from_trials(
    trials: &Trials<Duration<Simulated>>,
    left: &Arm,
    right: &Arm,
) -> Result<Comparison<Simulated>, NotComparable> {
    let (one, other) = isolated(left, right);
    Comparison::from_trials(trials, &one, &other)
}

/// A cross-session comparison of two arms, isolated.
fn from_separate_sessions(
    left_trials: &Trials<Duration<Simulated>>,
    left: &Arm,
    right_trials: &Trials<Duration<Simulated>>,
    right: &Arm,
) -> Result<Comparison<Simulated>, NotComparable> {
    let (one, other) = isolated(left, right);
    Comparison::from_separate_sessions(left_trials, &one, right_trials, &other)
}

/// A runner over two isolated arms.
fn interleaving(left: &Arm, right: &Arm, seed: u64) -> Interleaving<Simulated> {
    let (one, other) = isolated(left, right);
    Interleaving::new(
        one,
        other,
        session(),
        seed,
        Discipline::Timing {
            seed: 0,
            tokens: 128,
        },
    )
}

/// Trials laid out in a stated order of arms, positions counting up from zero.
fn laid_out(order: &[&Arm], values: &[u64], session: &SessionId) -> Trials<Duration<Simulated>> {
    Trials::from(order.iter().enumerate().map(|(at, arm)| {
        Trial::new(
            ns(values.get(at).copied().unwrap_or(0)),
            (*arm).clone(),
            Position(u32::try_from(at).unwrap_or(0)),
            session.clone(),
            // A timing run, which is what these are: the seed is held still and
            // the length pinned (D19). Both runs of a pair draw the same thing,
            // so this does not vary with the position.
            timing(),
        )
    }))
}

/// The timing discipline these tests take their trials under.
fn timing() -> Draw {
    Draw::LengthPinned {
        seed: 0,
        tokens: 128,
    }
}

/// **The finding B-250 exists for.** Six of A then six of B, recorded in one
/// session, is refused by name — the positions say the arms did not alternate,
/// so anything that drifted between the two blocks is inside the difference.
#[test]
fn block_then_subtract_is_refused_by_name() {
    let (left, right) = arms();
    let order: Vec<&Arm> = std::iter::repeat_n(&left, 6)
        .chain(std::iter::repeat_n(&right, 6))
        .collect();
    let values: Vec<u64> = (0..12)
        .map(|at| SECOND.saturating_add(at * 1_000_000))
        .collect();
    let trials = laid_out(&order, &values, &session());
    match from_trials(&trials, &left, &right) {
        Err(NotComparable::RanInBlocks { arm, at }) => {
            assert_eq!(arm, left);
            assert_eq!(at, Position(0), "the first couple is where it shows");
        }
        other => panic!("blocked trials were accepted as a comparison: {other:?}"),
    }
}

/// Interleaved trials in one session are a comparison, and the pairing comes
/// back in interleaving order.
#[test]
fn interleaved_trials_are_a_comparison() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..12)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let values: Vec<u64> = (0..12)
        .map(|at| {
            if at % 2 == 0 {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            }
        })
        .collect();
    let trials = laid_out(&order, &values, &session());
    let held = from_trials(&trials, &left, &right).expect("interleaved trials pair");
    assert_eq!(held.pairs().len(), 6);
    assert!(held.strength().is_paired());
    assert_eq!(held.trials_per_arm(), (6, 6));
    let differences = held
        .paired_differences()
        .expect("a paired comparison has them");
    assert_eq!(differences.len(), 6);
    assert!(
        differences.iter().all(|held| matches!(
            held,
            Difference::Quicker {
                side: Side::Right,
                ..
            }
        )),
        "the right arm took half as long in every pair: {differences:?}"
    );
}

/// Order randomization within a pair does not stop the pairing being read back
/// — *B, A, A, B* is two pairs, not a defect.
#[test]
fn a_randomized_order_within_pairs_still_pairs() {
    let (left, right) = arms();
    let order = [&right, &left, &left, &right, &right, &left];
    let doubled = SECOND.saturating_mul(2);
    let trials = laid_out(
        &order,
        &[SECOND, doubled, doubled, SECOND, SECOND, doubled],
        &session(),
    );
    let held = from_trials(&trials, &left, &right).expect("three pairs");
    assert_eq!(held.pairs().len(), 3);
    assert_eq!(
        held.order_balance(),
        (1, 2),
        "one pair ran the left arm first and two ran the right"
    );
    assert!(
        held.paired_differences()
            .expect("paired")
            .iter()
            .all(|d| matches!(
                d,
                Difference::Quicker {
                    side: Side::Right,
                    ..
                }
            )),
        "the right arm is the quicker one in every pair whichever ran first"
    );
}

/// Arms of unequal length are refused rather than truncated: a caller who ran
/// one arm more than the other has not run a paired trial, and evening it up
/// quietly would produce the shape this module exists to refuse.
#[test]
fn unequal_arms_are_reported_not_truncated() {
    let (left, right) = arms();
    let order = [&left, &right, &left, &right, &left];
    let trials = laid_out(&order, &[10, 20, 10, 20, 10], &session());
    assert_eq!(
        from_trials(&trials, &left, &right),
        Err(NotComparable::Unbalanced { left: 3, right: 2 })
    );
}

/// One pair decides nothing, so it is not a comparison.
#[test]
fn a_single_pair_is_not_a_comparison() {
    let (left, right) = arms();
    let trials = laid_out(&[&left, &right], &[10, 20], &session());
    assert_eq!(
        from_trials(&trials, &left, &right),
        Err(NotComparable::TooFew { have: 1 })
    );
}

/// Trials from two sessions in one set are not paired, and the refusal names
/// the construction that does admit them.
#[test]
fn trials_from_two_sessions_are_not_paired() {
    let (left, right) = arms();
    let one = session();
    let other = SessionId::new("2026-08-27T14-00-00Z");
    let trials = Trials::from([
        Trial::new(ns(10), left.clone(), Position(0), one.clone(), timing()),
        Trial::new(ns(20), right.clone(), Position(1), other, timing()),
        Trial::new(ns(10), left.clone(), Position(2), one.clone(), timing()),
        Trial::new(ns(20), right.clone(), Position(3), one, timing()),
    ]);
    assert!(matches!(
        from_trials(&trials, &left, &right),
        Err(NotComparable::SeveralSessions { .. })
    ));
}

/// §3.27's *it may be all that exists*: two sessions can be put side by side,
/// and the result says so and has no paired difference to report.
#[test]
fn a_cross_session_comparison_is_constructible_and_weaker() {
    let (left, right) = arms();
    let morning = SessionId::new("2026-08-27T09-00-00Z");
    let afternoon = SessionId::new("2026-08-27T14-00-00Z");
    let one = laid_out(
        &[&left, &left, &left, &left],
        &[SECOND.saturating_mul(2); 4],
        &morning,
    );
    let other = laid_out(&[&right, &right, &right, &right], &[SECOND; 4], &afternoon);
    let held = from_separate_sessions(&one, &left, &other, &right)
        .expect("two sessions may be put side by side");
    assert!(!held.strength().is_paired());
    assert!(
        held.paired_differences().is_none(),
        "there is no pairing, so there is no paired difference"
    );
    assert_eq!(held.pairs().len(), 0);
    let finding = held.finding(FIVE);
    assert!(matches!(finding.strength(), Strength::Assembled { .. }));
    assert!(
        format!("{finding}").contains("weaker claim"),
        "the weakness is printed, not merely stored: {finding}"
    );
}

/// The weaker construction is refused where the stronger one is available:
/// two arms from one session are paired, and choosing not to pair them would
/// be discarding evidence.
#[test]
fn one_session_may_not_be_weakened_on_purpose() {
    let (left, right) = arms();
    let held = session();
    let one = laid_out(
        &[&left, &left, &left, &left],
        &[SECOND.saturating_mul(2); 4],
        &held,
    );
    let other = laid_out(&[&right, &right, &right, &right], &[SECOND; 4], &held);
    assert_eq!(
        from_separate_sessions(&one, &left, &other, &right),
        Err(NotComparable::OneSession { session: held })
    );
}

/// The runner alternates the arms and randomizes which goes first, and the
/// order it drew is legible afterwards.
#[test]
fn the_runner_interleaves_and_randomizes() {
    let (left, right) = arms();
    let mut order = Vec::new();
    let mut running = interleaving(&left, &right, 7);
    for _ in 0..40 {
        let _ran = running.round(|arm, _drew| {
            order.push(arm.clone());
            warm(if *arm == left {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            })
        });
    }
    let held = running.finish();
    assert_eq!(held.pairs().len(), 40);
    let (left_first, right_first) = held.order_balance();
    assert_eq!(left_first.saturating_add(right_first), 40);
    assert!(
        left_first > 8 && right_first > 8,
        "order is randomized, not fixed: {left_first} left-first and {right_first} right-first"
    );
    for couple in order.as_chunks::<2>().0 {
        assert_ne!(
            couple.first(),
            couple.last(),
            "the two runs of a pair are of different arms"
        );
    }
}

/// The runner replays: the same seed draws the same order, because a
/// comparison whose order came from the time of day is one more thing that
/// differs between two sittings (§3.12).
#[test]
fn the_same_seed_draws_the_same_order() {
    let (left, right) = arms();
    let mut orders = Vec::new();
    for _ in 0..2 {
        let mut seen = Vec::new();
        let mut running = interleaving(&left, &right, 99);
        for _ in 0..12 {
            let _ran = running.round(|arm, _drew| {
                seen.push(arm.clone());
                warm(SECOND)
            });
        }
        orders.push(seen);
    }
    assert_eq!(orders.first(), orders.last());
}

/// A real difference is found through the paired path, and the count it took
/// is part of the answer.
#[test]
fn the_runner_finds_a_real_difference_and_says_what_it_cost() {
    let (left, right) = arms();
    // A wobble that lands on both arms of a pair — which is what pairing is
    // for — plus a true thirty-percent gap between the arms.
    let mut round = 0_u64;
    let mut running = interleaving(&left, &right, 3);
    let mut finding = running.finding(FIVE);
    while matches!(finding.verdict(), Some(Verdict::NotYet { .. })) && round < 200 {
        let drift = round
            .wrapping_mul(37)
            .wrapping_rem(11)
            .wrapping_mul(5_000_000);
        let _ran = running.round(|arm, _drew| {
            warm(if *arm == left {
                1_300_000_000_u64.saturating_add(drift)
            } else {
                SECOND.saturating_add(drift)
            })
        });
        round = round.saturating_add(1);
        finding = running.finding(FIVE);
    }
    let Some(Verdict::Differ { by, after, .. }) = finding.verdict() else {
        panic!("a thirty-percent difference was never found: {finding}");
    };
    assert!(
        (250_000..350_000).contains(&by.low.0),
        "the paired difference is about thirty percent: {by:?}"
    );
    assert_eq!(
        *after,
        usize::try_from(round).unwrap_or(0),
        "the count is part of the answer"
    );
    assert!(finding.strength().is_paired());
}

/// **The experiment B-250 rests on.** One machine, forty-eight consecutive
/// runs, something else starting halfway through. The two arms are identical
/// — neither is faster at any moment — and the *same forty-eight timings* are
/// arranged two ways.
///
/// Interleaved, the drift lands on both runs of every pair and the comparison
/// reports the truth: no difference. Blocked, it reports a fifty-percent
/// difference between two things that are the same, at one chance in a
/// thousand of being noise. Only the arrangement differs, and it is the whole
/// of the answer.
#[test]
fn a_drift_invents_a_difference_in_blocks_and_cancels_in_pairs() {
    let (left, right) = arms();
    // What a run at position `at` took. Three ingredients, and only the first
    // is about the arms: the machine's level, which steps up halfway through;
    // a wobble belonging to the *round* — the moment a pair shares — and a
    // small per-run jitter that does not.
    //
    // The jitter's cycle is four long, so it contributes equally to the even
    // and odd positions and cannot masquerade as a difference between the arms
    // in the interleaved arrangement. A fixture whose noise is confounded with
    // the thing under test measures the fixture, which the first draft of this
    // test did.
    let took = |at: usize| {
        let level = if at < 24 { SECOND } else { SLOWED };
        let rounds: [i64; 5] = [0, 3, -3, 1, -2];
        let jitters: [i64; 4] = [1, -1, -1, 1];
        let round = at.wrapping_div(2);
        let wobble = rounds.get(round % rounds.len()).copied().unwrap_or(0);
        let jitter = jitters.get(at % jitters.len()).copied().unwrap_or(0);
        let thousandths = wobble
            .saturating_mul(10)
            .saturating_add(jitter.saturating_mul(5));
        let off = i128::from(level)
            .saturating_mul(i128::from(thousandths))
            .wrapping_div(1000);
        u64::try_from(i128::from(level).saturating_add(off)).unwrap_or(level)
    };
    let machine: Vec<u64> = (0_usize..48).map(took).collect();

    // Interleaved: the two runs of a pair are adjacent, so they see the same
    // machine.
    let paired_order: Vec<&Arm> = (0..48)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let paired = from_trials(
        &laid_out(&paired_order, &machine, &session()),
        &left,
        &right,
    )
    .expect("interleaved");
    let honest = paired.finding(TWENTY);
    println!("  interleaved: {honest}");
    assert!(
        matches!(honest.verdict(), Some(Verdict::Same { .. })),
        "the paired comparison reports no difference, which is the truth: {honest}"
    );

    // Blocked: the same forty-eight timings, all of one arm and then all of
    // the other.
    let (Some(blocked_left), Some(blocked_right)) = (machine.get(..24), machine.get(24..)) else {
        panic!("forty-eight timings split in half");
    };
    let morning = SessionId::new("blocks-first-half");
    let afternoon = SessionId::new("blocks-second-half");
    let assembled = from_separate_sessions(
        &laid_out(&[&left; 24], blocked_left, &morning),
        &left,
        &laid_out(&[&right; 24], blocked_right, &afternoon),
        &right,
    )
    .expect("two sessions may be put side by side");
    let invented = assembled.finding(TWENTY);
    println!("  blocked:     {invented}");
    // `Apart` rather than `Differ`: never paired, so the size is a point with
    // no interval behind it (F92, B53). The invention is still visible, which
    // is the whole demonstration.
    let Some(Verdict::Apart { by, by_chance, .. }) = invented.verdict() else {
        panic!("the blocked arrangement of the very same timings invents no difference: {invented}")
    };
    assert!(
        (450_000..550_000).contains(&by.low.0),
        "the difference it invents is the drift, about fifty percent: {invented}"
    );
    assert!(
        by_chance.0 < 10_000,
        "and it is confident about it, which is the danger: {invented}"
    );
}

/// **A8's condition, and B-085's done-when.** A comparison in which two
/// conditions differ has no delta to give: the verdict is `None` and the
/// rendering says *these are not comparable* instead of a number.
#[test]
fn a_confounded_comparison_reports_no_delta() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..12)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let values: Vec<u64> = (0..12)
        .map(|at| {
            if at % 2 == 0 {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            }
        })
        .collect();
    let trials = laid_out(&order, &values, &session());

    // Two variables moved: the quantization under test, and the machine's
    // thermal state — which is A8's own example of the violation.
    let one = under_test(&left, &[]);
    let other = under_test(
        &right,
        &[("quantization", "q2_k"), ("thermal_state", "hot")],
    );
    let held = Comparison::from_trials(&trials, &one, &other).expect("the trials are still paired");

    assert!(held.isolation().is_confounded());
    let finding = held.finding(FIVE);
    assert!(
        finding.verdict().is_none(),
        "a doubling is plainly there arithmetically, and it says nothing: {finding}"
    );
    let rendered = format!("{finding}");
    assert!(
        rendered.contains("not comparable") && rendered.contains("thermal_state"),
        "the refusal names what differed: {rendered}"
    );
    assert!(
        !rendered.contains("they differ by"),
        "no delta may reach a reader from a confounded comparison: {rendered}"
    );
}

/// A confound the operator declares is science (A8): the delta comes back,
/// with the declaration and every variable that differed beside it.
#[test]
fn a_declared_confound_is_reported_with_its_declaration() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..12)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let values: Vec<u64> = (0..12)
        .map(|at| {
            if at % 2 == 0 {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            }
        })
        .collect();
    let trials = laid_out(&order, &values, &session());
    let one = under_test(&left, &[]);
    let other = under_test(
        &right,
        &[("quantization", "q2_k"), ("context_length", "4096")],
    );
    let held = Comparison::from_trials(&trials, &one, &other)
        .expect("paired")
        .declaring("a smaller quantization is the only way to reach this context here");

    let finding = held.finding(FIVE);
    assert!(
        finding.verdict().is_some(),
        "a declared confound is science, not a refusal: {finding}"
    );
    let rendered = format!("{finding}");
    for expected in [
        "confound declared",
        "only way to reach this context",
        "quantization",
        "context_length",
    ] {
        assert!(
            rendered.contains(expected),
            "a declared confound travels with what it declared — no `{expected}` in: {rendered}"
        );
    }
}

/// An isolated comparison says which variable it was about, and the rendering
/// does not repeat itself about it.
#[test]
fn an_isolated_comparison_names_its_variable() {
    let (left, right) = arms();
    let mut running = interleaving(&left, &right, 11);
    for _ in 0..30 {
        let _ran = running.round(|arm, _drew| {
            warm(if *arm == left {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            })
        });
    }
    let held = running.finish();
    assert_eq!(
        held.isolation(),
        Isolation::Isolated {
            variable: "quantization"
        }
    );
    let finding = held.finding(FIVE);
    assert!(finding.verdict().is_some());
    assert_eq!(finding.isolation().differing(), ["quantization"]);
}

/// **A7 applied here.** A comparison whose conditions MCF has not read is
/// neither confounded nor isolated: the delta is reported, and so is the fact
/// that nobody established what it is a delta *of*.
#[test]
fn unread_conditions_report_the_delta_and_the_doubt() {
    let (left, right) = arms();
    let unread = |arm: &Arm| {
        UnderTest::new(
            arm.clone(),
            Conditions::new(BuildIdentity::current(), Floor::nothing_known()),
        )
    };
    let mut running = Interleaving::<Simulated>::new(
        unread(&left),
        unread(&right),
        session(),
        13,
        Discipline::Timing {
            seed: 0,
            tokens: 128,
        },
    );
    for _ in 0..30 {
        let _ran = running.round(|arm, _drew| {
            warm(if *arm == left {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            })
        });
    }
    let finding = running.finish().finding(FIVE);
    assert!(
        finding.verdict().is_some(),
        "an unread condition is not a confound: {finding}"
    );
    assert!(!finding.isolation().isolates_a_variable());
    assert!(
        format!("{finding}").contains("isolation is undetermined"),
        "the doubt is printed beside the delta: {finding}"
    );
}

/// A behaviour comparison: trial *i* draws seed *i*, and both arms of a pair
/// draw the same one so that what differs between them is the arm and not the
/// trajectory (D19, §3.27).
#[test]
fn both_arms_of_a_pair_draw_the_same_seed() {
    let (left, right) = arms();
    let (one, other) = isolated(&left, &right);
    let mut running = Interleaving::<Simulated>::new(
        one,
        other,
        session(),
        5,
        Discipline::Behaviour {
            seeds: SeedSet::Standard,
        },
    );
    let mut handed: Vec<(Arm, u64)> = Vec::new();
    for _ in 0..8 {
        let ran = running.round(|arm, drew| {
            handed.push((arm.clone(), drew.seed()));
            warm(SECOND)
        });
        assert!(ran, "the published set never runs out");
    }
    let held = running.finish();

    for (at, pair) in held.pairs().iter().enumerate() {
        let expected = SeedSet::Standard.seed_for(at).expect("unbounded");
        assert_eq!(
            pair.drew(),
            &Draw::Seeded {
                seed: expected,
                from: mcf_core::trial::STANDARD.to_owned(),
            },
            "trial {at} draws seed {at}"
        );
    }
    for couple in handed.as_chunks::<2>().0 {
        let (Some(one), Some(other)) = (couple.first(), couple.last()) else {
            continue;
        };
        assert_ne!(one.0, other.0, "the two runs of a pair are different arms");
        assert_eq!(one.1, other.1, "and they draw the same seed");
    }
    let mut drawn: Vec<u64> = handed.iter().map(|(_, seed)| *seed).collect();
    drawn.sort_unstable();
    drawn.dedup();
    assert_eq!(drawn.len(), 8, "eight pairs, eight distinct seeds (B61)");
}

/// **B61's violation, refused at the comparison.** A run that gave every trial
/// the same seed is `n=1` wearing the costume of `n=8`, and the comparison
/// will not read it back as a behaviour run.
#[test]
fn one_seed_repeated_is_not_a_behaviour_comparison() {
    let (left, right) = arms();
    let (one, other) = isolated(&left, &right);
    let fixed = Draw::Seeded {
        seed: 42,
        from: mcf_core::trial::STANDARD.to_owned(),
    };
    let trials = Trials::from((0..8_usize).map(|at| {
        Trial::new(
            ns(SECOND),
            if at % 2 == 0 {
                left.clone()
            } else {
                right.clone()
            },
            Position(u32::try_from(at).unwrap_or(0)),
            session(),
            fixed.clone(),
        )
    }));
    assert_eq!(
        Comparison::from_trials(&trials, &one, &other),
        Err(NotComparable::DisciplinesDiffer),
        "eight trials on one seed are one trajectory counted four times"
    );
}

/// **D19's refusal.** Two arms that drew from different sets are not
/// comparable: a difference between them is a difference between the draws as
/// much as between the arms.
#[test]
fn arms_from_different_seed_sets_are_refused() {
    let (left, right) = arms();
    let (one, other) = isolated(&left, &right);
    let mine = SeedSet::declared("mine", vec![1, 2, 3, 4]).expect("a set");
    let trials = Trials::from((0..8_usize).map(|at| {
        let round = at.wrapping_div(2);
        let of_left = at % 2 == 0;
        Trial::new(
            ns(SECOND),
            if of_left { left.clone() } else { right.clone() },
            Position(u32::try_from(at).unwrap_or(0)),
            session(),
            Draw::Seeded {
                seed: if of_left {
                    SeedSet::Standard.seed_for(round).unwrap_or(0)
                } else {
                    mine.seed_for(round).unwrap_or(0)
                },
                from: if of_left {
                    SeedSet::Standard.identifier()
                } else {
                    mine.identifier()
                },
            },
        )
    }));
    let held = Comparison::from_trials(&trials, &one, &other);
    let Err(NotComparable::SeedSetsDiffer { left: a, right: b }) = &held else {
        panic!("two seed sets must be refused: {held:?}")
    };
    assert_eq!(a, &SeedSet::Standard.identifier());
    assert_eq!(b, "declared:mine");
    assert!(
        format!("{}", held.unwrap_err()).contains("no repeat count separates"),
        "the refusal says why a longer run does not fix it"
    );
}

/// A behaviour arm and a timing arm are two different experiments put side by
/// side, and the refusal says so.
#[test]
fn a_behaviour_arm_and_a_timing_arm_are_not_a_comparison() {
    let (left, right) = arms();
    let (one, other) = isolated(&left, &right);
    let trials = Trials::from((0..8_usize).map(|at| {
        let of_left = at % 2 == 0;
        Trial::new(
            ns(SECOND),
            if of_left { left.clone() } else { right.clone() },
            Position(u32::try_from(at).unwrap_or(0)),
            session(),
            if of_left {
                Draw::Seeded {
                    seed: SeedSet::Standard.seed_for(at.wrapping_div(2)).unwrap_or(0),
                    from: SeedSet::Standard.identifier(),
                }
            } else {
                timing()
            },
        )
    }));
    assert!(
        matches!(
            Comparison::from_trials(&trials, &one, &other),
            Err(NotComparable::SeedSetsDiffer { .. })
        ),
        "one arm drew from a set and the other pinned its length"
    );
}

/// A declared set runs out, and the runner stops rather than repeating a
/// trajectory (B61).
#[test]
fn a_run_stops_when_its_declared_set_runs_out() {
    let (left, right) = arms();
    let (one, other) = isolated(&left, &right);
    let mut running = Interleaving::<Simulated>::new(
        one,
        other,
        session(),
        5,
        Discipline::Behaviour {
            seeds: SeedSet::declared("three", vec![10, 20, 30]).expect("a set"),
        },
    );
    let mut rounds = 0;
    for _ in 0..10 {
        if !running.round(|_arm, _drew| warm(SECOND)) {
            break;
        }
        rounds += 1;
    }
    assert_eq!(rounds, 3, "three seeds, three pairs, and then it stops");
    assert_eq!(running.comparison().pairs().len(), 3);
}

/// A timing comparison reads back as a timing one, with the length it pinned.
#[test]
fn a_timing_comparison_names_what_it_pinned() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..8)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let trials = laid_out(&order, &[SECOND; 8], &session());
    let held = from_trials(&trials, &left, &right).expect("paired");
    assert_eq!(
        held.discipline(),
        &Discipline::Timing {
            seed: 0,
            tokens: 128
        }
    );
    assert!(
        format!("{}", held.discipline()).contains("measuring the stop"),
        "the discipline says why it pinned a length: {}",
        held.discipline()
    );
}

/// **B-081's condition.** A run whose trials were all warm records that, and a
/// run that mixed warm and cold records *that* — in both arms' conditions, so
/// a measurement taken warm is distinguishable in the record from one taken
/// cold (§6.13).
#[test]
fn what_a_run_reused_becomes_a_condition_of_both_arms() {
    let (left, right) = arms();
    let mut running = interleaving(&left, &right, 17);
    for _ in 0..6 {
        let _ran = running.round(|_arm, _drew| warm(SECOND));
    }
    let held = running.finish();

    assert_eq!(held.reuse(), Reuse::Uniform(Warmth::Warm));
    for arm in [held.arms().0, held.arms().1] {
        let stated = arm
            .conditions()
            .floor()
            .reuse
            .known()
            .map(ToString::to_string)
            .expect("what the run reused is a condition of the arm");
        assert!(stated.starts_with("warm:"), "{stated}");
    }
}

/// A run that loaded the model for some trials and not others is **not one
/// measurement**, and the condition says so rather than averaging over it.
#[test]
fn a_mixed_run_says_it_is_not_one_measurement() {
    let (left, right) = arms();
    let mut round = 0_usize;
    let mut running = interleaving(&left, &right, 19);
    for _ in 0..6 {
        let _ran = running.round(|_arm, _drew| {
            round = round.saturating_add(1);
            // The engine finds the model resident for some trials and not for
            // others, which is what alternating arms against a daemon that
            // holds one model at a time actually does.
            let found = if round.is_multiple_of(3) {
                Warmth::Cold
            } else {
                Warmth::Warm
            };
            Some((ns(SECOND), found))
        });
    }
    let held = running.finish();

    assert!(!held.reuse().is_uniform());
    let stated = held
        .arms()
        .0
        .conditions()
        .floor()
        .reuse
        .known()
        .map(ToString::to_string)
        .expect("a mixed run states its mixture");
    assert!(stated.contains("MIXED"), "{stated}");
    assert!(stated.contains("not one measurement"), "{stated}");

    // **And it has no delta to give.** §6.13: a run whose trials were not
    // alike has measured two things and would be reporting one, and part of
    // any difference between the arms would be the difference between a trial
    // that loaded the model and one that did not.
    let finding = held.finding(FIVE);
    assert_eq!(finding.withheld(), Some(Withheld::MixedReuse));
    assert!(finding.verdict().is_none(), "{finding}");
    assert!(
        format!("{finding}").contains("no delta"),
        "and the rendering says so first: {finding}"
    );
}

/// A mixed run is **not declarable**. An operator can say *I know these two
/// variables moved together*, and cannot say *I know some of my trials loaded
/// the model* — the first is a statement about the question, the second about
/// the instrument (§6.13, A8).
#[test]
fn a_mixed_run_cannot_be_declared_away() {
    let (left, right) = arms();
    let mut round = 0_usize;
    let mut running = interleaving(&left, &right, 23);
    for _ in 0..6 {
        let _ran = running.round(|_arm, _drew| {
            round = round.saturating_add(1);
            let found = if round.is_multiple_of(2) {
                Warmth::Cold
            } else {
                Warmth::Warm
            };
            Some((ns(SECOND.saturating_mul(2)), found))
        });
    }
    let held = running
        .finish()
        .declaring("I meant to compare a warm arm with a cold one");
    let finding = held.finding(FIVE);
    assert_eq!(
        finding.withheld(),
        Some(Withheld::MixedReuse),
        "a declaration answers A8's question and not §6.13's: {finding}"
    );
    assert!(finding.verdict().is_none());
}

/// And a uniform run gives its delta, whichever uniform state it was in.
#[test]
fn a_uniform_run_gives_its_delta() {
    let (left, right) = arms();
    for found in [Warmth::Cold, Warmth::Warm, Warmth::Unstated] {
        let mut running = interleaving(&left, &right, 29);
        for _ in 0..30 {
            let _ran = running.round(|arm, _drew| {
                Some((
                    ns(if *arm == left {
                        SECOND.saturating_mul(2)
                    } else {
                        SECOND
                    }),
                    found,
                ))
            });
        }
        let finding = running.finish().finding(FIVE);
        assert_eq!(finding.withheld(), None, "{found}: {finding}");
        assert!(finding.verdict().is_some(), "{found}: {finding}");
    }
}

/// **And it has teeth.** A comparison whose arms differ in the thing under test
/// *and* in what they reused is confounded, so A8 withholds the delta — which
/// is §6.13's *anything that could change a result must be visible in that
/// result's conditions*, enforced rather than printed.
#[test]
fn a_warm_arm_against_a_cold_one_is_confounded() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..12)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let trials = laid_out(&order, &[SECOND; 12], &session());

    // Two arms differing in quantization, and in what each found loaded.
    let mut cold = everything_known();
    cold.reuse = known("cold: every trial loaded the model for itself");
    let mut hot = everything_known();
    hot.quantization = known("q2_k");
    hot.reuse = known("warm: the model was already resident for every trial");
    let one = UnderTest::new(
        left.clone(),
        Conditions::new(BuildIdentity::current(), cold),
    );
    let other = UnderTest::new(
        right.clone(),
        Conditions::new(BuildIdentity::current(), hot),
    );

    let held = Comparison::from_trials(&trials, &one, &other).expect("still paired");
    assert!(held.isolation().is_confounded());
    let finding = held.finding(FIVE);
    assert!(
        finding.verdict().is_none(),
        "a warm arm against a cold one has no delta to give (§6.13, A8): {finding}"
    );
    assert!(
        format!("{finding}").contains("reuse"),
        "and the refusal names what differed: {finding}"
    );
}

/// A comparison read back out of the record cannot say what its trials
/// reused — a `Trial` carries the value, not the engine's account of it — and
/// says *unstated* rather than guessing (A7).
#[test]
fn a_comparison_rebuilt_from_trials_does_not_claim_to_know_what_it_reused() {
    let (left, right) = arms();
    let order: Vec<&Arm> = (0..8)
        .map(|at| if at % 2 == 0 { &left } else { &right })
        .collect();
    let held =
        from_trials(&laid_out(&order, &[SECOND; 8], &session()), &left, &right).expect("paired");
    assert_eq!(held.reuse(), Reuse::Uniform(Warmth::Unstated));
    assert!(
        held.reuse().condition().contains("did not say"),
        "{}",
        held.reuse()
    );
}

/// **A4, which is absolute.** Nine of ten trials completing is nine data
/// points: a run cut short keeps every pair it took, the verdict over them
/// stands, and what was lost is said rather than thrown away with the evidence.
#[test]
fn a_run_cut_short_keeps_what_it_produced() {
    let (left, right) = arms();
    let mut running = interleaving(&left, &right, 31);
    for _ in 0..9 {
        let _ran = running.round(|arm, _drew| {
            warm(if *arm == left {
                SECOND.saturating_mul(2)
            } else {
                SECOND
            })
        });
    }
    running.stopped_short("the tenth trial's engine died");
    let held = running.finish();

    assert_eq!(held.pairs().len(), 9, "nine pairs are nine pairs");
    assert_eq!(held.cut_short(), Some("the tenth trial's engine died"));

    // And the verdict over them stands: each pair is two runs of two arms
    // taken back to back, and an interruption afterwards does not reach back.
    let finding = held.finding(FIVE);
    assert!(
        matches!(finding.verdict(), Some(Verdict::Differ { after: 9, .. })),
        "the nine pairs still separate a doubling: {finding}"
    );
    assert_eq!(finding.cut_short(), Some("the tenth trial's engine died"));
    assert!(
        format!("{finding}").contains("CUT SHORT"),
        "and the rendering says what was lost: {finding}"
    );
}

/// A run that was **not** cut short says so by saying nothing — *it finished*
/// and *it was interrupted and nobody recorded why* are different facts.
#[test]
fn a_run_that_finished_carries_no_reason() {
    let (left, right) = arms();
    let mut running = interleaving(&left, &right, 37);
    for _ in 0..4 {
        let _ran = running.round(|_arm, _drew| warm(SECOND));
    }
    let held = running.finish();
    assert_eq!(held.cut_short(), None);
    assert!(!format!("{}", held.finding(FIVE)).contains("CUT SHORT"));
}

/// **A run that did not happen is not a trial** (A4, A1). A failed request
/// records no pair — a zero-duration stand-in would put a number nobody
/// measured into the distribution, which is worse than losing the pair.
#[test]
fn a_run_that_did_not_happen_is_not_a_trial() {
    let (left, right) = arms();
    let mut attempts = 0_usize;
    let mut running = interleaving(&left, &right, 47);
    for _ in 0..10 {
        let ran = running.round(|_arm, _drew| {
            attempts = attempts.saturating_add(1);
            // The eighth run does not happen, which is the middle of the
            // fourth pair.
            (attempts != 8).then(|| (ns(SECOND), Warmth::Warm))
        });
        if !ran {
            break;
        }
    }
    let held = running.finish();
    assert_eq!(
        held.pairs().len(),
        3,
        "three pairs completed; the fourth did not, and half a pair is not a pair"
    );
    assert_eq!(
        attempts, 8,
        "and the run stopped there rather than going on without it"
    );
    for pair in held.pairs() {
        assert_ne!(
            pair.left().as_nanos(),
            0,
            "no fabricated zero reached the distribution"
        );
        assert_ne!(pair.right().as_nanos(), 0);
    }
}

/// And where the *first* run of a pair does not happen, the second is not
/// attempted: a pair is two runs taken back to back, and one of them alone is
/// not half a pair (§3.27).
#[test]
fn the_second_run_of_a_pair_is_not_attempted_without_the_first() {
    let (left, right) = arms();
    let mut attempts = 0_usize;
    let mut running = interleaving(&left, &right, 53);
    for _ in 0..4 {
        let ran = running.round(|_arm, _drew| {
            attempts = attempts.saturating_add(1);
            // The fifth run — the first of the third pair — does not happen.
            (attempts != 5).then(|| (ns(SECOND), Warmth::Warm))
        });
        if !ran {
            break;
        }
    }
    assert_eq!(running.comparison().pairs().len(), 2);
    assert_eq!(
        attempts, 5,
        "the pair's second run was never asked for, because its first did not happen"
    );
}

/// What stops a real measurement travelling (B-217, F92, F95).
mod fitness {
    use mcf_core::hardware::headroom::Headroom;
    use mcf_core::measurement::PartsPerMillion;
    use mcf_core::trial::Arm;

    use super::super::{Comparison, NotFitToContribute};
    use super::{arms, from_trials, laid_out, session};

    /// Twelve alternating trials, whose values the caller supplies.
    fn pairs_of(values: &[u64]) -> Comparison<mcf_core::time::Simulated> {
        let (left, right) = arms();
        let order: Vec<&Arm> = (0..values.len())
            .map(|at| if at % 2 == 0 { &left } else { &right })
            .collect();
        let scaled: Vec<u64> = values
            .iter()
            .map(|held| held.saturating_mul(1_000_000))
            .collect();
        let trials = laid_out(&order, &scaled, &session());
        from_trials(&trials, &left, &right).expect("interleaved trials pair")
    }

    /// A busy machine marks the result and does not suppress it.
    #[test]
    fn a_run_outside_the_band_is_marked_and_kept() {
        let held = pairs_of(&[100, 130, 100, 132, 100, 128, 100, 131, 100, 129, 100, 133])
            .on_a_machine_with(Headroom {
                competing: 27_000,
                capacity: 32_000,
                band: mcf_core::hardware::headroom::MEASURED,
            });
        let finding = held.finding(PartsPerMillion(50_000));
        assert!(
            finding.verdict().is_some(),
            "the delta stands: a busy machine is not a confound, it is a condition (A4)"
        );
        let why = finding.not_fit_to_contribute();
        assert!(
            why.iter()
                .any(|held| matches!(held, NotFitToContribute::OutsideTheBand(_))),
            "and it is marked: {why:?}"
        );
    }

    /// A quiet machine leaves nothing to mark.
    #[test]
    fn a_run_inside_the_band_is_fit() {
        let held = pairs_of(&[100, 130, 100, 132, 100, 128, 100, 131, 100, 129, 100, 133])
            .on_a_machine_with(Headroom {
                competing: 1_260,
                capacity: 32_000,
                band: mcf_core::hardware::headroom::MEASURED,
            });
        assert!(
            held.finding(PartsPerMillion(50_000))
                .not_fit_to_contribute()
                .is_empty()
        );
    }

    /// A machine nobody read is not a machine that was free (A7).
    #[test]
    fn an_unread_machine_marks_nothing_and_claims_nothing() {
        let held = pairs_of(&[100, 130, 100, 132, 100, 128, 100, 131, 100, 129, 100, 133]);
        assert!(
            held.finding(PartsPerMillion(50_000))
                .not_fit_to_contribute()
                .is_empty(),
            "an absent reading cannot mark a run unfit, and must not claim it fit either — the \
             claim is made by what is rendered, and nothing is rendered here"
        );
    }

    /// Every reason, not the first one found.
    #[test]
    fn a_run_that_fails_twice_says_both() {
        // Ordered (the size straddles five percent) on a busy machine.
        let held = pairs_of(&[100, 101, 100, 140, 100, 103, 100, 160, 100, 102, 100, 150])
            .on_a_machine_with(Headroom {
                competing: 27_000,
                capacity: 32_000,
                band: mcf_core::hardware::headroom::MEASURED,
            });
        let why = held
            .finding(PartsPerMillion(50_000))
            .not_fit_to_contribute();
        assert!(
            why.len() >= 2,
            "a reader told only one reason will fix that one and be surprised again (A1): \
             {why:?}"
        );
    }
}

/// The order within a pair does not depend on how round the seed is.
///
/// F: the seed went into the xorshift as its state, and a xorshift started
/// from a small word takes many rounds to mix. `--seed 41` drew right-first
/// nine times running, so a six-pair comparison ran every pair in the same
/// order — and interleaving, whose entire job is to cancel order effects,
/// cancelled nothing. Whatever advantage there is in going first then sits
/// inside the difference being reported.
///
/// What is checked is not that a run of one order never happens: six coins
/// land the same way about three times in a hundred, and a generator forbidden
/// from doing so would not be one. What is checked is that it happens about
/// that often rather than for the particular small numbers a person types.
#[test]
fn the_order_within_a_pair_does_not_depend_on_how_round_the_seed_is() {
    let draws = |seed: u64, many: usize| -> Vec<bool> {
        let mut state = {
            let scrambled = super::scramble(seed);
            if scrambled == 0 {
                0x2545_F491_4F6C_DD1D
            } else {
                scrambled
            }
        };
        (0..many)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state >> 63 == 0
            })
            .collect()
    };
    let one_sided = |seed: u64| {
        let six = draws(seed, 6);
        six.iter().all(|held| *held) || six.iter().all(|held| !*held)
    };

    // The two seeds actually observed running every pair one way.
    for seed in [1_u64, 41] {
        assert!(
            !one_sided(seed),
            "seed {seed} still runs a whole short comparison in one order"
        );
    }

    // And across the small numbers generally it is a coin, not a habit.
    // Chance is about 3 in 100; the bound is loose enough not to be a flake
    // and tight enough to catch a generator that has stopped mixing.
    let stuck = (0_u64..256).filter(|seed| one_sided(*seed)).count();
    assert!(
        stuck <= 26,
        "{stuck} of 256 seeds run six pairs in one order, which is not chance"
    );

    // Over a longer run neither arm is systematically favoured.
    for seed in [1_u64, 41, 2026] {
        let lefts = draws(seed, 400).iter().filter(|held| **held).count();
        assert!(
            (150..=250).contains(&lefts),
            "seed {seed} drew left first {lefts} times in 400, which is not a balance"
        );
    }

    // Still a pure function of the seed, so a comparison replays (§3.12) —
    // and two seeds are still two runs.
    assert_eq!(draws(41, 32), draws(41, 32));
    assert_ne!(draws(41, 32), draws(42, 32));
}
