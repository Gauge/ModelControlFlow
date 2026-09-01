//! What a laboratory may and may not conclude (B-110, B40, B41, A7).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
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

/// A run that broke is not a model that failed (B40).
///
/// The failure this whole type exists to prevent: a container that would not
/// start, scored as zero, becomes a verdict about a model arrived at by
/// grading MCF's own machinery.
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

/// A model that wrote nothing is told apart from one whose code broke.
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

/// A partly-right function is not partly right.
///
/// One case of two is a function that is wrong. Giving it half a mark would be
/// partial credit nobody defined, and it would let a model that handles the
/// easy input outrank one that handles neither.
#[test]
fn a_function_that_satisfies_some_cases_is_not_partly_correct() {
    let graded = trials(&[Ran::Checked { passed: 1, of: 2 }, Ran::Checked { passed: 2, of: 2 }])
        .graded();
    let score = graded.score().expect("something ran, so there is a reading");
    assert_eq!(
        score.parts_per_million(),
        500_000,
        "one attempt of two was whole, so the reading is one in two — not three quarters"
    );
}

/// The distribution is kept, not the best of it.
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
    // And the reading is over the attempts made, not over the ones that ran:
    // a model whose answers often fail to run is a model that often fails.
    let score = held.graded().score().expect("something ran").parts_per_million();
    assert_eq!(score, 333_333);
}

/// No attempts at all is Unknown, and never zero.
#[test]
fn no_attempts_is_unknown() {
    let graded = trials(&[]).graded();
    assert!(graded.score().is_none());
    assert!(matches!(graded, Graded::Unknown { .. }));
}

/// A score belongs to its own laboratory and compares with nothing else.
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

/// Asking and running are the caller's, and every attempt goes through both.
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

/// A model that says nothing is never handed to the runner.
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
