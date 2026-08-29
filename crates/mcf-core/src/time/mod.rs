//! The time model: two clocks, two types, and no arithmetic between them.
//!
//! D9 standardizes time because it is one of the conditions every measurement
//! carries (§3.4). B37 is its enforceable form, and B-184 is the item:
//! subtracting one wall-clock reading from another should not compile.
//!
//! Three separations are structural rather than conventional:
//!
//! * **[`Timestamp`] and [`Duration`] are different types with no subtraction
//!   between them.** A wall-clock reading names a moment in a calendar that
//!   steps, drifts and is adjusted underneath a running process; subtracting
//!   two of them silently reports an NTP correction as latency. There is no
//!   `impl Sub for Timestamp`, so the mistake has no spelling.
//! * **A duration comes from an [`Instant`], and an instant comes from a
//!   [`Clock`].** Instants are ordered and subtractable and have no calendar
//!   meaning at all, which is what a monotonic reading actually is.
//! * **A duration knows which clock produced it.** A11 forbids a performance
//!   number originating in simulation, so [`Duration::is_simulated`] is
//!   carried on the value rather than remembered by its caller, and B-082's
//!   check has something to stand on.
//!
//! What is *not* here is the clock-anomaly half of D9. A backward step during
//! a measurement invalidates it loudly (`time.jump.backward`), and
//! demonstrating that requires the laboratory to produce the step — B-009,
//! which waits on DEC-021. The types below are built so that the scenario has
//! somewhere to land: an [`Instant`] carries its clock's identity, so two
//! instants from different clock instances cannot be subtracted at all.

mod clock;
mod duration;
mod instant;
mod timestamp;
mod zone;

pub use clock::{Clock, ClockKind, Measurable, Monotonic, Simulated, SimulatedClock, SystemClock};
pub use duration::Duration;
pub use instant::Instant;
pub use timestamp::{Civil, Timestamp, UtcOffset};
pub use zone::{Zone, offset_at};

#[cfg(test)]
mod tests;
