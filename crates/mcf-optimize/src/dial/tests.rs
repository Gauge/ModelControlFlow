use super::{Dial, Scale, Span, Step, Sweep};

#[test]
fn every_dial_is_a_setting_the_engine_enforces_and_nothing_else() {
    for dial in Dial::ALL {
        let flag = dial.flag().is_some();
        let field = dial.field().is_some();
        assert!(
            flag != field,
            "{} must be a launch flag the engine acts on or a sampling field it applies — \
             one of the two. A setting that only reaches the model as words in its prompt \
             is a suggestion, and a suggestion cannot be measured",
            dial.label()
        );
        assert_eq!(
            flag,
            dial.reloads_the_engine(),
            "{} reloads exactly when it is a launch flag",
            dial.label()
        );
    }
}

#[test]
fn a_step_yields_a_whole_number_or_thousandths_never_both() {
    assert_eq!(Step::Whole(4096).whole(), Some(4096));
    assert!(Step::Whole(4096).thousandths().is_none());
    assert_eq!(
        Step::Thousandths(600).thousandths(),
        Some(mcf_core::configuration::Thousandths(600))
    );
    assert!(Step::Thousandths(600).whole().is_none());
}

#[test]
fn a_thousandths_step_reads_as_a_decimal() {
    assert_eq!(Step::Thousandths(0).said(), "0");
    assert_eq!(Step::Thousandths(200).said(), "0.2");
    assert_eq!(Step::Thousandths(950).said(), "0.95");
    assert_eq!(Step::Thousandths(1000).said(), "1");
    assert_eq!(Step::Whole(4096).said(), "4096");
}

#[test]
fn a_sweep_counts_the_trials_and_the_tasks_it_will_run() {
    let sweep = Sweep {
        dial: Dial::Temperature,
        steps: vec![Step::Thousandths(0), Step::Thousandths(600)],
        sets: vec![1, 2, 3],
        repeats: 2,
    };
    assert_eq!(sweep.trials(), 12, "two values, three sets, twice over");
    assert_eq!(sweep.tasks(), 96, "eight tasks to a set");
}

#[test]
fn the_default_sweep_covers_the_whole_corpus_once() {
    let sweep = Sweep::default();
    assert_eq!(sweep.sets.len(), 8);
    assert_eq!(sweep.repeats, 1);
    assert_eq!(sweep.tasks(), sweep.steps.len() * 64);
}

#[test]
fn an_empty_sweep_says_so_rather_than_reporting_zero_trials() {
    let sweep = Sweep {
        steps: Vec::new(),
        ..Sweep::default()
    };
    assert!(
        sweep.said().starts_with("nothing to run"),
        "{}",
        sweep.said()
    );
}

#[test]
fn each_dial_suggests_values_of_its_own_kind() {
    for dial in Dial::ALL {
        let steps = dial.suggested();
        assert!(!steps.is_empty(), "{} suggests something", dial.label());
        let decimal = matches!(dial, Dial::Temperature | Dial::TopP);
        for step in steps {
            assert_eq!(
                matches!(step, Step::Thousandths(_)),
                decimal,
                "{} suggests values of one kind",
                dial.label()
            );
        }
    }
}

#[test]
fn every_dial_starts_its_climb_inside_its_own_span() {
    for dial in Dial::ALL {
        let span = dial.span();
        let from = dial.climbs_from();
        assert!(
            span.holds(from),
            "{} starts climbing at {from}, outside its own span",
            dial.label()
        );
    }
}

#[test]
fn a_climb_goes_up_every_rung_and_stops_at_the_top() {
    for dial in Dial::ALL {
        let span = dial.span();
        let mut at = dial.climbs_from();
        let mut rungs = 1;
        while at < span.ceiling {
            let next = dial.climbs_to(at);
            assert!(
                next > at,
                "{} does not get above {at}, so a climb would stand still there",
                dial.label()
            );
            assert!(
                span.holds(next),
                "{} climbs from {at} to {next}, outside its own span",
                dial.label()
            );
            at = next;
            rungs += 1;
            assert!(rungs < 64, "{} never reaches its ceiling", dial.label());
        }
        assert_eq!(
            at,
            span.ceiling,
            "{} climbs to {at} and stops short of its own ceiling",
            dial.label()
        );
    }
}

#[test]
fn a_step_carries_the_scale_its_dial_reads_in() {
    for dial in Dial::ALL {
        let made = dial.step_of(100);
        match dial.scale() {
            Scale::Whole => assert_eq!(made.whole(), Some(100)),
            Scale::Thousandths => assert!(made.thousandths().is_some()),
        }
        assert!(
            matches!(
                (dial.step_of(dial.climbs_from()), made),
                (Step::Whole(_), Step::Whole(_)) | (Step::Thousandths(_), Step::Thousandths(_))
            ),
            "{} mixes scales within one search",
            dial.label()
        );
    }
}

#[test]
fn a_span_rounds_to_something_it_could_actually_run() {
    let span = Span::new(64, 4096, 16);
    assert_eq!(
        span.rounded(255),
        256,
        "255 is not a multiple of 16 above 64"
    );
    assert_eq!(span.rounded(0), 64, "below the floor is the floor");
    assert_eq!(
        span.rounded(99_999),
        4096,
        "above the ceiling is the ceiling"
    );
    assert_eq!(
        span.rounded(1024),
        1024,
        "a value already on the grid stays"
    );
}

#[test]
fn rounding_counts_from_nothing_so_a_coarse_grain_lands_on_round_numbers() {
    let span = Span::new(500, 1000, 10);
    assert_eq!(span.rounded(953), 950);
    assert_eq!(span.rounded(957), 960);

    let coarse = Span::new(64, 8192, 256);
    assert_eq!(
        coarse.rounded(1000),
        1024,
        "counted from a floor of 64 this would land on 1088, which is a micro-batch nobody \
         would type and no better than the one beside it"
    );
    assert_eq!(coarse.rounded(2100), 2048);
    assert_eq!(
        coarse.rounded(100),
        64,
        "the floor is still the floor, however the grain falls"
    );
}

/// A budget is only worth measuring if there is room left to answer in afterwards. A model
/// given the whole of a trial's room thinks until the trial stops and never writes a word,
/// and that costs what every other trial costs while saying nothing.
#[test]
fn no_thinking_budget_reaches_past_the_room_a_trial_has_to_answer_in() {
    let most = Dial::ThinkingBudget.span().ceiling;
    assert!(
        most.saturating_mul(2) <= crate::trial::TOKENS_ANSWERED,
        "a budget of {most} inside a trial of {} leaves too little to answer in",
        crate::trial::TOKENS_ANSWERED
    );
    assert!(
        most >= 4096,
        "and it still has to reach far enough up to be worth searching over"
    );
}

/// A climb walks the rungs a person would have picked by hand. Doubling suits a setting
/// whose useful values are spread over orders of magnitude and suits nothing else: through a
/// temperature it spends three of its first four rungs inside a tenth of the scale, where
/// nothing an answer does can be told apart, and then steps from 0.4 straight over 0.6.
#[test]
fn a_short_even_scale_is_climbed_in_even_steps() {
    let rungs = |dial: Dial| {
        let mut at = dial.climbs_from();
        let mut held = vec![at];
        while at < dial.span().ceiling && held.len() < 40 {
            at = dial.climbs_to(at);
            held.push(at);
        }
        held
    };
    assert_eq!(
        rungs(Dial::Temperature),
        vec![0, 200, 400, 600, 800, 1000],
        "which reads 0, 0.2, 0.4, 0.6, 0.8, 1"
    );
    assert_eq!(
        rungs(Dial::TopP),
        vec![500, 600, 700, 800, 900, 1000],
        "which reads 0.5, 0.6, 0.7, 0.8, 0.9, 1"
    );
    assert_eq!(rungs(Dial::TopK), vec![0, 40, 80, 120, 160, 200]);
    assert_eq!(
        rungs(Dial::MicroBatch),
        vec![256, 512, 1024, 2048, 4096, 8192, 16_384, 32_768],
        "a span of orders of magnitude is still doubled: stepping it evenly would be a \
         hundred and twenty rungs"
    );
    assert_eq!(
        rungs(Dial::ThinkingBudget),
        vec![0, 256, 512, 1024, 2048, 4096, 8192],
        "and nothing doubled is still nothing, so the rung above it is the finest step"
    );
}
