//! What a probe is: an experiment, its conditions, and an outcome that
//! configures nothing (B-051, D42, §3.18, §3.4).
//!
//! **The types here are the whole of D42's discipline.** A probe reports a
//! [`Probed`], which carries the [`Method`] that produced it, what it observed
//! across how many trials, what it cost, and the conditions it holds under. It
//! has no `unwrap_or`, no `is_supported()`, and no way to reach a value without
//! passing through [`Outcome`] — because the one thing D42 forbids is a probe
//! result becoming a default by being read carelessly.
//!
//! **Inconclusive is a variant, not an absence.** §3.18's third state exists
//! because *the model did not do the thing* and *MCF could not tell* are
//! different facts. The second is [`Outcome::Inconclusive`], it carries why,
//! and nothing downstream may treat it as either of the others.

use crate::build_identity::BuildIdentity;
use crate::measurement::{Conditions, Floor};

/// What a probe asks and what its answer decides.
///
/// Written down beside the result rather than in a manual: six months later
/// the question is *what did this actually test*, and the answer has to travel
/// with the number (§3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    /// The probe's name, stable so that a result can cite it (B-059).
    pub name: &'static str,
    /// What it does to the model.
    pub asks: &'static str,
    /// What a positive answer would license MCF to do.
    pub decides: &'static str,
}

/// What a probe saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    /// The probe decided, and this is what it observed.
    Observed(T),
    /// The probe ran and could not decide.
    ///
    /// Not a negative: a model that could not be reached, an engine that died,
    /// a budget too small to tell. It licenses nothing (D42).
    Inconclusive {
        /// Why it could not decide, in a sentence a person can act on.
        because: String,
    },
}

impl<T> Outcome<T> {
    /// What was observed, where anything was.
    ///
    /// Deliberately not `unwrap_or`: a caller that wants a value for an
    /// inconclusive probe has to write the `None` branch and decide what to do
    /// about it, which is the sentence D42 exists to make somebody write.
    pub const fn observed(&self) -> Option<&T> {
        match self {
            Self::Observed(value) => Some(value),
            Self::Inconclusive { .. } => None,
        }
    }

    /// Whether the probe could not decide.
    pub const fn is_inconclusive(&self) -> bool {
        matches!(self, Self::Inconclusive { .. })
    }
}

/// One probe's result, with everything needed to read it later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed<T> {
    /// What was asked.
    pub method: Method,
    /// What came back.
    pub outcome: Outcome<T>,
    /// How many trials it took.
    pub trials: usize,
    /// What it cost, in tokens rather than seconds (B49): a budget in seconds
    /// would make the cost a property of the machine.
    pub tokens: usize,
    /// What it holds under. A probe result caches exactly as far as these hold
    /// and no further (D42) — a different engine or build is a different
    /// observation, not a stale one.
    pub conditions: Conditions,
}

impl<T> Probed<T> {
    /// A probe that could not decide, with the reason.
    #[must_use]
    pub fn inconclusive(
        method: Method,
        because: impl Into<String>,
        trials: usize,
        tokens: usize,
        conditions: Conditions,
    ) -> Self {
        Self {
            method,
            outcome: Outcome::Inconclusive {
                because: because.into(),
            },
            trials,
            tokens,
            conditions,
        }
    }
}

/// Conditions with nothing but the instrument known.
///
/// A probe's caller fills in what it knows — which engine, which build, which
/// model. This is the floor beneath that, and it exists so that a probe that
/// could not even reach a model still reports its conditions rather than
/// omitting them (§3.4).
#[must_use]
pub fn nothing_known() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

#[cfg(test)]
mod tests {
    use super::{Method, Outcome, Probed, nothing_known};
    const METHOD: Method = Method {
        name: "example",
        asks: "nothing",
        decides: "nothing",
    };

    #[test]
    fn an_inconclusive_outcome_yields_no_value() {
        let probed: Probed<bool> =
            Probed::inconclusive(METHOD, "the engine died", 3, 12, nothing_known());
        assert!(probed.outcome.is_inconclusive());
        assert_eq!(probed.outcome.observed(), None);
    }

    #[test]
    fn an_observed_outcome_yields_what_was_seen() {
        let outcome = Outcome::Observed(4_usize);
        assert_eq!(outcome.observed(), Some(&4));
        assert!(!outcome.is_inconclusive());
    }
}
