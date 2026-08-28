//! What the stopping condition has to get right.

use mcf_core::measurement::PartsPerMillion;

use super::{Verdict, verdict};

/// Five percent, and twenty, as this module spells them.
const FIVE: PartsPerMillion = PartsPerMillion(50_000);
const TWENTY: PartsPerMillion = PartsPerMillion(200_000);
const TWO: PartsPerMillion = PartsPerMillion(20_000);

/// A repeatable spread with no randomness: values walk a fixed cycle around a
/// centre, so a test asserts on arithmetic rather than on a seed.
///
/// Nanoseconds, because that is what the module takes — exact, orderable, and
/// with no value that compares false against itself (A6).
fn around(centre: u64, spread_ppm: u64, count: usize) -> Vec<u64> {
    let steps: [i64; 10] = [0, 10, -10, 5, -5, 7, -7, 2, -2, 9];
    (0..count)
        .map(|at| {
            let step = steps.get(at % steps.len()).copied().unwrap_or(0);
            let offset = i128::from(centre)
                .saturating_mul(i128::from(spread_ppm))
                .saturating_mul(i128::from(step))
                .wrapping_div(10_000_000);
            u64::try_from(i128::from(centre).saturating_add(offset)).unwrap_or(centre)
        })
        .collect()
}

/// Two arms that plainly differ are reported as differing, with the size of
/// the gap and how often noise made one that big.
#[test]
fn a_real_difference_is_found() {
    let slow = around(2_000_000_000, 20_000, 20);
    let fast = around(1_000_000_000, 20_000, 20);
    match verdict(&slow, &fast, FIVE) {
        Verdict::Differ { by, by_chance, .. } => {
            assert!(
                by.0 > 500_000,
                "a doubling is about a hundred percent: {by:?}"
            );
            assert!(by_chance <= super::FALSE_ALARMS_ALLOWED, "{by_chance:?}");
        }
        other => panic!("a doubling was not seen: {other}"),
    }
}

/// Two arms that are the same are reported as the same *to a stated
/// resolution*, which is an answer rather than a failure to find one (B-086).
#[test]
fn no_difference_is_a_result_and_says_what_it_could_have_seen() {
    let one = around(1_000_000_000, 10_000, 40);
    let other = around(1_000_000_000, 10_000, 40);
    match verdict(&one, &other, TWENTY) {
        Verdict::Same { resolving, after } => {
            assert_eq!(resolving, TWENTY);
            assert_eq!(after, 40);
        }
        other => panic!("identical arms were not called the same: {other}"),
    }
}

/// Noisy arms that have not separated are *not yet decided* — never rounded to
/// "the same", which would be a null result manufactured from impatience (A7).
#[test]
fn noise_wider_than_the_question_is_not_yet_decided() {
    let one = around(1_000_000_000, 400_000, 6);
    let other = around(1_020_000_000, 400_000, 6);
    assert!(
        matches!(verdict(&one, &other, TWO), Verdict::NotYet { .. }),
        "arms noisier than the difference must not be called the same"
    );
}

/// More trials turn *not yet* into an answer, which is the whole point of a
/// stopping condition rather than a count.
#[test]
fn repeating_resolves_what_a_few_trials_could_not() {
    let early = verdict(
        &around(1_000_000_000, 100_000, 4),
        &around(1_300_000_000, 100_000, 4),
        FIVE,
    );
    assert!(
        matches!(early, Verdict::NotYet { .. }),
        "four trials should not settle it: {early}"
    );
    let later = verdict(
        &around(1_000_000_000, 100_000, 60),
        &around(1_300_000_000, 100_000, 60),
        FIVE,
    );
    assert!(
        matches!(later, Verdict::Differ { .. }),
        "sixty should: {later}"
    );
}

/// Unequal arms are not a paired comparison, and the pairing is the defence
/// against the drift F51 measured — so it is refused rather than truncated.
#[test]
fn unpaired_arms_are_not_a_comparison() {
    let one = around(1_000_000_000, 10_000, 20);
    let other = around(2_000_000_000, 10_000, 19);
    assert!(
        matches!(verdict(&one, &other, FIVE), Verdict::NotYet { .. }),
        "a doubling must not be reported from arms that were not paired"
    );
}

/// The count is part of the answer, because it is the thing that varies
/// between sittings (F53).
#[test]
fn the_answer_carries_what_it_cost() {
    let Verdict::Differ { after, .. } = verdict(
        &around(3_000_000_000, 20_000, 12),
        &around(1_000_000_000, 20_000, 12),
        FIVE,
    ) else {
        panic!("a threefold difference was not seen");
    };
    assert_eq!(after, 12, "the trials it took travel with the verdict");
}

/// Nothing is decided from a single pair, whatever it shows.
#[test]
fn one_pair_decides_nothing() {
    assert!(matches!(
        verdict(&[1_000_000], &[9_000_000], FIVE),
        Verdict::NotYet { so_far: 1 }
    ));
}

/// A run that took no measurable time is not something to divide by: it is
/// *not yet decided*, never an infinity and never a NaN — which is the whole
/// reason this module counts in integers (A6, A7).
#[test]
fn a_zero_duration_is_not_divided_by() {
    assert!(matches!(
        verdict(&[0, 0, 0], &[5, 5, 5], FIVE),
        Verdict::NotYet { .. }
    ));
}
