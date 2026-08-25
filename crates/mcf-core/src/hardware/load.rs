//! What else the machine is doing, and whether a reading can be attributed.
//!
//! §3.8 makes what else the machine is doing part of what the machine *is* at a
//! moment, so the load average is read with everything else and recorded as a
//! condition (§3.4).
//!
//! **It decides nothing.** MCF tried to decide attributability with it and F3
//! in `doc/findings.md` records why that failed: a one-minute average cannot
//! answer a question about a 140-millisecond measurement, and it read 0.29 both
//! on a quiet machine and while thirty-two processes were spinning. D30 moved
//! the verdict to the scheduling delay, which is measured over the measurement
//! itself. What remains here is a true statement about the machine, kept
//! because a reader looking at an unattributable result wants to know what else
//! was running.

use core::fmt;

use crate::attested::Attested;

/// The one-minute load average, in thousandths.
///
/// Thousandths rather than a float, for the reason `Quantity` gives: nothing
/// measured in MCF is floating point, and a condition recorded beside a
/// measurement is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadAverage(pub u64);

impl fmt::Display for LoadAverage {
    // Integer division is the conversion; both operands are bounded by the
    // type and neither can overflow or lose a sign.
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.0 / 1000, self.0 % 1000)
    }
}

/// Reads the one-minute load average.
///
/// `None` where the platform does not publish one (A7).
#[must_use]
pub fn load_average() -> Attested<LoadAverage> {
    let Ok(text) = std::fs::read_to_string("/proc/loadavg") else {
        return Attested::Unknown;
    };
    let Some(first) = text.split_whitespace().next() else {
        return Attested::Unknown;
    };
    match parse_thousandths(first) {
        Some(load) => Attested::Known(LoadAverage(load)),
        None => Attested::Unknown,
    }
}

/// `"1.23"` → `1230`. Parsed as two integers, because this crate has no
/// floating point (see `Quantity`) and a condition is not a reason to acquire
/// one.
fn parse_thousandths(text: &str) -> Option<u64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, "0"));
    let whole: u64 = whole.parse().ok()?;
    let mut digits = fraction.chars().filter(char::is_ascii_digit);
    let mut thousandths = 0_u64;
    for place in [100_u64, 10, 1] {
        let digit = digits.next().and_then(|c| c.to_digit(10)).unwrap_or(0);
        thousandths = thousandths.saturating_add(u64::from(digit).saturating_mul(place));
    }
    Some(whole.saturating_mul(1000).saturating_add(thousandths))
}
