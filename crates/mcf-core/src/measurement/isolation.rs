//! Which variable a comparison isolated, and whether it isolated one at all
//! (A8, B-085, §3.4).
//!
//! **A8 in one sentence:** *a comparison is only meaningful when one thing
//! differs; when more than one did, the honest output is "these are not
//! comparable", not a delta.* Its violation is subtracting two numbers taken
//! at different thermal states and reporting the difference.
//!
//! That is a question about [`Conditions`], not about timings, so it is
//! answered here — beside the floor, and from [`Floor::entries`] rather than
//! from a list of field names written out again. A condition added to the
//! floor becomes a condition this checks, on the same commit, without anybody
//! remembering to come back. §3.3 says the floor never shrinks and does not say
//! it never grows, and a second copy of the list would be the thing that
//! silently stopped growing with it.
//!
//! **Four answers, and the fourth is the interesting one.** Nothing differs,
//! one thing differs, several things differ — and *MCF cannot tell*, because a
//! condition is [`Unknown`] on one side or the other. Two unknowns are not a
//! match: A7 forbids filling an unknown with a plausible value, and *they were
//! probably the same* is exactly that. A comparison whose conditions are
//! mostly unread is not confounded and is not isolated; it is a comparison
//! whose isolation nobody established, and it says so.
//!
//! **What this does not do is judge.** A confound the operator declares is
//! science and a confound nobody declared is an error (A8), and which of those
//! a given comparison is depends on something no condition set contains. This
//! reports what differs; refusing the delta is the caller's, and
//! `mcf_bench::compare` is where that happens.
//!
//! [`Unknown`]: crate::attested::Attested::Unknown

use core::fmt;

use super::Conditions;

/// The name of the instrument, as a condition.
///
/// Not in [`Floor`], because [`Conditions`] keeps it separately — it cannot be
/// unknown, since the running binary knows what it is. It is still a variable:
/// two arms measured by different builds of MCF differ in the instrument, and
/// B64 makes the versions in the path a condition of every absolute figure.
///
/// [`Floor`]: super::Floor
pub const INSTRUMENT: &str = "mcf_build";

/// What separates two arms of a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Isolation {
    /// Every condition is known and every one matches.
    ///
    /// Not a failure and not a confound: two arms of one configuration measure
    /// the machine's own noise, which is the control every comparison should
    /// be able to run against itself.
    SameConfiguration,
    /// Exactly one condition differs, and every other is known to match.
    ///
    /// The only shape in which a delta means what a reader will take it to
    /// mean.
    Isolated {
        /// The variable the comparison is about.
        variable: &'static str,
    },
    /// More than one condition differs.
    ///
    /// A8's *these are not comparable*. The delta exists arithmetically and
    /// says nothing, because nothing here says which of the differences
    /// produced it.
    Confounded {
        /// Every condition that differs, in the floor's order.
        differ: Vec<&'static str>,
    },
    /// Some condition could not be compared, because it is unknown on one side
    /// or the other.
    ///
    /// Reported rather than assumed either way. It is the ordinary state of a
    /// comparison taken before the condition producers exist, and it is honest:
    /// *these two may or may not be isolated, and MCF has not read enough to
    /// say*.
    Undetermined {
        /// The conditions known to differ, which may be none.
        differ: Vec<&'static str>,
        /// The conditions MCF could not compare.
        unread: Vec<&'static str>,
    },
}

impl Isolation {
    /// What separates two arms.
    ///
    /// Ordering matters and is stated: **two known differences are a confound
    /// whatever else is unread.** A comparison that has already lost its
    /// meaning does not recover it by MCF failing to read a twelfth condition,
    /// and reporting *undetermined* there would be the softer of two answers
    /// where A8 wants the harder one.
    #[must_use]
    pub fn between(one: &Conditions, other: &Conditions) -> Self {
        let mut differ = Vec::new();
        let mut unread = Vec::new();

        if one.mcf() != other.mcf() {
            differ.push(INSTRUMENT);
        }
        for ((question, mine), (_, theirs)) in one
            .floor()
            .entries()
            .into_iter()
            .zip(other.floor().entries())
        {
            match (mine.known(), theirs.known()) {
                (Some(mine), Some(theirs)) if mine != theirs => differ.push(question),
                (Some(_), Some(_)) => {}
                // One side or both is unknown. Two unknowns are not a match:
                // A7 forbids reading *probably the same* out of *not read*.
                _ => unread.push(question),
            }
        }

        if differ.len() > 1 {
            return Self::Confounded { differ };
        }
        if !unread.is_empty() {
            return Self::Undetermined { differ, unread };
        }
        match differ.first() {
            Some(variable) => Self::Isolated { variable },
            None => Self::SameConfiguration,
        }
    }

    /// Whether a delta from these two arms means what a reader will take it to
    /// mean.
    ///
    /// True only for [`Isolation::Isolated`]. A comparison of one configuration
    /// with itself is *not* isolating a variable — it is measuring noise, which
    /// is a different and useful thing — and an undetermined one has not been
    /// shown to isolate anything.
    #[must_use]
    pub const fn isolates_a_variable(&self) -> bool {
        matches!(*self, Self::Isolated { .. })
    }

    /// Whether more than one condition is known to differ.
    ///
    /// The state A8 refuses a delta from unless the operator declares it.
    #[must_use]
    pub const fn is_confounded(&self) -> bool {
        matches!(*self, Self::Confounded { .. })
    }

    /// Every condition known to differ, in the floor's order.
    #[must_use]
    pub fn differing(&self) -> &[&'static str] {
        match self {
            Self::SameConfiguration => &[],
            Self::Isolated { variable } => core::slice::from_ref(variable),
            Self::Confounded { differ } | Self::Undetermined { differ, .. } => differ,
        }
    }
}

impl fmt::Display for Isolation {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameConfiguration => form.write_str(
                "the two arms are one configuration: nothing differs, so what this measures is \
                 the machine rather than a difference between them",
            ),
            Self::Isolated { variable } => {
                write!(form, "one variable differs, and it is {variable}")
            }
            Self::Confounded { differ } => write!(
                form,
                "these are not comparable: {} conditions differ ({}), so a delta between them \
                 says nothing about which one produced it (A8)",
                differ.len(),
                differ.join(", ")
            ),
            Self::Undetermined { differ, unread } => {
                if differ.is_empty() {
                    write!(
                        form,
                        "isolation is undetermined: nothing known differs, and {} condition(s) \
                         could not be compared ({})",
                        unread.len(),
                        unread.join(", ")
                    )
                } else {
                    write!(
                        form,
                        "isolation is undetermined: {} differs, and {} condition(s) could not be \
                         compared ({})",
                        differ.join(", "),
                        unread.len(),
                        unread.join(", ")
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
