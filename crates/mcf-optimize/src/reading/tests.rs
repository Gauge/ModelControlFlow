use super::{Ending, Reading, Report};
use crate::dial::{Dial, Step};

fn reading(step: u32, set: usize, passed: u32, ending: Ending) -> Reading {
    Reading {
        dial: Dial::ThinkingBudget,
        step: Step::Whole(step),
        set,
        repeat: 0,
        passed,
        of: 8,
        produced: 10_000,
        milliseconds: 100_000,
        ending,
        per_task: Vec::new(),
    }
}

#[test]
fn readings_gather_into_one_summary_for_each_value() {
    let mut report = Report::default();
    report.record(reading(1024, 1, 5, Ending::Answered));
    report.record(reading(1024, 2, 3, Ending::Looped));
    report.record(reading(4096, 1, 6, Ending::Answered));
    let held = report.by_step();
    assert_eq!(held.len(), 2, "two values were swept");
    let Some(first) = held.iter().find(|s| s.step == Step::Whole(1024)) else {
        panic!("the 1024 summary is present");
    };
    assert_eq!(first.trials, 2);
    assert_eq!((first.passed, first.of), (8, 16));
    assert_eq!(first.runaways, 1, "the looped trial is a runaway");
    assert_eq!(first.share(), Some(50.0));
}

#[test]
fn the_best_value_is_the_one_that_answered_most() {
    let mut report = Report::default();
    report.record(reading(512, 1, 2, Ending::Answered));
    report.record(reading(4096, 1, 6, Ending::Answered));
    let Some(best) = report.best() else {
        panic!("a best value exists once anything is recorded");
    };
    assert_eq!(best.step, Step::Whole(4096));
}

#[test]
fn a_trial_that_answered_nothing_reports_no_cost_an_answer_rather_than_zero() {
    let held = reading(1024, 1, 0, Ending::Filled);
    assert_eq!(held.tokens_an_answer(), None);
    assert_eq!(held.seconds_an_answer(), None);
    assert_eq!(held.tokens_a_second(), Some(100.0));
}

#[test]
fn a_runaway_is_a_loop_or_a_filled_budget_and_nothing_else() {
    assert!(Ending::Looped.is_a_runaway());
    assert!(Ending::Filled.is_a_runaway());
    assert!(!Ending::Answered.is_a_runaway());
    assert!(!Ending::Failed.is_a_runaway());
}

#[test]
fn an_empty_report_offers_no_best_and_no_rows() {
    let report = Report::default();
    assert!(report.best().is_none());
    assert!(report.to_rows().is_empty());
    assert_eq!(Report::COLUMNS.len(), 9);
}

#[test]
fn a_row_carries_one_cell_for_each_column() {
    let mut report = Report::default();
    report.record(reading(2048, 4, 7, Ending::Answered));
    for row in report.to_rows() {
        assert_eq!(row.len(), Report::COLUMNS.len());
    }
}
