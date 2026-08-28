//! What the stopping condition has to get right.
//!
//! Two nulls, tested separately: the sign flip a paired comparison uses, and
//! the pooled redraw that is all a comparison assembled from separate sessions
//! can have.

use mcf_core::measurement::PartsPerMillion;

use super::{Verdict, over_paired_differences, over_separate_arms};

/// Five percent, twenty and two, as this module spells them.
const FIVE: PartsPerMillion = PartsPerMillion(50_000);
const TWENTY: PartsPerMillion = PartsPerMillion(200_000);
const TWO: PartsPerMillion = PartsPerMillion(20_000);

/// A repeatable spread with no randomness: values walk a fixed cycle around a
/// centre, so a test asserts on arithmetic rather than on a seed.
///
/// Nanoseconds, because that is what the pooled path takes — exact, orderable,
/// and with no value that compares false against itself (A6).
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

/// Paired differences around a true effect, with a repeatable wobble on top.
///
/// The wobble alternates sign so that the differences are not all the same
/// number — a set of identical differences is separable by any test and would
/// prove nothing about the null.
fn differences(effect_ppm: i64, wobble_ppm: i64, count: usize) -> Vec<i64> {
    let steps: [i64; 8] = [0, 3, -2, 5, -4, 1, -5, 2];
    (0..count)
        .map(|at| {
            let step = steps.get(at % steps.len()).copied().unwrap_or(0);
            effect_ppm.saturating_add(wobble_ppm.saturating_mul(step).wrapping_div(5))
        })
        .collect()
}

/// A paired effect that plainly exists is found, with its size and how often
/// the sign flip made one that big.
#[test]
fn a_real_paired_effect_is_found() {
    match over_paired_differences(&differences(300_000, 40_000, 20), FIVE) {
        Verdict::Differ { by, by_chance, .. } => {
            assert!(by.0 > 250_000, "a thirty-percent effect: {by:?}");
            assert!(by_chance <= super::FALSE_ALARMS_ALLOWED, "{by_chance:?}");
        }
        other => panic!("a thirty-percent paired effect was not seen: {other}"),
    }
}

/// Differences centred on zero are *the same to a stated resolution*, which is
/// an answer rather than a failure to find one (B-086).
#[test]
fn no_paired_difference_is_a_result_and_says_what_it_could_have_seen() {
    match over_paired_differences(&differences(0, 10_000, 40), TWENTY) {
        Verdict::Same {
            resolving,
            by,
            after,
        } => {
            assert_eq!(resolving, TWENTY);
            assert_eq!(after, 40);
            assert!(
                by < TWENTY,
                "the measured difference travels inside the null result, and is smaller than \
                 the resolution by construction: {by:?}"
            );
        }
        other => panic!("arms with no difference were not called the same: {other}"),
    }
}

/// Differences noisier than the question are *not yet decided* — never rounded
/// to "the same", which would be a null manufactured from impatience (A7).
#[test]
fn noise_wider_than_the_question_is_not_yet_decided() {
    let held = over_paired_differences(&differences(5_000, 400_000, 6), TWO);
    assert!(
        matches!(held, Verdict::NotYet { .. }),
        "differences noisier than the question must not be called the same: {held}"
    );
}

/// More pairs turn *not yet* into an answer, which is the whole point of a
/// stopping condition rather than a count.
#[test]
fn repeating_resolves_what_a_few_pairs_could_not() {
    let early = over_paired_differences(&differences(60_000, 150_000, 4), FIVE);
    assert!(
        matches!(early, Verdict::NotYet { .. }),
        "four pairs should not settle it: {early}"
    );
    let later = over_paired_differences(&differences(60_000, 150_000, 60), FIVE);
    assert!(
        matches!(later, Verdict::Differ { .. }),
        "sixty should: {later}"
    );
}

/// The count is part of the answer, because it is the thing that varies
/// between sittings (F53).
#[test]
fn the_answer_carries_what_it_cost() {
    let Verdict::Differ { after, .. } =
        over_paired_differences(&differences(400_000, 30_000, 12), FIVE)
    else {
        panic!("a fourfold-sized effect was not seen");
    };
    assert_eq!(after, 12, "the pairs it took travel with the verdict");
}

/// Nothing is decided from a single pair, whatever it shows.
#[test]
fn one_pair_decides_nothing() {
    assert!(matches!(
        over_paired_differences(&[900_000], FIVE),
        Verdict::NotYet { so_far: 1 }
    ));
}

/// The test is symmetric: an effect the other way round is found the same way,
/// at the same size and the same chance — and **the direction is the one thing
/// that changes**, which is what makes the verdict a comparison rather than a
/// magnitude (F67).
#[test]
fn direction_changes_only_the_direction() {
    let one = over_paired_differences(&differences(300_000, 40_000, 20), FIVE);
    let held: Vec<i64> = differences(300_000, 40_000, 20)
        .into_iter()
        .map(i64::saturating_neg)
        .collect();
    let other = over_paired_differences(&held, FIVE);

    let (
        Verdict::Differ {
            by,
            left_quicker,
            by_chance,
            after,
        },
        Verdict::Differ {
            by: mirrored,
            left_quicker: the_other_way,
            by_chance: as_likely,
            after: as_many,
        },
    ) = (&one, &other)
    else {
        panic!("both directions separate: {one} and {other}");
    };
    assert_eq!(by, mirrored, "the same size");
    assert_eq!(by_chance, as_likely, "at the same chance");
    assert_eq!(after, as_many, "after the same count");
    assert!(
        left_quicker != the_other_way,
        "and the other way round, which is the whole of what a comparison adds to a magnitude"
    );
    assert!(
        *left_quicker,
        "positive differences mean the left arm is quicker"
    );
}

/// The pooled null still works, because a comparison assembled from separate
/// sessions has nothing else — and its answers are labelled by the caller that
/// chose it.
#[test]
fn the_pooled_null_finds_a_real_difference() {
    let slow = around(2_000_000_000, 20_000, 20);
    let fast = around(1_000_000_000, 20_000, 20);
    match over_separate_arms(&slow, &fast, FIVE) {
        Verdict::Differ { by, .. } => assert!(by.0 > 500_000, "a doubling: {by:?}"),
        other => panic!("a doubling was not seen: {other}"),
    }
}

/// A run that took no measurable time is not something to divide by: it is
/// *not yet decided*, never an infinity and never a NaN — which is the whole
/// reason this module counts in integers (A6, A7).
#[test]
fn a_zero_duration_is_not_divided_by() {
    assert!(matches!(
        over_separate_arms(&[0, 0, 0], &[5, 5, 5], FIVE),
        Verdict::NotYet { .. }
    ));
}

/// **The defect F55 found on a real machine.** Four paired trials of one
/// command against itself gave differences of thirteen, seven, nought-point-two
/// and one-point-four percent, and the first draft called that *no difference
/// as large as five percent*.
///
/// It cannot be. A sign flip over four pairs has sixteen assignments, so the
/// smallest false-alarm rate reachable is one in sixteen — above the one in
/// twenty this module requires — and `Differ` is therefore **unreachable** at
/// four pairs whatever the data. A rule that can only ever answer one way is
/// not a test, so the honest answer is *not yet*.
#[test]
fn a_null_result_is_not_declared_where_a_difference_could_not_have_been() {
    let real: [i64; 4] = [129_000, -74_000, -2_000, -14_000];
    let held = over_paired_differences(&real, FIVE);
    assert!(
        matches!(held, Verdict::NotYet { so_far: 4 }),
        "four pairs cannot reach one in twenty, so they cannot declare anything: {held}"
    );
}

/// And the refusal above is about the **count**, not about the data: noise of
/// the same size, centred so that there is genuinely no effect in it, reaches
/// a null result once there are enough pairs of it.
///
/// The count it takes is printed rather than asserted, because F53's whole
/// finding is that the count is a property of the sitting.
#[test]
fn the_same_noise_decides_once_there_are_enough_pairs_of_it() {
    // The four real differences, centred on their own median so that they
    // carry no effect — only their spread, which is what decides the count.
    let real: [i64; 4] = [129_000, -74_000, -2_000, -14_000];
    let centre = -8_000_i64;
    let noise: Vec<i64> = real
        .iter()
        .map(|held| held.saturating_sub(centre))
        .collect();

    let mut decided = None;
    for copies in [1_usize, 2, 5, 10, 20, 40, 80] {
        let many: Vec<i64> = std::iter::repeat_n(noise.clone(), copies)
            .flatten()
            .collect();
        let held = over_paired_differences(&many, FIVE);
        println!("  {} pairs: {held}", many.len());
        if matches!(held, Verdict::Same { .. }) && decided.is_none() {
            decided = Some(many.len());
        }
    }
    let Some(at) = decided else {
        panic!("noise this size never resolves to a null result at any count tried");
    };
    assert!(
        at > 4,
        "four pairs cannot decide anything, so the answer must arrive later: {at}"
    );
}

/// **The degeneracy F57 found.** Two arms whose timings are *identical* — every
/// paired difference exactly zero — are the same, and the resampling test that
/// stood here first could not say so: flipping the signs of a set of equal
/// magnitudes cannot move the median's size, so the null was a single point and
/// the answer was *cannot tell* about data that could not be clearer.
#[test]
fn identical_arms_are_the_same_rather_than_undecided() {
    let held = over_paired_differences(&[0; 40], FIVE);
    assert!(
        matches!(held, Verdict::Same { .. }),
        "arms that ran to the same nanosecond forty times are the same: {held}"
    );
}

/// **More evidence never gives a weaker verdict.** The exact tail leaves
/// `u128` past about a hundred and twenty pairs, and the first draft answered
/// *not decided* there about data it had decided at eighty — an instrument
/// whose confidence falls as its evidence grows.
#[test]
fn a_longer_run_never_decides_less() {
    let noise: [i64; 4] = [137_000, -66_000, 6_000, -6_000];
    let mut decided_at = None;
    for copies in [5_usize, 10, 20, 40, 80, 160] {
        let many: Vec<i64> = std::iter::repeat_n(noise, copies).flatten().collect();
        let held = over_paired_differences(&many, FIVE);
        let decided = !matches!(held, Verdict::NotYet { .. });
        if decided && decided_at.is_none() {
            decided_at = Some(many.len());
        }
        if decided_at.is_some() {
            assert!(
                decided,
                "{} pairs is undecided after {} decided it: {held}",
                many.len(),
                decided_at.unwrap_or(0)
            );
        }
    }
    assert!(decided_at.is_some(), "this noise resolves at some count");
}

/// The chance a verdict reports is the exact sign-test tail, not an estimate.
///
/// Ten pairs won by one arm is two in one thousand and twenty-four, which is
/// 1953 parts per million after rounding down. A resampling would have given
/// something near it and different every time the seed changed.
#[test]
fn the_reported_chance_is_exact() {
    let Verdict::Differ { by_chance, .. } = over_paired_differences(&[100_000; 10], FIVE) else {
        panic!("ten pairs won by one arm separate them");
    };
    assert_eq!(
        by_chance,
        PartsPerMillion(1953),
        "two in one thousand and twenty-four, computed rather than sampled"
    );
}

/// Six pairs is the fewest that can reach one in twenty at all, and five
/// cannot — which is a property of the test rather than of the data, and is
/// the general form of the defect F55 caught at four.
#[test]
fn five_pairs_cannot_reach_the_threshold_and_six_can() {
    assert!(
        matches!(
            over_paired_differences(&[100_000; 5], FIVE),
            Verdict::NotYet { .. } | Verdict::Same { .. }
        ),
        "five coins landing the same way is one chance in sixteen, which is not one in twenty"
    );
    assert!(
        matches!(
            over_paired_differences(&[100_000; 6], FIVE),
            Verdict::Differ { .. }
        ),
        "six is one in thirty-two, which is"
    );
}

/// A tie supports neither arm and is excluded from the count, which is the
/// standard treatment — and the pair is still reported as having happened.
#[test]
fn ties_leave_the_count_but_not_the_record_of_having_run() {
    let mut differences = vec![100_000_i64; 8];
    differences.extend([0, 0, 0, 0]);
    let Verdict::Differ { after, .. } = over_paired_differences(&differences, FIVE) else {
        panic!("eight pairs won by one arm separate them: {differences:?}");
    };
    assert_eq!(after, 12, "every pair that ran is counted in what it cost");
}

/// **The defect F59 found on a provisioned engine.** A difference must be real
/// *and* as large as the caller said they care about.
///
/// Two quantizations of one model, asked about at five percent, were reported
/// as *they differ by 0.8%* after a hundred and thirteen paired trials — a
/// real difference, found honestly, and an answer to a question nobody asked.
/// A caller who says five percent has said that eight tenths of one is beneath
/// notice; reporting it invites acting on it (§3.28).
///
/// The honest verdict below the resolution is the null one, and the
/// measurement travels inside it so nothing is lost (A1).
#[test]
fn a_real_difference_smaller_than_the_question_is_a_null_result() {
    // A consistent eight-tenths-of-a-percent difference, over enough pairs
    // that the sign test finds it easily: forty of forty is one chance in
    // five hundred billion.
    let tiny = vec![8_000_i64; 40];
    match over_paired_differences(&tiny, FIVE) {
        Verdict::Same { resolving, by, .. } => {
            assert_eq!(resolving, FIVE);
            assert_eq!(
                by,
                PartsPerMillion(8_000),
                "the measured difference is kept, because a reader who later cares about a \
                 smaller resolution needs it"
            );
        }
        other => {
            panic!("a difference below the resolution asked about is not a difference: {other}")
        }
    }

    // And the same data, asked about at a resolution it exceeds, is a
    // difference — the size test is against the caller's question and not
    // against a number this module chose.
    assert!(
        matches!(
            over_paired_differences(&tiny, PartsPerMillion(5_000)),
            Verdict::Differ { .. }
        ),
        "asked about at half a percent, eight tenths of one is a difference"
    );
}
