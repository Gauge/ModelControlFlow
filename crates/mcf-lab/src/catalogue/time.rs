use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::time::{Instant, Timestamp};
use mcf_record::journal::{Reading, clock_anomaly_between};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const BACKWARD_STEP: Scenario = Scenario {
    id: "time/backward-step",
    produces: Category::TimeJumpBackward,
    summary: "the wall clock steps backwards while the monotonic clock advances",
    run: backward_step,
};

pub(super) const FORWARD_JUMP: Scenario = Scenario {
    id: "time/forward-jump",
    produces: Category::TimeJumpForward,
    summary: "the wall clock advances far further than time actually elapsed",
    run: forward_jump,
};

const SECOND: u64 = 1_000_000_000;

const MILLISECOND: u64 = 1_000_000;

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
    let earlier = reading(HOUR, 0);
    let later = reading(0, MILLISECOND);

    match clock_anomaly_between(earlier, later) {
        Some(failure) => Outcome::Produced(failure),
        None => Outcome::Unexpected("a backwards clock was not noticed".to_owned()),
    }
}

fn forward_jump(_world: &World) -> Outcome {
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

    #[test]
    fn agreeing_clocks_are_not_an_anomaly() {
        let earlier = reading(0, 0);
        let later = reading(i128::from(SECOND), SECOND);
        assert!(clock_anomaly_between(earlier, later).is_none());
    }

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
