use core::fmt::Write as _;

use super::{Change, Run, State, followed, said};

fn lines_for(states: &[State]) -> String {
    states
        .iter()
        .enumerate()
        .fold(String::new(), |mut held, (at, state)| {
            let _wrote = writeln!(held, "{}: {}", at + 1, said(*state));
            held
        })
}

fn answer_for(run: &Run) -> String {
    lines_for(&run.states())
}

#[test]
fn the_same_seed_makes_the_same_run_and_another_seed_another() {
    let one = Run::seeded(7, 150);
    assert_eq!(one, Run::seeded(7, 150));
    assert_ne!(one, Run::seeded(8, 150));
    assert_eq!(one.changes.len(), 150);
}

#[test]
fn a_run_comes_back_whole_from_its_check() {
    let run = Run::seeded(301, 150);
    assert_eq!(Run::from_checked(&run.checked()), Some(run));
    assert_eq!(Run::from_checked("assert f(1) == 2"), None);
}

#[test]
fn every_change_is_said_so_it_reads_back_as_itself() {
    for change in Run::seeded(11, 400).changes {
        assert_eq!(
            Change::read(&change.said()),
            Some(change),
            "{}",
            change.said()
        );
    }
}

#[test]
fn sums_wrap_round_within_a_digit() {
    let start: State = [8, 2, 0, 0, 0];
    assert_eq!(Change::Add(0, 5).applied(start), [3, 2, 0, 0, 0]);
    assert_eq!(Change::Take(1, 4).applied(start), [8, 8, 0, 0, 0]);
    assert_eq!(Change::Swap(0, 4).applied([1, 2, 3, 4, 5]), [5, 2, 3, 4, 1]);
    assert_eq!(
        Change::Copy { into: 2, from: 0 }.applied([1, 2, 3, 4, 5]),
        [1, 2, 1, 4, 5]
    );
}

#[test]
fn a_run_followed_exactly_holds_every_step() {
    let run = Run::seeded(3, 120);
    let came = followed(&run, &answer_for(&run));
    assert_eq!((came.held, came.of, came.first_slip), (120, 120, None));
}

#[test]
fn one_slip_costs_one_step_and_not_every_step_after_it() {
    let run = Run::seeded(5, 60);
    let mut states = run.states();
    // A wrong line at step 20, carried on from faithfully: the model lost its place once.
    let slipped = states.get(19).copied().unwrap_or_default();
    let wrong = [
        (slipped[0] + 1) % 10,
        slipped[1],
        slipped[2],
        slipped[3],
        slipped[4],
    ];
    let mut before = wrong;
    for (at, change) in run.changes.iter().enumerate().skip(20) {
        before = change.applied(before);
        if let Some(held) = states.get_mut(at) {
            *held = before;
        }
    }
    if let Some(held) = states.get_mut(19) {
        *held = wrong;
    }
    let came = followed(&run, &lines_for(&states));
    assert_eq!((came.held, came.of), (59, 60));
    assert_eq!(came.first_slip.map(|slip| slip.step), Some(20));
}

#[test]
fn stopping_half_way_counts_every_step_not_written() {
    let run = Run::seeded(9, 100);
    let whole = answer_for(&run);
    let half = whole.lines().take(50).collect::<Vec<_>>().join("\n");
    let came = followed(&run, &half);
    assert_eq!((came.held, came.of), (50, 100));
    assert_eq!(
        came.first_slip.map(|slip| (slip.step, slip.wrote)),
        Some((51, None))
    );
}

#[test]
fn what_is_written_around_a_line_is_not_held_against_it() {
    let run = Run {
        start: [1, 2, 3, 4, 5],
        changes: vec![Change::Add(0, 1), Change::Swap(0, 1), Change::Set(4, 0)],
    };
    let answer = "Here we go.\n\
                  **Step 1:** 2 2 3 4 5\n\
                  - 2. b=2 a=2 c=3 d=4 e=5\n\
                  `3: 2, 2, 3, 4, 0` (e set to nought)\n";
    let came = followed(&run, answer);
    assert_eq!((came.held, came.of), (3, 3));
}

#[test]
fn a_line_that_is_not_five_registers_is_no_line() {
    let run = Run {
        start: [1, 2, 3, 4, 5],
        changes: vec![Change::Add(0, 1)],
    };
    assert_eq!(followed(&run, "1: 2 2 3 4").held, 0);
    assert_eq!(followed(&run, "1. a += 1").held, 0);
    assert_eq!(followed(&run, "1: 2 2 3 4 5").held, 1);
}

#[test]
fn only_the_first_line_for_a_step_is_marked() {
    let run = Run {
        start: [1, 2, 3, 4, 5],
        changes: vec![Change::Add(0, 1)],
    };
    assert_eq!(followed(&run, "1: 9 9 9 9 9\n1: 2 2 3 4 5").held, 0);
}

#[test]
fn a_line_that_says_its_step_is_read_after_the_arrow_and_by_name() {
    let run = Run {
        start: [1, 2, 3, 4, 5],
        changes: vec![Change::Swap(3, 1), Change::Add(4, 7)],
    };
    let answer = "1: swap d b -> a=1 b=4 c=3 d=2 e=5\n\
                  2: e += 7 -> e=2 a=1 b=4 c=3 d=2\n";
    assert_eq!(followed(&run, answer).held, 2);
}

#[test]
fn a_slip_is_said_by_the_registers_that_were_wrong() {
    let slip = super::Slip {
        step: 23,
        wrote: Some([5, 2, 9, 5, 5]),
        wanted: [5, 2, 9, 1, 5],
    };
    assert_eq!(slip.said(), "first slip at step 23: wrote d=5, wanted d=1");
    let missing = super::Slip {
        wrote: None,
        ..slip
    };
    assert_eq!(missing.said(), "first slip at step 23: no line for it");
}
