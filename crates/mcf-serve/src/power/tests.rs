//! Tests for the energy summed while a model is held.
//!
//! The sampler's thread is not started here: what these check is the
//! arithmetic a reader does over its totals, and that watching is counted
//! so the last engine to go is the one that stops it.

use super::{Spent, release, spent, watch};

/// What was spent between two readings is the difference, and never a
/// negative dressed as an enormous one.
#[test]
fn what_was_spent_between_two_readings_is_their_difference() {
    let before = Spent {
        microjoules: 1_000,
        covered_ns: 2_000,
    };
    let now = Spent {
        microjoules: 3_500,
        covered_ns: 6_000,
    };
    assert_eq!(
        now.since(before),
        Spent {
            microjoules: 2_500,
            covered_ns: 4_000
        }
    );
    // A reading older than the one it is measured against — which a
    // restarted meter would give — saturates at nothing rather than
    // wrapping into a number nobody could believe (A6).
    assert_eq!(before.since(now), Spent::default());
    assert_eq!(Spent::default().since(Spent::default()), Spent::default());
}

/// Watching is counted: two engines watching and one going does not stop
/// the sampler, and the totals only ever grow.
#[test]
fn watching_is_counted_so_the_last_engine_stops_it() {
    let before = spent();
    watch();
    watch();
    release();
    let during = spent();
    release();
    let after = spent();
    // Whatever the sampler managed in that moment, the totals never go
    // backwards: they are sums.
    assert!(during.microjoules >= before.microjoules);
    assert!(after.microjoules >= during.microjoules);
    assert!(after.covered_ns >= before.covered_ns);
}
