//! Reduced capability, and the mark that travels with everything it touched.
//!
//! §3.2 prefers reduced capability to failure — GPU unavailable, run on CPU;
//! hub unreachable, serve from cache — and then says the important half at
//! full volume: **the mark is not optional. A degraded result that is not
//! labelled as degraded is a corrupted result.** A5 makes that absolute, and
//! its check is `compiler`: a degraded result is a distinct type that cannot
//! be rendered or exported as an undegraded one (B-008).
//!
//! So [`Degraded<T>`] is not a flag on a value. It is a different type, and
//! the difference is load-bearing:
//!
//! * A function that takes a `Measurement<Duration<Monotonic>>` does not take
//!   a `Degraded<Measurement<Duration<Monotonic>>>`. A CPU-derived timing
//!   cannot be passed where an accelerator-derived one is expected, so it
//!   cannot be displayed or exported as one.
//! * There is no `Deref`, no `From<Degraded<T>> for T`, and no `unwrap`.
//!   Getting at the value is [`Degraded::into_parts`], which hands back the
//!   mark alongside it — so a caller that drops the mark has visibly written
//!   the line that drops it.
//! * Degradation is **contagious by construction**: [`Degraded::zip`] combines
//!   two values and their marks, and there is no combining operation that
//!   returns an undegraded value from a degraded input.
//!
//! A degradation is a [`Failure`] whose disposition is
//! [`Disposition::Degraded`], rather than a parallel vocabulary. That is not a
//! convenience: A2 requires every failure be classified, attributed and
//! contextualized, and a degradation is exactly a failure MCF continued past.
//! Reusing the type means the taxonomy, the attribution and the context come
//! along, and the M0 mockup's `⚠ DEGRADED` block renders from the same fields
//! as any other failure.

use core::fmt;

use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};

/// What was lost, and why.
///
/// Holds one or more classified causes. More than one because degradations
/// compose: a run with no accelerator *and* a stale driver reading has lost
/// two things, and A1 says never lose information — including about what else
/// went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degradation {
    causes: Vec<Failure>,
}

impl Degradation {
    /// A degradation with one cause.
    ///
    /// The disposition is [`Disposition::Degraded`] and is not a parameter:
    /// that is what makes this a degradation rather than a refusal or an
    /// abort, and letting a caller choose would let a degradation be recorded
    /// as something MCF recovered from.
    #[must_use]
    pub fn because(
        category: Category,
        attribution: Attribution,
        subsystem: Subsystem,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            causes: vec![Failure::new(
                category,
                attribution,
                Disposition::Degraded,
                subsystem,
                detail,
            )],
        }
    }

    /// A degradation from an already-classified failure, if that failure is
    /// one MCF continued past.
    ///
    /// Returns `None` when the disposition is anything else. A refusal or an
    /// abort produced no value, so there is nothing for a mark to travel with,
    /// and accepting one here would let a failure be relabelled as a partial
    /// success.
    #[must_use]
    pub fn from_failure(failure: Failure) -> Option<Self> {
        if failure.disposition() == Disposition::Degraded {
            Some(Self {
                causes: vec![failure],
            })
        } else {
            None
        }
    }

    /// Everything that was lost, in the order it was lost.
    #[must_use]
    pub fn causes(&self) -> &[Failure] {
        &self.causes
    }

    /// Both degradations, kept whole.
    #[must_use]
    pub fn and(mut self, other: Self) -> Self {
        self.causes.extend(other.causes);
        self
    }
}

impl fmt::Display for Degradation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DEGRADED")?;
        for cause in &self.causes {
            write!(f, " — {cause}")?;
        }
        Ok(())
    }
}

/// A value produced under reduced capability, and the mark saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degraded<T> {
    value: T,
    degradation: Degradation,
}

impl<T> Degraded<T> {
    /// Marks a value.
    #[must_use]
    pub const fn new(value: T, degradation: Degradation) -> Self {
        Self { value, degradation }
    }

    /// The value, without taking it.
    ///
    /// Reading is allowed — arithmetic has to happen somewhere — but the type
    /// still is what it is: whatever this returns is a `&T` obtained from a
    /// `Degraded<T>`, and putting it on a surface means writing the code that
    /// separates it from its mark.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// What was lost.
    #[must_use]
    pub const fn degradation(&self) -> &Degradation {
        &self.degradation
    }

    /// The value and the mark, together.
    ///
    /// Deliberately not `into_inner`. A function that returns only the value
    /// is a function whose callers can forget the mark without saying so; this
    /// one hands back both, so dropping the mark is a visible act.
    #[must_use]
    pub fn into_parts(self) -> (T, Degradation) {
        (self.value, self.degradation)
    }

    /// Transforms the value, keeping the mark.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Degraded<U> {
        Degraded {
            value: f(self.value),
            degradation: self.degradation,
        }
    }

    /// Combines two degraded values, keeping both marks.
    ///
    /// There is no counterpart that takes a degraded value and an undegraded
    /// one and returns an undegraded result. That absence is the contagion:
    /// anything computed from a degraded input is degraded.
    #[must_use]
    pub fn zip<U>(self, other: Degraded<U>) -> Degraded<(T, U)> {
        Degraded {
            value: (self.value, other.value),
            degradation: self.degradation.and(other.degradation),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Degraded<T> {
    /// The value and the mark, always in that order and never one without the
    /// other. A5's violation is a CPU-derived timing rendered beside
    /// accelerator-derived ones with no distinction, and this type has no
    /// rendering that produces one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.value, self.degradation)
    }
}

/// A value that may or may not have been produced under reduced capability.
///
/// What a producer returns when it does not know in advance which it will be —
/// §3.2's "GPU unavailable → run on CPU and mark every resulting measurement".
/// A consumer has to name both arms, which is the point: there is no shape of
/// this type that lets the degraded case be handled by forgetting about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeDegraded<T> {
    /// Produced with everything MCF wanted available.
    Full(T),
    /// Produced under reduced capability.
    Reduced(Degraded<T>),
}

impl<T> MaybeDegraded<T> {
    /// Whether capability was reduced.
    #[must_use]
    pub const fn is_degraded(&self) -> bool {
        matches!(self, Self::Reduced(_))
    }

    /// What was lost, if anything was.
    #[must_use]
    pub const fn degradation(&self) -> Option<&Degradation> {
        match self {
            Self::Full(_) => None,
            Self::Reduced(degraded) => Some(degraded.degradation()),
        }
    }

    /// The value, whichever arm holds it.
    #[must_use]
    pub const fn value(&self) -> &T {
        match self {
            Self::Full(value) => value,
            Self::Reduced(degraded) => degraded.value(),
        }
    }

    /// Marks the value as degraded, whether or not it already was.
    ///
    /// This is how a degradation propagates through a pipeline: a stage that
    /// loses a capability marks whatever it produces, and a value that was
    /// already marked keeps both marks (A1).
    #[must_use]
    pub fn degrade(self, degradation: Degradation) -> Degraded<T> {
        match self {
            Self::Full(value) => Degraded::new(value, degradation),
            Self::Reduced(degraded) => {
                let (value, existing) = degraded.into_parts();
                Degraded::new(value, existing.and(degradation))
            }
        }
    }
}

impl<T: fmt::Display> fmt::Display for MaybeDegraded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(value) => value.fmt(f),
            Self::Reduced(degraded) => degraded.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests;
