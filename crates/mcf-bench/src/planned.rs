use core::fmt;

use mcf_core::measurement::Estimate;
use mcf_core::time::{Duration, Monotonic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    pub trials: usize,
    pub arms: usize,
    pub tokens: u32,
}

impl Work {
    #[must_use]
    pub fn generations(&self) -> u64 {
        let count = |held: usize| u64::try_from(held).unwrap_or(u64::MAX);
        count(self.trials).saturating_mul(count(self.arms))
    }

    #[must_use]
    pub fn tokens(&self) -> u64 {
        self.generations().saturating_mul(u64::from(self.tokens))
    }

    #[must_use]
    pub fn expected(&self, each: &Estimate<Duration<Monotonic>>) -> Estimate<Duration<Monotonic>> {
        let times = self.generations();
        Estimate::band(
            Duration::from_nanos(each.low().as_nanos().saturating_mul(times)),
            Duration::from_nanos(each.high().as_nanos().saturating_mul(times)),
            each.basis().clone(),
        )
    }
}

impl fmt::Display for Work {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "at most {} paired trial(s) across {} arm(s) — {} generation(s) of {} token(s), \
             {} token(s) in all",
            self.trials,
            self.arms,
            self.generations(),
            self.tokens,
            self.tokens()
        )
    }
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    Whole { running: Work },
    Fewer { running: Work, excluded: Work },
    NotEnough { least: Work },
}

impl Proposal {
    #[must_use]
    pub fn within(
        ceiling: Work,
        each: &Estimate<Duration<Monotonic>>,
        budget: Duration<Monotonic>,
        least: usize,
    ) -> Self {
        let per_trial = each
            .high()
            .as_nanos()
            .saturating_mul(u64::try_from(ceiling.arms).unwrap_or(u64::MAX));
        let affordable = if per_trial == 0 {
            ceiling.trials
        } else {
            usize::try_from(budget.as_nanos().wrapping_div(per_trial)).unwrap_or(usize::MAX)
        };
        if affordable < least {
            return Self::NotEnough {
                least: Work {
                    trials: least,
                    ..ceiling
                },
            };
        }
        if affordable >= ceiling.trials {
            return Self::Whole { running: ceiling };
        }
        Self::Fewer {
            running: Work {
                trials: affordable,
                ..ceiling
            },
            excluded: Work {
                trials: ceiling.trials.saturating_sub(affordable),
                ..ceiling
            },
        }
    }

    #[must_use]
    pub const fn running(&self) -> Option<Work> {
        match self {
            Self::Whole { running } | Self::Fewer { running, .. } => Some(*running),
            Self::NotEnough { .. } => None,
        }
    }
}

impl fmt::Display for Proposal {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Whole { running } => {
                write!(form, "the whole declaration fits the budget: {running}")
            }
            Self::Fewer { running, excluded } => write!(
                form,
                "the budget buys {running} — EXCLUDED, and not silently: {excluded}, because at \
                 the slow edge of this machine's measured band that is what would not fit \
                 (§3.1, B-226)"
            ),
            Self::NotEnough { least } => write!(
                form,
                "the budget buys less than a comparison. The least that would be one is \
                 {least}, and a run below it would answer a smaller question without saying so"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Tokens(u64),
    Elapsed(Duration<Monotonic>),
}

impl Bound {
    #[must_use]
    pub const fn tokens(&self) -> Option<u64> {
        match self {
            Self::Tokens(held) => Some(*held),
            Self::Elapsed(_) => None,
        }
    }

    #[must_use]
    pub const fn suits_behaviour(&self) -> bool {
        matches!(self, Self::Tokens(_))
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tokens(held) => write!(form, "{held} token(s), which count the same anywhere"),
            Self::Elapsed(held) => write!(
                form,
                "{} ns of wall clock, which is a bound only a timing laboratory may use",
                held.as_nanos()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    work: Work,
    bound: Bound,
    calibrated: mcf_core::configuration::Calibrated,
    behaviour: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotPlannable {
    BehaviourCannotWatchTheClock,
}

impl fmt::Display for NotPlannable {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BehaviourCannotWatchTheClock => form.write_str(
                "a behaviour laboratory's deadline is a token budget, never a wall clock: \
                 minutes give a model on a busy machine fewer attempts than the same model on \
                 a quiet one, and the result stops being about the model without saying so \
                 (B-230, D8, §3.8)",
            ),
        }
    }
}

impl Planned {
    pub fn new(
        work: Work,
        bound: Bound,
        calibrated: mcf_core::configuration::Calibrated,
        behaviour: bool,
    ) -> Result<Self, NotPlannable> {
        if behaviour && !bound.suits_behaviour() {
            return Err(NotPlannable::BehaviourCannotWatchTheClock);
        }
        Ok(Self {
            work,
            bound,
            calibrated,
            behaviour,
        })
    }

    #[must_use]
    pub const fn work(&self) -> &Work {
        &self.work
    }

    #[must_use]
    pub const fn bound(&self) -> &Bound {
        &self.bound
    }

    #[must_use]
    pub const fn calibrated(&self) -> &mcf_core::configuration::Calibrated {
        &self.calibrated
    }

    #[must_use]
    pub const fn is_behaviour(&self) -> bool {
        self.behaviour
    }
}
