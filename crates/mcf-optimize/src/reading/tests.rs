use super::{Ending, Measure, Reading, Report};
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

fn at(step: Step, produced: u64, milliseconds: u64, passed: u32) -> Reading {
    Reading {
        dial: Dial::MicroBatch,
        step,
        set: 1,
        repeat: 1,
        passed,
        of: 8,
        produced,
        milliseconds,
        ending: Ending::Answered,
        per_task: Vec::new(),
    }
}

#[test]
fn the_fastest_and_the_most_correct_can_be_different_values() {
    let mut report = Report::default();
    report.record(at(Step::Whole(256), 1000, 1000, 2));
    report.record(at(Step::Whole(512), 500, 1000, 8));
    assert_eq!(
        report.best_by(Measure::Speed).map(|held| held.step),
        Some(Step::Whole(256)),
        "256 produced twice the tokens in the same time"
    );
    assert_eq!(
        report.best_by(Measure::Correctness).map(|held| held.step),
        Some(Step::Whole(512)),
        "512 got more of the tasks right"
    );
}

#[test]
fn a_value_that_only_ever_ran_away_is_not_the_best_at_anything() {
    let mut report = Report::default();
    let mut looped = at(Step::Whole(256), 100_000, 1000, 0);
    looped.ending = Ending::Looped;
    report.record(looped);
    report.record(at(Step::Whole(512), 500, 1000, 4));
    assert_eq!(
        report.best_by(Measure::Speed).map(|held| held.step),
        Some(Step::Whole(512)),
        "a loop produces tokens quickly and none of them are an answer"
    );
}

#[test]
fn a_value_that_ran_away_once_out_of_several_still_counts() {
    let mut report = Report::default();
    let mut looped = at(Step::Whole(256), 1000, 1000, 0);
    looped.ending = Ending::Looped;
    report.record(looped);
    report.record(at(Step::Whole(256), 1000, 1000, 8));
    assert!(
        report.best_by(Measure::Speed).is_some(),
        "one runaway among several is a reading, not a disqualification"
    );
}

#[test]
fn an_empty_report_has_no_best_of_any_kind() {
    let report = Report::default();
    for measure in Measure::ALL {
        assert!(report.best_by(measure).is_none());
    }
}

#[test]
fn only_one_of_the_two_measures_needs_a_model_s_code_to_be_run() {
    assert!(!Measure::Speed.needs_the_answers_run());
    assert!(Measure::Correctness.needs_the_answers_run());
}

#[test]
fn a_timed_reading_is_shown_without_the_columns_that_would_be_dashes() {
    for dial in [Dial::MicroBatch, Dial::DraftDepth] {
        let columns = Report::columns_of(dial, Measure::Speed);
        for gone in ["Set", "Score", "Tok/✓"] {
            assert!(
                !columns.contains(&gone),
                "{gone} runs no set and marks no answer in a timed sweep, so it would be a \
                 column of nothing: {columns:?}"
            );
        }
        for kept in ["Value", "Take", "Seconds", "Ending"] {
            assert!(columns.contains(&kept), "{kept} is missing: {columns:?}");
        }
    }
}

#[test]
fn a_sweep_that_times_reading_names_the_tokens_it_counted_as_the_prompt() {
    let reading = Report::columns_of(Dial::MicroBatch, Measure::Speed);
    let writing = Report::columns_of(Dial::DraftDepth, Measure::Speed);
    assert!(
        reading.contains(&"Prompt") && !reading.contains(&"Tokens"),
        "a micro-batch sweep counts the prompt it read, not an answer it wrote: {reading:?}"
    );
    assert!(
        writing.contains(&"Tokens") && !writing.contains(&"Prompt"),
        "a draft head sweep counts the answer it wrote: {writing:?}"
    );
}

#[test]
fn a_reading_of_the_tasks_keeps_the_columns_about_the_answers() {
    let columns = Report::columns_of(Dial::ThinkingBudget, Measure::Correctness);
    for kept in ["Value", "Set", "Take", "Score", "Tokens", "Tok/s"] {
        assert!(columns.contains(&kept), "{kept} is missing: {columns:?}");
    }
}

/// The columns follow how the sweep is being ranked, not which setting it moves. A draft
/// head can be marked or timed, and a table that showed a score column for a timed run
/// would show a column of dashes.
#[test]
fn the_columns_follow_the_measure_rather_than_the_setting() {
    let marked = Report::columns_of(Dial::DraftDepth, Measure::Correctness);
    let timed = Report::columns_of(Dial::DraftDepth, Measure::Speed);
    assert!(
        marked.contains(&"Score") && marked.contains(&"Set"),
        "{marked:?}"
    );
    assert!(
        !timed.contains(&"Score") && !timed.contains(&"Set"),
        "{timed:?}"
    );
}

#[test]
fn a_row_has_exactly_as_many_cells_as_the_table_has_columns() {
    for dial in Dial::ALL {
        for measure in Measure::ALL {
            let held = at(dial.step_of(2), 1000, 1000, 4);
            let cells = Report::cells_of(&held, dial, measure, &[]);
            assert_eq!(
                cells.len(),
                Report::columns_of(dial, measure).len(),
                "{} ranked by {} draws {cells:?} under {:?}",
                dial.label(),
                measure.label(),
                Report::columns_of(dial, measure)
            );
        }
    }
}
