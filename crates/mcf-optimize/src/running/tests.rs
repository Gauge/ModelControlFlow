use super::{Heard, Orders, Running};
use crate::course::Course;
use crate::dial::{Dial, Step};
use crate::hunt::Way;
use crate::ledger::{CORPUS, Ledger, Under};
use crate::reading::Measure;
use crate::trial::{Asked, Endpoint};

struct Scratch {
    path: std::path::PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-running-{name}-{}-{:?}",
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
        model: "/m/a-model.gguf".to_owned(),
        corpus: CORPUS,
        ..Under::default()
    }
}

fn begun(scratch: &Scratch, steps: &[u32], sets: &[usize]) -> Running {
    let held: Vec<Step> = steps.iter().map(|held| Step::Whole(*held)).collect();
    let course = Course::laid_out(
        under(),
        Way::ByHand,
        Dial::MicroBatch,
        &held,
        sets,
        1,
        Measure::Speed,
    );
    Running::begun(
        Orders {
            switch: false,
            endpoint: Endpoint {
                port: 1,
                key: None,
                patience: std::time::Duration::from_millis(200),
            },
            under: under(),
            dial: Dial::MicroBatch,
            ceiling: 64,
            named: Vec::new(),
            mark: false,
            room: scratch.path.join("marking"),
            ready_within: std::time::Duration::from_millis(50),
        },
        course,
        Ledger::open(&scratch.at()).expect("opens"),
        std::boxed::Box::new(|_step, _along| Ok(1)),
        || "now".to_owned(),
    )
}

fn settled(running: &mut Running) {
    for _ in 0..600 {
        let _moved = running.hear();
        if running.finished {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn a_run_against_nothing_listening_stops_and_says_so_rather_than_hanging() {
    let scratch = Scratch::new("nothing-listening");
    let mut running = begun(&scratch, &[256], &[1]);
    settled(&mut running);
    assert!(running.finished, "it does not hang when nothing answers");
    let why = running.refused.clone().unwrap_or_default();
    assert!(
        why.contains("never started answering"),
        "the refusal names what went wrong: {why}"
    );
    assert_eq!(running.taken, 0, "a trial that never ran is not a reading");
}

#[test]
fn nothing_is_written_down_for_a_trial_that_could_not_be_taken() {
    let scratch = Scratch::new("nothing-written");
    let mut running = begun(&scratch, &[256, 512], &[1]);
    settled(&mut running);
    let ledger = Ledger::open(&scratch.at()).expect("opens");
    assert!(
        ledger.rows().is_empty(),
        "a reading is recorded when it is taken, never before"
    );
}

#[test]
fn a_run_with_nothing_laid_out_finishes_at_once() {
    let scratch = Scratch::new("empty");
    let mut running = begun(&scratch, &[], &[]);
    settled(&mut running);
    assert!(running.finished);
    assert!(running.refused.is_none(), "finishing is not a failure");
    assert_eq!(running.taken, 0);
}

#[test]
fn a_run_told_to_stop_says_it_is_stopping() {
    let scratch = Scratch::new("stopping");
    let running = begun(&scratch, &[256], &[1]);
    assert!(!running.stopping());
    running.stop();
    assert!(running.stopping());
}

#[test]
fn everything_already_in_the_ledger_is_reported_as_known_rather_than_measured() {
    let scratch = Scratch::new("all-known");
    let mut ledger = Ledger::open(&scratch.at()).expect("opens");
    for step in [256_u32, 512] {
        let spot = crate::ledger::At {
            dial: Dial::MicroBatch,
            step: Step::Whole(step),
            set: 1,
            repeat: 1,
        };
        ledger
            .record(
                &under(),
                spot,
                &crate::reading::Reading {
                    dial: Dial::MicroBatch,
                    step: Step::Whole(step),
                    set: 1,
                    repeat: 1,
                    passed: 5,
                    of: 8,
                    produced: 900,
                    milliseconds: 9000,
                    ending: crate::reading::Ending::Answered,
                    why: None,
                    per_task: Vec::new(),
                },
                "before",
            )
            .expect("written");
    }
    let mut running = begun(&scratch, &[256, 512], &[1]);
    settled(&mut running);
    assert!(running.finished);
    assert_eq!(running.taken, 0, "nothing needed measuring again");
    assert_eq!(running.skipped, 2, "both were already known");
    assert!(
        running.refused.is_none(),
        "a run with nothing left to do is not a failure: {:?}",
        running.refused
    );
}

#[test]
fn a_run_that_cannot_hold_the_model_says_so_and_measures_nothing() {
    let scratch = Scratch::new("cannot-hold");
    let held: Vec<Step> = vec![Step::Whole(256)];
    let course = Course::laid_out(
        under(),
        Way::ByHand,
        Dial::MicroBatch,
        &held,
        &[1],
        1,
        Measure::Speed,
    );
    let mut running = Running::begun(
        super::Orders {
            switch: false,
            endpoint: Endpoint {
                port: 1,
                key: None,
                patience: std::time::Duration::from_millis(200),
            },
            under: under(),
            dial: Dial::MicroBatch,
            ceiling: 64,
            named: Vec::new(),
            mark: false,
            room: scratch.path.join("marking"),
            ready_within: std::time::Duration::from_millis(50),
        },
        course,
        Ledger::open(&scratch.at()).expect("opens"),
        std::boxed::Box::new(|_step, _along| Err("the engine would not start".to_owned())),
        || "now".to_owned(),
    );
    settled(&mut running);
    assert!(running.finished);
    assert_eq!(
        running.refused.as_deref(),
        Some("the engine would not start"),
        "the reason the hold failed is the reason the sweep stopped"
    );
    assert_eq!(running.taken, 0);
}

#[test]
fn nothing_is_measured_until_the_model_is_held_at_all() {
    for dial in Dial::ALL {
        assert!(
            super::needs_a_fresh_hold(dial, None, dial.step_of(1)),
            "{} starts by holding the model, whether or not one was held before",
            dial.label()
        );
    }
}

#[test]
fn a_dial_that_is_a_launch_flag_holds_the_model_again_for_each_value() {
    for dial in Dial::ALL
        .into_iter()
        .filter(|dial| dial.reloads_the_engine())
    {
        let held = dial.step_of(2);
        assert!(
            super::needs_a_fresh_hold(dial, Some(held), dial.step_of(4)),
            "{} is a launch flag, so a new value means a new hold",
            dial.label()
        );
        assert!(
            !super::needs_a_fresh_hold(dial, Some(held), held),
            "{} at the value already held needs no reload",
            dial.label()
        );
    }
}

#[test]
fn a_dial_that_rides_in_the_request_holds_the_model_only_once() {
    for dial in Dial::ALL
        .into_iter()
        .filter(|dial| !dial.reloads_the_engine())
    {
        let held = dial.step_of(2);
        assert!(
            !super::needs_a_fresh_hold(dial, Some(held), dial.step_of(900)),
            "{} rides in each request, so reloading between values would be wasted minutes",
            dial.label()
        );
    }
}

fn one_task() -> Vec<crate::corpus::Task> {
    vec![crate::corpus::Task {
        name: "adds".to_owned(),
        asked: String::new(),
        checked: "assert add(2, 2) == 4\nassert add(1, 5) == 6".to_owned(),
    }]
}

fn said_with(answer: &str) -> crate::trial::Said {
    crate::trial::Said {
        answer: answer.to_owned(),
        produced: 10,
        ending: crate::reading::Ending::Answered,
        why: None,
        counted: None,
        read_in: None,
    }
}

fn spot() -> crate::ledger::At {
    crate::ledger::At {
        dial: Dial::MicroBatch,
        step: Step::Whole(256),
        set: 1,
        repeat: 1,
    }
}

#[test]
fn a_sweep_that_is_not_marking_calls_nothing_right_and_starts_no_container() {
    let scratch = Scratch::new("not-marking");
    let room = scratch.path.join("marking");
    let (judged, unmarked) = super::judged_by(
        false,
        &room,
        spot(),
        &one_task(),
        &said_with("```python\ndef add(a,b): return a+b\n```"),
    );
    assert_eq!(
        judged,
        crate::marking::nothing_held(&one_task()),
        "no claim held, and every task still says how many it makes"
    );
    assert!(unmarked.is_none());
    assert!(!room.exists(), "nothing was written anywhere");
}

#[test]
fn a_marked_sweep_actually_runs_the_code_and_says_whether_it_passed() {
    if crate::marking::where_podman_is().is_none() {
        return;
    }
    let scratch = Scratch::new("marking-live");
    let room = scratch.path.join("marking");
    let (judged, unmarked) = super::judged_by(
        true,
        &room,
        spot(),
        &one_task(),
        &said_with("### SOLUTION 1\n```python\ndef add(a, b):\n    return a + b\n```"),
    );
    assert_eq!(
        unmarked, None,
        "podman is here, so there is no reason marking could not happen"
    );
    let first = judged.first().expect("the one task");
    assert!(
        first.whole() && first.of > 0,
        "code that satisfies every claim the check makes is code that passed: {judged:?}"
    );
    assert!(
        !room.join("set-1-256-1").exists(),
        "the scratch the container read is swept up afterwards"
    );
}

#[test]
fn a_marked_sweep_fails_code_that_does_not_satisfy_the_check() {
    if crate::marking::where_podman_is().is_none() {
        return;
    }
    let scratch = Scratch::new("marking-wrong");
    let room = scratch.path.join("marking");
    let (judged, _unmarked) = super::judged_by(
        true,
        &room,
        spot(),
        &one_task(),
        &said_with("### SOLUTION 1\n```python\ndef add(a, b):\n    return a * b\n```"),
    );
    // The check makes two claims: add(2,2)==4 and add(1,5)==6. Multiplication gets the
    // first of those right by luck and the second wrong, and that is the whole point of
    // counting claims — half right and nowhere at all used to read the same.
    let first = judged.first().expect("the one task");
    assert_eq!(
        (first.passed, first.of),
        (1, 2),
        "code that satisfies one claim of two is marked one of two: {judged:?}"
    );
    assert!(!first.whole(), "and it did not pass");
}

#[test]
fn nothing_is_asked_of_an_engine_that_is_not_answering_yet() {
    let scratch = Scratch::new("not-ready-yet");
    let mut running = begun(&scratch, &[256], &[1]);
    settled(&mut running);
    assert_eq!(
        running.taken, 0,
        "a reading taken while the model was still loading would be a lie written down"
    );
    let ledger = Ledger::open(&scratch.at()).expect("opens");
    assert!(ledger.rows().is_empty());
}

#[test]
fn an_engine_that_goes_away_is_held_again_before_the_sweep_gives_up() {
    let scratch = Scratch::new("held-again");
    let (send, heard) = std::sync::mpsc::channel();
    let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let kept = std::sync::Arc::clone(&asked);
    let mut doing = super::Doing {
        switch: false,
        send,
        course: Course::laid_out(
            under(),
            Way::ByHand,
            Dial::MicroBatch,
            &[Step::Whole(256)],
            &[1],
            1,
            Measure::Speed,
        ),
        ledger: Ledger::open(&scratch.at()).expect("opens"),
        report: crate::reading::Report::default(),
        endpoint: Endpoint {
            port: 1,
            key: None,
            patience: std::time::Duration::from_millis(50),
        },
        under: under(),
        dial: Dial::MicroBatch,
        ceiling: 64,
        named: Vec::new(),
        mark: false,
        room: scratch.path.join("marking"),
        ready_within: std::time::Duration::from_millis(20),
        host: std::boxed::Box::new(move |_step, _along| {
            kept.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(1)
        }),
        recorded: std::boxed::Box::new(|| "now".to_owned()),
        asked_to_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        asked_to_wait: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let trial = Asked {
        switch: false,
        set: crate::corpus::Set::numbered(1).expect("set one"),
        dial: Dial::MicroBatch,
        step: Step::Whole(256),
        repeat: 1,
        ceiling: 64,
        named: Vec::new(),
        timing: true,
    };
    let outcome = super::answered(&mut doing, &trial, Step::Whole(256));
    assert_eq!(
        asked.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "an engine that dies four hours into a sweep should cost one hold, not the sweep"
    );
    let Err(super::Stopped::Refused(why)) = outcome else {
        panic!("nothing was listening, so it cannot have answered");
    };
    assert!(
        why.contains("did not work either"),
        "and when holding it again does not help, it says so: {why}"
    );
    assert!(
        heard
            .try_iter()
            .any(|held| matches!(held, Heard::Holding(_))),
        "it says it is holding the model again rather than going quiet"
    );
}
#[test]
fn a_clock_counts_hours_minutes_and_seconds_with_their_noughts() {
    use super::as_a_clock;
    use std::time::Duration;
    assert_eq!(as_a_clock(Duration::from_secs(0)), "00:00:00");
    assert_eq!(as_a_clock(Duration::from_secs(9)), "00:00:09");
    assert_eq!(as_a_clock(Duration::from_secs(61)), "00:01:01");
    assert_eq!(as_a_clock(Duration::from_secs(3600)), "01:00:00");
    assert_eq!(as_a_clock(Duration::from_secs(3661)), "01:01:01");
    assert_eq!(as_a_clock(Duration::from_hours(24)), "24:00:00");
}

fn labelled(dial: Dial, at: Option<crate::ledger::At>, holding: Option<&str>) -> String {
    let measure = dial.ranked_by();
    labelled_by(dial, measure, at, holding)
}

fn labelled_by(
    dial: Dial,
    measure: crate::reading::Measure,
    at: Option<crate::ledger::At>,
    holding: Option<&str>,
) -> String {
    let scratch = Scratch::new("label");
    let mut running = begun(&scratch, &[], &[]);
    running.doing = at;
    running.holding = holding.map(str::to_owned);
    running.produced = 512;
    running.label(&[], dial, measure, 4096)
}

#[test]
fn a_trial_of_the_tasks_names_the_value_the_set_and_how_far_in_it_is() {
    let said = labelled(
        Dial::ThinkingBudget,
        Some(crate::ledger::At {
            dial: Dial::ThinkingBudget,
            step: Step::Whole(4096),
            set: 3,
            repeat: 2,
        }),
        None,
    );
    assert!(said.contains("4096"), "{said}");
    assert!(said.contains("set 3"), "{said}");
    assert!(said.contains("take 2"), "{said}");
    assert!(
        said.contains("wrote 512 of 4096"),
        "how far through this one trial it is, against what it asked for: {said}"
    );
    assert!(said.starts_with("00:00:0"), "the clock leads: {said}");
}

#[test]
fn a_take_that_is_the_only_take_is_not_counted_out_loud() {
    let said = labelled(
        Dial::MicroBatch,
        Some(crate::ledger::At {
            dial: Dial::MicroBatch,
            step: Step::Whole(1024),
            set: 1,
            repeat: 1,
        }),
        None,
    );
    assert!(
        !said.contains("take"),
        "a take counter that is always one is a number to read and discard: {said}"
    );
}

#[test]
fn a_trial_that_reads_a_prompt_counts_what_it_read_rather_than_what_it_wrote() {
    let said = labelled(
        Dial::MicroBatch,
        Some(crate::ledger::At {
            dial: Dial::MicroBatch,
            step: Step::Whole(1024),
            set: 1,
            repeat: 1,
        }),
        None,
    );
    assert!(
        said.contains("read 512 of 4096"),
        "nothing is written while a prompt is read, so a count of what was written would sit \
         at nothing for the whole trial: {said}"
    );
    assert!(
        !said.contains("set"),
        "a timed trial runs no tasks, so naming a set would be naming something that did not \
         happen: {said}"
    );
}

#[test]
fn how_far_along_a_search_is_counts_rounds_because_it_has_no_total_to_count_towards() {
    let scratch = Scratch::new("far-along");
    let mut running = begun(&scratch, &[], &[]);
    running.round = 4;
    running.taken = 7;
    assert_eq!(running.far_along(), "round 4 · 7 measured");
    running.skipped = 2;
    assert_eq!(running.far_along(), "round 4 · 7 measured, 2 already known");
}

#[test]
fn a_sweep_that_is_holding_the_model_says_that_with_the_clock_still_running() {
    let said = labelled(Dial::MicroBatch, None, Some("holding the model at 1024"));
    assert!(said.contains("holding the model at 1024"), "{said}");
    assert!(said.starts_with("00:00:0"), "{said}");
}

#[test]
fn a_sweep_between_trials_still_shows_a_clock() {
    let said = labelled(Dial::MicroBatch, None, None);
    assert!(said.starts_with("00:00:0"), "{said}");
}

#[test]
fn the_token_count_starts_again_with_each_trial() {
    let scratch = Scratch::new("tokens-reset");
    let mut running = begun(&scratch, &[], &[]);
    running.produced = 4096;
    assert_eq!(running.produced, 4096);
    running.doing = None;
    let said = running.label(&[], Dial::MicroBatch, crate::reading::Measure::Speed, 8192);
    assert!(
        !said.contains("4096"),
        "between trials there is no count to show: {said}"
    );
}

#[test]
fn a_trial_says_how_far_it_has_got_often_enough_to_look_alive() {
    let told = crate::trial::TOLD_EVERY;
    assert!(
        (8..=64).contains(&told),
        "at eighty tokens a second, saying so every {told} is about once a second; saying so \
         on every token would make the saying the work"
    );
}

/// Pause and carry on, tested where a pause can be seen without an engine: a sweep paused
/// before its first trial gets no further, and nothing is written down, until it is told to
/// carry on. What a sweep measures needs an engine; that it stops when asked does not.
#[test]
fn a_sweep_paused_before_it_starts_takes_nothing_until_it_is_told_to_carry_on() {
    let scratch = Scratch::new("paused-before-starting");
    let mut running = begun(&scratch, &[256], &[1]);
    running.pause();
    assert!(running.asked_to_wait());

    for _ in 0..60 {
        let _moved = running.hear();
        if running.waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        running.waiting,
        "a sweep asked to pause must actually stop, not merely be marked as asked"
    );
    assert!(
        !running.finished,
        "a paused sweep has not finished: there is more of it to take"
    );

    running.resume();
    assert!(!running.asked_to_wait());
    settled(&mut running);
    assert!(
        running.finished,
        "a sweep told to carry on carries on to the end"
    );
}

#[test]
fn stopping_a_paused_sweep_does_not_leave_it_asleep() {
    let scratch = Scratch::new("stopped-while-paused");
    let mut running = begun(&scratch, &[256], &[1]);
    running.pause();
    for _ in 0..60 {
        let _moved = running.hear();
        if running.waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(running.waiting);

    running.stop();
    settled(&mut running);
    assert!(
        running.finished,
        "a sweep stopped while paused must end, not wait for a carry-on nobody will give"
    );
}

#[test]
fn the_clock_beside_a_paused_sweep_counts_work_rather_than_waiting() {
    let scratch = Scratch::new("clock-while-paused");
    let mut running = begun(&scratch, &[256], &[1]);
    running.pause();
    for _ in 0..60 {
        let _moved = running.hear();
        if running.waiting {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(running.waiting);

    let before = running.running_for();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let after = running.running_for();
    assert!(
        after.saturating_sub(before) < std::time::Duration::from_millis(250),
        "a clock that counts a pause reads as though the sweep were slower than it is: \
         {before:?} then {after:?}"
    );
    running.stop();
    settled(&mut running);
}
