//! What was watching while a measurement was taken (B-163, B-164, B30, B31,
//! §3.4, §6.25, §6.2).
//!
//! **B3's condition, given a type.** *Instrumentation during measurement is
//! itself a condition*: a run watched by a profiler is a different run, and a
//! timing taken under one is a timing of the profiler as much as of the model.
//! The condition floor has carried an `instrumentation` entry as free text
//! since it was written, which is enough to *record* the profile and not
//! enough to *refuse* on it — and B-164 asks for a refusal.
//!
//! **Three levels, and the middle one is the interesting one.** Nothing beyond
//! the clock is what a timing run wants. Deep instrumentation is what a
//! debugging run wants and what a timing run must never be built from. Light
//! is the case that needs a number rather than a name: it perturbs the
//! measurement by *some* amount, and the honest thing is to characterize that
//! amount and carry it, rather than to declare it negligible — which is a
//! claim MCF would be making about its own instrument (§6.16).
//!
//! **Why a residual and not a threshold.** B31 asks that the overhead be
//! characterized, not that it be small. A threshold here would be MCF choosing
//! how much perturbation is acceptable for somebody else's measurement, which
//! is the same figure DEC-007 exists to derive elsewhere. So `Light` carries
//! what was measured and every result carries it onward as a condition.

use core::fmt;

use crate::measurement::PartsPerMillion;

/// What was watching.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Profile {
    /// Nothing beyond the clock the measurement is made with.
    ///
    /// The only profile a timing-class result may be built from.
    NothingBeyondTheClock,
    /// Something was watching, and this is what it cost.
    ///
    /// The residual is a measurement rather than an assurance: *characterized*
    /// is B31's word, and a profile that claimed to be free would be an
    /// instrument grading itself (§6.16).
    Light {
        /// What was watching, named.
        watcher: String,
        /// How much it moved the measurement, in parts per million, as
        /// measured against the same run unwatched.
        residual: PartsPerMillion,
    },
    /// Deep instrumentation: tracing, sampling profilers, per-operation
    /// accounting.
    ///
    /// Legitimate and useful, and never the origin of a timing result.
    Deep {
        /// What was watching, named.
        watcher: String,
    },
}

impl Profile {
    /// Whether a timing-class result may be built from a run under this
    /// (B-164).
    ///
    /// `Light` is admitted **because** it carries its residual: a known
    /// perturbation is a condition, and A6 lets a measurement travel with its
    /// conditions. `Deep` is refused because there is no residual to carry —
    /// characterizing tracing overhead is not a number, it is a distribution
    /// that depends on what the model did.
    #[must_use]
    pub const fn suits_timing(&self) -> bool {
        match self {
            Self::NothingBeyondTheClock | Self::Light { .. } => true,
            Self::Deep { .. } => false,
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NothingBeyondTheClock => form.write_str("nothing beyond the clock"),
            Self::Light { watcher, residual } => write!(
                form,
                "{watcher}, which moved the measurement by {}.{}% as characterized against the \
                 same run unwatched (B31)",
                residual.0.wrapping_div(10_000),
                residual.0.wrapping_div(1_000).wrapping_rem(10)
            ),
            Self::Deep { watcher } => write!(
                form,
                "{watcher}, deep — which makes this a debugging run and not a timing one \
                 (B-164, §6.2)"
            ),
        }
    }
}

/// Why a timing result cannot be built from this run (B-164).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotATiming {
    /// What was watching.
    pub watcher: String,
}

impl fmt::Display for NotATiming {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "this run was watched by {}, so a timing taken under it is a timing of the \
             instrument as much as of the model. Deep instrumentation has no single residual \
             to carry as a condition — the overhead depends on what the model did — so the \
             result is a debugging observation and not a measurement (B31, §6.2, §6.25)",
            self.watcher
        )
    }
}

/// A timing-class result, which knows what was watching (B-163, B-164).
///
/// **Cannot be constructed without a profile**, and cannot be constructed at
/// all from a deep-instrumentation run. That is B-163's *a result without its
/// profile cannot be constructed* and B-164's *the type system refuses the
/// construction*, in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timed<T> {
    value: T,
    under: Profile,
}

impl<T> Timed<T> {
    /// A timing, if what was watching permits one.
    ///
    /// # Errors
    ///
    /// [`NotATiming`] where the run was deeply instrumented.
    pub fn new(value: T, under: Profile) -> Result<Self, NotATiming> {
        match &under {
            Profile::Deep { watcher } => Err(NotATiming {
                watcher: watcher.clone(),
            }),
            Profile::NothingBeyondTheClock | Profile::Light { .. } => Ok(Self { value, under }),
        }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// What was watching, which travels with it as a condition (§3.4).
    #[must_use]
    pub const fn under(&self) -> &Profile {
        &self.under
    }
}

#[cfg(test)]
mod tests;
