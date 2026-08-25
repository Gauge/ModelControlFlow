//! Noticing that the wall clock moved when it should not have.
//!
//! D9: *clock anomalies are events, not corrections. A backward step or a large
//! forward jump during a measurement invalidates that measurement loudly rather
//! than being smoothed away.* B37 is the rule and B-184's second condition is
//! the demonstration: **a clock-jump scenario invalidates rather than
//! corrupts.**
//!
//! **The detector needs both clocks, which is why it lives here.** A wall-clock
//! reading alone cannot say whether time passed or the clock moved — that is
//! the whole reason B37 keeps [`Timestamp`] and [`Instant`] apart. Between two
//! appends MCF holds both: how far the calendar advanced, and how much time
//! actually elapsed. When those disagree, the calendar is what moved.
//!
//! | What the two clocks say | What it is |
//! |---|---|
//! | The calendar went backwards | `time.jump.backward` |
//! | The calendar advanced much further than the monotonic clock | `time.jump.forward` |
//! | They agree within [`TOLERANCE`] | Nothing; time passed |
//!
//! **The anomaly does not stop the append.** A1 forbids losing information and
//! the event being recorded did happen; what is unsound is anything that was
//! being *measured* across the anomaly. So the entry is written, the anomaly is
//! written beside it, and the caller is told — with a disposition of
//! `invalidated`, which is what §3.4 says about a measurement whose conditions
//! moved underneath it.
//!
//! [`Timestamp`]: mcf_core::time::Timestamp
//! [`Instant`]: mcf_core::time::Instant

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::{Duration, Instant, Monotonic, Timestamp};

const WHERE: Subsystem = Subsystem::new("mcf-record::journal::anomaly");

/// How far the two clocks may disagree before it is the calendar that moved.
///
/// One second. The number is a judgement and is stated rather than hidden: the
/// two readings are taken microseconds apart around the same append, so a
/// second of divergence cannot be scheduling — an NTP step, a suspend-resume, a
/// virtual machine migration or a hand-set clock are what produce it. A smaller
/// tolerance would report a busy machine as a clock jump; a larger one would
/// miss the smallest correction that still ruins a millisecond-scale
/// measurement.
pub const TOLERANCE: Duration<Monotonic> = Duration::from_nanos(1_000_000_000);

/// The two clocks, read together.
///
/// Read at the same moment on purpose. A pair taken microseconds apart is a
/// pair; two readings taken at different points are two facts that cannot be
/// compared, which is the mistake this type exists to make unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// What the calendar said.
    pub wall: Timestamp,
    /// What the monotonic clock said.
    pub monotonic: Instant<Monotonic>,
}

/// Compares two readings and says whether the calendar moved.
///
/// Returns `None` when the two clocks agree, which is the ordinary case and is
/// not an event: B4 writes when something happens, and time passing is not
/// something happening.
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

    // The calendar's advance, in nanoseconds. Both are non-negative here
    // because the backward case returned above.
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
        // §3.4: a measurement whose conditions moved underneath it is unsound,
        // and *invalidated* is the disposition for a result that completed and
        // cannot be believed. The record entry itself is fine; what the anomaly
        // invalidates is anything being measured across it.
        Disposition::Invalidated,
        WHERE,
        detail,
    )
}
