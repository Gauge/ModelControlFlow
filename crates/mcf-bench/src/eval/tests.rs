#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use super::*;

const TASK: Task = Task {
    name: "a-task",
    function: "solve",
    asks: "write a function",
    cases: &[
        Case {
            call: "solve(1)",
            expects: "1",
        },
        Case {
            call: "solve(2)",
            expects: "2",
        },
    ],
};

fn trials(attempts: &[Ran]) -> Trials {
    Trials {
        task: TASK.name,
        attempts: attempts.to_vec(),
    }
}

#[test]
fn a_run_that_broke_is_never_a_score() {
    let graded = trials(&[
        Ran::Refused {
            because: "the container would not start".to_owned(),
            wrote_something: true,
        },
        Ran::Refused {
            because: "the container would not start".to_owned(),
            wrote_something: true,
        },
    ])
    .graded();
    assert!(
        graded.score().is_none(),
        "nothing ran, so there is no reading: {graded}"
    );
    let Graded::Unknown { why } = &graded else {
        panic!("a run that broke is Unknown, not a low score: {graded}");
    };
    assert!(why.contains("failed to run"), "{why}");
}

#[test]
fn writing_nothing_is_a_different_fact_from_writing_something_broken() {
    let graded = trials(&[Ran::Refused {
        because: "the model said nothing".to_owned(),
        wrote_something: false,
    }])
    .graded();
    let Graded::Unknown { why } = &graded else {
        panic!("expected Unknown, got {graded}");
    };
    assert!(why.contains("wrote nothing"), "{why}");
}

#[test]
fn a_function_that_satisfies_some_cases_is_not_partly_correct() {
    let graded = trials(&[
        Ran::Checked { passed: 1, of: 2 },
        Ran::Checked { passed: 2, of: 2 },
    ])
    .graded();
    let score = graded
        .score()
        .expect("something ran, so there is a reading");
    assert_eq!(
        score.parts_per_million(),
        500_000,
        "one attempt of two was whole, so the reading is one in two — not three quarters"
    );
}

#[test]
fn every_attempt_is_kept_including_the_ones_that_did_not_run() {
    let held = trials(&[
        Ran::Checked { passed: 2, of: 2 },
        Ran::Refused {
            because: "timed out".to_owned(),
            wrote_something: true,
        },
        Ran::Checked { passed: 0, of: 2 },
    ]);
    assert_eq!(held.attempts.len(), 3, "nothing is dropped");
    assert_eq!(held.ran(), 2, "two of the three ran");
    assert_eq!(held.whole(), 1, "one of them satisfied every case");
    let score = held
        .graded()
        .score()
        .expect("something ran")
        .parts_per_million();
    assert_eq!(score, 333_333);
}

#[test]
fn no_attempts_is_unknown() {
    let graded = trials(&[]).graded();
    assert!(graded.score().is_none());
    assert!(matches!(graded, Graded::Unknown { .. }));
}

#[test]
fn a_score_is_only_comparable_within_its_own_task() {
    let one = trials(&[Ran::Checked { passed: 2, of: 2 }]).graded();
    let other = Trials {
        task: "a-different-task",
        attempts: vec![Ran::Checked { passed: 2, of: 2 }],
    }
    .graded();
    let (one, other) = (
        one.score().expect("measured"),
        other.score().expect("measured"),
    );
    assert!(
        one.against(other).is_none(),
        "two laboratories are two scales, and there is no operation that relates them (B41)"
    );
}

#[test]
fn each_attempt_asks_once_and_runs_what_came_back() {
    let mut asked = 0_usize;
    let mut ran = 0_usize;
    let held = {
        let mut ask = |_: &Task| {
            asked = asked.saturating_add(1);
            Some("def solve(n): return n".to_owned())
        };
        let mut run = |_: &Task, _: &str| {
            ran = ran.saturating_add(1);
            Ran::Checked { passed: 2, of: 2 }
        };
        measure(&TASK, 3, &mut ask, &mut run)
    };
    assert_eq!(asked, 3);
    assert_eq!(ran, 3);
    assert_eq!(held.whole(), 3);
}

#[test]
fn nothing_written_is_never_run() {
    let mut ran = 0_usize;
    let held = {
        let mut ask = |_: &Task| None;
        let mut run = |_: &Task, _: &str| {
            ran = ran.saturating_add(1);
            Ran::Checked { passed: 2, of: 2 }
        };
        measure(&TASK, 2, &mut ask, &mut run)
    };
    assert_eq!(ran, 0, "there was nothing to run");
    assert_eq!(held.attempts.len(), 2, "and both attempts are still kept");
}
