use super::{Course, Next};
use crate::dial::{Dial, Step};
use crate::hunt::Way;
use crate::ledger::{At, CORPUS, Ledger, Under};
use crate::reading::{Ending, Measure, Reading, Report};

struct Scratch {
    path: std::path::PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-course-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&path);
        Self { path }
    }

    fn at(&self) -> std::path::PathBuf {
        self.path.join("optimize").join("readings.jsonl")
    }

    fn ledger(&self) -> Ledger {
        Ledger::open(&self.at()).expect("a ledger opens")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.path);
    }
}

fn under() -> Under {
    Under {
        model: "/m/a-model.gguf".to_owned(),
        engine: "an-engine".to_owned(),
        commit: "abc".to_owned(),
        context: 4096,
        corpus: CORPUS,
        ..Under::default()
    }
}

fn reading(step: Step, set: usize, repeat: u8, passed: u32) -> Reading {
    Reading {
        dial: Dial::MicroBatch,
        step,
        set,
        repeat,
        passed,
        of: 8,
        produced: 1000,
        milliseconds: 10_000,
        ending: Ending::Answered,
        per_task: Vec::new(),
    }
}

fn by_hand(steps: &[u32], sets: &[usize], repeats: u8) -> Course {
    let held: Vec<Step> = steps.iter().map(|held| Step::Whole(*held)).collect();
    Course::laid_out(
        under(),
        Way::ByHand,
        Dial::MicroBatch,
        &held,
        sets,
        repeats,
        Measure::Correctness,
    )
}

fn drain(course: &mut Course, ledger: &Ledger, report: &mut Report) -> Vec<At> {
    let mut taken = Vec::new();
    for _ in 0..500 {
        match course.next(Dial::MicroBatch, ledger, report) {
            Next::Take(at) => taken.push(at),
            Next::Finished => break,
        }
    }
    taken
}

#[test]
fn a_course_by_hand_visits_every_value_set_and_repeat_once() {
    let scratch = Scratch::new("by-hand");
    let mut course = by_hand(&[256, 512], &[1, 2, 3], 2);
    let mut report = Report::default();
    let taken = drain(&mut course, &scratch.ledger(), &mut report);
    assert_eq!(taken.len(), 12, "two values, three sets, twice each");
    let mut seen = taken.clone();
    seen.sort_by_key(|at| {
        (
            match at.step {
                Step::Whole(held) | Step::Thousandths(held) => held,
            },
            at.set,
            at.repeat,
        )
    });
    seen.dedup();
    assert_eq!(seen.len(), 12, "no trial is laid out twice");
}

#[test]
fn repeats_are_numbered_from_one() {
    let scratch = Scratch::new("repeat-numbers");
    let mut course = by_hand(&[256], &[1], 3);
    let mut report = Report::default();
    let taken = drain(&mut course, &scratch.ledger(), &mut report);
    let numbers: Vec<u8> = taken.iter().map(|at| at.repeat).collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[test]
fn a_trial_already_in_the_ledger_is_skipped_and_its_result_used() {
    let scratch = Scratch::new("skip-ahead");
    let mut ledger = scratch.ledger();
    let known = At {
        dial: Dial::MicroBatch,
        step: Step::Whole(256),
        set: 1,
        repeat: 1,
    };
    ledger
        .record(
            &under(),
            known,
            &reading(Step::Whole(256), 1, 1, 7),
            "before",
        )
        .expect("written");

    let mut course = by_hand(&[256, 512], &[1], 1);
    let mut report = Report::default();
    let taken = drain(&mut course, &ledger, &mut report);
    assert_eq!(taken.len(), 1, "only the one that was never run");
    assert_eq!(taken.first().map(|at| at.step), Some(Step::Whole(512)));
    assert_eq!(course.skipped(), 1);
    assert_eq!(
        report.readings.len(),
        1,
        "what was already measured is in the report without being measured again"
    );
    assert_eq!(report.readings.first().map(|held| held.passed), Some(7));
}

#[test]
fn a_course_resumed_after_everything_ran_takes_nothing_at_all() {
    let scratch = Scratch::new("all-known");
    let mut ledger = scratch.ledger();
    for step in [256_u32, 512] {
        for set in [1_usize, 2] {
            let spot = At {
                dial: Dial::MicroBatch,
                step: Step::Whole(step),
                set,
                repeat: 1,
            };
            ledger
                .record(
                    &under(),
                    spot,
                    &reading(Step::Whole(step), set, 1, 5),
                    "before",
                )
                .expect("written");
        }
    }
    let mut course = by_hand(&[256, 512], &[1, 2], 1);
    let mut report = Report::default();
    let taken = drain(&mut course, &ledger, &mut report);
    assert!(taken.is_empty(), "a finished sweep does not run again");
    assert_eq!(course.skipped(), 4);
    assert_eq!(report.readings.len(), 4, "the whole picture is still there");
}

#[test]
fn a_crash_halfway_resumes_where_it_stopped() {
    let scratch = Scratch::new("crash");
    let mut ledger = scratch.ledger();
    let mut first = by_hand(&[256, 512, 1024], &[1], 1);
    let mut report = Report::default();
    let Next::Take(one) = first.next(Dial::MicroBatch, &ledger, &mut report) else {
        panic!("there is a first trial");
    };
    ledger
        .record(
            &under(),
            one,
            &reading(one.step, 1, 1, 6),
            "before the crash",
        )
        .expect("written");

    let ledger = Ledger::open(&scratch.at()).expect("reopens, as after a restart");
    let mut again = by_hand(&[256, 512, 1024], &[1], 1);
    let mut report = Report::default();
    let taken = drain(&mut again, &ledger, &mut report);
    assert_eq!(taken.len(), 2, "the two that never finished");
    assert!(
        !taken.contains(&one),
        "the one written down before the crash is not taken again"
    );
    assert_eq!(again.skipped(), 1);
}

#[test]
fn an_automatic_course_opens_where_its_setting_says_to_start() {
    let scratch = Scratch::new("automatic-open");
    let climbing = Course::laid_out(
        under(),
        Way::Halving,
        Dial::MicroBatch,
        &[],
        &[1],
        1,
        Measure::Correctness,
    );
    assert_eq!(
        climbing.steps(),
        vec![Step::Whole(Dial::MicroBatch.climbs_from())],
        "a setting that climbs opens on one rung and doubles from there"
    );
    assert!(climbing.hunt().is_some());

    let laddered = Course::laid_out(
        under(),
        Way::Halving,
        Dial::TopK,
        &[],
        &[1],
        1,
        Measure::Correctness,
    );
    assert_eq!(
        laddered.steps(),
        vec![Step::Whole(Dial::TopK.climbs_from())],
        "and so does every other setting, from wherever it is off"
    );
    let _unused = scratch.at();
}

#[test]
fn an_automatic_course_doubles_until_a_value_is_worse_and_then_closes_in() {
    let scratch = Scratch::new("automatic-close");
    let ledger = scratch.ledger();
    let mut course = Course::laid_out(
        under(),
        Way::Halving,
        Dial::MicroBatch,
        &[],
        &[1],
        1,
        Measure::Correctness,
    );
    let mut report = Report::default();
    let mut climbed = Vec::new();
    let mut closed = Vec::new();
    while let Next::Take(at) = course.next(Dial::MicroBatch, &ledger, &mut report) {
        let value = match at.step {
            Step::Whole(held) | Step::Thousandths(held) => held,
        };
        let away = value.abs_diff(1024).div_euclid(256);
        let passed = 8_u32.saturating_sub(away);
        report.record(reading(at.step, at.set, at.repeat, passed));
        if [256, 512, 1024, 2048].contains(&value) {
            climbed.push(value);
        } else {
            closed.push(value);
        }
    }
    climbed.sort_unstable();
    assert_eq!(
        climbed,
        vec![256, 512, 1024, 2048],
        "it doubles from the bottom until 2048 comes back worse than 1024"
    );
    assert!(
        !closed.contains(&4096) && !closed.contains(&8192),
        "and having turned, it does not go on doubling: {closed:?}"
    );
    assert!(
        closed.iter().any(|held| (1024..2048).contains(held)),
        "it closes in between the rung that improved and the one that did not: {closed:?}"
    );
}

#[test]
fn an_automatic_course_finishes_rather_than_halving_forever() {
    let scratch = Scratch::new("automatic-settles");
    let ledger = scratch.ledger();
    let mut course = Course::laid_out(
        under(),
        Way::Halving,
        Dial::DraftDepth,
        &[],
        &[1],
        1,
        Measure::Correctness,
    );
    let mut report = Report::default();
    let mut rounds = 0;
    while let Next::Take(at) = course.next(Dial::DraftDepth, &ledger, &mut report) {
        report.record(Reading {
            dial: Dial::DraftDepth,
            step: at.step,
            set: at.set,
            repeat: at.repeat,
            passed: 4,
            of: 8,
            produced: 100,
            milliseconds: 1000,
            ending: Ending::Answered,
            per_task: Vec::new(),
        });
        rounds += 1;
        assert!(rounds < 200, "an automatic course must terminate");
    }
    assert!(course.taken() > 0);
}

#[test]
fn a_course_with_no_sets_asks_for_nothing_rather_than_looping() {
    let scratch = Scratch::new("no-sets");
    let mut course = by_hand(&[256], &[], 1);
    let mut report = Report::default();
    assert_eq!(
        course.next(Dial::MicroBatch, &scratch.ledger(), &mut report),
        Next::Finished
    );
}

#[test]
fn a_course_with_no_values_asks_for_nothing() {
    let scratch = Scratch::new("no-values");
    let mut course = by_hand(&[], &[1], 1);
    let mut report = Report::default();
    assert_eq!(
        course.next(Dial::MicroBatch, &scratch.ledger(), &mut report),
        Next::Finished
    );
}

#[test]
fn a_course_never_hands_out_the_same_trial_twice_even_with_nothing_written_down() {
    let scratch = Scratch::new("no-repeats-in-a-session");
    let ledger = scratch.ledger();
    let mut course = Course::laid_out(
        under(),
        Way::Halving,
        Dial::MicroBatch,
        &[],
        &[1],
        1,
        Measure::Correctness,
    );
    let mut report = Report::default();
    let mut handed: Vec<At> = Vec::new();
    for _ in 0..300 {
        match course.next(Dial::MicroBatch, &ledger, &mut report) {
            Next::Take(at) => {
                assert!(
                    !handed.contains(&at),
                    "a course that forgets what it handed out measures the same thing twice: \
                     {at:?}"
                );
                report.record(reading(at.step, at.set, at.repeat, 5));
                handed.push(at);
            }
            Next::Finished => break,
        }
    }
    assert!(
        handed.len() > 1,
        "a course that opens on one rung and never asks for another has not searched"
    );
}

#[test]
fn a_search_that_finds_nothing_to_climb_says_so_rather_than_stopping_quietly() {
    let scratch = Scratch::new("all-ran-away");
    let ledger = scratch.ledger();
    let mut course = Course::laid_out(
        under(),
        Way::Halving,
        Dial::DraftDepth,
        &[],
        &[1],
        1,
        Measure::Speed,
    );
    let mut report = Report::default();
    while let Next::Take(at) = course.next(Dial::DraftDepth, &ledger, &mut report) {
        let mut held = reading(at.step, at.set, at.repeat, 0);
        held.dial = Dial::DraftDepth;
        held.ending = Ending::Filled;
        report.record(held);
    }
    let why = course.stopped().unwrap_or_default();
    assert!(
        why.contains("ran away"),
        "when every value ran away there is no best to search around, and the tool says \
         that instead of looking finished: {why}"
    );
}

#[test]
fn a_search_that_runs_out_of_room_to_halve_says_it_settled() {
    let scratch = Scratch::new("settled-said");
    let ledger = scratch.ledger();
    let mut course = Course::laid_out(
        under(),
        Way::Halving,
        Dial::DraftDepth,
        &[],
        &[1],
        1,
        Measure::Speed,
    );
    let mut report = Report::default();
    while let Next::Take(at) = course.next(Dial::DraftDepth, &ledger, &mut report) {
        let mut held = reading(at.step, at.set, at.repeat, 4);
        held.dial = Dial::DraftDepth;
        report.record(held);
    }
    let why = course.stopped().unwrap_or_default();
    assert!(why.contains("settled"), "{why}");
}

#[test]
fn a_search_by_hand_that_simply_ran_out_says_nothing_about_settling() {
    let scratch = Scratch::new("by-hand-end");
    let ledger = scratch.ledger();
    let mut course = by_hand(&[256], &[1], 1);
    let mut report = Report::default();
    while let Next::Take(at) = course.next(Dial::MicroBatch, &ledger, &mut report) {
        report.record(reading(at.step, at.set, at.repeat, 4));
    }
    assert!(
        course.stopped().is_none(),
        "a list of values that has been worked through is not a search that gave up"
    );
}

#[test]
fn a_course_over_chosen_readings_runs_those_and_nothing_beside_them() {
    let scratch = Scratch::new("exactly");
    let ledger = scratch.ledger();
    let wanted = vec![
        At {
            dial: Dial::MicroBatch,
            step: Step::Whole(256),
            set: 3,
            repeat: 2,
        },
        At {
            dial: Dial::MicroBatch,
            step: Step::Whole(1024),
            set: 7,
            repeat: 1,
        },
    ];
    let mut course = Course::over(under(), &wanted, Measure::Speed);
    let mut report = Report::default();
    let taken = drain(&mut course, &ledger, &mut report);
    assert_eq!(
        taken, wanted,
        "picking one take of one set is a request for that take of that set, not for every \
         take up to it"
    );
    assert!(course.is_exactly());
}

#[test]
fn a_course_over_chosen_readings_never_opens_a_round_of_its_own() {
    let scratch = Scratch::new("exactly-no-hunt");
    let ledger = scratch.ledger();
    let wanted = vec![At {
        dial: Dial::MicroBatch,
        step: Step::Whole(256),
        set: 1,
        repeat: 1,
    }];
    let mut course = Course::over(under(), &wanted, Measure::Speed);
    let mut report = Report::default();
    let taken = drain(&mut course, &ledger, &mut report);
    assert_eq!(taken.len(), 1);
    assert_eq!(
        course.laid(),
        1,
        "one reading asked for is one trial laid out"
    );
}

#[test]
fn a_chosen_reading_still_in_the_record_is_skipped_rather_than_run_twice() {
    let scratch = Scratch::new("exactly-known");
    let mut ledger = scratch.ledger();
    let spot = At {
        dial: Dial::MicroBatch,
        step: Step::Whole(256),
        set: 1,
        repeat: 1,
    };
    ledger
        .record(
            &under(),
            spot,
            &reading(Step::Whole(256), 1, 1, 4),
            "before",
        )
        .expect("written");
    let mut course = Course::over(under(), &[spot], Measure::Speed);
    let mut report = Report::default();
    let taken = drain(&mut course, &ledger, &mut report);
    assert!(
        taken.is_empty(),
        "forgetting the old reading is what makes a rerun a rerun; the course does not \
         second-guess the record"
    );
    assert_eq!(course.skipped(), 1);
}
