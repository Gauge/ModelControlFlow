//! What a measurement is a measurement *of*.
//!
//! A6's violation looks like `throughput: 41.2` — a number with no unit, no
//! conditions and no spread. [`Measurement`] supplies the last two; this
//! module supplies the first, by making the *type* carry the unit rather than
//! a field beside it. A `Measurement<Bytes>` and a `Measurement<PartsPerMillion>`
//! are different types and cannot be mixed, compared or averaged together.
//!
//! **Quantities are totally ordered, which rules out floating point.** Every
//! summary MCF reports from a measurement is an order statistic — minimum,
//! median, percentiles, maximum — and order statistics need ordering, not
//! arithmetic. That lets [`Quantity`] require [`Ord`] rather than
//! `PartialOrd`, and the consequence is worth stating plainly: **no NaN can
//! enter a measurement**, because there is no floating-point sample type to
//! carry one. A NaN is a number that has lost the information about what went
//! wrong, which is precisely what A1 forbids.
//!
//! Where a quantity is naturally fractional it is represented in a small
//! integral unit — parts per million for a fraction, bytes for a size,
//! nanoseconds for a duration when B-184 brings one — rather than as a float
//! that would reintroduce the problem.
//!
//! [`Measurement`]: super::Measurement

use core::fmt;

/// Something that can be measured.
///
/// Implemented by newtypes rather than by primitives on purpose: an
/// implementation for `u64` would make every count, size and duration the same
/// type again, and A8's isolation of the variable starts with knowing what two
/// numbers are numbers *of*.
pub trait Quantity: Copy + Ord + fmt::Debug + fmt::Display {
    /// The unit, as it is written when the quantity is rendered.
    const UNIT: &'static str;
}

/// A size in bytes.
///
/// D24 budgets four quantities in bytes — installed footprint, resident memory
/// idle, resident memory above the engine, and memory growth over thirty
/// simulated days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub u64);

impl Quantity for Bytes {
    const UNIT: &'static str = "B";
}

impl Bytes {
    /// The size in the largest binary unit that leaves a whole part, to one
    /// decimal place.
    ///
    /// Computed with integer arithmetic, because this crate has no
    /// floating-point anywhere by design (see [`Quantity`]) and a rendering is
    /// not a reason to introduce one.
    ///
    /// C3: a minimalist surface shows less decoration, not less information.
    /// The exact count is what a record holds and what a comparison uses; this
    /// is what a person reads, and [`fmt::Display`] shows both rather than
    /// making anyone choose.
    #[must_use]
    // Integer division is the conversion, and both operands are bounded: the
    // unit is a power of two no larger than the value, so neither the quotient
    // nor the remainder can overflow.
    #[allow(clippy::integer_division)]
    pub fn human(self) -> String {
        const UNITS: [(&str, u64); 4] = [
            ("GiB", 1024 * 1024 * 1024),
            ("MiB", 1024 * 1024),
            ("KiB", 1024),
            ("B", 1),
        ];
        for (name, size) in UNITS {
            if self.0 >= size && size > 1 {
                let whole = self.0 / size;
                let tenths = (self.0 % size).saturating_mul(10) / size;
                return format!("{whole}.{tenths} {name}");
            }
        }
        format!("{} B", self.0)
    }
}

impl fmt::Display for Bytes {
    /// The exact count, and the same size in a unit a person reads.
    ///
    /// Both, always. The exact number is the measurement; the rounded one is a
    /// courtesy, and a surface that showed only the courtesy would be dropping
    /// information A6 requires travel with a value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1024 {
            write!(f, "{} {} ({})", self.0, Self::UNIT, self.human())
        } else {
            write!(f, "{} {}", self.0, Self::UNIT)
        }
    }
}

/// A fraction, in parts per million.
///
/// D24 budgets idle CPU at 0.05 % of one core, which is 500 ppm, and the
/// interface's daemon CPU at 0.02 %, which is 200. Parts per million rather
/// than a percentage as a float: the budgets are three decimal places apart
/// from zero, and a float sample type would put a NaN one division by zero
/// away from a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartsPerMillion(pub u64);

impl Quantity for PartsPerMillion {
    const UNIT: &'static str = "ppm";
}

impl fmt::Display for PartsPerMillion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, Self::UNIT)
    }
}

/// A count of things that happened.
///
/// D24 budgets two counts as **prohibitions rather than thresholds**: timer
/// wakeups while idle, and external requests from the interface, both exactly
/// zero. A count is still a quantity: "zero wakeups over sixty seconds,
/// observed twenty times" is a measurement, and "zero" on its own is an
/// assertion nobody can check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count(pub u64);

impl Quantity for Count {
    const UNIT: &'static str = "";
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
