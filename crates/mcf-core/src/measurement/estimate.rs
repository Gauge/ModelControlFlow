//! A fast, clearly-labelled guess — and the wall between it and a measurement.
//!
//! A20 permits estimates and then draws the line absolutely: *an estimate can
//! never be mistaken for a measurement, never be promoted into one, and never
//! be compared with one. It can only be **replaced** by one.* Its check is
//! `compiler`, and the way to hold it is for [`Estimate`] and [`Measurement`]
//! to be different types with no conversion in either direction — no `From`, no
//! `Into`, no `as_measurement`, no `PartialEq` between them.
//!
//! "Replaced" needs no mechanism, and that is the point. A caller that has
//! taken a measurement uses the measurement; there is no function that turns
//! one of these into the other, so there is no line of code that could be read
//! as a promotion.
//!
//! **An estimate is a band, not a point.** B46 requires it: a duration
//! predicted from a rate is a range, and rendering it as a single number is
//! false precision — "the smallest possible version of a confident wrong
//! number". [`Estimate::point`] exists for the case where the band genuinely
//! has zero width, and it is still an estimate.
//!
//! Every estimate carries its [`Basis`], because A20's *clearly-labelled* is
//! not satisfied by a type name nobody sees. A corpus-derived basis carries its
//! sample count, which B44 requires: a claim resting on two reports reads
//! differently from one resting on four hundred.
//!
//! [`Measurement`]: super::Measurement

use core::fmt;

use super::Quantity;

/// What an estimate rests on.
///
/// Enumerated rather than free text, so that a surface can render the basis
/// and a reader can tell how much to believe it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Basis {
    /// Measurements taken on this machine — B46's "the machine supplies the
    /// rate from the characterization tier".
    ///
    /// The strongest basis available, and still an estimate: a prediction from
    /// a measured rate is not a measurement of the thing predicted.
    LocalHistory,
    /// The contributed corpus, and how many reports it rests on.
    ///
    /// B43 lets the corpus advise and never decide; B44 requires the sample
    /// count travel with the claim, so it is a field rather than a footnote.
    Corpus {
        /// How many contributed reports the estimate draws on.
        reports: usize,
    },
    /// A figure the vendor modelled rather than measured.
    ///
    /// B39's case: a modelled power figure is an estimate, and a platform with
    /// no interface yields `unknown` rather than a number derived from
    /// utilization.
    VendorModel,
    /// What the artifact's own metadata claims.
    ///
    /// A21 keeps *declared* and *verified* apart; this is the declared side
    /// wearing the only shape it is allowed to have when it reaches a number.
    Declared,
}

impl fmt::Display for Basis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalHistory => f.write_str("local history"),
            Self::Corpus { reports } => write!(f, "corpus, n={reports}"),
            Self::VendorModel => f.write_str("vendor model"),
            Self::Declared => f.write_str("declared"),
        }
    }
}

/// A banded guess, labelled with what it rests on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Estimate<Q: Quantity> {
    low: Q,
    high: Q,
    basis: Basis,
}

impl<Q: Quantity> Estimate<Q> {
    /// A band.
    ///
    /// The two bounds are sorted rather than validated: a caller that passes
    /// them the other way round meant a band, and refusing would turn a
    /// harmless argument order into a failure that has no taxonomy category.
    #[must_use]
    pub fn band(first: Q, second: Q, basis: Basis) -> Self {
        Self {
            low: first.min(second),
            high: first.max(second),
            basis,
        }
    }

    /// A band of zero width.
    ///
    /// Still an estimate. The width of a band says how uncertain the guess is;
    /// it says nothing about whether it is a guess.
    #[must_use]
    pub fn point(value: Q, basis: Basis) -> Self {
        Self {
            low: value,
            high: value,
            basis,
        }
    }

    /// The lower bound.
    #[must_use]
    pub const fn low(&self) -> Q {
        self.low
    }

    /// The upper bound.
    #[must_use]
    pub const fn high(&self) -> Q {
        self.high
    }

    /// What it rests on.
    #[must_use]
    pub const fn basis(&self) -> &Basis {
        &self.basis
    }

    /// Whether the band has zero width.
    #[must_use]
    pub fn is_point(&self) -> bool {
        self.low == self.high
    }
}

impl<Q: Quantity> fmt::Display for Estimate<Q> {
    /// The band, the word *estimate*, and the basis — never fewer.
    ///
    /// A20's *clearly-labelled* is a property of what a reader sees, so the
    /// label is in the rendering rather than only in the type. There is no
    /// rendering of this type that omits it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_point() {
            write!(f, "~{} (estimate: {})", self.low, self.basis)
        } else {
            write!(f, "{}–{} (estimate: {})", self.low, self.high, self.basis)
        }
    }
}
