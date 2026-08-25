//! A value MCF read, or the fact that it did not.
//!
//! A7 as a type. What is not known is never filled with a plausible value —
//! this governs provenance, licensing, hardware attributes, model metadata and
//! capability verdicts alike — and the way to make that hold under pressure is
//! for the absence to be a *variant* rather than a sentinel, so a caller has to
//! handle it (§3.16).
//!
//! There is no third state here. A21's *declared / verified / unknown* is a
//! different question — what an artifact claims versus what MCF observed — and
//! it gets its own type at B-050. This one answers only whether MCF could read
//! something.

use core::fmt;

/// A value MCF read, or the fact that it did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attested<T> {
    /// MCF read it.
    Known(T),
    /// MCF did not, and says so.
    Unknown,
}

impl<T> Attested<T> {
    /// The value, if it is known.
    pub const fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown => None,
        }
    }

    /// Whether MCF read it.
    pub const fn is_known(&self) -> bool {
        matches!(self, Self::Known(_))
    }

    /// Applies a function to the value, if there is one.
    ///
    /// Deliberately no `unwrap_or`, no `unwrap_or_default` and no
    /// `unwrap_or_else`: each of those is a plausible value substituted for an
    /// unknown, which is the thing A7 forbids. A caller that genuinely has a
    /// fallback writes the `match` and can be read doing it.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Attested<U> {
        match self {
            Self::Known(value) => Attested::Known(f(value)),
            Self::Unknown => Attested::Unknown,
        }
    }
}

impl<T> Default for Attested<T> {
    /// [`Attested::Unknown`].
    ///
    /// This is A7 rather than an exception to it. A7 forbids filling an unknown
    /// with a *plausible value*; the default here **is** the absence, so a
    /// struct built with `..Default::default()` starts out claiming nothing and
    /// every field it does not set stays unclaimed.
    ///
    /// It does not make a type built from these fields defaultable by
    /// accident: `Floor` deliberately has no `Default`, so that adding a
    /// condition to the §3.3 floor breaks every construction site, and a check
    /// asserts it stays that way.
    fn default() -> Self {
        Self::Unknown
    }
}

impl<T: fmt::Display> fmt::Display for Attested<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(value) => value.fmt(f),
            // C8: a surface that must show something it does not know shows
            // `unknown`.
            Self::Unknown => f.write_str("unknown"),
        }
    }
}
