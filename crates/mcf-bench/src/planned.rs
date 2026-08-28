//! What a laboratory says it will do, in things that can be counted
//! (B-224, B-225, B46, D14, A20).
//!
//! **The rule, and the reason for it.** B-224: *a laboratory declares its work
//! in countable units — trials, sweep points, tokens, documents — never in
//! minutes.* A lab that declares *twenty minutes* has declared a property of
//! the machine it happened to be written on. Move it to a slower machine and
//! the declaration is wrong; move it to a faster one and it is wrong in the
//! other direction, and in neither case has anything about the work changed.
//! The work is two hundred paired trials of one hundred and twenty-eight
//! tokens. That is true everywhere.
//!
//! **And then the duration, derived rather than declared.** The operator still
//! wants to know what it will cost them, and refusing to say is not honesty,
//! it is unhelpfulness with a rule attached. So the duration is *computed* —
//! the declared count multiplied by a rate this machine measured — which makes
//! it an [`Estimate`], carrying A20's wall in the type: it cannot be mistaken
//! for a measurement, promoted into one, or compared with one.
//!
//! **Banded, because a rate has a spread** (B46). The band is the same one
//! [`crate::project`] builds: the fastest and slowest trials actually seen,
//! carried through. Multiplying a band by a count widens it proportionally,
//! which is right — twice the work has twice the uncertainty in absolute
//! terms — and it is the honest shape. A single number here would be, in
//! B46's phrase, the smallest possible version of a confident wrong number.
//!
//! **Absent where there is nothing to derive from.** No history, no estimate:
//! the declaration of work still stands, and the duration is reported as
//! unknown (A7). What MCF must never do is fill the hole with a figure it
//! chose, because that figure would be indistinguishable on the page from one
//! this machine measured.
//!
//! **Scored** (B-225). The error of these estimates is not tracked separately,
//! and does not need to be: an expectation here is exactly the per-trial band
//! multiplied by a count both sides agree on, so it is inside its band if and
//! only if the per-trial band contained the per-trial truth. That is what
//! [`crate::project::score`] scores, by leaving each measured point out and
//! projecting it from the others. `mcf doctor` reports it. A lab whose
//! estimates are persistently wrong therefore surfaces without a second
//! bookkeeping path that could disagree with the first.
//!
//! [`Estimate`]: mcf_core::measurement::Estimate

use core::fmt;

use mcf_core::measurement::Estimate;
use mcf_core::time::{Duration, Monotonic};

/// What a run will do, counted.
///
/// Every field is a count. There is deliberately no field for a duration, a
/// deadline or a timeout: a type with one would let a laboratory declare in
/// minutes, and B-224 is a statement about what may be declared and not merely
/// about what is usually declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// The most paired trials the run will take.
    ///
    /// A ceiling and not a target: a run that decides earlier stops earlier,
    /// so this is the largest amount of work, which is the number an operator
    /// deciding whether to start needs.
    pub trials: usize,
    /// How many arms each of those trials touches.
    pub arms: usize,
    /// The generation length each trial is pinned to (D19).
    pub tokens: u32,
}

impl Work {
    /// How many generations the run will ask for, at most.
    #[must_use]
    pub fn generations(&self) -> u64 {
        let count = |held: usize| u64::try_from(held).unwrap_or(u64::MAX);
        count(self.trials).saturating_mul(count(self.arms))
    }

    /// How many tokens the run will ask to be produced, at most.
    #[must_use]
    pub fn tokens(&self) -> u64 {
        self.generations().saturating_mul(u64::from(self.tokens))
    }

    /// What that much work would take here, from a measured per-generation
    /// band.
    ///
    /// The multiplication is the whole derivation, and it is stated rather than
    /// hidden: the rate came from measurement, the count came from the
    /// declaration, and the product is neither.
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

/// What a time budget can buy, and what it cannot (B-226, B47, §3.1).
///
/// **The failure this exists to prevent** is silent truncation: an operator
/// gives a budget, the run quietly does six of the twenty things it would have
/// done, and reports the six. Nothing on the page is false, and the reader has
/// still been misled — they are looking at a sixth of an experiment believing
/// it is the experiment. §3.1's rule is that *ran 6 of 20* is always
/// accompanied by the fourteen.
///
/// So a budget produces a **proposal**, which names both halves, or it
/// produces a refusal that says why no proposal could be made. It never
/// produces a smaller run with no note attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// The whole declaration fits.
    Whole {
        /// What will run.
        running: Work,
    },
    /// Some of it fits, and this is the rest.
    Fewer {
        /// What will run.
        running: Work,
        /// What will not, which is never left unsaid.
        excluded: Work,
    },
    /// Not even the least that would be a comparison fits.
    ///
    /// A refusal rather than a one-pair run: two trials of two arms is the
    /// smallest thing that is a paired comparison at all, and producing
    /// something below it would be answering a different question quietly.
    NotEnough {
        /// The least work that would have been a comparison.
        least: Work,
    },
}

impl Proposal {
    /// What a budget buys, planned against the slow edge of a measured band.
    ///
    /// **The slow edge on purpose.** Planning against the fast edge would put
    /// the run over budget about as often as under it, and the whole point of
    /// a budget is the ceiling. Planning conservatively means a run sometimes
    /// finishes early, which is the harmless direction.
    ///
    /// **`least` is the caller's**, not this module's: what counts as the
    /// smallest worthwhile comparison belongs to whoever is asking, and a
    /// number chosen here would be one more figure MCF invented.
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

    /// What will run, where anything will.
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
