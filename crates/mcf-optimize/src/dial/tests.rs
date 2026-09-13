use super::{Dial, Scale, Span, Step, Sweep};

#[test]
fn every_dial_reaches_the_model_exactly_one_way() {
    for dial in Dial::ALL {
        let ways = usize::from(dial.flag().is_some())
            + usize::from(dial.field().is_some())
            + usize::from(dial.template_kwarg().is_some());
        assert_eq!(
            ways,
            1,
            "{} must be a launch flag, a request field or a template switch — one of the \
             three, never two and never none",
            dial.label()
        );
        assert_eq!(
            dial.flag().is_some(),
            dial.reloads_the_engine(),
            "{} reloads exactly when it is a launch flag",
            dial.label()
        );
    }
}

#[test]
fn a_thinking_level_is_a_word_the_template_reads_and_a_number_the_record_keeps() {
    for (at, word) in super::LEVELS.iter().enumerate() {
        let step = Dial::ThinkingEffort.step_of(u32::try_from(at).unwrap_or(0));
        assert_eq!(&Dial::ThinkingEffort.said(step), word);
        assert_eq!(Dial::ThinkingEffort.read(word), Some(step));
        assert_eq!(
            Dial::ThinkingEffort.read(&word.to_uppercase()),
            Some(step),
            "a level typed in capitals is the same level"
        );
    }
    assert_eq!(Dial::ThinkingEffort.read("enormous"), None);
}

#[test]
fn a_dial_that_is_not_about_thinking_reads_its_values_as_numbers() {
    for dial in Dial::ALL
        .into_iter()
        .filter(|dial| *dial != Dial::ThinkingEffort)
    {
        assert_eq!(dial.read("low"), None, "{}", dial.label());
        assert_eq!(
            dial.said(dial.step_of(2)),
            dial.step_of(2).said(),
            "{} renders its values as it always did",
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
fn every_dial_offers_a_coarse_ladder_inside_its_own_span() {
    for dial in Dial::ALL {
        let span = dial.span();
        let coarse = dial.coarse();
        assert!(
            coarse.len() >= 3,
            "{} offers too few coarse points to halve between",
            dial.label()
        );
        for step in &coarse {
            let value = match *step {
                Step::Whole(held) | Step::Thousandths(held) => held,
            };
            assert!(
                span.holds(value),
                "{} offers {value}, outside its own span",
                dial.label()
            );
        }
    }
}

#[test]
fn a_coarse_ladder_is_ordered_and_has_no_repeats() {
    for dial in Dial::ALL {
        let values: Vec<u32> = dial
            .coarse()
            .iter()
            .map(|step| match *step {
                Step::Whole(held) | Step::Thousandths(held) => held,
            })
            .collect();
        let mut sorted = values.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(values, sorted, "{} repeats or misorders", dial.label());
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
            dial.coarse().iter().all(|step| matches!(
                (step, made),
                (Step::Whole(_), Step::Whole(_)) | (Step::Thousandths(_), Step::Thousandths(_))
            )),
            "{} mixes scales within one ladder",
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
fn rounding_counts_from_the_floor_not_from_zero() {
    let span = Span::new(500, 1000, 10);
    assert_eq!(span.rounded(953), 950);
    assert_eq!(span.rounded(957), 960);
}
