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
//! it lands on both arms and cancels.
//!
//! That shape is not this module's to enforce and never could be, because a
//! function taking two slices cannot tell an interleaved pair of arms from two
//! blocks. It is [`compare`]'s: a [`Comparison`] can only be
//! built by a runner that alternates the arms, or from record positions that
//! prove they alternated. **Nothing here is public** — the entry point is
//! [`Comparison::finding`], and that is B-250's *block-then-subtract does not
//! compile*.
//!
//! **Two nulls, because there are two data shapes.** A paired comparison is
//! decided by flipping the signs of its own paired differences: under *the
//! arms are the same*, which arm came out ahead in a given pair is a coin
//! toss, so re-tossing the coins gives the distribution of medians that noise
//! alone produces. That test uses the pairing, which is the point of having
//! it. A comparison assembled from separate sessions has no pairs, so its null
//! is the older and weaker one — the two arms' timings pooled and redrawn as
//! independent groups — and the [`Strength`] travelling beside the verdict is
//! what stops the two being read as the same claim.
//!
//! [`compare`]: super::compare
//! [`Comparison`]: super::compare::Comparison
//! [`Comparison::finding`]: super::compare::Comparison::finding
//! [`Strength`]: super::compare::Strength
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// The verdict over a paired comparison's own differences.
///
/// `differences` is one signed ratio per pair, in parts per million against
/// the quicker arm, positive where the left arm was quicker. The statistic is
/// their median — the median *of the differences*, which is what B53 means by
/// the reported quantity, and not the difference of two medians.
///
/// The null is a sign flip. Under *these two arms are the same*, the sign of
/// each paired difference is a coin toss, because whatever made one pair's
/// left run slower was the arm or was the noise and the null says it was the
/// noise. Re-tossing every sign four thousand times gives the medians noise
/// alone produces, and the observed median is believed when it is bigger than
/// all but one in twenty of them.
///
/// This throws away nothing the pairing bought: each difference keeps the
/// conditions it was taken under, because both its runs were taken under them.
pub(super) fn over_paired_differences(differences: &[i64], resolving: PartsPerMillion) -> Verdict {
    let pairs = differences.len();
    if pairs < 2 {
        return Verdict::NotYet { so_far: pairs };
    }
    let observed = magnitude(median_signed(&sorted_signed(differences)));

    let by_chance = flipped(differences, observed);
    if observed > 0 && by_chance <= FALSE_ALARMS_ALLOWED {
        return Verdict::Differ {
            by: PartsPerMillion(observed),
            by_chance,
            after: pairs,
        };
    }

    // They have not separated. That is only an answer if a difference worth
    // caring about would have shown — otherwise more pairs are owed.
    if a_paired_effect_would_show(differences, resolving) {
        Verdict::Same {
            resolving,
            after: pairs,
        }
    } else {
        Verdict::NotYet { so_far: pairs }
    }
}

/// The verdict over two arms that were never paired.
///
/// The weaker test, for the weaker construction (§3.27's *it may be all that
/// exists*). With no pairing there is nothing to flip, so the null is built by
/// pooling both arms' timings and drawing two independent groups from them:
/// under it there is no difference between the arms, and every gap it finds is
/// the noise pretending to be one.
///
/// What it cannot see is the thing pairing exists for. A level shift between
/// the two sessions is, to this test, indistinguishable from a difference
/// between the arms — which is why [`Strength::Assembled`] travels with every
/// verdict it produces.
///
/// [`Strength::Assembled`]: super::compare::Strength::Assembled
pub(super) fn over_separate_arms(
    one: &[u64],
    other: &[u64],
    resolving: PartsPerMillion,
) -> Verdict {
    let each = one.len().min(other.len());
    if each < 2 {
        return Verdict::NotYet { so_far: each };
    }

    let (a, b) = (median(&sorted(one)), median(&sorted(other)));
    let Some(observed) = ratio(a, b) else {
        return Verdict::NotYet { so_far: each };
    };

    let by_chance = manufactured(one, other, observed, each);
    if by_chance <= FALSE_ALARMS_ALLOWED {
        return Verdict::Differ {
            by: observed,
            by_chance,
            after: each,
        };
    }

    if a_separate_effect_would_show(one, other, resolving, each) {
        Verdict::Same {
            resolving,
            after: each,
        }
    } else {
        Verdict::NotYet { so_far: each }
    }
}

/// Whether a paired effect of `resolving` would have been declared, on this
/// run's own noise.
///
/// **This is a question about power, and the first draft asked a different
/// one.** It asked whether *these* differences' sign flips reach `resolving`
/// — which is circular, because the null is built out of the very differences
/// an effect would have moved. On four real paired trials it declared *no
/// difference as large as five percent* from differences of thirteen and seven
/// percent, and a sign-flip test on four pairs has sixteen assignments and so
/// **cannot ever** produce a p-value below one in sixteen: `Differ` was
/// unreachable at that count, and a rule that can only answer one way is not a
/// test (F55).
///
/// What is asked instead: take this run's differences, remove whatever effect
/// they already carry by centring them on their median, add an effect of
/// exactly `resolving`, and ask whether *that* would have been declared. If it
/// would, the arms are the same to that resolution and B-086's null result is
/// earned. If it would not, more pairs are owed.
///
/// Both directions are asked although the statistic and the null are both
/// symmetric under negation, so they agree — the cost is one more resampling
/// and the gain is that the symmetry is asserted rather than assumed.
fn a_paired_effect_would_show(differences: &[i64], resolving: PartsPerMillion) -> bool {
    let centre = median_signed(&sorted_signed(differences));
    let effect = i64::try_from(resolving.0).unwrap_or(i64::MAX);
    [effect, effect.saturating_neg()].into_iter().all(|shift| {
        let moved: Vec<i64> = differences
            .iter()
            .map(|held| held.saturating_sub(centre).saturating_add(shift))
            .collect();
        let observed = magnitude(median_signed(&sorted_signed(&moved)));
        observed > 0 && flipped(&moved, observed) <= FALSE_ALARMS_ALLOWED
    })
}

/// Whether an effect of `resolving` would have been declared between two arms
/// that were never paired.
///
/// The same question as [`a_paired_effect_would_show`], asked of the pooled
/// null: scale one arm by `resolving` — which is what *an effect that big*
/// means when there are no differences to shift — and ask whether the pooled
/// redraw would then have separated them.
fn a_separate_effect_would_show(
    one: &[u64],
    other: &[u64],
    resolving: PartsPerMillion,
    each: usize,
) -> bool {
    let scaled: Vec<u64> = other
        .iter()
        .map(|held| {
            let grown = u128::from(*held)
                .saturating_mul(u128::from(MILLION.saturating_add(resolving.0)))
                .wrapping_div(u128::from(MILLION));
            u64::try_from(grown).unwrap_or(u64::MAX)
        })
        .collect();
    let Some(observed) = ratio(median(&sorted(one)), median(&sorted(&scaled))) else {
        return false;
    };
    observed.0 > 0 && manufactured(one, &scaled, observed, each) <= FALSE_ALARMS_ALLOWED
}

/// How often re-tossing the sign of every paired difference produces a median
/// at least this big.
fn flipped(differences: &[i64], effect: u64) -> PartsPerMillion {
    if differences.is_empty() {
        return PartsPerMillion(MILLION);
    }
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut tossed = vec![0_i64; differences.len()];
    let mut alarms = 0_u64;
    for _ in 0..RESAMPLINGS {
        let mut bits = next();
        for (slot, held) in differences.iter().enumerate() {
            // A fresh word every sixty-four signs; the generator is cheap but
            // not free, and a sign needs one bit.
            if slot % 64 == 0 && slot > 0 {
                bits = next();
            }
            if let Some(into) = tossed.get_mut(slot) {
                *into = if bits >> (slot % 64) & 1 == 0 {
                    *held
                } else {
                    held.saturating_neg()
                };
            }
        }
        if magnitude(median_signed(&sorted_signed(&tossed))) >= effect {
            alarms = alarms.saturating_add(1);
        }
    }
    PartsPerMillion(
        alarms
            .saturating_mul(MILLION)
            .wrapping_div(u64::try_from(RESAMPLINGS).unwrap_or(1).max(1)),
    )
}

/// A signed ratio's size, whichever way it pointed.
fn magnitude(held: i64) -> u64 {
    held.unsigned_abs()
}

/// A sorted copy of signed differences.
fn sorted_signed(held: &[i64]) -> Vec<i64> {
    let mut out = held.to_vec();
    out.sort_unstable();
    out
}

/// The middle of a sorted slice of signed differences.
///
/// **Symmetric under negation, which the upper-middle element is not.** With
/// an even number of pairs the two middles are averaged toward zero, so
/// negating every difference negates the statistic exactly. That is not a
/// nicety: the null here is a sign flip, and a statistic that reported a
/// different size depending on which arm was called *left* would make the
/// verdict depend on the order the caller named the arms — a defect the
/// symmetry test in this module's `tests` caught on the first draft.
///
/// It is still an order statistic and still not a mean: it is a function of
/// the two middle values only, and it discards nothing (B56).
fn median_signed(sorted: &[i64]) -> i64 {
    let count = sorted.len();
    if count == 0 {
        return 0;
    }
    let upper = count.wrapping_div(2);
    let above = sorted.get(upper).copied().unwrap_or(0);
    if count % 2 == 1 {
        return above;
    }
    let below = sorted
        .get(upper.saturating_sub(1))
        .copied()
        .unwrap_or(above);
    i64::midpoint(below, above)
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
