//! Interior detail, and the thinning that has to travel with it.
//!
//! D16 and B56: *interior detail — per-token timings, per-turn outcomes — is
//! declared by the laboratory that needs it and off by default, and where it
//! must be thinned the thinning is recorded as a condition.* B-271's condition
//! is the second half: **a downsampled series carries its thinning factor and
//! cannot be read as full resolution.**
//!
//! The constraint the design has to meet is that a thinned series looks exactly
//! like a full one — a list of values, ordered — so nothing about the data
//! distinguishes them. The only thing that can is the type, which is why
//! [`Series`] has no constructor that omits [`Thinning`] and no accessor that
//! returns the values without it.
//!
//! **Off by default is a property of the caller, not of this type.** A series
//! exists because something asked for one; what this module supplies is that
//! asking for one means stating what resolution it is.

use core::fmt;

use crate::measurement::Quantity;

/// How much of a series was kept.
///
/// One means every point. Two means every other point, and so on. There is no
/// *unknown* variant: a series MCF built knows what it did to it, and one MCF
/// read from a record either says or is not a series (A7 governs what MCF could
/// not read, and this is not that).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Thinning(u32);

impl Thinning {
    /// Every point kept.
    pub const FULL: Self = Self(1);

    /// A thinning factor, if it is one.
    ///
    /// `None` for zero, which would mean a series built from no points at all.
    #[must_use]
    pub const fn every(nth: u32) -> Option<Self> {
        if nth == 0 { None } else { Some(Self(nth)) }
    }

    /// The factor.
    #[must_use]
    pub const fn factor(self) -> u32 {
        self.0
    }

    /// Whether every point was kept.
    #[must_use]
    pub const fn is_full_resolution(self) -> bool {
        self.0 == 1
    }
}

impl fmt::Display for Thinning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_full_resolution() {
            f.write_str("full resolution")
        } else {
            write!(f, "thinned, every {}th point", self.0)
        }
    }
}

/// Interior detail from one trial: a sequence of values, and what was done to
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series<Q: Quantity> {
    points: Vec<Q>,
    thinning: Thinning,
}

impl<Q: Quantity> Series<Q> {
    /// A series at a stated resolution.
    ///
    /// The thinning is an argument and has no default. A `Series::from(points)`
    /// would be a full-resolution claim made by omission, which is exactly the
    /// silent alteration of evidence D16 refuses.
    #[must_use]
    pub fn new(points: impl IntoIterator<Item = Q>, thinning: Thinning) -> Self {
        Self {
            points: points.into_iter().collect(),
            thinning,
        }
    }

    /// Keeps every `nth` point of this series, and records that it did.
    ///
    /// The factors compose: thinning a series that was already thinned by three
    /// by a further two gives a series thinned by six, and it says so. A
    /// re-thinned series that reported only the last factor would be claiming a
    /// resolution it does not have.
    #[must_use]
    pub fn thinned(&self, nth: u32) -> Option<Self> {
        let further = Thinning::every(nth)?;
        let factor = self.thinning.factor().checked_mul(further.factor())?;
        Some(Self {
            points: self
                .points
                .iter()
                .copied()
                .step_by(further.factor() as usize)
                .collect(),
            thinning: Thinning(factor),
        })
    }

    /// The points, with what was done to them.
    ///
    /// Returned together, always. There is no accessor that hands back the
    /// points alone, because a caller holding a bare slice cannot tell a thinned
    /// series from a full one — which is B-271's whole requirement.
    #[must_use]
    pub fn points(&self) -> (&[Q], Thinning) {
        (&self.points, self.thinning)
    }

    /// How many points were kept.
    #[must_use]
    pub fn kept(&self) -> usize {
        self.points.len()
    }

    /// What this series says about resolution.
    #[must_use]
    pub const fn thinning(&self) -> Thinning {
        self.thinning
    }

    /// Whether this is every point.
    #[must_use]
    pub const fn is_full_resolution(&self) -> bool {
        self.thinning.is_full_resolution()
    }
}

impl<Q: Quantity> fmt::Display for Series<Q> {
    /// The count and the resolution, never one without the other.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} points, {}", self.points.len(), self.thinning)
    }
}
