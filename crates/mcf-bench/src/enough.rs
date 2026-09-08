use core::fmt;

use mcf_core::measurement::PartsPerMillion;

const MILLION: u64 = 1_000_000;

pub const FALSE_ALARMS_ALLOWED: PartsPerMillion = PartsPerMillion(50_000);

const RESAMPLINGS: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Differ {
        by: Spread,
        left_quicker: bool,
        by_chance: PartsPerMillion,
        after: usize,
    },
    Ordered {
        left_quicker: bool,
        by: Spread,
        resolving: PartsPerMillion,
        by_chance: PartsPerMillion,
        after: usize,
    },
    Apart {
        by: Spread,
        left_quicker: bool,
        by_chance: PartsPerMillion,
        after: usize,
    },
    Same {
        resolving: PartsPerMillion,
        by: PartsPerMillion,
        after: usize,
    },
    NotYet {
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
                "the {} arm is quicker by {by}, after {after} trial(s) each — noise alone \
                 manufactured a gap that big {} of the time. These arms were never paired, so \
                 the interval is over every pairwise comparison rather than over pairs, and \
                 B53 makes an assembled comparison the weaker claim (B-388, §3.27)",
                if *left_quicker { "left" } else { "right" },
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

pub(super) fn over_paired_differences(differences: &[i64], resolving: PartsPerMillion) -> Verdict {
    let pairs = differences.len();
    if pairs < 2 {
        return Verdict::NotYet { so_far: pairs };
    }
    let middle = median_signed(&sorted_signed(differences));
    let observed = magnitude(middle);
    let by_chance = one_sided_luck(differences);

    if observed >= resolving.0 && by_chance <= FALSE_ALARMS_ALLOWED {
        let left_quicker = middle > 0;
        return match spread_of(differences) {
            Some(by) if by.clears(resolving) => Verdict::Differ {
                by,
                left_quicker,
                by_chance,
                after: pairs,
            },
            Some(by) => Verdict::Ordered {
                left_quicker,
                by,
                resolving,
                by_chance,
                after: pairs,
            },
            None => Verdict::NotYet { so_far: pairs },
        };
    }

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

fn one_sided_luck(differences: &[i64]) -> PartsPerMillion {
    let ahead = differences.iter().filter(|held| **held > 0).count();
    let behind = differences.iter().filter(|held| **held < 0).count();
    let counted = ahead.saturating_add(behind);
    if counted == 0 {
        return PartsPerMillion(MILLION);
    }
    let (n, m) = at_most_countable(counted, ahead.max(behind));
    let (Some(tail), Some(total)) = (binomial_tail(n, m), two_to_the(n)) else {
        return PartsPerMillion(MILLION);
    };
    let chance = tail
        .saturating_mul(2)
        .saturating_mul(u128::from(MILLION))
        .wrapping_div(total);
    PartsPerMillion(u64::try_from(chance.min(u128::from(MILLION))).unwrap_or(MILLION))
}

fn at_most_countable(counted: usize, lopsided: usize) -> (u32, u32) {
    const COUNTABLE: usize = 126;
    let (counted, lopsided) = if counted > COUNTABLE {
        let scaled = lopsided
            .saturating_mul(COUNTABLE)
            .wrapping_div(counted.max(1));
        (COUNTABLE, scaled.max(COUNTABLE.wrapping_div(2)))
    } else {
        (counted, lopsided)
    };
    (
        u32::try_from(counted).unwrap_or(1),
        u32::try_from(lopsided).unwrap_or(1),
    )
}

fn binomial_tail(n: u32, m: u32) -> Option<u128> {
    let mut total: u128 = 0;
    for k in m..=n {
        total = total.checked_add(choose(n, k)?)?;
    }
    Some(total)
}

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

fn two_to_the(n: u32) -> Option<u128> {
    if n >= 127 { None } else { Some(1_u128 << n) }
}

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
    if observed >= resolving && by_chance <= FALSE_ALARMS_ALLOWED {
        return match spread_of_separate(one, other) {
            Some(by) => Verdict::Apart {
                by,
                left_quicker: a < b,
                by_chance,
                after: each,
            },
            None => Verdict::NotYet { so_far: each },
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

fn magnitude(held: i64) -> u64 {
    held.unsigned_abs()
}

fn sorted_signed(held: &[i64]) -> Vec<i64> {
    let mut out = held.to_vec();
    out.sort_unstable();
    out
}

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

fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

fn sorted(held: &[u64]) -> Vec<u64> {
    let mut out = held.to_vec();
    out.sort_unstable();
    out
}

fn median(sorted: &[u64]) -> u64 {
    let at = sorted.len().wrapping_div(2);
    sorted.get(at).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    pub low: PartsPerMillion,
    pub high: PartsPerMillion,
    pub coverage: PartsPerMillion,
}

impl Spread {
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

#[must_use]
pub fn spread_of(differences: &[i64]) -> Option<Spread> {
    let sorted = sorted_signed(differences);
    let n = u32::try_from(sorted.len()).ok()?;
    let total = two_to_the(n)?;
    let mut best: Option<(usize, u64)> = None;
    for k in 1..=n.wrapping_div(2) {
        let below = binomial_tail(n, n.checked_sub(k)?.checked_add(1)?)
            .unwrap_or(0)
            .min(total);
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

pub const WANTED_COVERAGE: PartsPerMillion = PartsPerMillion(MILLION - FALSE_ALARMS_ALLOWED.0);

const LARGEST_ARM: usize = 40;

fn rank_sum_counts(n: usize, m: usize) -> Option<Vec<u128>> {
    let span = n.checked_mul(m)?.checked_add(1)?;
    let mut table = vec![vec![0_u128; span]; m.checked_add(1)?];
    for row in &mut table {
        *row.first_mut()? = 1;
    }
    for i in 1..=n {
        let mut next = vec![vec![0_u128; span]; m.checked_add(1)?];
        *next.first_mut()?.first_mut()? = 1;
        for j in 1..=m {
            for u in 0..span {
                let carried = u
                    .checked_sub(j)
                    .and_then(|less| table.get(j).and_then(|row| row.get(less)).copied());
                let level = next
                    .get(j.checked_sub(1)?)
                    .and_then(|row| row.get(u))
                    .copied();
                let held = carried.unwrap_or(0).checked_add(level.unwrap_or(0))?;
                *next.get_mut(j)?.get_mut(u)? = held;
            }
        }
        table = next;
        let _ = i;
    }
    table.into_iter().nth(m)
}

fn spread_of_separate(one: &[u64], other: &[u64]) -> Option<Spread> {
    let (n, m) = (one.len().min(LARGEST_ARM), other.len().min(LARGEST_ARM));
    if n == 0 || m == 0 {
        return None;
    }
    let mut pairwise: Vec<i64> = Vec::new();
    for left in one.iter().take(n) {
        for right in other.iter().take(m) {
            let smaller = (*left).min(*right).max(1);
            let signed = i128::from(*right).checked_sub(i128::from(*left))?;
            pairwise.push(
                i64::try_from(
                    signed
                        .checked_mul(i128::from(MILLION))?
                        .checked_div(i128::from(smaller))?,
                )
                .ok()?,
            );
        }
    }
    pairwise.sort_unstable();

    let counts = rank_sum_counts(n, m)?;
    let total: u128 = counts.iter().copied().try_fold(0_u128, u128::checked_add)?;
    let mut best: Option<(usize, u64)> = None;
    let mut below: u128 = 0;
    for (k, held) in counts.iter().enumerate() {
        let missed = below
            .checked_mul(2)?
            .checked_mul(u128::from(MILLION))?
            .checked_div(total)?;
        let coverage = u64::try_from(u128::from(MILLION).saturating_sub(missed)).ok()?;
        if coverage >= WANTED_COVERAGE.0 && k >= 1 && k <= pairwise.len().wrapping_div(2) {
            best = Some((k, coverage));
        }
        below = below.checked_add(*held)?;
    }
    let (k, coverage) = best?;
    let low = *pairwise.get(k.checked_sub(1)?)?;
    let high = *pairwise.get(pairwise.len().checked_sub(k)?)?;
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
