use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::{Duration, Instant, Monotonic, Timestamp};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::anomaly");

pub const TOLERANCE: Duration<Monotonic> = Duration::from_nanos(1_000_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub wall: Timestamp,
    pub monotonic: Instant<Monotonic>,
}

#[must_use]
pub fn between(earlier: Reading, later: Reading) -> Option<Failure> {
    let elapsed = later.monotonic.saturating_duration_since(earlier.monotonic);

    if later.wall.utc_nanos() < earlier.wall.utc_nanos() {
        return Some(
            anomaly(
                Category::TimeJumpBackward,
                "the wall clock stepped backwards between two records",
            )
            .with_context("from", earlier.wall.to_string())
            .with_context("to", later.wall.to_string())
            .with_context("monotonic_elapsed_ns", elapsed.as_nanos().to_string()),
        );
    }

    let calendar = later
        .wall
        .utc_nanos()
        .saturating_sub(earlier.wall.utc_nanos());
    let monotonic = i128::from(elapsed.as_nanos());
    let overshoot = calendar.saturating_sub(monotonic);

    if overshoot > i128::from(TOLERANCE.as_nanos()) {
        return Some(
            anomaly(
                Category::TimeJumpForward,
                "the wall clock advanced much further than time actually elapsed",
            )
            .with_context("calendar_advance_ns", calendar.to_string())
            .with_context("monotonic_elapsed_ns", monotonic.to_string())
            .with_context("tolerance_ns", TOLERANCE.as_nanos().to_string()),
        );
    }

    None
}

fn anomaly(category: Category, detail: &'static str) -> Failure {
    Failure::new(
        category,
        Attribution::Machine,
        Disposition::Invalidated,
        WHERE,
        detail,
    )
}
