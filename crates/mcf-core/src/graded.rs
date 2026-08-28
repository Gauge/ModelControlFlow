//! What a laboratory reports, in a shape that cannot be averaged
//! (B-200, B-201, B40, B41, D2, §3.23, §3.9).
//!
//! **The failure this exists to make unrepresentable.** B40: *a model with no
//! tool-calling that scores 4% on an agentic suite has not been measured
//! badly — it has not been measured.* The four percent is the wrong
//! instrument's reading, and once it is a number in a column nothing
//! downstream can tell it from a real one. It sorts. It averages. It loses a
//! comparison. It becomes a verdict about a model, arrived at by grading it on
//! a capability it does not have.
//!
//! So the four outcomes are **variants and not values**: `Measured` carries a
//! score, and `NotApplicable`, `Unknown` and `Failed` carry no place to put
//! one. There is no accessor that yields a number from the other three, no
//! `unwrap_or`, no `Default`, and no ordering. A caller that wants a number
//! must match, and the match is where they meet the case they were about to
//! flatten.
//!
//! **And no total** (B41, B-201). [`Profile`] holds one outcome per laboratory
//! and offers no arithmetic across them. There is no weighted average, no
//! overall rating, no `Ord`: *which model is better* has no referent once
//! quality is plural, and §5's leaderboard wearing local clothes is exactly
//! what an `overall` column would be. What a profile *does* offer is coverage —
//! which laboratories informed it and which the candidate was inapplicable to —
//! because B41 makes that travel with every answer.
//!
//! **Why the crate cannot cheat.** `Score` is deliberately not a number type:
//! it has no `Add`, no `Sum`, no `PartialOrd` across laboratories. Two scores
//! from one laboratory can be compared, because that is what a laboratory is
//! for; two from different ones cannot, because the comparison has no meaning
//! and a type that permits it is an invitation.

use core::fmt;

use crate::failure::Failure;

/// Which laboratory a result came from.
///
/// A name rather than an enumeration, for the reason [`crate::trial::Arm`]
/// gives: there is no fixed number of laboratories, and a type that fixed the
/// list would have to be edited to add one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabId(String);

impl LabId {
    /// Names a laboratory.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LabId {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(&self.0)
    }
}

/// What one laboratory measured, in its own units.
///
/// **Comparable only within a laboratory**, which is what the `lab` field is
/// doing here: two scores from the same laboratory are two readings of one
/// instrument and [`Score::against`] compares them; two from different ones
/// are not a comparison at all, and there is no operation that produces one.
///
/// The value is parts per million of whatever the laboratory's own scale is,
/// which is a unit and not a rating: what *high* means is the laboratory's to
/// say, and MCF does not know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    lab: LabId,
    parts_per_million: u64,
}

impl Score {
    /// Records a score, in the laboratory's own scale.
    #[must_use]
    pub const fn new(lab: LabId, parts_per_million: u64) -> Self {
        Self {
            lab,
            parts_per_million,
        }
    }

    /// Which laboratory's scale this is on.
    #[must_use]
    pub const fn lab(&self) -> &LabId {
        &self.lab
    }

    /// The reading, in parts per million of that laboratory's scale.
    #[must_use]
    pub const fn parts_per_million(&self) -> u64 {
        self.parts_per_million
    }

    /// How this compares with another score **from the same laboratory**.
    ///
    /// `None` where they are from different laboratories, which is not a
    /// failure to compare but the absence of anything to compare: the two
    /// scales have no relation, and any ordering returned here would be one
    /// MCF invented (B41, §3.9).
    #[must_use]
    pub fn against(&self, other: &Self) -> Option<core::cmp::Ordering> {
        (self.lab == other.lab).then(|| self.parts_per_million.cmp(&other.parts_per_million))
    }
}

impl fmt::Display for Score {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{}% on {}",
            self.parts_per_million.wrapping_div(10_000),
            self.parts_per_million.wrapping_div(1_000).wrapping_rem(10),
            self.lab
        )
    }
}

/// What a laboratory reports about one candidate.
///
/// Four outcomes, never collapsed. Three of them carry no score and there is
/// no operation on this type that produces one from them (B40, B-200).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Graded {
    /// The laboratory ran and read something.
    Measured(Score),
    /// The capability the laboratory depends on was **verified absent**.
    ///
    /// Not *scored zero* and not *unknown*: MCF looked, and the thing is not
    /// there. Grading it anyway would be measuring the absence with the wrong
    /// instrument.
    NotApplicable {
        /// Which capability, so that the reader can see what would make it
        /// applicable.
        capability: String,
    },
    /// Whether the capability is there has not been established.
    ///
    /// A7's case, and distinct from `NotApplicable` in the way that matters:
    /// *MCF has not looked* and *MCF looked and it is absent* are different
    /// claims, and running the laboratory would resolve one of them.
    Unknown {
        /// What was not established, and why.
        why: String,
    },
    /// The laboratory ran and could not finish.
    ///
    /// The classified failure travels whole, because *the run broke* and *the
    /// model was bad at it* are the confusion B40 exists to prevent and a
    /// failure with no taxonomy behind it is indistinguishable from a low
    /// score at a glance.
    Failed(Box<Failure>),
}

impl Graded {
    /// The score, where there is one.
    ///
    /// The **only** way a number leaves this type, and it is an `Option` on
    /// purpose: a caller reaching for a number meets the three cases that have
    /// none at the point where they were about to flatten them. There is
    /// deliberately no `unwrap_or`, no `Default` and no `score_or_zero`.
    #[must_use]
    pub const fn score(&self) -> Option<&Score> {
        match self {
            Self::Measured(score) => Some(score),
            Self::NotApplicable { .. } | Self::Unknown { .. } | Self::Failed(_) => None,
        }
    }

    /// Whether this is a reading at all.
    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(self, Self::Measured(_))
    }
}

impl fmt::Display for Graded {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measured(score) => write!(form, "{score}"),
            Self::NotApplicable { capability } => write!(
                form,
                "not applicable: {capability} was verified absent, so this was not measured \
                 badly — it was not measured (B40)"
            ),
            Self::Unknown { why } => write!(
                form,
                "unknown: {why} — which is not the same as absent, and not a low score (A7)"
            ),
            Self::Failed(failure) => write!(form, "failed: {failure}"),
        }
    }
}

/// One candidate's outcomes across laboratories, with no total.
///
/// **There is no overall rating here and no way to compute one** (B41, D2,
/// §3.9). The type holds the outcomes and reports coverage; it has no `Ord`,
/// no arithmetic, and no method that returns a single number about the
/// candidate. *Which model is better* has no referent once quality is plural,
/// and an `overall` column is §5's leaderboard wearing local clothes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    outcomes: Vec<(LabId, Graded)>,
}

impl Profile {
    /// A profile with nothing in it.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            outcomes: Vec::new(),
        }
    }

    /// Records what one laboratory reported.
    #[must_use]
    pub fn and(mut self, lab: LabId, graded: Graded) -> Self {
        self.outcomes.push((lab, graded));
        self
    }

    /// Everything reported, in the order it was recorded.
    #[must_use]
    pub fn all(&self) -> &[(LabId, Graded)] {
        &self.outcomes
    }

    /// Which laboratories actually measured this candidate.
    #[must_use]
    pub fn measured(&self) -> Vec<&LabId> {
        self.outcomes
            .iter()
            .filter(|(_, graded)| graded.is_measured())
            .map(|(lab, _)| lab)
            .collect()
    }

    /// Which laboratories the candidate was inapplicable to.
    ///
    /// B41 makes coverage travel with every answer, and *inapplicable to four
    /// of the seven* is the half of the coverage that a reader most needs and
    /// is least likely to be shown.
    #[must_use]
    pub fn inapplicable(&self) -> Vec<&LabId> {
        self.outcomes
            .iter()
            .filter(|(_, graded)| matches!(graded, Graded::NotApplicable { .. }))
            .map(|(lab, _)| lab)
            .collect()
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.outcomes.is_empty() {
            return form.write_str("no laboratory has reported on this");
        }
        write!(
            form,
            "measured by {} of {} laboratory(ies)",
            self.measured().len(),
            self.outcomes.len()
        )?;
        let absent = self.inapplicable();
        if !absent.is_empty() {
            write!(
                form,
                "; inapplicable to {}",
                absent
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
