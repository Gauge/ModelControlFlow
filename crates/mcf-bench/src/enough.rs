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
//! decided by the **sign test**: under *the arms are the same*, which arm came
//! out ahead in a given pair is a coin toss, so the chance of a split this
//! lopsided is a sum of binomial coefficients — exact, in whole numbers, with
//! no resampling and no seed in it. That test uses the pairing, which is the
//! point of having it, and uses nothing but the ordering of two values within
//! a pair, which is all [`Quantity`] guarantees. A comparison assembled from
//! separate sessions has no pairs, so its null is the older and weaker one —
//! the two arms' timings pooled and redrawn as independent groups — and the
//! [`Strength`] travelling beside the verdict is what stops the two being read
//! as the same claim.
//!
//! [`Quantity`]: mcf_core::measurement::Quantity
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
        /// How large the difference is, as a range (F92).
        ///
        /// A range and not a number, for the reason B46 gives about every
        /// other derived figure in MCF: one number is the smallest possible
        /// version of a confident wrong number. The sign test beside it
        /// governs the *direction* only — it discards magnitudes before it
        /// computes anything — so without this the size carried no measure of
        /// its own reliability at all.
        by: Spread,
        /// Whether the left arm was the quicker one.
        ///
        /// **A size without a direction is not a comparison.** The first
        /// frontier MCF produced reported seven differences and said of none
        /// of them which way round it was, which is a table nobody can read
        /// (F67). The sign is in the paired differences already; this is
        /// carrying it out.
        left_quicker: bool,
        /// How often the noise alone produced one that big.
        by_chance: PartsPerMillion,
        /// How many paired trials it took.
        after: usize,
    },
    /// Which arm is quicker is established; how much by is not, at the
    /// resolution the caller asked about (F92).
    ///
    /// **Both halves are true and they are different claims.** The sign test
    /// establishes the order and does it rigorously. The interval on the size
    /// runs from below the caller's resolution to above it, so the question
    /// *is the difference one I would care about* has not been answered —
    /// however certain the ordering is.
    ///
    /// A distinct verdict rather than a flag on [`Verdict::Differ`], so that
    /// nothing downstream can render it as a measurement by forgetting to look
    /// at a boolean. It is the marking: a result in this state is not fit to
    /// contribute, and the record says which state it was in.
    ///
    /// MCF invents no threshold here. The resolution came from the caller.
    Ordered {
        /// Whether the left arm was the quicker one.
        left_quicker: bool,
        /// What the size might be, which is the point: it spans the
        /// resolution rather than clearing it.
        ///
        by: Spread,
        /// The resolution the caller asked about, carried so the sentence can
        /// name the bar that was not cleared.
        resolving: PartsPerMillion,
        /// How often noise alone puts them in this order.
        by_chance: PartsPerMillion,
        /// How many paired trials it took.
        after: usize,
    },
    /// Two arms that were never paired are apart, by a point estimate that
    /// carries no interval (F92, B53, B-388).
    ///
    /// **Why this is not [`Verdict::Differ`].** The interval `Differ` carries
    /// is an order statistic of *paired* differences, and there are no pairs
    /// here. The unpaired equivalent needs a rank-sum distribution this crate
    /// does not have, and inventing a range from the two arms' own ranges
    /// would be precisely the confident wrong number the interval exists to
    /// prevent. So the size is reported as what it is — a point, with no
    /// measure of its own reliability — in a variant that cannot be mistaken
    /// for the paired one by anything reading the record.
    ///
    /// B53 already makes an assembled comparison the weaker claim. This is
    /// where the weakness stops being a label.
    Apart {
        /// The difference between the two arms' medians.
        by: PartsPerMillion,
        /// Whether the left arm was the quicker one.
        left_quicker: bool,
        /// How often noise alone manufactured a gap this big.
        by_chance: PartsPerMillion,
        /// How many trials each arm had.
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
        /// The difference actually measured, which is smaller.
        ///
        /// Kept rather than discarded, because A1 forbids losing what was
        /// observed and because *no difference as large as five percent* and
        /// *a measured difference of nought point eight percent* are two true
        /// statements about one run. A reader who later cares about a smaller
        /// resolution needs the second.
        by: PartsPerMillion,
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
                left_quicker,
                by_chance,
                after,
            } => write!(
                form,
                "the {} arm is quicker by {by}, after {after} paired trial(s) — noise alone \
                 put them in this order {} of the time",
                if *left_quicker { "left" } else { "right" },
                percent(*by_chance)
            ),
            Self::Ordered {
                left_quicker,
                by,
                resolving,
                by_chance,
                after,
            } => write!(
                form,
                "the {} arm is quicker — noise alone put them in this order {} of the time \
                 after {after} paired trial(s). HOW MUCH quicker is NOT established at the {} \
                 you asked about: the evidence spans {}. The order is a result; the size is \
                 not, and this comparison is not fit to contribute (F92)",
                if *left_quicker { "left" } else { "right" },
                percent(*by_chance),
                percent(*resolving),
                by
            ),
            Self::Apart {
                by,
                left_quicker,
                by_chance,
                after,
            } => write!(
                form,
                "the {} arm is quicker by {}, after {after} trial(s) each — noise alone \
                 manufactured a gap that big {} of the time. These arms were never paired, so \
                 this size is a point with no interval: MCF has no method to bound it without \
                 pairs, and B53 already makes an assembled comparison the weaker claim \
                 (F92, B-388)",
                if *left_quicker { "left" } else { "right" },
                percent(*by),
                percent(*by_chance)
            ),
            Self::Same {
                resolving,
                by,
                after,
            } => write!(
                form,
                "no difference as large as {}, after {after} paired trial(s) — one that big \
                 would have shown; the measured difference was {}",
                percent(*resolving),
                percent(*by)
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
/// the quicker arm, positive where the left arm was quicker.
///
/// **The test is the sign test, and it is exact.** Under *these two arms are
/// the same*, which arm came out ahead in a given pair is a coin toss, so the
/// number of pairs won by one arm is binomial with a half — and that tail is a
/// sum of binomial coefficients, which is arithmetic on whole numbers with no
/// resampling, no seed and no approximation in it. Ten pairs won by one arm is
/// one chance in five hundred and twelve, computed rather than estimated.
///
/// **Why not a statistic that uses the magnitudes.** Two reasons, both
/// measured. The resampling test that stood here first was degenerate on a
/// pair of arms with *identical* timings: every difference is the same
/// magnitude, so flipping the signs cannot move the median's size, the null
/// distribution is a single point, and the answer is *cannot tell* about data
/// that could not be clearer (F57). And the sign test uses only the ordering
/// of two values within a pair, which is what [`Quantity`] guarantees and all
/// it guarantees — F51 measured a tail made of whatever else the machine was
/// doing, and a statistic that weighs by magnitude carries that tail into the
/// answer.
///
/// **The cost is stated.** Discarding the magnitudes discards power when the
/// noise is well behaved, and the test can call a difference real that is far
/// too small to act on. That is what `resolving` is for, and why the size of
/// the difference is reported beside the verdict and never in place of it.
///
/// **Ties are excluded and their count is not lost.** A pair in which both
/// arms took exactly the same time supports neither, so it leaves the count —
/// the standard treatment — and `after` still reports every pair that was run,
/// because A1 forbids losing the fact that a trial happened.
///
/// [`Quantity`]: mcf_core::measurement::Quantity
pub(super) fn over_paired_differences(differences: &[i64], resolving: PartsPerMillion) -> Verdict {
    let pairs = differences.len();
    if pairs < 2 {
        return Verdict::NotYet { so_far: pairs };
    }
    let middle = median_signed(&sorted_signed(differences));
    let observed = magnitude(middle);
    let by_chance = one_sided_luck(differences);

    // **Both halves, and the second was missing.** A difference must be real
    // *and* as large as the caller said they care about. Without the size
    // test, a long enough run reports every real difference however small —
    // and a caller who asked about five percent was told *they differ by 0.8%*
    // after a hundred and thirteen paired trials, which answers a question
    // nobody asked and invites acting on it (F59, §3.28).
    //
    // Below the resolution, the honest verdict is the null one: *no difference
    // as large as five percent* is true of a measured eight tenths of one, and
    // the measurement travels inside it so nothing is lost (A1).
    if observed >= resolving.0 && by_chance <= FALSE_ALARMS_ALLOWED {
        // Positive is the left arm quicker, which is how a paired difference
        // is signed throughout this crate.
        let left_quicker = middle > 0;
        // **The size, with its own measure of reliability** (F92). The sign
        // test above settled the direction and knows nothing about the
        // magnitude, so the magnitude brings its own interval or it is not
        // stated as a measurement.
        return match spread_of(differences) {
            Some(by) if by.clears(resolving) => Verdict::Differ {
                by,
                left_quicker,
                by_chance,
                after: pairs,
            },
            // The interval runs from below the caller's resolution to above
            // it: the order is established and the size is not, at the bar
            // they set. Both are said.
            Some(by) => Verdict::Ordered {
                left_quicker,
                by,
                resolving,
                by_chance,
                after: pairs,
            },
            // Too few pairs to bound the size at all. That is not a verdict
            // about the arms, it is a statement about how much evidence there
            // is, and `NotYet` is where MCF says so.
            None => Verdict::NotYet { so_far: pairs },
        };
    }

    // They have not separated by as much as was asked about. That is only an
    // answer if a difference worth caring about would have shown — otherwise
    // more pairs are owed.
    if a_paired_effect_would_show(differences, resolving) {
        Verdict::Same {
            resolving,
            by: PartsPerMillion(observed),
            after: pairs,
        }
    } else {
        Verdict::NotYet { so_far: pairs }
    }
}

/// How often a coin toss wins as lopsidedly as these pairs did.
///
/// The two-sided sign test: with `n` pairs that were not ties and `m` of them
/// won by whichever arm won more, this is the chance that tossing `n` coins
/// gives `m` or more of either face. Exact, in whole numbers, computed in
/// `u128` — which holds the arithmetic up to a hundred and twenty-odd pairs
/// and saturates rather than wraps beyond that, so a very long run is
/// conservative rather than wrong.
fn one_sided_luck(differences: &[i64]) -> PartsPerMillion {
    let ahead = differences.iter().filter(|held| **held > 0).count();
    let behind = differences.iter().filter(|held| **held < 0).count();
    let counted = ahead.saturating_add(behind);
    if counted == 0 {
        // Every pair a tie. That is not evidence of a difference, and the
        // `Same` branch is where it is turned into an answer.
        return PartsPerMillion(MILLION);
    }
    let (n, m) = at_most_countable(counted, ahead.max(behind));
    let (Some(tail), Some(total)) = (binomial_tail(n, m), two_to_the(n)) else {
        // Unreachable given the reduction above, and answered rather than
        // asserted: *the noise does this all the time* refuses, where the
        // alternative would overclaim.
        return PartsPerMillion(MILLION);
    };
    let chance = tail
        .saturating_mul(2)
        .saturating_mul(u128::from(MILLION))
        .wrapping_div(total);
    PartsPerMillion(u64::try_from(chance.min(u128::from(MILLION))).unwrap_or(MILLION))
}

/// The counts, reduced to a size the exact arithmetic holds.
///
/// `2^n` leaves `u128` at a hundred and twenty-seven, and a run may have more
/// pairs than that. Refusing to answer there would make **more evidence give a
/// weaker verdict**, which is not a property any instrument may have — the
/// first draft did exactly that, going back to *not decided* at a hundred and
/// sixty pairs of data it had decided at eighty.
///
/// So the counts are scaled down to the largest size that can be computed
/// exactly, with the winning side's count rounded **down**. That is
/// conservative in the only direction that matters: the same lopsidedness over
/// fewer pairs is less surprising, so the chance reported is never smaller than
/// the true one. What is lost is a little power on very long runs, which is the
/// cheap side of the trade.
fn at_most_countable(counted: usize, lopsided: usize) -> (u32, u32) {
    /// The largest `n` for which `2^n` and the tail both fit in `u128`.
    const COUNTABLE: usize = 126;
    let (counted, lopsided) = if counted > COUNTABLE {
        let scaled = lopsided
            .saturating_mul(COUNTABLE)
            .wrapping_div(counted.max(1));
        // A side that won cannot be reduced below half, which would be a
        // reduction into a different question.
        (COUNTABLE, scaled.max(COUNTABLE.wrapping_div(2)))
    } else {
        (counted, lopsided)
    };
    (
        u32::try_from(counted).unwrap_or(1),
        u32::try_from(lopsided).unwrap_or(1),
    )
}

/// The sum of `n` choose `k` for every `k` from `m` to `n`.
///
/// `None` where the arithmetic would leave `u128`, which is a run long enough
/// that the caller reports *undecided* rather than a number it cannot stand
/// behind.
fn binomial_tail(n: u32, m: u32) -> Option<u128> {
    let mut total: u128 = 0;
    for k in m..=n {
        total = total.checked_add(choose(n, k)?)?;
    }
    Some(total)
}

/// `n` choose `k`, exactly.
///
/// Multiplied and divided in step so the running value stays as small as the
/// answer allows: `C(n, k) = C(n, k-1) * (n - k + 1) / k`, and each division is
/// exact because the running value is a binomial coefficient at every step.
fn choose(n: u32, k: u32) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n.saturating_sub(k));
    let mut held: u128 = 1;
    for step in 1..=k {
        held = held.checked_mul(u128::from(n.saturating_sub(step).saturating_add(1)))?;
        held = held.checked_div(u128::from(step))?;
    }
    Some(held)
}

/// Two to the power of `n`, or `None` past what `u128` holds.
fn two_to_the(n: u32) -> Option<u128> {
    if n >= 127 { None } else { Some(1_u128 << n) }
}

/// Whether a paired effect of `resolving` would have been declared, on this
/// run's own noise.
///
/// **This is a question about power, and the first draft asked a different
/// one.** It asked whether *these* differences reach `resolving` — which is
/// circular, because the yardstick is built out of the very differences an
/// effect would have moved. On four real paired trials it declared *no
/// difference as large as five percent* from differences of thirteen and seven
/// percent, at a count where a sign test over four pairs cannot reach one in
/// twenty at all: `Differ` was unreachable, and a rule that can only answer one
/// way is not a test (F55).
///
/// What is asked instead: take this run's differences, remove whatever effect
/// they already carry by centring them on their median, add an effect of
/// exactly `resolving`, and ask whether *that* would have been declared. If it
/// would, the arms are the same to that resolution and A9's null result is
/// earned. If it would not, more pairs are owed.
///
/// Both directions are asked, because the sign test is not symmetric on a set
/// with ties and the cheaper assumption would be one nobody had checked.
fn a_paired_effect_would_show(differences: &[i64], resolving: PartsPerMillion) -> bool {
    let centre = median_signed(&sorted_signed(differences));
    let effect = i64::try_from(resolving.0).unwrap_or(i64::MAX);
    [effect, effect.saturating_neg()].into_iter().all(|shift| {
        let moved: Vec<i64> = differences
            .iter()
            .map(|held| held.saturating_sub(centre).saturating_add(shift))
            .collect();
        one_sided_luck(&moved) <= FALSE_ALARMS_ALLOWED
    })
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
    // The same two halves as the paired path, for the same reason (F59).
    if observed >= resolving && by_chance <= FALSE_ALARMS_ALLOWED {
        // **And a third half this path cannot supply** (F92). `Differ` now
        // carries an interval on the size, and the interval MCF computes is an
        // order statistic of paired differences — of which there are none
        // here. So an assembled comparison reports the order it established
        // and says the size is unbounded, rather than reporting a point
        // estimate that would look exactly like a paired one. B53 already
        // makes this the weaker claim; this is where the weakness becomes
        // visible instead of being a label.
        return Verdict::Apart {
            by: observed,
            left_quicker: a < b,
            by_chance,
            after: each,
        };
    }

    if a_separate_effect_would_show(one, other, resolving, each) {
        Verdict::Same {
            resolving,
            by: observed,
            after: each,
        }
    } else {
        Verdict::NotYet { so_far: each }
    }
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

/// How wide the effect really is, and how sure that width is (F92).
///
/// **What this exists to correct.** The verdict used to carry a single number
/// — *quicker by 356.0%* — beside a sign test's probability, in a sentence
/// that read as though the probability qualified the size. It never did: the
/// sign test discards magnitudes before it computes anything, so its answer is
/// identical for six pairs spanning fifteen points and six spanning a
/// thousand. One number to one decimal place, from evidence that varied in
/// quality by a factor of sixty-nine, all wearing the same 3.1%.
///
/// B46 has said all along that a duration predicted from a rate is a range and
/// that one number is *the smallest possible version of a confident wrong
/// number*. Every estimated figure in MCF is banded. The one **measured**
/// figure — the headline the whole benchmark exists to produce — was not.
///
/// **Exact, and from the machinery already here.** The bounds are order
/// statistics of the paired differences: with the differences sorted, the
/// *k*-th and *(n+1-k)*-th bracket the true median with a probability that is
/// a binomial tail, which the sign test beside it already computes in whole numbers.
/// No resampling, no floats, no distributional assumption beyond the
/// exchangeability the interleaving is there to provide.
///
/// **The coverage is reported, not aimed at.** Six pairs cannot express 95%;
/// the widest interval available covers 96.9%. Saying *96.9%* rather than
/// *95%* is the same discipline as reporting the count a run actually needed
/// rather than the one it hoped for (F53).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    /// The smallest effect the evidence is consistent with.
    pub low: PartsPerMillion,
    /// The largest.
    pub high: PartsPerMillion,
    /// How often an interval built this way contains the true value.
    pub coverage: PartsPerMillion,
}

impl Spread {
    /// Whether the whole interval clears a resolution the caller asked about.
    ///
    /// The question that decides whether a magnitude was established *for this
    /// caller*: an interval running from below their threshold to far above it
    /// does not answer *is the difference one I care about*, however tight the
    /// direction is.
    #[must_use]
    pub const fn clears(&self, resolving: PartsPerMillion) -> bool {
        self.low.0 >= resolving.0
    }
}

impl fmt::Display for Spread {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{} to {} ({} of such intervals contain the true value)",
            percent(self.low),
            percent(self.high),
            percent(self.coverage)
        )
    }
}

/// The exact distribution-free interval for the median of `differences`.
///
/// Public so that a surface reading the record can recompute it from the
/// trials rather than from a stored summary — which is what B55 and B56 have
/// always required, and what lets a comparison recorded before F92 gain the
/// interval its own pairs always supported.
///
/// `None` where no interval reaching [`WANTED_COVERAGE`] exists, which is
/// every run of fewer than six pairs — the same floor at which the sign test
/// cannot reach one in twenty either (F57). That is a fact about how much
/// evidence there is, and it is reported rather than filled in.
///
/// **Where the interval on the signed difference contains zero**, the
/// magnitude is not bounded away from nothing and the low bound is zero. A run
/// whose pairs disagree that much has established a direction it cannot put a
/// floor under, and pretending otherwise by taking absolute values first would
/// manufacture a floor out of the disagreement itself.
#[must_use]
pub fn spread_of(differences: &[i64]) -> Option<Spread> {
    let sorted = sorted_signed(differences);
    let n = u32::try_from(sorted.len()).ok()?;
    let total = two_to_the(n)?;
    let mut best: Option<(usize, u64)> = None;
    for k in 1..=n.wrapping_div(2) {
        // The mass below the k-th order statistic: P(X < k) for X ~ B(n, 1/2),
        // which by symmetry is the upper tail from `n - k + 1`. `binomial_tail`
        // sums from its argument upwards, so that index is what it is given —
        // and getting it wrong by one silently widens every interval MCF
        // reports, which is why the coverage is checked against hand-computed
        // values in the tests rather than only against itself.
        let below = binomial_tail(n, n.checked_sub(k)?.checked_add(1)?)
            .unwrap_or(0)
            .min(total);
        // **Truncated so that the claim is never larger than the truth**
        // (F94). Computing `MILLION - missed` truncates the part being
        // subtracted, which rounds the *coverage* up: at eight pairs the true
        // figure is 99.21875% and that arithmetic states 99.2188% — a claim
        // one part in a million stronger than the evidence. Counting the
        // patterns that fall inside and truncating those states 99.2187%.
        // The size is trivial and the direction is not: a coverage MCF
        // overstates is a guarantee it cannot keep, and the brute-force
        // cross-check found it on its first run.
        let inside = total.checked_sub(below.checked_mul(2)?)?;
        let coverage = u64::try_from(
            inside
                .checked_mul(u128::from(MILLION))?
                .checked_div(total)?,
        )
        .ok()?;
        if coverage >= WANTED_COVERAGE.0 {
            best = Some((usize::try_from(k).ok()?, coverage));
        }
    }
    let (k, coverage) = best?;
    let low = *sorted.get(k.checked_sub(1)?)?;
    let high = *sorted.get(sorted.len().checked_sub(k)?)?;
    // An interval spanning zero puts no floor under the magnitude.
    let (near, far) = if (low <= 0 && high >= 0) || (low >= 0 && high <= 0) {
        (0, magnitude(low).max(magnitude(high)))
    } else {
        (
            magnitude(low).min(magnitude(high)),
            magnitude(low).max(magnitude(high)),
        )
    };
    Some(Spread {
        low: PartsPerMillion(near),
        high: PartsPerMillion(far),
        coverage: PartsPerMillion(coverage),
    })
}

/// The coverage an interval must reach to be worth reporting.
///
/// The complement of [`FALSE_ALARMS_ALLOWED`], so that the interval and the
/// sign test are asking for the same standard of evidence rather than two
/// numbers that happen to sit near each other.
pub const WANTED_COVERAGE: PartsPerMillion = PartsPerMillion(MILLION - FALSE_ALARMS_ALLOWED.0);
