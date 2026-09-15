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

/// What this machine actually read, micro-batch by micro-batch, over a prompt of 32768
/// tokens with nothing else running — straight lines drawn between the rungs that were
/// measured. The dip at 512 is real and repeatable, and it is the shape that decides
/// whether a climb survives one bad reading or answers the rung before it.
fn as_this_machine_reads(value: u32) -> Option<f64> {
    let curve: [(u32, f64); 8] = [
        (256, 712.8),
        (512, 652.7),
        (1024, 718.6),
        (2048, 757.1),
        (4096, 780.5),
        (8192, 783.5),
        (16_384, 744.3),
        (32_768, 729.4),
    ];
    let mut held = curve.first().map(|pair| pair.1)?;
    for pair in curve.windows(2) {
        let (Some((low, at_low)), Some((high, at_high))) =
            (pair.first().copied(), pair.get(1).copied())
        else {
            continue;
        };
        if value >= low && value <= high && high > low {
            let across = f64::from(value.saturating_sub(low)) / f64::from(high.saturating_sub(low));
            held = at_high.mul_add(across, at_low * (1.0 - across));
        }
    }
    Some(held)
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
        run.get(..8),
        Some([256, 512, 1024, 2048, 4096, 8192, 16_384, 32_768].as_slice()),
        "nothing ever came back worse, so it doubled the whole way up"
    );
    // The last gap of a climb that doubles is half the span, so it gets looked into even
    // where the reading was still rising at the top: a peak inside it and a span that is
    // simply too short read the same from the ceiling. What ends it quickly is that the
    // probes find nothing better.
    for held in run.iter().skip(8) {
        assert!(
            (16_384..32_768).contains(held),
            "and what it looked at afterwards was the last gap and nothing else: {run:?}"
        );
    }
    assert!(
        run.len() <= 11,
        "which costs a few trials, not a sweep: {run:?}"
    );
    assert_eq!(
        hunt.why_it_settled(),
        Some(super::Settled::NothingGotWorse),
        "and the answer is still that the span is the thing to raise"
    );
}

#[test]
fn doubling_from_nothing_is_the_finest_step_the_setting_takes() {
    let dial = Dial::ThinkingBudget;
    let (_, run) = hunted(dial, |value| Some(f64::from(value)));
    assert_eq!(
        run.first().copied(),
        Some(0),
        "nothing is where it starts, and twice nothing is still nothing"
    );
    assert_eq!(
        run.get(1).copied(),
        Some(dial.span().finest),
        "so the rung above nothing is the smallest step there is: {run:?}"
    );
    let climbed: Vec<u32> = run
        .iter()
        .copied()
        .take_while(|held| *held <= dial.span().ceiling && held.is_power_of_two() || *held == 0)
        .collect();
    for pair in climbed.windows(2).skip(1) {
        let (Some(below), Some(above)) = (pair.first().copied(), pair.get(1).copied()) else {
            continue;
        };
        assert!(
            above == below.saturating_mul(2) || above == dial.span().ceiling,
            "every rung of the climb after that is a doubling, or the top of the span: \
             {climbed:?}"
        );
    }
}

#[test]
fn the_first_round_after_the_turn_is_halfway_to_each_value_beside_the_peak() {
    let (_, run) = hunted(Dial::MicroBatch, peaking_at(2048));
    assert_eq!(
        run.get(..6),
        Some([256, 512, 1024, 2048, 4096, 8192].as_slice()),
        "it doubles until 4096 comes back worse than 2048, and once more to be sure: {run:?}"
    );
    assert_eq!(
        run.get(6..8),
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
    let climb: Vec<u32> = vec![256, 512, 1024, 2048, 4096, 8192];
    for rung in &climb {
        assert!(run.contains(rung), "the climb up to the turn: {run:?}");
    }
    assert!(
        run.contains(&8192) && !run.contains(&16_384),
        "4096 came back worse than 2048, and 8192 confirms it — one rung to be sure, not a \
         climb to the top of the span: {run:?}"
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
fn one_value_that_could_not_be_measured_does_not_end_a_climb_and_two_do() {
    let (one_gone, run) = hunted(Dial::MicroBatch, |value| {
        (value != 1024).then(|| f64::from(value))
    });
    assert!(
        run.contains(&2048) && run.contains(&4096),
        "one value that would not run is one bad reading, and a climb that ended on one of \
         those would have stopped at 1024, wherever the machine happened to hiccup: {run:?}"
    );
    let _phase = one_gone.phase();

    let (both_gone, run) = hunted(Dial::MicroBatch, |value| {
        (!(1024..=2048).contains(&value)).then(|| f64::from(value))
    });
    assert_eq!(
        both_gone.phase(),
        Phase::Closing,
        "two in a row is the setting, not the machine: {run:?}"
    );
    assert!(
        !run.contains(&4096),
        "and there is nothing above two values that would not run worth climbing to: {run:?}"
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
        // every rung the climb can turn on, not a few likely ones: the worst case is the
        // thing being bounded, and it does not announce which value it lives at.
        let mut aims = vec![span.floor, span.ceiling];
        let mut at = dial.climbs_from();
        aims.push(at);
        while at < span.ceiling && aims.len() < 64 {
            at = dial.climbs_to(at);
            aims.push(at);
        }
        for aim in aims {
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

#[test]
fn a_search_says_which_of_the_three_ways_it_stopped() {
    use super::Settled;
    let (nothing_worse, _) = hunted(Dial::MicroBatch, |value| Some(f64::from(value)));
    assert_eq!(
        nothing_worse.why_it_settled(),
        Some(Settled::NothingGotWorse),
        "a search that ran out of span, on a reading still climbing at the top of it, has \
         not been shown a peak — and saying so is what tells somebody the span is the thing \
         to raise"
    );
    assert!(
        nothing_worse.said().contains("raise the span"),
        "{}",
        nothing_worse.said()
    );

    let (too_close, run) = hunted(Dial::MicroBatch, as_this_machine_reads);
    assert_eq!(
        too_close.why_it_settled(),
        Some(Settled::TooCloseToTell),
        "a round that came back with nothing better than the round before it is where a \
         search has converged: {run:?}"
    );
    assert!(
        too_close
            .said()
            .contains("as close as one take can place it"),
        "and it says so, because how much to trust the answer is part of the answer: {}",
        too_close.said()
    );

    // All three have to be reachable, or one of them is a branch nobody takes.
    let mut seen = std::collections::BTreeSet::new();
    for dial in Dial::ALL {
        let span = dial.span();
        let mut aim = span.floor;
        while aim <= span.ceiling {
            let (hunt, _) = hunted(dial, scoring_around(aim, span));
            if let Some(why) = hunt.why_it_settled() {
                let _first = seen.insert(format!("{why:?}"));
            }
            let (hunt, _) = hunted(dial, peaking_at(aim));
            if let Some(why) = hunt.why_it_settled() {
                let _first = seen.insert(format!("{why:?}"));
            }
            aim = aim.saturating_add(span.finest.max(1));
        }
    }
    assert_eq!(
        seen.len(),
        3,
        "every way a search can stop has to be a way it does stop: {seen:?}"
    );
}
#[test]
fn the_search_finds_the_peak_of_a_curve_with_a_real_dip_in_it() {
    let (hunt, run) = hunted(Dial::MicroBatch, as_this_machine_reads);
    let best = run
        .iter()
        .copied()
        .filter_map(|held| as_this_machine_reads(held).map(|score| (held, score)))
        .max_by(|one, two| one.1.total_cmp(&two.1))
        .map(|(held, _)| held);
    assert_eq!(
        best,
        Some(8192),
        "the best reading on this curve is at 8192, and a search that stops short of it has \
         answered the shape of its own rule rather than the shape of the machine: {run:?}"
    );
    assert!(
        run.contains(&512),
        "512 is the dip, and the climb walks through it rather than round it: {run:?}"
    );
    assert!(
        run.len() <= 12,
        "and surviving the dip costs one trial, not a sweep nobody waits out: {run:?}"
    );
    assert!(hunt.settled(), "{}", hunt.said());
}

#[test]
fn nothing_a_search_says_says_the_same_thing_twice() {
    for dial in Dial::ALL {
        let (hunt, _) = hunted(dial, peaking_at(dial.climbs_from()));
        let said = hunt.said();
        assert!(
            !said.contains("settled: settled") && !said.matches("round").count().gt(&1),
            "{}: {said}",
            dial.label()
        );
    }
}

/// What a search is for: landing on a value as good as the best there is. Not on the exact
/// value — where two readings are alike within what one take can tell apart, so is
/// everything between them, and a search that kept splitting them would be measuring the
/// noise. Scored the way a marked set scores, out of a hundred, so that "alike" means what
/// it means in the table.
#[test]
fn a_search_lands_on_a_value_as_good_as_the_best_there_is() {
    for dial in [Dial::TopP, Dial::TopK, Dial::Temperature, Dial::MicroBatch] {
        let span = dial.span();
        // Every value on the grain, not the handful somebody would have thought to try: a
        // peak that sits between two rungs is the case a climb is most likely to walk past,
        // and it does not announce itself.
        let mut aims: Vec<u32> = Vec::new();
        let mut aim = span.floor;
        while aim <= span.ceiling {
            aims.push(aim);
            aim = aim.saturating_add(span.finest.max(1));
        }
        for aim in aims {
            let scoring = scoring_around(aim, span);
            let (_, run) = hunted(dial, &scoring);
            let best = run
                .iter()
                .copied()
                .filter_map(|held| scoring(held).map(|score| (held, score)))
                .max_by(|one, two| one.1.total_cmp(&two.1))
                .map(|(held, _)| held);
            let Some(best) = best else {
                panic!("{} found nothing aiming at {aim}", dial.label());
            };
            let (Some(there), Some(here)) = (scoring(aim), scoring(best)) else {
                continue;
            };
            // What the search promises: it stops when a round gains less than the margin,
            // and it cannot resolve finer than the setting's own grain. So it may finish
            // that much short of the best and no more.
            let width = f64::from(span.ceiling.saturating_sub(span.floor).max(1));
            let a_step = 30.0 * f64::from(span.finest.max(1)) / width;
            let margin = there.abs() / f64::from(super::Hunt::AS_GOOD) + a_step;
            assert!(
                there - here <= margin,
                "{} aimed at {} landed on {}, which reads {here:.1} against {there:.1} — \
                 further off than one take can tell apart: {:?}",
                dial.label(),
                dial.step_of(aim).said(),
                dial.step_of(best).said(),
                run.iter()
                    .map(|held| dial.step_of(*held).said())
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// A score out of a hundred that falls away either side of a peak: the shape a marked set
/// has, on the scale its margins are judged on.
///
/// Gentle on purpose. One step of the setting's own grain has to cost less than the margin
/// the search stops inside, or the curve is asking for a precision the search never claimed
/// and the test is about the fixture rather than the search. A real reading of a top-p is
/// far flatter than this.
fn scoring_around(peak: u32, span: crate::dial::Span) -> impl Fn(u32) -> Option<f64> {
    let width = f64::from(span.ceiling.saturating_sub(span.floor).max(1));
    move |value| {
        let away = f64::from(value.abs_diff(peak)) / width;
        Some(90.0 - 30.0 * away)
    }
}

#[test]
fn a_climb_is_short_enough_that_its_first_rungs_are_worth_the_trials() {
    for dial in Dial::ALL {
        if dial.is_named_by_the_model() {
            // Its values are places in a list the model gave, and it is never climbed.
            continue;
        }
        let span = dial.span();
        let mut at = dial.climbs_from();
        let mut rungs = 1;
        while at < span.ceiling && rungs < 64 {
            at = dial.climbs_to(at);
            rungs += 1;
        }
        let most = if dial.climbs_by() == super::super::dial::Climb::Doubling {
            9
        } else {
            7
        };
        assert!(
            rungs <= most,
            "{} climbs {rungs} rungs before it can turn, and a climb that long is a grid \
             laid out before anything was measured",
            dial.label()
        );
    }
}
