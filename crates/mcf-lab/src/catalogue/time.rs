//! Scenarios in which the wall clock moves.
//!
//! D9: *clock anomalies are events, not corrections. A backward step or a large
//! forward jump during a measurement invalidates that measurement loudly rather
//! than being smoothed away.* B-184's second condition is that a clock-jump
//! scenario **invalidates rather than corrupts**, and these are it.
//!
//! **What is simulated is the observation, not the cause** (D26). MCF cannot
//! step a machine's clock and should not try: what it observes after an NTP
//! correction, a suspend-resume or a virtual-machine migration is two readings
//! that disagree, and two readings that disagree is what these construct.
//!
//! The disposition on both is `invalidated`, which is the point. The record
//! entry is sound; what is unsound is anything that was being measured across
//! the anomaly, and §3.4 has one word for a result that completed and cannot be
//! believed.

use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::time::{Instant, Timestamp};
use mcf_record::journal::{Reading, clock_anomaly_between};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The calendar steps back between two records.
pub(super) const BACKWARD_STEP: Scenario = Scenario {
    id: "time/backward-step",
    produces: Category::TimeJumpBackward,
    summary: "the wall clock steps backwards while the monotonic clock advances",
    run: backward_step,
};

/// The calendar leaps forward while almost no time passes.
pub(super) const FORWARD_JUMP: Scenario = Scenario {
    id: "time/forward-jump",
    produces: Category::TimeJumpForward,
    summary: "the wall clock advances far further than time actually elapsed",
    run: forward_jump,
};

/// One second, in nanoseconds, as both clocks count them.
const SECOND: u64 = 1_000_000_000;

/// One millisecond, which is about how long a scenario's two readings would
/// really be apart.
const MILLISECOND: u64 = 1_000_000;

/// An hour, which is what a time-zone correction or a resumed virtual machine
/// moves the calendar by.
#[allow(
    clippy::cast_possible_wrap,
    reason = "3600 seconds in nanoseconds is 3.6e12, far inside i128"
)]
const HOUR: i128 = 3_600 * SECOND as i128;

fn reading(wall_nanos: i128, monotonic_nanos: u64) -> Reading {
    Reading {
        wall: Timestamp::from_utc_nanos(wall_nanos, Attested::Unknown),
        monotonic: Instant::from_nanos(monotonic_nanos),
    }
}

fn backward_step(_world: &World) -> Outcome {
    // A millisecond of real time passes; the calendar goes back an hour, which
    // is what an NTP correction or a hand-set clock looks like from inside.
    let earlier = reading(HOUR, 0);
    let later = reading(0, MILLISECOND);

    match clock_anomaly_between(earlier, later) {
        Some(failure) => Outcome::Produced(failure),
        None => Outcome::Unexpected("a backwards clock was not noticed".to_owned()),
    }
}

fn forward_jump(_world: &World) -> Outcome {
    // A millisecond of real time passes; the calendar advances an hour. A
    // suspend-resume looks like this, and so does a virtual machine that was
    // paused.
    let earlier = reading(0, 0);
    let later = reading(HOUR, MILLISECOND);

    match clock_anomaly_between(earlier, later) {
        Some(failure) => Outcome::Produced(failure),
        None => Outcome::Unexpected("a forward jump was not noticed".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SECOND, reading};
    use mcf_record::journal::{TOLERANCE, clock_anomaly_between};

    /// Time passing is not an event (B4). Two readings that agree produce
    /// nothing, which is what stops the detector reporting every append.
    #[test]
    fn agreeing_clocks_are_not_an_anomaly() {
        let earlier = reading(0, 0);
        let later = reading(i128::from(SECOND), SECOND);
        assert!(clock_anomaly_between(earlier, later).is_none());
    }

    /// The tolerance is a boundary, and both sides of it are checked: a
    /// divergence inside it is scheduling, and one outside it is the calendar.
    #[test]
    fn the_tolerance_separates_scheduling_from_a_jump() {
        let inside =
            clock_anomaly_between(reading(0, 0), reading(i128::from(TOLERANCE.as_nanos()), 0));
        assert!(inside.is_none(), "the tolerance itself reported a jump");

        let outside = clock_anomaly_between(
            reading(0, 0),
            reading(i128::from(TOLERANCE.as_nanos()) + 1, 0),
        );
        assert!(
            outside.is_some(),
            "one nanosecond past the tolerance did not"
        );
    }
}
