#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]

use super::{Origin, Threads, WORTH_A_WORKER, each_row};

#[test]
fn a_count_says_where_it_came_from() {
    assert_eq!(Threads::definition().count(), 1);
    assert_eq!(Threads::definition().origin(), Origin::Definition);
    assert_eq!(Threads::stated(8).count(), 8);
    assert_eq!(Threads::stated(8).origin(), Origin::Stated);

    let machine = Threads::what_the_machine_reports();
    assert!(machine.count() >= 1);
    assert!(matches!(
        machine.origin(),
        Origin::MachineReported | Origin::MachineUnreadable
    ));
    if machine.origin() == Origin::MachineUnreadable {
        assert_eq!(machine.count(), 1, "an unreadable machine is one thread");
    }
}

#[test]
fn zero_threads_is_one_thread_and_still_stated() {
    assert_eq!(Threads::stated(0).count(), 1);
    assert_eq!(Threads::stated(0).origin(), Origin::Stated);
}

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

#[test]
fn every_row_is_computed_once_and_knows_its_own_index() {
    for rows in [0_usize, 1, 2, 3, 7, 8, 9, 64, 65] {
        for width in [1_usize, 3] {
            for count in [1_usize, 2, 3, 8, 64] {
                let mut out = vec![-1.0_f32; rows * width];
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
    assert_eq!(out, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0]);
}

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

    assert_eq!(Threads::definition().worth_starting(usize::MAX), 1);
    assert_eq!(Threads::stated(4).worth_starting(usize::MAX), 4);
}

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
