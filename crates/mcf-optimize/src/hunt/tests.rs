use super::{Hunt, Phase, Way};
use crate::dial::{Dial, Step};
use crate::trial::TIMES_TIMED;

fn one(step: Step) -> u32 {
    match step {
        Step::Whole(held) | Step::Thousandths(held) => held,
    }
}

fn values(steps: &[Step]) -> Vec<u32> {
    steps.iter().copied().map(one).collect()
}

/// Drives a hunt the way a sweep does: every value it asks for is measured, and the whole
/// picture so far goes back to it each round.
fn hunted(dial: Dial, score: impl Fn(u32) -> Option<f64>) -> (Hunt, Vec<u32>) {
    let mut hunt = Hunt::started(dial);
    let mut run: Vec<Step> = hunt.asked();
    for _ in 0..64 {
        let scored: Vec<(Step, Option<f64>)> =
            run.iter().map(|step| (*step, score(one(*step)))).collect();
        let Some(best) = scored
            .iter()
            .filter_map(|(step, held)| held.map(|held| (*step, held)))
            .max_by(|one, two| one.1.total_cmp(&two.1))
            .map(|(step, _)| step)
        else {
            break;
        };
        let next = hunt.stepped_on(best, &scored, &run);
        if next.is_empty() {
            break;
        }
        run.extend(next);
    }
    let mut held = values(&run);
    held.sort_unstable();
    (hunt, held)
}

/// Bigger is always better, right to the top of the span.
fn rising(value: u32) -> Option<f64> {
    (value > 0).then(|| f64::from(value))
}

/// Better up to a point and worse after it, which is what a real setting does.
fn peaking_at(peak: u32) -> impl Fn(u32) -> Option<f64> {
    move |value| Some(-f64::from(value.abs_diff(peak)))
}

#[test]
fn a_search_of_a_setting_that_climbs_opens_on_one_value_and_nothing_else() {
    let hunt = Hunt::started(Dial::MicroBatch);
    assert_eq!(
        values(&hunt.asked()),
        vec![256],
        "there is nothing to learn from a ladder laid out before anything was measured, so \
         a climb starts on one rung"
    );
    assert_eq!(hunt.phase(), Phase::Climbing);
    assert_eq!(hunt.round(), 1);
    assert!(!hunt.settled());
}

#[test]
fn a_climb_doubles_and_keeps_doubling_while_each_value_beats_the_one_below_it() {
    let (hunt, run) = hunted(Dial::MicroBatch, rising);
    assert_eq!(
        run,
        vec![256, 512, 1024, 2048, 4096, 8192],
        "nothing ever came back worse, so there was never anything to close in on and the \
         top of the span is the answer"
    );
    assert_eq!(hunt.phase(), Phase::Climbing);
    assert!(hunt.settled());
}

#[test]
fn a_climb_starts_halving_the_first_time_a_value_comes_back_worse_and_not_before() {
    let (hunt, run) = hunted(Dial::MicroBatch, peaking_at(1024));
    assert_eq!(hunt.phase(), Phase::Closing);
    assert!(hunt.settled());
    for rung in [256, 512, 1024, 2048] {
        assert!(run.contains(&rung), "the climb up to the turn: {run:?}");
    }
    assert!(
        !run.contains(&4096) && !run.contains(&8192),
        "2048 came back worse than 1024, so there was no reason to go on doubling: {run:?}"
    );
    assert!(
        run.iter().any(|held| (1024..2048).contains(held)),
        "the best is somewhere between the last rung that improved and the one that did \
         not, and that is where the halving looks: {run:?}"
    );
}

#[test]
fn a_value_that_could_not_be_measured_ends_the_climb_rather_than_being_climbed_past() {
    let (hunt, run) = hunted(Dial::MicroBatch, |value| {
        (value != 1024).then(|| f64::from(value))
    });
    assert_eq!(hunt.phase(), Phase::Closing);
    assert!(
        !run.contains(&2048),
        "a value with no reading to score is not a value to climb past: {run:?}"
    );
}

#[test]
fn a_climb_asks_for_no_value_twice_and_settles_rather_than_going_on_forever() {
    for dial in Dial::ALL {
        let (hunt, run) = hunted(dial, peaking_at(one(dial.coarse()[1])));
        assert!(hunt.settled(), "{} does not settle", dial.label());
        let mut seen = run.clone();
        seen.dedup();
        assert_eq!(
            seen,
            run,
            "{} asked for a value twice: {run:?}",
            dial.label()
        );
        for value in &run {
            assert!(
                dial.span().holds(*value),
                "{} proposed {value}, outside its own span",
                dial.label()
            );
        }
    }
}

#[test]
fn the_opening_gap_of_a_setting_that_does_not_climb_is_the_widest_its_ladder_leaves() {
    let hunt = Hunt::started(Dial::TopK);
    assert_eq!(hunt.phase(), Phase::Closing);
    assert_eq!(values(&hunt.asked()), values(&Dial::TopK.coarse()));
    assert_eq!(
        hunt.gap(),
        100,
        "the ladder is the two ends and the middle, so 100 either side is the widest gap it \
         leaves and that is what halving starts from"
    );
}

#[test]
fn closing_in_asks_either_side_of_the_best_at_half_the_gap() {
    let mut hunt = Hunt::started(Dial::TopK);
    let ladder = Dial::TopK.coarse();
    let scored: Vec<(Step, Option<f64>)> = ladder.iter().map(|step| (*step, Some(1.0))).collect();
    let next = hunt.stepped_on(Step::Whole(100), &scored, &ladder);
    assert_eq!(
        values(&next),
        vec![50, 150],
        "half of 100 either side of the ladder's middle is somewhere new in both directions"
    );
    assert_eq!(hunt.gap(), 50);
}

#[test]
fn a_settled_hunt_asks_for_nothing_more() {
    let (mut hunt, run) = hunted(Dial::DraftDepth, peaking_at(4));
    assert!(hunt.settled());
    let steps: Vec<Step> = run.iter().map(|held| Step::Whole(*held)).collect();
    let scored: Vec<(Step, Option<f64>)> = steps.iter().map(|step| (*step, Some(1.0))).collect();
    assert!(hunt.stepped_on(Step::Whole(4), &scored, &steps).is_empty());
}

#[test]
fn a_search_never_proposes_below_the_floor_or_above_the_ceiling() {
    for dial in Dial::ALL {
        let span = dial.span();
        for aim in [span.floor, span.ceiling] {
            let (_, run) = hunted(dial, peaking_at(aim));
            for value in run {
                assert!(
                    span.holds(value),
                    "{} proposed {value} while aiming at {aim}, outside {} to {}",
                    dial.label(),
                    span.floor,
                    span.ceiling
                );
            }
        }
    }
}

/// What an automatic search asks for, end to end, counted rather than guessed at. Every
/// value of a setting that reloads the engine costs a held model as well as a trial, so
/// this count is the sweep's running time, and it is the thing a rough search is rough for.
#[test]
fn an_automatic_search_of_any_setting_asks_for_few_enough_values_to_sit_through() {
    const AT_MOST: usize = 16;
    for dial in Dial::ALL {
        let span = dial.span();
        for aim in [span.floor, one(dial.coarse()[1]), span.ceiling] {
            let (_, run) = hunted(dial, peaking_at(aim));
            assert!(
                run.len() <= AT_MOST,
                "{} works its way down to {} values aiming at {aim}, and a sweep nobody \
                 waits out is a sweep nobody runs: {run:?}",
                dial.label(),
                run.len()
            );
        }
    }
    assert_eq!(
        TIMES_TIMED, 1,
        "and a timed value is measured once, so for those the value count is the trial count"
    );
}

#[test]
fn the_automatic_way_is_the_one_offered_first() {
    assert_eq!(
        Way::ALL.first(),
        Some(&Way::Halving),
        "closing in automatically is the usual way to dial something in"
    );
}
