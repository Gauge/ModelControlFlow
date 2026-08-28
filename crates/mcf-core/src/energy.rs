//! What a run cost in energy, and how MCF came to that number
//! (B-188, B39, D11, A20, A7, §3.4).
//!
//! **Three provenances, never collapsed.** A figure read from a counter, a
//! figure a vendor modelled, and no figure at all are three different things,
//! and B39's violation is the third quietly becoming the second: *a platform
//! with no interface yields `unknown`, never a number derived from
//! utilization*. Multiplying a percentage by a nameplate wattage produces a
//! number with a unit and no measurement behind it, which is the most
//! convincing kind of wrong.
//!
//! **A20's wall applies here too.** An estimate cannot be mistaken for a
//! measurement, promoted into one, or compared with one — it can only be
//! replaced by one. So [`Energy`] has no accessor that yields joules
//! regardless of provenance: a caller that wants a number matches, and the
//! match is where they meet the two cases that are not readings.
//!
//! **The sampling rate is a condition, not a footnote** (B-188, §3.4). Energy
//! read at one hertz across a two-second run has seen two samples, and what it
//! missed is most of the run. A reading whose rate does not travel with it
//! cannot be compared with another taken at a different one, and nobody can
//! tell that from the number.

use core::fmt;

/// How often a counter was read, in reads per second.
///
/// A condition of any measured energy figure, carried in the type so that it
/// cannot be dropped by a surface that only wanted the joules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerSecond(pub u32);

impl fmt::Display for PerSecond {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(form, "sampled {} time(s) a second", self.0)
    }
}

/// What a run cost, and where the figure came from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Energy {
    /// Read from a counter on this machine.
    Measured {
        /// Millijoules, integer because this crate holds no float (A6).
        millijoules: u64,
        /// How often the counter was read (B-188, §3.4).
        rate: PerSecond,
        /// Which counter, so that a reader can tell a package-level figure
        /// from a device-level one.
        counter: String,
    },
    /// A figure somebody modelled rather than read.
    ///
    /// B39's case: a vendor's model is an estimate, and A20 forbids it
    /// standing beside a measurement or being promoted into one.
    Modelled {
        /// Millijoules, as modelled.
        millijoules: u64,
        /// Whose model, and what it rests on.
        by: String,
    },
    /// There is no interface to read, so there is no number.
    ///
    /// **Not zero and not derived** (A7, B39). A platform MCF cannot read
    /// yields this, and the temptation it exists to refuse is deriving a
    /// figure from processor utilization — which produces a plausible number
    /// with nothing behind it.
    Unknown {
        /// Why nothing could be read.
        why: String,
    },
}

impl Energy {
    /// The reading, where there is one.
    ///
    /// The **only** way joules leave this type, and fallible on purpose: a
    /// caller reaching for a number meets the modelled and unknown cases at
    /// the point where they were about to flatten them. There is deliberately
    /// no `unwrap_or`, no `Default` and no `millijoules_or_zero`.
    #[must_use]
    pub const fn measured_millijoules(&self) -> Option<u64> {
        match self {
            Self::Measured { millijoules, .. } => Some(*millijoules),
            Self::Modelled { .. } | Self::Unknown { .. } => None,
        }
    }

    /// Whether this is a reading at all.
    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(self, Self::Measured { .. })
    }

    /// Whether two figures may be put side by side (A8, §3.4).
    ///
    /// Two measurements compare only where they were sampled at the same rate
    /// and from the same counter; a modelled figure compares with nothing,
    /// because A20 forbids an estimate standing beside a measurement and two
    /// models are two authors' opinions rather than two readings.
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

#[cfg(test)]
mod tests;
