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
        },
        course,
        Ledger::open(&scratch.at()).expect("opens"),
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
