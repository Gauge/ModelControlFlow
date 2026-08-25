//! Where a number came from, in the type.
//!
//! B43 and B34: *the corpus narrows the search; local measurement decides.*
//! Contributed data may order candidates, prune a search space, warn that a
//! configuration has no working reports on hardware like this, and supply a
//! first duration estimate — and it *may never be the source of a number MCF
//! reports about this machine*. B43's check is `compiler`: a corpus-sourced
//! value and a locally-measured value are distinct types, and only the second
//! can back a recommendation (B-221).
//!
//! **Why a wrapper rather than a field.** A field is checked; a type is not
//! checkable-past. `Recommendation::from(x)` that takes only a
//! [`LocallyMeasured`] cannot be handed a [`FromCorpus`] however tired the
//! author is, and no configuration relaxes it. B43's violation is *a sorted
//! candidate list whose ordering nobody explains* — a foreign conclusion
//! wearing a local interface — and the way that happens is a foreign number
//! reaching a local code path through a field somebody forgot to check.
//!
//! **What a corpus value is allowed to do**, and it is not nothing: order,
//! prune, annotate, warn, and supply a first estimate. Every one of those is a
//! statement *about the corpus*, and [`FromCorpus`] carries the sample count
//! that B44 requires so a claim resting on two reports reads differently from
//! one resting on four hundred.
//!
//! There is no conversion in either direction. A corpus value does not become a
//! local one by being confirmed; it is *replaced* by the local measurement,
//! exactly as A20 replaces an estimate.

use core::fmt;

/// A value MCF measured on this machine.
///
/// The only kind that may back a recommendation (B34).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocallyMeasured<T> {
    value: T,
}

impl<T> LocallyMeasured<T> {
    /// Marks a value as measured here.
    ///
    /// Deliberately not `From`: taking this step is a claim about where a
    /// number came from, and a claim should be written rather than inferred.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Transforms the value, keeping the claim.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> LocallyMeasured<U> {
        LocallyMeasured {
            value: f(self.value),
        }
    }
}

impl<T: fmt::Display> fmt::Display for LocallyMeasured<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (measured here)", self.value)
    }
}

/// A value that came from somebody else's machine.
///
/// It may advise and may not decide (B43). It carries its sample count because
/// B44 requires every corpus statement to render one — a claim resting on
/// nothing has to say so.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FromCorpus<T> {
    value: T,
    reports: usize,
}

impl<T> FromCorpus<T> {
    /// Marks a value as coming from the corpus, with how many reports it rests
    /// on.
    #[must_use]
    pub const fn new(value: T, reports: usize) -> Self {
        Self { value, reports }
    }

    /// The value.
    ///
    /// Named `advises` rather than `value` because that is the whole of what it
    /// may do. B43: *arithmetic may refuse; the corpus may only advise.*
    #[must_use]
    pub const fn advises(&self) -> &T {
        &self.value
    }

    /// How many contributed reports it rests on (B44).
    #[must_use]
    pub const fn reports(&self) -> usize {
        self.reports
    }

    /// Transforms the value, keeping the origin and the count.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> FromCorpus<U> {
        FromCorpus {
            value: f(self.value),
            reports: self.reports,
        }
    }
}

impl<T: fmt::Display> fmt::Display for FromCorpus<T> {
    /// The value, that it is not from here, and how much it rests on.
    ///
    /// B43 requires every corpus-derived statement be *labelled, carry its
    /// sample count, and be visibly distinguishable from a local measurement*.
    /// All three are in the rendering, because a label that lives only in the
    /// type is a label nobody sees.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reports {
            0 => write!(f, "{} (from the corpus, resting on nothing)", self.value),
            1 => write!(f, "{} (from the corpus, 1 report)", self.value),
            n => write!(f, "{} (from the corpus, {n} reports)", self.value),
        }
    }
}

#[cfg(test)]
mod tests;
