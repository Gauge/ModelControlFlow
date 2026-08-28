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
