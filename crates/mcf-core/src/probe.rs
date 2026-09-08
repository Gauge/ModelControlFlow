use crate::build_identity::BuildIdentity;
use crate::measurement::{Conditions, Floor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub name: &'static str,
    pub asks: &'static str,
    pub decides: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T> {
    Observed(T),
    Inconclusive { because: String },
}

impl<T> Outcome<T> {
    pub const fn observed(&self) -> Option<&T> {
        match self {
            Self::Observed(value) => Some(value),
            Self::Inconclusive { .. } => None,
        }
    }

    pub const fn is_inconclusive(&self) -> bool {
        matches!(self, Self::Inconclusive { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probed<T> {
    pub method: Method,
    pub outcome: Outcome<T>,
    pub trials: usize,
    pub tokens: usize,
    pub conditions: Conditions,
}

impl<T> Probed<T> {
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
