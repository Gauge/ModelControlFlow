use core::fmt;

/// What a kilowatt-hour costs where this machine is, in millionths of a
/// currency unit. MCF holds no currency: the number is the one the person
/// typed, and it comes back out beside the energy it priced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PricePerKwh {
    pub millionths: u64,
}

/// A millijoule is this many millionths of a kilowatt-hour's price divisor:
/// one kilowatt-hour is 3.6e6 joules, which is 3.6e9 millijoules.
pub(crate) const MILLIJOULES_IN_A_KWH: u64 = 3_600_000_000;

impl PricePerKwh {
    /// Reads a price a person typed: `0.28`, `.28`, `28`, up to six decimal
    /// places. Anything else is refused rather than rounded into something
    /// plausible.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let (whole, rest) = text.split_once('.').unwrap_or((text, ""));
        if rest.len() > 6 || !rest.chars().all(|held| held.is_ascii_digit()) {
            return None;
        }
        if !whole.is_empty() && !whole.chars().all(|held| held.is_ascii_digit()) {
            return None;
        }
        let whole: u64 = if whole.is_empty() {
            0
        } else {
            whole.parse().ok()?
        };
        let mut fraction: u64 = if rest.is_empty() {
            0
        } else {
            rest.parse().ok()?
        };
        for _ in rest.len()..6 {
            fraction = fraction.checked_mul(10)?;
        }
        Some(Self {
            millionths: whole.checked_mul(1_000_000)?.checked_add(fraction)?,
        })
    }
}

impl fmt::Display for PricePerKwh {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{:06} a kilowatt-hour",
            self.millionths.wrapping_div(1_000_000),
            self.millionths.wrapping_rem(1_000_000)
        )
    }
}

/// What some energy cost, in millionths of a currency unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cost {
    pub millionths: u64,
}

impl fmt::Display for Cost {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{:04}",
            self.millionths.wrapping_div(1_000_000),
            self.millionths.wrapping_rem(1_000_000).wrapping_div(100)
        )
    }
}

#[cfg(test)]
mod tests;
