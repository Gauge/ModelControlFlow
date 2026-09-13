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
/// picture so far goes back to it each round. What comes out is the hunt and every value it
/// ever asked for, in the order it asked.
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
    (hunt, values(&run))
}

/// Better up to a point and worse after it, which is what a real setting does.
fn peaking_at(peak: u32) -> impl Fn(u32) -> Option<f64> {
    move |value| Some(-f64::from(value.abs_diff(peak)))
}

#[test]
fn every_automatic_search_opens_on_one_value_and_doubles_from_there() {
    for dial in Dial::ALL {
        let hunt = Hunt::started(dial);
        assert_eq!(
            values(&hunt.asked()),
            vec![dial.climbs_from()],
            "{} lays out a grid before it has measured anything",
            dial.label()
        );
        assert_eq!(hunt.phase(), Phase::Climbing, "{}", dial.label());
        assert_eq!(hunt.round(), 1);
        assert!(!hunt.settled());
    }
}

#[test]
fn a_search_starts_where_the_setting_is_off_so_that_off_is_tried_at_all() {
    for dial in [
        Dial::ThinkingBudget,
        Dial::Temperature,
        Dial::TopK,
        Dial::DraftDepth,
    ] {
        assert_eq!(
            dial.climbs_from(),
            dial.span().floor,
            "{} turns off at the bottom of its span, and a search that started above it \
             would never try it",
            dial.label()
        );
    }
    assert_eq!(
        Dial::MicroBatch.climbs_from(),
        256,
        "a pass smaller than this is slower than it is worth holding the model again to \
         measure"
    );
}

#[test]
fn a_climb_doubles_and_keeps_doubling_while_each_value_beats_the_one_below_it() {
    let (hunt, run) = hunted(Dial::MicroBatch, |value| Some(f64::from(value)));
    assert_eq!(
        run,
        vec![256, 512, 1024, 2048, 4096, 8192, 16_384, 32_768],
        "nothing ever came back worse, so there was never anything to close in on and the \
         top of the span is the answer"
    );
    assert_eq!(hunt.phase(), Phase::Climbing);
    assert!(hunt.settled());
}

#[test]
fn doubling_from_nothing_is_the_finest_step_the_setting_takes() {
    let (_, run) = hunted(Dial::TopK, |value| Some(f64::from(value)));
    assert_eq!(
        run.first().copied(),
        Some(0),
        "nothing is where it starts, and twice nothing is still nothing"
    );
    assert_eq!(
        run.get(1).copied(),
        Some(Dial::TopK.span().finest),
        "so the rung above nothing is the smallest step there is: {run:?}"
    );
    for pair in run.windows(2).skip(1) {
        let (below, above) = (pair[0], pair[1]);
        assert!(
            above == below.saturating_mul(2) || above == Dial::TopK.span().ceiling,
            "every rung after that is a doubling, or the top of the span: {run:?}"
        );
    }
}

#[test]
fn the_first_round_after_the_turn_is_halfway_to_each_value_beside_the_peak() {
    let (_, run) = hunted(Dial::MicroBatch, peaking_at(2048));
    assert_eq!(
        run.get(..5),
        Some([256, 512, 1024, 2048, 4096].as_slice()),
        "it doubles until 4096 comes back worse than 2048: {run:?}"
    );
    assert_eq!(
        run.get(5..7),
        Some([1536, 3072].as_slice()),
        "then the first round after the turn is 2048 - (2048 - 1024) / 2 below and \
         2048 + (4096 - 2048) / 2 above, and nothing else: {run:?}"
    );
}

#[test]
fn halving_cuts_the_bracket_the_two_values_beside_the_peak_make() {
    let (hunt, run) = hunted(Dial::MicroBatch, peaking_at(2048));
    assert_eq!(hunt.phase(), Phase::Closing);
    assert!(hunt.settled());
    let climb: Vec<u32> = vec![256, 512, 1024, 2048, 4096];
    for rung in &climb {
        assert!(run.contains(rung), "the climb up to the turn: {run:?}");
    }
    assert!(
        !run.contains(&8192),
        "4096 came back worse than 2048, so there was no reason to go on doubling: {run:?}"
    );
    assert!(
        run.contains(&3072) && run.contains(&1536),
        "halfway to 4096 above and halfway to 1024 below are the first two it asks for, \
         which is peak + (beyond - peak) / 2 and peak - (peak - before) / 2: {run:?}"
    );
    let mut closing: Vec<u32> = run
        .iter()
        .copied()
        .filter(|held| !climb.contains(held))
        .collect();
    closing.sort_unstable();
    for held in &closing {
        assert!(
            (1024..=4096).contains(held),
            "nothing outside the bracket can be the answer once both sides came back \
             worse, so nothing outside it is asked for: {closing:?}"
        );
    }
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
fn a_search_settles_without_asking_for_any_value_twice_or_stepping_outside_its_span() {
    for dial in Dial::ALL {
        let span = dial.span();
        for aim in [span.floor, dial.climbs_from(), span.ceiling] {
            let (hunt, run) = hunted(dial, peaking_at(aim));
            assert!(
                hunt.settled(),
                "{} aiming at {aim} does not settle: {run:?}",
                dial.label()
            );
            let mut seen = run.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(
                seen.len(),
                run.len(),
                "{} aiming at {aim} asked for a value twice: {run:?}",
                dial.label()
            );
            for value in &run {
                assert!(
                    span.holds(*value),
                    "{} proposed {value} aiming at {aim}, outside {} to {}",
                    dial.label(),
                    span.floor,
                    span.ceiling
                );
            }
        }
    }
}

#[test]
fn a_settled_hunt_asks_for_nothing_more() {
    let (mut hunt, run) = hunted(Dial::DraftDepth, peaking_at(4));
    assert!(hunt.settled());
    let steps: Vec<Step> = run.iter().map(|held| Step::Whole(*held)).collect();
    let scored: Vec<(Step, Option<f64>)> = steps.iter().map(|step| (*step, Some(1.0))).collect();
    assert!(hunt.stepped_on(Step::Whole(4), &scored, &steps).is_empty());
}

/// What an automatic search asks for, end to end, counted rather than guessed at. Every
/// value of a setting that reloads the engine costs a held model as well as a trial, so
/// this count is the sweep's running time, and it is the thing a rough search is rough for.
#[test]
fn an_automatic_search_of_any_setting_asks_for_few_enough_values_to_sit_through() {
    const AT_MOST: usize = 20;
    for dial in Dial::ALL {
        let span = dial.span();
        for aim in [span.floor, dial.climbs_from(), span.ceiling] {
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
fn a_micro_batch_search_can_reach_past_the_prompt_it_is_timed_over_only_if_the_prompt_grows() {
    assert!(
        crate::trial::TOKENS_PREFILLED >= Dial::MicroBatch.span().ceiling,
        "a pass takes as much of the prompt as it can hold, so every micro-batch at or \
         above the length of the prompt is the same single pass. A span that reaches past \
         the prompt is a span whose top values cannot be told apart, and a climb that \
         cannot tell them apart never finds a peak — it runs out of span instead"
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
