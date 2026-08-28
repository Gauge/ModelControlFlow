//! What the construction has to refuse, and what it has to find.
//!
//! The clock here is [`Simulated`], because these are questions about the
//! *shape* of a comparison rather than about how long anything took, and A11
//! keeps the two kinds of duration apart in the type system. A test that
//! reached for the monotonic clock would be measuring the test machine.

use mcf_core::measurement::PartsPerMillion;
use mcf_core::time::{Duration, Simulated};
use mcf_core::trial::{Arm, Position, SessionId, Trial, Trials};

use super::{Comparison, Difference, Interleaving, NotComparable, Side, Strength};
use crate::enough::Verdict;

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

/// Trials laid out in a stated order of arms, positions counting up from zero.
fn laid_out(order: &[&Arm], values: &[u64], session: &SessionId) -> Trials<Duration<Simulated>> {
    Trials::from(order.iter().enumerate().map(|(at, arm)| {
        Trial::new(
            ns(values.get(at).copied().unwrap_or(0)),
            (*arm).clone(),
            Position(u32::try_from(at).unwrap_or(0)),
            session.clone(),
        )
    }))
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
    match Comparison::from_trials(&trials, &left, &right) {
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
    let held = Comparison::from_trials(&trials, &left, &right).expect("interleaved trials pair");
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
    let held = Comparison::from_trials(&trials, &left, &right).expect("three pairs");
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
        Comparison::from_trials(&trials, &left, &right),
        Err(NotComparable::Unbalanced { left: 3, right: 2 })
    );
}

/// One pair decides nothing, so it is not a comparison.
#[test]
fn a_single_pair_is_not_a_comparison() {
    let (left, right) = arms();
    let trials = laid_out(&[&left, &right], &[10, 20], &session());
    assert_eq!(
        Comparison::from_trials(&trials, &left, &right),
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
        Trial::new(ns(10), left.clone(), Position(0), one.clone()),
        Trial::new(ns(20), right.clone(), Position(1), other),
        Trial::new(ns(10), left.clone(), Position(2), one.clone()),
        Trial::new(ns(20), right.clone(), Position(3), one),
    ]);
    assert!(matches!(
        Comparison::from_trials(&trials, &left, &right),
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
    let held = Comparison::from_separate_sessions(&one, &left, &other, &right)
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
        Comparison::from_separate_sessions(&one, &left, &other, &right),
        Err(NotComparable::OneSession { session: held })
    );
}

/// The runner alternates the arms and randomizes which goes first, and the
/// order it drew is legible afterwards.
#[test]
fn the_runner_interleaves_and_randomizes() {
    let (left, right) = arms();
    let mut order = Vec::new();
    let mut running = Interleaving::<Simulated>::new(left.clone(), right, session(), 7);
    for _ in 0..40 {
        running.round(|arm| {
            order.push(arm.clone());
            ns(if *arm == left {
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
        let mut running =
            Interleaving::<Simulated>::new(left.clone(), right.clone(), session(), 99);
        for _ in 0..12 {
            running.round(|arm| {
                seen.push(arm.clone());
                ns(SECOND)
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
    let mut running = Interleaving::<Simulated>::new(left.clone(), right.clone(), session(), 3);
    let mut finding = running.finding(FIVE);
    while matches!(finding.verdict(), Verdict::NotYet { .. }) && round < 200 {
        let drift = round
            .wrapping_mul(37)
            .wrapping_rem(11)
            .wrapping_mul(5_000_000);
        running.round(|arm| {
            ns(if *arm == left {
                1_300_000_000_u64.saturating_add(drift)
            } else {
                SECOND.saturating_add(drift)
            })
        });
        round = round.saturating_add(1);
        finding = running.finding(FIVE);
    }
    let Verdict::Differ { by, after, .. } = finding.verdict() else {
        panic!("a thirty-percent difference was never found: {finding}");
    };
    assert!(
        (250_000..350_000).contains(&by.0),
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
    let paired = Comparison::from_trials(
        &laid_out(&paired_order, &machine, &session()),
        &left,
        &right,
    )
    .expect("interleaved");
    let honest = paired.finding(TWENTY);
    println!("  interleaved: {honest}");
    assert!(
        matches!(honest.verdict(), Verdict::Same { .. }),
        "the paired comparison reports no difference, which is the truth: {honest}"
    );

    // Blocked: the same forty-eight timings, all of one arm and then all of
    // the other.
    let (Some(blocked_left), Some(blocked_right)) = (machine.get(..24), machine.get(24..)) else {
        panic!("forty-eight timings split in half");
    };
    let morning = SessionId::new("blocks-first-half");
    let afternoon = SessionId::new("blocks-second-half");
    let assembled = Comparison::from_separate_sessions(
        &laid_out(&[&left; 24], blocked_left, &morning),
        &left,
        &laid_out(&[&right; 24], blocked_right, &afternoon),
        &right,
    )
    .expect("two sessions may be put side by side");
    let invented = assembled.finding(TWENTY);
    println!("  blocked:     {invented}");
    let Verdict::Differ { by, by_chance, .. } = invented.verdict() else {
        panic!("the blocked arrangement of the very same timings invents no difference: {invented}")
    };
    assert!(
        (450_000..550_000).contains(&by.0),
        "the difference it invents is the drift, about fifty percent: {invented}"
    );
    assert!(
        by_chance.0 < 10_000,
        "and it is confident about it, which is the danger: {invented}"
    );
}
