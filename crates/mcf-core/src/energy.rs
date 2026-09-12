use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerSecond(pub u32);

impl fmt::Display for PerSecond {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(form, "sampled {} time(s) a second", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Energy {
    Measured {
        millijoules: u64,
        rate: PerSecond,
        counter: String,
    },
    Modelled {
        millijoules: u64,
        by: String,
    },
    Unknown {
        why: String,
    },
}

impl Energy {
    #[must_use]
    pub const fn measured_millijoules(&self) -> Option<u64> {
        match self {
            Self::Measured { millijoules, .. } => Some(*millijoules),
            Self::Modelled { .. } | Self::Unknown { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(self, Self::Measured { .. })
    }

    #[must_use]
    pub fn comparable_with(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Measured {
                    rate: one,
                    counter: from,
                    ..
                },
                Self::Measured {
                    rate: other,
                    counter: also,
                    ..
                },
            ) => one == other && from == also,
            _ => false,
        }
    }
}

impl fmt::Display for Energy {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measured {
                millijoules,
                rate,
                counter,
            } => write!(
                form,
                "{}.{} J read from {counter}, {rate}",
                millijoules.wrapping_div(1_000),
                millijoules.wrapping_rem(1_000).wrapping_div(100)
            ),
            Self::Modelled { millijoules, by } => write!(
                form,
                "{}.{} J MODELLED by {by} — an estimate, which A20 forbids standing beside a \
                 measurement or being promoted into one",
                millijoules.wrapping_div(1_000),
                millijoules.wrapping_rem(1_000).wrapping_div(100)
            ),
            Self::Unknown { why } => write!(
                form,
                "unknown: {why}. Not zero, and not derived from utilization — a figure with a \
                 unit and no measurement behind it is the most convincing kind of wrong \
                 (B39, A7)"
            ),
        }
    }
}

impl Energy {
    #[must_use]
    #[expect(
        clippy::integer_division,
        reason = "millionths of a currency unit, floored; the remainder is less than                   a millionth and there is no smaller unit to carry it into"
    )]
    pub const fn cost_at(&self, price: crate::price::PricePerKwh) -> Option<crate::price::Cost> {
        match self {
            Self::Measured { millijoules, .. } => Some(crate::price::Cost {
                millionths: millijoules.saturating_mul(price.millionths)
                    / crate::price::MILLIJOULES_IN_A_KWH,
            }),
            Self::Modelled { .. } | Self::Unknown { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
