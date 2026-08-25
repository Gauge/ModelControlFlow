//! What else the machine is doing, and whether a reading can be attributed.
//!
//! B35: *a timing taken under contention measures the contention.* B24 makes
//! **unattributable** a verdict rather than a gap — MCF knows the difference
//! between "this model is slow" and "this machine was busy", and says so when
//! it cannot tell. D27 applies both to MCF's own budgets: a figure is asserted
//! only on a run the machine was quiet enough to attribute, and a busy machine
//! makes a run unattributable rather than failing it.
//!
//! **The threshold is declared, not measured.** There is no reading that tells
//! MCF how quiet is quiet enough; that is a judgement, and the honest thing is
//! to state it, record it with every result, and let a reader disagree. It is
//! stated in [`QUIET_PER_CORE`] with its reasoning.

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

/// How much of a core's capacity may be busy with something else before a
/// timing stops being about MCF.
///
/// Half. The number is a judgement and is stated rather than hidden: below it,
/// a measurement competes with at most one other runnable process per two
/// cores and the effect on a millisecond-scale figure is small; above it, F1
/// showed a cold-start tail move by a factor of twenty-five. It is recorded as
/// a condition with every budget result, so a reader who thinks it wrong can
/// see the reading it was applied to.
pub const QUIET_PER_CORE: u64 = 500;

/// Whether a reading can be attributed to what it was measuring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attributability {
    /// The machine was quiet enough.
    Attributable {
        /// What the load was.
        load: LoadAverage,
    },
    /// It was not, and this is what was competing.
    ///
    /// B24: a verdict, not a gap. A run marked this way is neither a pass nor a
    /// failure — D27 makes an unattributable budget run count as neither, and
    /// B38's staleness discipline is what stops that becoming a hiding place.
    Unattributable {
        /// What the load was.
        load: LoadAverage,
        /// What it would have had to be below.
        quiet_below: LoadAverage,
    },
    /// MCF could not read the load at all, so it cannot say either way (A7).
    Unknown,
}

impl fmt::Display for Attributability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attributable { load } => write!(f, "attributable (load {load})"),
            Self::Unattributable { load, quiet_below } => {
                write!(f, "UNATTRIBUTABLE (load {load}, quiet below {quiet_below})")
            }
            Self::Unknown => f.write_str("attributability unknown — the load could not be read"),
        }
    }
}

impl Attributability {
    /// Whether a figure measured under this may be asserted against a ceiling.
    ///
    /// Only [`Attributability::Attributable`]. Unknown is not permission: A7
    /// forbids reading an absent value as a favourable one.
    #[must_use]
    pub const fn permits_assertion(&self) -> bool {
        matches!(self, Self::Attributable { .. })
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

/// Whether a measurement taken now can be attributed.
///
/// `cores` is what the load is judged against: the same load means something
/// different on four cores and on thirty-two. Where the core count is unknown
/// the answer is [`Attributability::Unknown`] rather than a guess.
#[must_use]
pub fn attributability(cores: Attested<u32>) -> Attributability {
    let (Attested::Known(load), Attested::Known(cores)) = (load_average(), cores) else {
        return Attributability::Unknown;
    };
    // The measuring process is itself runnable, so one is subtracted before the
    // machine is judged: a load of exactly 1.00 on an otherwise idle machine is
    // this process and nothing else.
    let competing = load.0.saturating_sub(1000);
    let quiet_below = LoadAverage(u64::from(cores).saturating_mul(QUIET_PER_CORE));
    if competing <= quiet_below.0 {
        Attributability::Attributable { load }
    } else {
        Attributability::Unattributable { load, quiet_below }
    }
}
