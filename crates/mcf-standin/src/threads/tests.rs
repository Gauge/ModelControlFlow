//! The partition, checked where its edges are.
//!
//! The property that matters — *the same bytes at one thread and at many* —
//! lives in `tests/threads_do_not_change_the_answer.rs`, over generated inputs
//! and over a model's own forward pass. What is here is the mechanism: that
//! every row is computed exactly once, that a row index is the one the caller
//! will read it back at, and that the origin of a count is not lost.

// Every item in this file is test code.
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]

use super::{Origin, Threads, WORTH_A_WORKER, each_row};

/// A count that says nothing about itself would be a number nobody chose.
#[test]
fn a_count_says_where_it_came_from() {
    assert_eq!(Threads::definition().count(), 1);
    assert_eq!(Threads::definition().origin(), Origin::Definition);
    assert_eq!(Threads::stated(8).count(), 8);
    assert_eq!(Threads::stated(8).origin(), Origin::Stated);

    let machine = Threads::what_the_machine_reports();
    assert!(machine.count() >= 1);
    // Either the machine answered or it did not, and the two are told apart.
    assert!(matches!(
        machine.origin(),
        Origin::MachineReported | Origin::MachineUnreadable
    ));
    if machine.origin() == Origin::MachineUnreadable {
        assert_eq!(machine.count(), 1, "an unreadable machine is one thread");
    }
}

/// Zero threads is not a thread count, and asking for it is still asking.
///
/// The count falls to one because that is the smallest number of threads that
/// exists; the origin stays `Stated` because A7's rule is about not inventing a
/// provenance, and *the caller said zero* is a fact about the caller.
#[test]
fn zero_threads_is_one_thread_and_still_stated() {
    assert_eq!(Threads::stated(0).count(), 1);
    assert_eq!(Threads::stated(0).origin(), Origin::Stated);
}

/// Every origin renders a sentence that says both halves.
#[test]
fn every_origin_describes_itself() {
    assert!(Threads::definition().describe().contains("definition"));
    assert!(Threads::stated(4).describe().contains('4'));
    assert!(Threads::stated(4).describe().contains("asked for"));

    let reported = Threads {
        count: std::num::NonZeroUsize::new(12).unwrap(),
        origin: Origin::MachineReported,
    };
    assert!(reported.describe().contains("12"));
    assert!(reported.describe().contains("this machine reports"));

    let unreadable = Threads {
        count: std::num::NonZeroUsize::MIN,
        origin: Origin::MachineUnreadable,
    };
    assert!(unreadable.describe().contains("does not report"));
}

/// Each row is written exactly once, with the index the caller reads it at.
///
/// Writing the row index into the row is what catches the two failures a
/// partition has: a row nobody computed, and a row computed as if it were
/// somewhere else.
#[test]
fn every_row_is_computed_once_and_knows_its_own_index() {
    for rows in [0_usize, 1, 2, 3, 7, 8, 9, 64, 65] {
        for width in [1_usize, 3] {
            for count in [1_usize, 2, 3, 8, 64] {
                let mut out = vec![-1.0_f32; rows * width];
                // Work per row large enough that every count asked for is
                // earned: this test is about the partition, not about the rule
                // that decides how much of it to use.
                each_row(
                    &mut out,
                    width,
                    WORTH_A_WORKER,
                    Threads::stated(count),
                    &|row, slot| {
                        for (column, cell) in slot.iter_mut().enumerate() {
                            *cell = (row * 100 + column) as f32;
                        }
                    },
                );
                for row in 0..rows {
                    for column in 0..width {
                        assert_eq!(
                            out[row * width + column],
                            (row * 100 + column) as f32,
                            "rows {rows}, width {width}, threads {count}"
                        );
                    }
                }
            }
        }
    }
}

/// A buffer that is not a whole number of rows is computed rather than
/// abandoned.
///
/// The partition of a shape that does not exist is not guessed at — the work
/// still happens, serially, because a caller that got the shape wrong should
/// get the same answer it would have got before threads existed.
#[test]
fn a_buffer_that_is_not_whole_rows_is_still_computed() {
    let mut out = vec![0.0_f32; 7];
    each_row(
        &mut out,
        3,
        WORTH_A_WORKER,
        Threads::stated(4),
        &|row, slot| {
            for cell in slot.iter_mut() {
                *cell = row as f32;
            }
        },
    );
    // Seven cells of three-wide rows: the last row is short and is still
    // written, and nothing is left at its initial value by accident.
    assert_eq!(out, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0]);
}

/// A zero width has no rows and writes nothing, rather than dividing by it.
#[test]
fn a_zero_width_writes_nothing_it_was_not_given() {
    let mut out: Vec<f32> = Vec::new();
    each_row(
        &mut out,
        0,
        WORTH_A_WORKER,
        Threads::stated(4),
        &|_row, _slot| {
            panic!("there is nothing to compute");
        },
    );
    assert!(out.is_empty());
}

/// A job earns workers by its size, and a small one earns few (F99).
///
/// The rule errs toward fewer: a product that earns nothing still gets one
/// worker, because one is not a partition and the serial path is the
/// definition.
#[test]
fn a_job_earns_workers_by_how_much_work_it_is() {
    let many = Threads::stated(32);
    assert_eq!(many.worth_starting(0), 1, "nothing earns one worker");
    assert_eq!(
        many.worth_starting(WORTH_A_WORKER - 1),
        1,
        "less than one worker's worth of work earns one worker"
    );
    assert_eq!(many.worth_starting(WORTH_A_WORKER * 8), 8);
    assert_eq!(
        many.worth_starting(WORTH_A_WORKER * 1_000),
        32,
        "a job can never earn more workers than the caller has threads"
    );

    // And the count is a ceiling the rule cannot lift.
    assert_eq!(Threads::definition().worth_starting(usize::MAX), 1);
    assert_eq!(Threads::stated(4).worth_starting(usize::MAX), 4);
}

/// The rule changes how many workers run and never what they produce.
///
/// Which is the whole reason it is allowed to exist: a speed rule that could
/// move an answer would be exactly the thing B-366 forbids.
#[test]
fn the_worker_rule_does_not_change_the_answer() {
    for work_per_row in [1_usize, 7, WORTH_A_WORKER, WORTH_A_WORKER * 100] {
        let mut produced = vec![0.0_f32; 129];
        each_row(
            &mut produced,
            1,
            work_per_row,
            Threads::stated(16),
            &|row, slot| {
                if let Some(cell) = slot.first_mut() {
                    *cell = row as f32;
                }
            },
        );
        let expected: Vec<f32> = (0..129).map(|row| row as f32).collect();
        assert_eq!(produced, expected, "work per row {work_per_row}");
    }
}
