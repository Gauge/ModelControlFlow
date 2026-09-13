use super::{Orders, Running};
use crate::course::Course;
use crate::dial::{Dial, Step};
use crate::hunt::Way;
use crate::ledger::{CORPUS, Ledger, Under};
use crate::reading::Measure;
use crate::trial::Endpoint;

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
            endpoint: Endpoint {
                port: 1,
                key: None,
                patience: std::time::Duration::from_millis(200),
            },
            under: under(),
            dial: Dial::MicroBatch,
            ceiling: 64,
            thinking: None,
            effort: None,
            mark: false,
            room: scratch.path.join("marking"),
        },
        course,
        Ledger::open(&scratch.at()).expect("opens"),
        std::boxed::Box::new(|_step| Ok(1)),
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
        why.contains("could not be reached"),
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
            endpoint: Endpoint {
                port: 1,
                key: None,
                patience: std::time::Duration::from_millis(200),
            },
            under: under(),
            dial: Dial::MicroBatch,
            ceiling: 64,
            thinking: None,
            effort: None,
            mark: false,
            room: scratch.path.join("marking"),
        },
        course,
        Ledger::open(&scratch.at()).expect("opens"),
        std::boxed::Box::new(|_step| Err("the engine would not start".to_owned())),
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
    assert_eq!(judged, vec![("adds".to_owned(), false)]);
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
    assert_eq!(
        judged,
        vec![("adds".to_owned(), true)],
        "code that satisfies the check is code that passed"
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
    assert_eq!(judged, vec![("adds".to_owned(), false)]);
}
