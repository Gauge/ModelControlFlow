use super::{Dial, Step, Sweep};

#[test]
fn every_dial_is_either_an_engine_flag_or_a_request_field_never_both() {
    for dial in Dial::ALL {
        let flag = dial.flag().is_some();
        let field = dial.field().is_some();
        assert!(flag != field, "{} must be exactly one of the two", dial.label());
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
    assert!(sweep.said().starts_with("nothing to run"), "{}", sweep.said());
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
