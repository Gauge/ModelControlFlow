use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mcf_core::time::{Clock as _, Instant, Monotonic, SystemClock};

const EVERY: std::time::Duration = std::time::Duration::from_millis(500);

pub const PRICE_VARIABLE: &str = "MCF_PRICE_PER_KWH";

#[must_use]
pub fn price() -> Option<mcf_core::price::PricePerKwh> {
    std::env::var(PRICE_VARIABLE)
        .ok()
        .as_deref()
        .and_then(mcf_core::price::PricePerKwh::parse)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Spent {
    pub microjoules: u64,
    pub covered_ns: u64,
}

impl Spent {
    #[must_use]
    pub const fn since(self, before: Self) -> Self {
        Self {
            microjoules: self.microjoules.saturating_sub(before.microjoules),
            covered_ns: self.covered_ns.saturating_sub(before.covered_ns),
        }
    }
}

#[derive(Debug, Default)]
struct Meter {
    spent: Spent,
    watching: usize,
    stop: Option<Arc<AtomicBool>>,
}

fn meter() -> &'static Mutex<Meter> {
    static METER: std::sync::OnceLock<Mutex<Meter>> = std::sync::OnceLock::new();
    METER.get_or_init(|| Mutex::new(Meter::default()))
}

#[must_use]
pub fn spent() -> Spent {
    meter()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .spent
}

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
    let started = std::thread::Builder::new()
        .name("mcf-power".to_owned())
        .spawn(move || sampling(&stop));
    if started.is_err() {
        let mut held = meter()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.stop = None;
    }
}

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
            before = None;
        }
        std::thread::sleep(EVERY);
    }
}

#[cfg(test)]
mod tests;
