use mcf_core::measurement::PartsPerMillion;

use super::{Verdict, over_paired_differences, over_separate_arms};

const FIVE: PartsPerMillion = PartsPerMillion(50_000);
const TWENTY: PartsPerMillion = PartsPerMillion(200_000);
const TWO: PartsPerMillion = PartsPerMillion(20_000);

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

fn differences(effect_ppm: i64, wobble_ppm: i64, count: usize) -> Vec<i64> {
    let steps: [i64; 8] = [0, 3, -2, 5, -4, 1, -5, 2];
    (0..count)
        .map(|at| {
            let step = steps.get(at % steps.len()).copied().unwrap_or(0);
            effect_ppm.saturating_add(wobble_ppm.saturating_mul(step).wrapping_div(5))
        })
        .collect()
}

#[test]
fn a_real_paired_effect_is_found() {
    match over_paired_differences(&differences(300_000, 40_000, 20), FIVE) {
        Verdict::Differ { by, by_chance, .. } => {
            assert!(by.low.0 > 250_000, "a thirty-percent effect: {by:?}");
            assert!(by.high.0 >= by.low.0, "an interval is ordered: {by:?}");
            assert!(by_chance <= super::FALSE_ALARMS_ALLOWED, "{by_chance:?}");
        }
        other => panic!("a thirty-percent paired effect was not seen: {other}"),
    }
}

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

#[test]
fn noise_wider_than_the_question_is_not_yet_decided() {
    let held = over_paired_differences(&differences(5_000, 400_000, 6), TWO);
    assert!(
        matches!(held, Verdict::NotYet { .. }),
        "differences noisier than the question must not be called the same: {held}"
    );
}

#[test]
fn repeating_resolves_what_a_few_pairs_could_not() {
    let early = over_paired_differences(&differences(60_000, 150_000, 4), FIVE);
    assert!(
        matches!(early, Verdict::NotYet { .. }),
        "four pairs should not settle it: {early}"
    );
    let later = over_paired_differences(&differences(60_000, 150_000, 60), FIVE);
    assert!(
        matches!(later, Verdict::Ordered { .. }),
        "sixty should settle the order: {later}"
    );
    let clean = over_paired_differences(&differences(300_000, 20_000, 30), FIVE);
    let Verdict::Differ { by, .. } = clean else {
        panic!("a thirty-percent effect under two-percent noise settles the size: {clean}");
    };
    assert!(
        by.clears(FIVE),
        "and the whole interval clears the resolution asked about: {by:?}"
    );
}

#[test]
fn the_answer_carries_what_it_cost() {
    let Verdict::Differ { after, .. } =
        over_paired_differences(&differences(400_000, 30_000, 12), FIVE)
    else {
        panic!("a fourfold-sized effect was not seen");
    };
    assert_eq!(after, 12, "the pairs it took travel with the verdict");
}

#[test]
fn one_pair_decides_nothing() {
    assert!(matches!(
        over_paired_differences(&[900_000], FIVE),
        Verdict::NotYet { so_far: 1 }
    ));
}

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

#[test]
fn the_pooled_null_finds_a_real_difference() {
    let slow = around(2_000_000_000, 20_000, 20);
    let fast = around(1_000_000_000, 20_000, 20);
    match over_separate_arms(&slow, &fast, FIVE) {
        Verdict::Apart { by, .. } => assert!(by.low.0 > 500_000, "a doubling: {by:?}"),
        other => panic!("a doubling was not seen: {other}"),
    }
}

#[test]
fn a_zero_duration_is_not_divided_by() {
    assert!(matches!(
        over_separate_arms(&[0, 0, 0], &[5, 5, 5], FIVE),
        Verdict::NotYet { .. }
    ));
}

#[test]
fn a_null_result_is_not_declared_where_a_difference_could_not_have_been() {
    let real: [i64; 4] = [129_000, -74_000, -2_000, -14_000];
    let held = over_paired_differences(&real, FIVE);
    assert!(
        matches!(held, Verdict::NotYet { so_far: 4 }),
        "four pairs cannot reach one in twenty, so they cannot declare anything: {held}"
    );
}

#[test]
fn the_same_noise_decides_once_there_are_enough_pairs_of_it() {
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

#[test]
fn identical_arms_are_the_same_rather_than_undecided() {
    let held = over_paired_differences(&[0; 40], FIVE);
    assert!(
        matches!(held, Verdict::Same { .. }),
        "arms that ran to the same nanosecond forty times are the same: {held}"
    );
}

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

#[test]
fn ties_leave_the_count_but_not_the_record_of_having_run() {
    let mut differences = vec![100_000_i64; 8];
    differences.extend([0, 0, 0, 0]);
    let Verdict::Ordered { after, by, .. } = over_paired_differences(&differences, FIVE) else {
        panic!("eight pairs won by one arm separate them: {differences:?}");
    };
    assert_eq!(after, 12, "every pair that ran is counted in what it cost");
    assert_eq!(
        by.low,
        mcf_core::measurement::PartsPerMillion(0),
        "four ties in twelve put no floor under the size: {by:?}"
    );
}

#[test]
fn a_real_difference_smaller_than_the_question_is_a_null_result() {
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

    assert!(
        matches!(
            over_paired_differences(&tiny, PartsPerMillion(5_000)),
            Verdict::Differ { .. }
        ),
        "asked about at half a percent, eight tenths of one is a difference"
    );
}

mod spreads {
    use super::super::{Spread, WANTED_COVERAGE, spread_of};
    use mcf_core::measurement::PartsPerMillion;

    #[test]
    fn six_pairs_reach_the_extremes_at_ninety_six_point_nine() {
        let held = spread_of(&[
            -1_508_000, -1_400_000, -1_332_000, -1_300_000, -1_200_000, -1_281_000,
        ])
        .expect("six pairs support an interval");
        assert_eq!(held.coverage, PartsPerMillion(968_750));
        assert_eq!(held.low, PartsPerMillion(1_200_000));
        assert_eq!(held.high, PartsPerMillion(1_508_000));
    }

    #[test]
    fn under_six_pairs_there_is_no_interval() {
        for count in 0..6_usize {
            let differences: Vec<i64> = (0..count)
                .map(|at| 100_000 + i64::try_from(at).unwrap_or(0))
                .collect();
            assert!(
                spread_of(&differences).is_none(),
                "{count} pairs cannot reach {} coverage, and a narrower one stated as this one \
                 would be a weaker claim wearing a stronger label",
                WANTED_COVERAGE.0
            );
        }
    }

    #[test]
    fn disagreeing_pairs_get_a_floor_of_nothing() {
        let held = spread_of(&[-400_000, -300_000, -100_000, 50_000, 200_000, 300_000])
            .expect("six pairs");
        assert_eq!(
            held.low,
            PartsPerMillion(0),
            "pairs that disagree about direction cannot bound the size away from nothing, and \
             taking absolute values first would manufacture a floor out of the disagreement"
        );
    }

    #[test]
    fn an_interval_starting_below_the_resolution_does_not_clear_it() {
        let straddling = Spread {
            low: PartsPerMillion(42_000),
            high: PartsPerMillion(3_494_000),
            coverage: PartsPerMillion(968_750),
        };
        assert!(!straddling.clears(PartsPerMillion(50_000)));
        assert!(straddling.clears(PartsPerMillion(40_000)));
    }

    #[test]
    fn the_saturated_run_is_wide_and_says_so() {
        let held = spread_of(&[
            -4_717_000, -1_820_000, -1_450_000, -1_378_000, -1_140_000, -1_090_000, -750_000,
            -335_000, 1_445_000,
        ])
        .expect("nine pairs");
        assert_eq!(held.coverage, PartsPerMillion(960_937));
        assert_eq!(
            (held.low, held.high),
            (PartsPerMillion(335_000), PartsPerMillion(1_820_000)),
            "reported as a flat *by 114.0%*, the evidence actually spans 33.5% to 182% — a \
             fivefold range presented as one number to a decimal place"
        );
    }
}

mod unpaired {
    use super::super::{rank_sum_counts, spread_of_separate};

    #[test]
    fn the_distribution_matches_a_brute_force_enumeration() {
        for n in 1_usize..=6 {
            for m in 1_usize..=6 {
                let held = rank_sum_counts(n, m).expect("small arms are countable");
                let total = n + m;
                let mut counted = vec![0_u128; n * m + 1];
                for pattern in 0_u32..(1 << total) {
                    if usize::try_from(pattern.count_ones()).unwrap_or(0) != m {
                        continue;
                    }
                    let mut seen_first = 0_usize;
                    let mut statistic = 0_usize;
                    for at in 0..total {
                        if pattern & (1 << at) == 0 {
                            seen_first += 1;
                        } else {
                            statistic += n - seen_first;
                        }
                    }
                    counted[statistic] += 1;
                }
                assert_eq!(
                    held, counted,
                    "at n={n}, m={m} the recurrence and the enumeration disagree"
                );
            }
        }
    }

    #[test]
    fn the_distribution_is_complete() {
        for (n, m, expected) in [(3_usize, 3_usize, 20_u128), (4, 6, 210), (8, 8, 12_870)] {
            let held: u128 = rank_sum_counts(n, m).expect("countable").iter().sum();
            assert_eq!(held, expected, "n={n}, m={m} must sum to C(n+m, n)");
        }
    }

    #[test]
    fn the_distribution_is_symmetric() {
        let held = rank_sum_counts(5, 7).expect("countable");
        let reversed: Vec<u128> = held.iter().copied().rev().collect();
        assert_eq!(held, reversed);
    }

    #[test]
    fn two_separated_arms_get_an_interval() {
        let slow: Vec<u64> = (0..8).map(|at| 2_000_000_000 + at * 1_000_000).collect();
        let quick: Vec<u64> = (0..8).map(|at| 1_000_000_000 + at * 1_000_000).collect();
        let held = spread_of_separate(&slow, &quick).expect("sixteen values support an interval");
        assert!(held.low.0 > 0, "the arms plainly differ: {held:?}");
        assert!(
            held.high.0 >= held.low.0,
            "an interval is ordered: {held:?}"
        );
        assert!(
            held.coverage.0 >= super::super::WANTED_COVERAGE.0,
            "and reaches the standard asked for: {held:?}"
        );
    }

    #[test]
    fn two_alike_arms_reach_zero() {
        let one: Vec<u64> = (0..8).map(|at| 1_000_000_000 + at * 1_000_000).collect();
        let other: Vec<u64> = (0..8).map(|at| 1_000_500_000 + at * 1_000_000).collect();
        let held = spread_of_separate(&one, &other).expect("sixteen values");
        assert_eq!(
            held.low,
            mcf_core::measurement::PartsPerMillion(0),
            "overlapping arms cannot bound the size away from nothing: {held:?}"
        );
    }

    #[test]
    fn tiny_arms_get_no_interval() {
        assert!(spread_of_separate(&[1_000], &[2_000]).is_none());
        assert!(spread_of_separate(&[], &[1, 2, 3]).is_none());
    }
}
