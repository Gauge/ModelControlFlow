use core::fmt;

use mcf_core::measurement::PartsPerMillion;

use super::enough::{self, Verdict};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Representative {
    Yes {
        resolving: PartsPerMillion,
        standard: usize,
        larger: usize,
    },
    No {
        by: PartsPerMillion,
        by_chance: PartsPerMillion,
        standard: usize,
        larger: usize,
    },
    NotYet {
        after: usize,
    },
}

impl Representative {
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

#[must_use]
pub fn representative(
    standard: &[u64],
    larger: &[u64],
    resolving: PartsPerMillion,
) -> Representative {
    if larger.len() <= standard.len() {
        return Representative::NotYet {
            after: standard.len().min(larger.len()),
        };
    }
    match enough::over_separate_arms(standard, larger, resolving) {
        Verdict::Same { resolving, .. } | Verdict::Ordered { resolving, .. } => {
            Representative::Yes {
                resolving,
                standard: standard.len(),
                larger: larger.len(),
            }
        }
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

fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

#[cfg(test)]
mod tests;
