//! Whether the published seed set is representative, rather than assumed to be
//! (B-291, D19, §6.16, §7.13).
//!
//! **§6.16's discipline, turned on MCF's own instrument.** *The instrument does
//! not get to grade itself.* D19 applies it to the seed set: *periodically, a
//! larger random set is run and its distribution compared with the fixed set's.
//! Divergence means the standard set is unrepresentative and is replaced, with
//! the replacement recorded as a break in comparability.*
//!
//! **The question, precisely.** MCF's published set is the first *n* draws of a
//! stated stream (`mcf_core::trial::published`). The set is unrepresentative if
//! that prefix behaves differently from the stream at large — if the first
//! thirty-two seeds happen to produce outcomes that a hundred times as many
//! seeds do not. That is a question about a prefix against a body, and it is
//! answerable by running both and comparing what came out.
//!
//! **The polarity is inverted, and that is the whole reason this module
//! exists separately.** Everywhere else in this crate, *they differ* is the
//! interesting answer and *they are the same* is a null result. Here the good
//! news is *the same to within a stated resolution* and the finding is
//! *distinguishable*. Wrapping the verdict in a type that says which is which
//! stops a reader — or a later surface — reading a green result as a
//! discovery.
//!
//! **The two sets cannot be paired.** Trial *i* of the standard set and trial
//! *i* of the larger draw share nothing but their index: different seeds,
//! different trajectories, and nothing that a pairing would cancel. So this
//! uses the pooled null, which §3.27 already calls the weaker construction —
//! and it is the right one, because there is genuinely nothing to pair
//! ([`Strength::Assembled`] is what a *comparison* built this way would carry).
//!
//! [`Strength::Assembled`]: super::compare::Strength::Assembled
//!
//! **Not an instrument:** it draws and describes seeds; the comparison it
//! makes is `enough`'s.

use core::fmt;

use mcf_core::measurement::PartsPerMillion;

use super::enough::{self, Verdict};

/// What a validation run found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Representative {
    /// The standard set is indistinguishable from the larger draw, to a stated
    /// resolution.
    ///
    /// The answer that lets the set stay. It is stated with its resolution
    /// because *no difference* without one is *we did not look hard enough*
    /// (A9, F55).
    Yes {
        /// The difference that would have shown.
        resolving: PartsPerMillion,
        /// How many trials the standard set contributed.
        standard: usize,
        /// How many the larger draw did.
        larger: usize,
    },
    /// The standard set behaves differently from the stream it is a prefix of.
    ///
    /// **This is a finding about MCF, not about a model.** D19's response is to
    /// replace the set and record the replacement as a break in comparability
    /// (§7.13) — every measurement taken against the old set stays valid and
    /// stops being comparable with what comes after, which is a fact to write
    /// down rather than a reason to discard evidence.
    No {
        /// How far apart the two distributions' medians are.
        by: PartsPerMillion,
        /// How often the pooled null produced a gap that big.
        by_chance: PartsPerMillion,
        /// How many trials the standard set contributed.
        standard: usize,
        /// How many the larger draw did.
        larger: usize,
    },
    /// Neither: the two have not separated, and a difference worth caring
    /// about would not yet have shown.
    ///
    /// A validation that stopped here has not cleared the set. Saying it had
    /// would be the instrument grading itself, which is the thing §6.16
    /// forbids.
    NotYet {
        /// How many trials each side contributed, the smaller of the two.
        after: usize,
    },
}

impl Representative {
    /// Whether the set may keep being used.
    ///
    /// True only for [`Representative::Yes`]: *not yet decided* is not
    /// clearance, and treating it as clearance is exactly how an unvalidated
    /// instrument stays unvalidated.
    #[must_use]
    pub const fn clears_the_set(&self) -> bool {
        matches!(*self, Self::Yes { .. })
    }
}

impl fmt::Display for Representative {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yes {
                resolving,
                standard,
                larger,
            } => write!(
                form,
                "the standard set is indistinguishable from a draw {} times larger, to within \
                 {} — {standard} trials against {larger}",
                larger.wrapping_div((*standard).max(1)),
                percent(*resolving)
            ),
            Self::No {
                by,
                by_chance,
                standard,
                larger,
            } => write!(
                form,
                "the standard set is UNREPRESENTATIVE: it differs from the larger draw by {}, \
                 which the pooled null produced {} of the time — {standard} trials against \
                 {larger}. D19's answer is to replace the set and record the replacement as a \
                 break in comparability (§7.13)",
                percent(*by),
                percent(*by_chance)
            ),
            Self::NotYet { after } => write!(
                form,
                "not decided after {after} trials a side: this run has not cleared the standard \
                 set, and *not decided* is not clearance (§6.16)"
            ),
        }
    }
}

/// Compares what the standard set produced against what a larger draw did.
///
/// `standard` and `larger` are one outcome per trial, in whatever quantity the
/// workload measures — this module does not care which, only that both sides
/// measured the same thing under the same conditions. `resolving` is how large
/// a difference would matter, and it is the caller's: a set that is within one
/// percent of the stream is representative for almost any purpose, and a set
/// twenty percent off is not representative for any.
#[must_use]
pub fn representative(
    standard: &[u64],
    larger: &[u64],
    resolving: PartsPerMillion,
) -> Representative {
    // The larger draw must actually be larger, or this is not the comparison
    // D19 asks for: *a larger random set* is the whole design, since a set the
    // same size could differ from the standard one by luck as easily as the
    // standard one differs from the stream.
    if larger.len() <= standard.len() {
        return Representative::NotYet {
            after: standard.len().min(larger.len()),
        };
    }
    match enough::over_separate_arms(standard, larger, resolving) {
        // *Same* and *ordered but unsized* answer this question the same way:
        // neither has shown the standard set unrepresentative at the
        // resolution asked about (F92).
        Verdict::Same { resolving, .. } | Verdict::Ordered { resolving, .. } => {
            Representative::Yes {
                resolving,
                standard: standard.len(),
                larger: larger.len(),
            }
        }
        // Both established differences answer this question the same way.
        // The low bound is what matters: *at least this much* is what makes a
        // standard set unrepresentative, and the interval's floor is that
        // (B-388). `Differ` cannot arrive from this path — these arms are
        // never paired — and is matched so that adding a pairing later does
        // not silently change the answer.
        Verdict::Differ { by, by_chance, .. } | Verdict::Apart { by, by_chance, .. } => {
            Representative::No {
                by: by.low,
                by_chance,
                standard: standard.len(),
                larger: larger.len(),
            }
        }
        Verdict::NotYet { so_far } => Representative::NotYet { after: so_far },
    }
}

/// A ratio, as a reader wants it.
fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

#[cfg(test)]
mod tests;
