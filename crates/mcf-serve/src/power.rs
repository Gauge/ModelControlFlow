//! The energy the machine spends while MCF is holding a model (B-596).
//!
//! **A power reading is an instant and energy is an integral**, so there is
//! no counter to read: the only way to know what a hold cost is to sample
//! the draw and sum it over the time between samples. B-593 did that at the
//! moments somebody asked what the hold was doing, which measures the hold
//! only while a window is open on it — and the case a person most wants the
//! figure for is the one where they are driving the model from another
//! machine and there is no window at all (F277).
//!
//! **So the sampler runs while MCF holds an engine, and not otherwise.** It
//! starts when the first engine registers as live and stops when the last
//! one goes, which is exactly the time MCF is doing something for somebody.
//! A daemon holding nothing samples nothing, so *it costs nothing while
//! nobody is asking* stands (B-031, B-071, F267); what it now means is that
//! nothing is held.
//!
//! **What is summed is the machine's, and it says so.** The driver
//! publishes one figure for the chip it is on, and on a machine whose
//! graphics are part of the processor that figure is the whole package —
//! the label the driver gives it is carried alongside so that no surface
//! has to guess whose watts these are (F277, A8).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mcf_core::time::{Clock as _, Instant, Monotonic, SystemClock};

/// How often the draw is read while something is held.
///
/// Twice a second: fast enough that a generation of a few seconds is
/// several samples, slow enough that the reading itself is nothing — two
/// small files from the kernel's hardware-monitor class.
const EVERY: std::time::Duration = std::time::Duration::from_millis(500);

/// The energy summed so far, and the time those samples cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spent {
    /// Microjoules, by the trapezoid rule over consecutive samples.
    pub microjoules: u64,
    /// The nanoseconds those samples span.
    pub covered_ns: u64,
}

impl Spent {
    /// What was spent between an earlier reading and this one.
    #[must_use]
    pub const fn since(self, before: Self) -> Self {
        Self {
            microjoules: self.microjoules.saturating_sub(before.microjoules),
            covered_ns: self.covered_ns.saturating_sub(before.covered_ns),
        }
    }
}

/// What the sampler holds between the thread and its readers.
#[derive(Debug, Default)]
struct Meter {
    /// The running totals.
    spent: Spent,
    /// How many engines are live; the thread runs while this is above
    /// nothing.
    watching: usize,
    /// Raised to end the thread.
    stop: Option<Arc<AtomicBool>>,
}

fn meter() -> &'static Mutex<Meter> {
    static METER: std::sync::OnceLock<Mutex<Meter>> = std::sync::OnceLock::new();
    METER.get_or_init(|| Mutex::new(Meter::default()))
}

/// What has been summed since the daemon started, over the time it covers.
#[must_use]
pub fn spent() -> Spent {
    meter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .spent
}

/// An engine is live: sample while it is, starting the sampler where this
/// is the first.
pub fn watch() {
    let mut held = meter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.watching = held.watching.saturating_add(1);
    if held.stop.is_some() {
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    held.stop = Some(Arc::clone(&stop));
    drop(held);
    // The thread ends when the flag is raised, which is when the last
    // engine goes; a daemon that holds nothing has no thread at all.
    let started = std::thread::Builder::new()
        .name("mcf-power".to_owned())
        .spawn(move || sampling(&stop));
    if started.is_err() {
        // No thread is no energy, which is said by the coverage staying at
        // nothing rather than by a figure nobody took (A7).
        let mut held = meter()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.stop = None;
    }
}

/// An engine has gone: stop sampling where it was the last.
pub fn release() {
    let mut held = meter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.watching = held.watching.saturating_sub(1);
    if held.watching == 0
        && let Some(stop) = held.stop.take()
    {
        stop.store(true, Ordering::Relaxed);
    }
}

/// Reads the draw until told to stop, summing each interval's mean over
/// its own length — the trapezoid rule, in microwatts and nanoseconds:
/// microwatts times nanoseconds over a thousand million is microjoules.
fn sampling(stop: &AtomicBool) {
    let mut before: Option<(Instant<Monotonic>, i64)> = None;
    while !stop.load(Ordering::Relaxed) {
        if let Some(power) = crate::engines::card_sensors().power_uw {
            let now = SystemClock.now();
            if let Some((then, was)) = before {
                let over = now.saturating_duration_since(then).as_nanos();
                let mean = was.saturating_add(power).max(0);
                #[expect(
                    clippy::integer_division,
                    reason = "the mean of two draws over the time between, as microjoules; \
                              what is discarded is under a microjoule"
                )]
                let spent = u64::try_from(mean).unwrap_or(0).saturating_mul(over) / 2_000_000_000;
                let mut held = meter()
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                held.spent.microjoules = held.spent.microjoules.saturating_add(spent);
                held.spent.covered_ns = held.spent.covered_ns.saturating_add(over);
            }
            before = Some((now, power));
        } else {
            // A card that will not say has no reading to carry forward: the
            // next interval is not spanned by a guess (A7).
            before = None;
        }
        std::thread::sleep(EVERY);
    }
}

#[cfg(test)]
mod tests;
