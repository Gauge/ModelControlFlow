//! When a comparison has been repeated enough (B-083, DEC-007, F51, F53).
//!
//! **Why this is not a repeat count.** DEC-007 asked how many trials make a
//! result publishable, and the first two answers were measured honestly and
//! disagreed: seven repeats on one engine, fifty on another. Then six clean
//! measurements of a *single* command on a *single* machine needed anywhere
//! from seven to over a hundred, with nothing changed but the time of day
//! (F53). The noise floor is a property of the half-hour, so a number written
//! down is a number about the sitting it was taken in.
//!
//! So a comparison carries a **stopping condition** instead: it repeats until
//! its own resampling separates the difference it is looking at from the noise
//! it is measuring, and reports how many that took. The count becomes part of
//! the result rather than part of the policy.
//!
//! **Paired and interleaved, which matters more than the count.** F51 measured
//! contention shifting a run's whole distribution by sixty-six percent while
//! widening it only from four to nine — so a comparison built as *all of A,
//! then all of B* carries any drift between them as an error landing on every
//! repeat in the same direction, and no repeat count removes it. Interleaved,
//! it lands on both arms and cancels. This module takes the two arms as equal
//! sequences and will not accept them otherwise, because the shape is the
//! defence.
//!
//! **Nanoseconds and parts per million, not floats.** A shipped crate here may
//! not hold a floating-point number, because that is how a NaN reaches a
//! record (A6, A1) — and the rule is right: every quantity in this module is a
//! duration or a ratio, and both are exact in integers. A ratio is
//! [`PartsPerMillion`], which the measurement vocabulary already has. Nothing
//! divides by something that can be zero, and there is no value here that
//! compares false against itself.
//!
//! **Three outcomes, not two.** *They differ*, *they are the same to within a
//! stated resolution*, and *not yet decided* — which is the same honesty D42
//! requires of a probe, for the same reason: a comparison that cannot tell
//! must not be rounded to either answer (A7).

use core::fmt;

use mcf_core::measurement::PartsPerMillion;

/// A million, as the ratios here are expressed.
const MILLION: u64 = 1_000_000;

/// How often the noise may manufacture a difference before the answer is
/// believed.
///
/// One in twenty. It is a convention rather than a measurement and is the only
/// number in this module that was chosen — stated here so that it is one line
/// to find and one line to change.
pub const FALSE_ALARMS_ALLOWED: PartsPerMillion = PartsPerMillion(50_000);

/// How many resamplings decide each question.
const RESAMPLINGS: usize = 4000;

/// What a comparison concluded, and after how many paired trials.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The two arms differ by more than this run's own noise produces.
    Differ {
        /// The difference, against the smaller median.
        by: PartsPerMillion,
        /// How often the noise alone produced one that big.
        by_chance: PartsPerMillion,
        /// How many paired trials it took.
        after: usize,
    },
    /// They did not differ, and the noise was tight enough that a difference
    /// of `resolving` would have been seen if it were there.
    ///
    /// This is a real answer and not a failure to find one — *no difference
    /// larger than this* is a finding, and B-086 says a null result is a
    /// result.
    Same {
        /// The difference that would have been detected.
        resolving: PartsPerMillion,
        /// How many paired trials it took.
        after: usize,
    },
    /// Neither: the arms have not separated and the noise is still wider than
    /// the difference being looked for.
    NotYet {
        /// How many paired trials have been run.
        so_far: usize,
    },
}

impl fmt::Display for Verdict {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Differ {
                by,
                by_chance,
                after,
            } => write!(
                form,
                "they differ by {}, after {after} paired trial(s) — noise alone produced a gap \
                 that big {} of the time",
                percent(*by),
                percent(*by_chance)
            ),
            Self::Same { resolving, after } => write!(
                form,
                "no difference as large as {}, after {after} paired trial(s) — one that big \
                 would have shown",
                percent(*resolving)
            ),
            Self::NotYet { so_far } => write!(
                form,
                "not decided after {so_far} paired trial(s): the arms have not separated and the \
                 noise is still wider than the difference being looked for"
            ),
        }
    }
}

/// Asks whether two arms have separated, and whether they have been repeated
/// enough to say they have not.
///
/// `resolving` is the difference the caller cares about — the size below which
/// they are content to call two things the same. It is the caller's, because
/// *how much is a difference* is a question about their purpose and not about
/// the machine: two percent matters to somebody choosing between builds of one
/// engine and matters to nobody choosing between quantizations.
///
/// # Panics
///
/// Never. Unequal arms return [`Verdict::NotYet`] with what was seen, because
/// a caller that has run one arm more than the other has not yet run a paired
/// trial (and the pairing is the point).
#[must_use]
pub fn verdict(one: &[u64], other: &[u64], resolving: PartsPerMillion) -> Verdict {
    let paired = one.len().min(other.len());
    if paired < 2 || one.len() != other.len() {
        return Verdict::NotYet { so_far: paired };
    }

    let (a, b) = (median(&sorted(one)), median(&sorted(other)));
    let Some(observed) = ratio(a, b) else {
        return Verdict::NotYet { so_far: paired };
    };

    // How often the noise *within* these arms produces a gap this size.
    // Within, not across: this is the run measuring its own noise, which is
    // the whole idea — a floor borrowed from another sitting is a floor from
    // another machine's afternoon (F53).
    let by_chance = manufactured(one, other, observed, paired);
    if by_chance <= FALSE_ALARMS_ALLOWED {
        return Verdict::Differ {
            by: observed,
            by_chance,
            after: paired,
        };
    }

    // They have not separated. That is only an answer if a difference worth
    // caring about would have shown — otherwise more trials are owed.
    if manufactured(one, other, resolving, paired) <= FALSE_ALARMS_ALLOWED {
        Verdict::Same {
            resolving,
            after: paired,
        }
    } else {
        Verdict::NotYet { so_far: paired }
    }
}

/// The gap between two durations, against the smaller of them.
///
/// `None` where the smaller is zero, which is a run that took no measurable
/// time and is not something to divide by (A7 rather than an infinity).
fn ratio(one: u64, other: u64) -> Option<PartsPerMillion> {
    let smaller = one.min(other);
    if smaller == 0 {
        return None;
    }
    let gap = one.abs_diff(other);
    Some(PartsPerMillion(
        gap.saturating_mul(MILLION).wrapping_div(smaller),
    ))
}

/// How often two groups of `each`, drawn from these arms' *combined* timings,
/// differ by at least `effect`.
///
/// Combining them is what makes this the null: under it there is no difference
/// between the arms, so every gap this finds is the noise pretending to be one.
fn manufactured(
    one: &[u64],
    other: &[u64],
    effect: PartsPerMillion,
    each: usize,
) -> PartsPerMillion {
    let together: Vec<u64> = one.iter().chain(other.iter()).copied().collect();
    if together.len() < 2 || each == 0 {
        return PartsPerMillion(MILLION);
    }
    // A fixed seed: a stopping condition that moves between runs is not one
    // (§3.12).
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let held = u64::try_from(together.len()).unwrap_or(1).max(1);
    let mut left = vec![0_u64; each];
    let mut right = vec![0_u64; each];
    let mut alarms = 0_u64;
    for _ in 0..RESAMPLINGS {
        for slot in 0..each {
            let a = usize::try_from(next() % held).unwrap_or(0);
            let b = usize::try_from(next() % held).unwrap_or(0);
            if let (Some(one), Some(other), Some(into_left), Some(into_right)) = (
                together.get(a),
                together.get(b),
                left.get_mut(slot),
                right.get_mut(slot),
            ) {
                *into_left = *one;
                *into_right = *other;
            }
        }
        if ratio(median(&sorted(&left)), median(&sorted(&right))).is_some_and(|gap| gap >= effect) {
            alarms = alarms.saturating_add(1);
        }
    }
    PartsPerMillion(
        alarms
            .saturating_mul(MILLION)
            .wrapping_div(u64::try_from(RESAMPLINGS).unwrap_or(1).max(1)),
    )
}

/// A ratio, as a reader wants it.
fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

/// A sorted copy.
fn sorted(held: &[u64]) -> Vec<u64> {
    let mut out = held.to_vec();
    out.sort_unstable();
    out
}

/// The middle of a sorted slice.
///
/// The median rather than the mean, because F51 measured a tail made of
/// whatever else the machine was doing, and a mean carries it.
fn median(sorted: &[u64]) -> u64 {
    let at = sorted.len().wrapping_div(2);
    sorted.get(at).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests;
