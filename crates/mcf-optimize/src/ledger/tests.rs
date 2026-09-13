use super::{At, CORPUS, Ledger, Row, Under};
use crate::dial::{Dial, Step};
use crate::reading::{Ending, Reading};

struct Scratch {
    path: std::path::PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-ledger-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&path);
        Self { path }
    }

    fn at(&self) -> std::path::PathBuf {
        self.path.join("optimize").join("readings.jsonl")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.path);
    }
}

fn under() -> Under {
    Under {
        model: "/m/a-model-Q4_K_M.gguf".to_owned(),
        model_bytes: 16_000_000_000,
        engine: "llama.cpp-vulkan".to_owned(),
        commit: "abcdef123456".to_owned(),
        context: 131_072,
        batch: 2048,
        ubatch: 1024,
        cache: "q8_0".to_owned(),
        flash_attention: true,
        draft_head: false,
        draft_depth: None,
        thinking_budget: Some(4096),
        thinking_level: None,
        temperature: Some(200),
        top_p: Some(950),
        top_k: Some(20),
        corpus: CORPUS,
    }
}

fn at(step: Step, set: usize) -> At {
    At {
        dial: Dial::MicroBatch,
        step,
        set,
        repeat: 1,
    }
}

fn reading(step: Step, set: usize, passed: u32) -> Reading {
    Reading {
        dial: Dial::MicroBatch,
        step,
        set,
        repeat: 1,
        passed,
        of: 8,
        produced: 4096,
        milliseconds: 60_000,
        ending: Ending::Answered,
        per_task: vec![("one".to_owned(), true), ("two".to_owned(), false)],
    }
}

#[test]
fn a_ledger_that_is_not_there_yet_opens_empty_rather_than_failing() {
    let scratch = Scratch::new("absent");
    let ledger = Ledger::open(&scratch.at()).expect("an absent ledger is not an error");
    assert!(ledger.rows().is_empty());
}

#[test]
fn a_reading_written_is_a_reading_read_back() {
    let scratch = Scratch::new("round-trip");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    ledger
        .record(
            &under(),
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 6),
            "now",
        )
        .expect("a reading is written");
    let again = Ledger::open(&scratch.at()).expect("opens again");
    assert_eq!(again.rows().len(), 1);
    let row = again.rows().first().expect("the row is there");
    assert_eq!(row.reading.passed, 6);
    assert_eq!(row.reading.of, 8);
    assert_eq!(row.reading.produced, 4096);
    assert_eq!(row.reading.ending, Ending::Answered);
    assert_eq!(row.under, under());
    assert_eq!(row.at, at(Step::Whole(256), 1));
    assert_eq!(
        row.reading.per_task,
        vec![("one".to_owned(), true), ("two".to_owned(), false)],
        "which task passed is what makes a reading worth going back to"
    );
}

#[test]
fn a_configuration_already_measured_is_recognised_as_such() {
    let scratch = Scratch::new("already");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    assert!(ledger.already(&under(), &spot).is_none());
    ledger
        .record(&under(), spot, &reading(Step::Whole(256), 1, 6), "now")
        .expect("written");
    let found = ledger
        .already(&under(), &spot)
        .expect("it knows it ran this");
    assert_eq!(found.reading.passed, 6, "and it knows what came of it");
}

#[test]
fn the_value_being_dialled_does_not_have_to_match_for_the_base_to() {
    let scratch = Scratch::new("dialled-out");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    ledger
        .record(&under(), spot, &reading(Step::Whole(256), 1, 6), "now")
        .expect("written");
    let mut moved = under();
    moved.ubatch = 4096;
    assert!(
        ledger.already(&moved, &spot).is_some(),
        "the micro-batch is what this sweep is dialling, so the base it belongs to is the \
         same base"
    );
}

#[test]
fn a_base_that_differs_anywhere_else_is_a_different_measurement() {
    let scratch = Scratch::new("different-base");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    ledger
        .record(&under(), spot, &reading(Step::Whole(256), 1, 6), "now")
        .expect("written");
    for changed in [
        {
            let mut held = under();
            held.model = "/m/another-model.gguf".to_owned();
            held
        },
        {
            let mut held = under();
            held.commit = "999999999999".to_owned();
            held
        },
        {
            let mut held = under();
            held.context = 262_144;
            held
        },
        {
            let mut held = under();
            held.cache = "f16".to_owned();
            held
        },
        {
            let mut held = under();
            held.flash_attention = false;
            held
        },
        {
            let mut held = under();
            held.temperature = Some(700);
            held
        },
        {
            let mut held = under();
            held.corpus = CORPUS.saturating_add(1);
            held
        },
    ] {
        assert!(
            ledger.already(&changed, &spot).is_none(),
            "a reading taken under other conditions is not this reading"
        );
    }
}

#[test]
fn a_different_set_or_repeat_is_a_different_measurement() {
    let scratch = Scratch::new("set-and-repeat");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    ledger
        .record(
            &under(),
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 6),
            "now",
        )
        .expect("written");
    assert!(ledger.already(&under(), &at(Step::Whole(256), 2)).is_none());
    let mut twice = at(Step::Whole(256), 1);
    twice.repeat = 2;
    assert!(ledger.already(&under(), &twice).is_none());
}

#[test]
fn everything_measured_against_one_base_can_be_gathered_back_up() {
    let scratch = Scratch::new("against");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    for (step, set) in [(256, 1), (256, 2), (512, 1)] {
        ledger
            .record(
                &under(),
                at(Step::Whole(step), set),
                &reading(Step::Whole(step), set, 6),
                "now",
            )
            .expect("written");
    }
    let mut elsewhere = under();
    elsewhere.model = "/m/another.gguf".to_owned();
    ledger
        .record(
            &elsewhere,
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 1),
            "now",
        )
        .expect("written");
    assert_eq!(
        ledger.against(&under(), Dial::MicroBatch).len(),
        3,
        "the other model's reading is not part of this base's picture"
    );
}

#[test]
fn a_line_that_cannot_be_read_is_counted_rather_than_losing_the_rest() {
    let scratch = Scratch::new("torn");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    ledger
        .record(
            &under(),
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 6),
            "now",
        )
        .expect("written");
    let torn = format!(
        "{}\n{{\"recorded\":\"half a line and then the power w",
        std::fs::read_to_string(scratch.at()).expect("read").trim()
    );
    std::fs::write(scratch.at(), torn).expect("written");
    let again = Ledger::open(&scratch.at()).expect("opens");
    assert_eq!(again.rows().len(), 1, "the whole line before it survives");
    assert_eq!(
        again.unreadable(),
        1,
        "and the torn one is counted, not hidden"
    );
}

#[test]
fn a_row_survives_a_line_of_its_own_making() {
    let row = Row {
        recorded: "2026-09-12T00:00:00Z".to_owned(),
        under: under(),
        at: at(Step::Thousandths(250), 3),
        reading: reading(Step::Thousandths(250), 3, 4),
    };
    let back = Row::from_line(&row.to_line()).expect("a row reads its own line");
    assert_eq!(back.under, row.under);
    assert_eq!(back.at, row.at);
    assert_eq!(back.reading.passed, row.reading.passed);
    assert_eq!(back.recorded, row.recorded);
    assert!(
        matches!(back.at.step, Step::Thousandths(250)),
        "a value in thousandths comes back in thousandths, not as a whole number"
    );
}

#[test]
fn every_ending_survives_the_round_trip() {
    for ending in [
        Ending::Answered,
        Ending::Looped,
        Ending::Filled,
        Ending::Failed,
    ] {
        let mut held = reading(Step::Whole(256), 1, 3);
        held.ending = ending;
        let row = Row {
            recorded: "now".to_owned(),
            under: under(),
            at: at(Step::Whole(256), 1),
            reading: held,
        };
        let back = Row::from_line(&row.to_line()).expect("reads");
        assert_eq!(back.reading.ending, ending);
    }
}

#[test]
fn readings_picked_out_are_forgotten_and_the_rest_are_kept() {
    let scratch = Scratch::new("forget-some");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    for step in [256_u32, 512, 1024] {
        ledger
            .record(
                &under(),
                at(Step::Whole(step), 1),
                &reading(Step::Whole(step), 1, 6),
                "before",
            )
            .expect("written");
    }
    let gone = ledger
        .forget(&under(), &[at(Step::Whole(512), 1)])
        .expect("forgotten");
    assert_eq!(gone, 1);
    assert_eq!(ledger.rows().len(), 2);
    let again = Ledger::open(&scratch.at()).expect("reopens");
    assert_eq!(again.rows().len(), 2, "and the file on disk agrees");
    assert!(
        again.already(&under(), &at(Step::Whole(512), 1)).is_none(),
        "what was forgotten is measured again next time"
    );
    assert!(
        again.already(&under(), &at(Step::Whole(256), 1)).is_some(),
        "and what was not picked is left alone"
    );
}

#[test]
fn forgetting_nothing_touches_nothing() {
    let scratch = Scratch::new("forget-none");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    ledger
        .record(
            &under(),
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 6),
            "before",
        )
        .expect("written");
    assert_eq!(ledger.forget(&under(), &[]).expect("nothing"), 0);
    assert_eq!(ledger.rows().len(), 1);
}

#[test]
fn a_reading_taken_under_another_configuration_is_not_forgotten_by_mistake() {
    let scratch = Scratch::new("forget-elsewhere");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let mut elsewhere = under();
    elsewhere.context = 262_144;
    ledger
        .record(
            &elsewhere,
            at(Step::Whole(256), 1),
            &reading(Step::Whole(256), 1, 6),
            "before",
        )
        .expect("written");
    let gone = ledger
        .forget(&under(), &[at(Step::Whole(256), 1)])
        .expect("forgotten");
    assert_eq!(
        gone, 0,
        "another configuration's reading is somebody else's"
    );
    assert_eq!(ledger.rows().len(), 1);
}

#[test]
fn the_thinking_budget_is_part_of_what_makes_a_configuration_that_configuration() {
    let scratch = Scratch::new("budget-in-the-base");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    ledger
        .record(&under(), spot, &reading(Step::Whole(256), 1, 6), "before")
        .expect("written");
    let mut longer = under();
    longer.thinking_budget = Some(16_384);
    assert!(
        ledger.already(&longer, &spot).is_none(),
        "a reading taken under one thinking budget says nothing about another"
    );
}

#[test]
fn a_sweep_of_the_thinking_budget_ignores_the_budget_in_the_base() {
    let scratch = Scratch::new("budget-dialled");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = At {
        dial: Dial::ThinkingBudget,
        step: Step::Whole(4096),
        set: 1,
        repeat: 1,
    };
    let mut held = reading(Step::Whole(4096), 1, 6);
    held.dial = Dial::ThinkingBudget;
    held.step = Step::Whole(4096);
    ledger
        .record(&under(), spot, &held, "before")
        .expect("written");
    let mut longer = under();
    longer.thinking_budget = Some(16_384);
    assert!(
        ledger.already(&longer, &spot).is_some(),
        "the budget is what this sweep varies, so it is not part of the base it varies against"
    );
}

#[test]
fn a_trial_that_failed_is_not_a_measurement_to_skip_over() {
    let scratch = Scratch::new("failed-not-known");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    let mut broke = reading(Step::Whole(256), 1, 0);
    broke.ending = Ending::Failed;
    ledger
        .record(&under(), spot, &broke, "before")
        .expect("written");
    assert!(
        ledger.already(&under(), &spot).is_none(),
        "a sweep that skips its own failures never measures them, and the table keeps \
         showing them"
    );
}

#[test]
fn a_measurement_taken_after_a_failure_is_what_is_known() {
    let scratch = Scratch::new("failed-then-taken");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    let spot = at(Step::Whole(256), 1);
    let mut broke = reading(Step::Whole(256), 1, 0);
    broke.ending = Ending::Failed;
    ledger
        .record(&under(), spot, &broke, "before")
        .expect("written");
    ledger
        .record(&under(), spot, &reading(Step::Whole(256), 1, 6), "after")
        .expect("written");
    let found = ledger.already(&under(), &spot).expect("the good one");
    assert_eq!(found.reading.passed, 6);
    assert_ne!(found.reading.ending, Ending::Failed);
}

#[test]
fn a_run_that_looped_or_filled_its_budget_is_still_a_measurement() {
    let scratch = Scratch::new("runaway-known");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    for (step, ending) in [(256_u32, Ending::Looped), (512, Ending::Filled)] {
        let spot = at(Step::Whole(step), 1);
        let mut held = reading(Step::Whole(step), 1, 0);
        held.ending = ending;
        ledger
            .record(&under(), spot, &held, "before")
            .expect("written");
        assert!(
            ledger.already(&under(), &spot).is_some(),
            "{ending:?} is what the model did, and doing it again would find the same thing"
        );
    }
}
