//! Work is counted, and a duration is only ever derived from it.

use mcf_core::measurement::{Basis, Estimate};
use mcf_core::time::{Duration, Monotonic};

use super::Work;

fn each(low: u64, high: u64) -> Estimate<Duration<Monotonic>> {
    Estimate::band(
        Duration::from_nanos(low),
        Duration::from_nanos(high),
        Basis::LocalHistory,
    )
}

#[test]
fn the_counts_multiply_out() {
    let work = Work {
        trials: 200,
        arms: 2,
        tokens: 128,
    };
    assert_eq!(work.generations(), 400);
    assert_eq!(work.tokens(), 51_200);
}

#[test]
fn a_band_widens_with_the_count() {
    let work = Work {
        trials: 10,
        arms: 2,
        tokens: 8,
    };
    let expected = work.expected(&each(1_000, 3_000));
    assert_eq!(expected.low().as_nanos(), 20_000);
    assert_eq!(expected.high().as_nanos(), 60_000);
}

#[test]
fn the_basis_survives_the_derivation() {
    // An estimate derived from local history is still from local history, and
    // a reader who is told otherwise cannot tell how much to believe it.
    let work = Work {
        trials: 1,
        arms: 1,
        tokens: 1,
    };
    assert_eq!(work.expected(&each(5, 7)).basis(), &Basis::LocalHistory);
}

#[test]
fn no_work_is_no_time() {
    let work = Work {
        trials: 0,
        arms: 2,
        tokens: 128,
    };
    assert_eq!(work.expected(&each(1_000, 3_000)).low().as_nanos(), 0);
}

#[test]
fn an_absurd_count_saturates_rather_than_wrapping() {
    // A wrapped multiplication would report a century of work as a
    // microsecond, which is the one wrong answer that reads as reassuring.
    let work = Work {
        trials: usize::MAX,
        arms: 2,
        tokens: u32::MAX,
    };
    assert_eq!(work.generations(), u64::MAX);
    assert_eq!(work.tokens(), u64::MAX);
    assert_eq!(work.expected(&each(1, 2)).high().as_nanos(), u64::MAX);
}

#[test]
fn the_declaration_reads_in_countable_units() {
    let shown = Work {
        trials: 200,
        arms: 2,
        tokens: 128,
    }
    .to_string();
    assert!(shown.contains("200 paired trial(s)"), "{shown}");
    assert!(shown.contains("400 generation(s)"), "{shown}");
    assert!(shown.contains("51200 token(s) in all"), "{shown}");
    // B-224: never in minutes.
    for unit in ["minute", "second", "hour", "ms"] {
        assert!(!shown.contains(unit), "{unit} in {shown}");
    }
}

/// A budget names what it excludes, or refuses.
mod budgets {
    use super::each;
    use crate::planned::{Proposal, Work};
    use mcf_core::time::Duration;

    fn ceiling() -> Work {
        Work {
            trials: 20,
            arms: 2,
            tokens: 128,
        }
    }

    #[test]
    fn a_budget_that_covers_everything_excludes_nothing() {
        let held = Proposal::within(ceiling(), &each(1, 1_000), Duration::from_nanos(60_000), 2);
        assert_eq!(held, Proposal::Whole { running: ceiling() });
    }

    #[test]
    fn six_of_twenty_arrives_with_the_fourteen() {
        // §3.1's acceptance, arithmetically: 2000ns a trial at the slow edge,
        // 12_000ns of budget, so six trials fit.
        let held = Proposal::within(
            ceiling(),
            &each(500, 1_000),
            Duration::from_nanos(12_000),
            2,
        );
        let Proposal::Fewer { running, excluded } = held else {
            panic!("a budget that covers part of the work proposes part of it: {held:?}");
        };
        assert_eq!(running.trials, 6);
        assert_eq!(excluded.trials, 14);
        assert_eq!(running.tokens, excluded.tokens);
    }

    #[test]
    fn the_excluded_half_is_in_the_sentence() {
        let shown = Proposal::within(
            ceiling(),
            &each(500, 1_000),
            Duration::from_nanos(12_000),
            2,
        )
        .to_string();
        assert!(shown.contains("EXCLUDED"), "{shown}");
        assert!(shown.contains("14 paired trial(s)"), "{shown}");
    }

    #[test]
    fn a_budget_below_a_comparison_refuses_rather_than_shrinking() {
        let held = Proposal::within(ceiling(), &each(500, 1_000), Duration::from_nanos(3_000), 2);
        assert_eq!(
            held,
            Proposal::NotEnough {
                least: Work {
                    trials: 2,
                    ..ceiling()
                }
            }
        );
        assert_eq!(held.running(), None);
        assert!(held.to_string().contains("less than a comparison"));
    }

    #[test]
    fn planning_is_against_the_slow_edge() {
        // Fast edge 1ns, slow edge 1000ns, budget 12_000ns. Against the fast
        // edge six thousand trials would "fit"; against the slow one, six.
        let held = Proposal::within(ceiling(), &each(1, 1_000), Duration::from_nanos(12_000), 2);
        assert_eq!(held.running().map(|work| work.trials), Some(6));
    }
}
