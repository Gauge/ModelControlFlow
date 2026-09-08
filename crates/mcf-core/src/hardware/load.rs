use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadAverage(pub u64);

impl fmt::Display for LoadAverage {
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.0 / 1000, self.0 % 1000)
    }
}

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
