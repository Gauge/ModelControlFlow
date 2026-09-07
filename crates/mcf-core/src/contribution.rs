//! What may leave this machine, and in what shape
//! (B-171, B-203, B-251, B-310, B42, B54, B63, D21, §6.30, §3.19, §3.20,
//! §3.27, §6.37, A25).
//!
//! **Outcomes, never artifacts** (B-171, §6.30). Scores, classifications,
//! conditions and distributions may leave. Tasks, tools, fixtures and model
//! outputs may not — and the reason is not only privacy. A benchmark task that
//! travels is a benchmark task that ends up in somebody's training data, and a
//! corpus that leaks its own tasks measures memorization from then on. The
//! rule is enforced by the shape: there is no field here that can hold a
//! prompt, a completion, a document or a file, so an audit of a contribution
//! finds no task content because there is nowhere for it to have been put.
//!
//! **Comparisons in preference to absolutes** (B-251, B54, §3.27). *This arm
//! was 12% quicker than that one, over forty pairs* survives travel: it is a
//! ratio taken on one machine in one afternoon, and both arms met the same
//! afternoon. *This took 380 ms* does not survive: it is a fact about somebody
//! else's hardware, and the reader has no way to scale it. So an [`Absolute`]
//! is constructible only with a complete condition set, and refuses otherwise
//! — while a [`Comparison`] needs the conditions it was taken under and not a
//! complete floor, because what differs between two arms of one pairing is the
//! arm.
//!
//! **A custom workload cannot become a contribution** (B-203, B42, §6.37,
//! A25). A result from a workload somebody supplied is local by definition:
//! nobody else has that workload, so nobody else can reproduce or interpret
//! the number. The marking travels **from production**, on the row, rather
//! than being applied at export — a marking added at the boundary is one that
//! can be forgotten at the boundary.
//!
//! **And nothing here retracts** (B-310, B63, D21, §3.20). There is no method
//! that unsends a contribution, because there is no such act: once something
//! has left, it has left, and an affordance suggesting otherwise would be the
//! most consequential false promise MCF could make. What exists instead is
//! [`Contribution::terms`], which says so before anything is sent.

use core::fmt;

use crate::measurement::{Conditions, PartsPerMillion};
use crate::trial::Arm;

/// Where a result's workload came from (B-203, B42, A25).
///
/// **On the row and set when the result is produced.** A result whose workload
/// nobody else has is not contributable at any later date, and deciding that
/// at the export boundary means deciding it in the one place under time
/// pressure, with the operator watching a progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workload {
    /// A workload MCF ships and everybody has.
    Declared,
    /// A workload the operator supplied.
    ///
    /// Non-comparable, and not because the numbers are worse: nobody else has
    /// it, so nobody else can reproduce or interpret the result. It stays
    /// local, which is A25's answer for user content and B42's for a
    /// customized workload.
    Custom,
}

impl Workload {
    /// Whether a result on this workload may be contributed.
    #[must_use]
    pub const fn is_contributable(self) -> bool {
        matches!(self, Self::Declared)
    }
}

impl fmt::Display for Workload {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::Declared => "a declared workload",
            Self::Custom => "a custom workload — local, and not contributable",
        })
    }
}

/// A paired comparison, which is what travels best (B-251, §3.27).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    /// One arm.
    pub left: Arm,
    /// The other.
    pub right: Arm,
    /// How many pairs it rests on.
    pub pairs: usize,
    /// How much quicker one was, in parts per million of the slower.
    pub effect: PartsPerMillion,
    /// Whether it was the left arm.
    pub left_quicker: bool,
    /// What the run was taken under.
    pub conditions: Conditions,
    /// Where the workload came from.
    pub workload: Workload,
}

/// A single number about a single arm.
///
/// Constructible only through [`Absolute::new`], which refuses an incomplete
/// condition set: a bare duration from a stranger's machine is nearly
/// uninterpretable, and the conditions are the only thing that makes it less
/// so (B54, §3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Absolute {
    arm: Arm,
    nanoseconds: u64,
    conditions: Conditions,
    workload: Workload,
}

/// Why an absolute cannot be contributed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotContributable {
    /// Conditions were missing, and how many.
    ConditionsIncomplete {
        /// How many of the floor's questions were answered.
        known: usize,
        /// How many there are.
        of: usize,
    },
    /// The workload was the operator's own.
    WorkloadIsCustom,
}

impl fmt::Display for NotContributable {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConditionsIncomplete { known, of } => write!(
                form,
                "{known} of {of} conditions are known: an absolute number without its \
                 conditions is a fact about somebody else's hardware that nobody can scale \
                 (B54, §3.4). A comparison of two arms travels where this does not (§3.27)"
            ),
            Self::WorkloadIsCustom => write!(
                form,
                "the workload is the operator's own: nobody else has it, so nobody else can \
                 reproduce or interpret the result (B42, A25)"
            ),
        }
    }
}

impl Absolute {
    /// An absolute reading, if it carries everything one needs.
    ///
    /// # Errors
    ///
    /// [`NotContributable`], by name.
    pub fn new(
        arm: Arm,
        nanoseconds: u64,
        conditions: Conditions,
        workload: Workload,
    ) -> Result<Self, NotContributable> {
        if !workload.is_contributable() {
            return Err(NotContributable::WorkloadIsCustom);
        }
        let floor = conditions.floor();
        let of = floor.entries().len();
        let known = floor.known_count();
        if known < of {
            return Err(NotContributable::ConditionsIncomplete { known, of });
        }
        Ok(Self {
            arm,
            nanoseconds,
            conditions,
            workload,
        })
    }

    /// Which arm.
    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    /// The reading.
    #[must_use]
    pub const fn nanoseconds(&self) -> u64 {
        self.nanoseconds
    }

    /// What it was taken under.
    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }

    /// Where the workload came from — always [`Workload::Declared`] here,
    /// because the constructor refuses otherwise.
    #[must_use]
    pub const fn workload(&self) -> Workload {
        self.workload
    }
}

/// One row of what would leave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A paired comparison.
    Compared(Box<Comparison>),
    /// A single reading, with its full conditions.
    Measured(Box<Absolute>),
}

/// What would leave this machine, in full.
///
/// **There is no `retract`, and there will not be** (B-310, B63, D21). Once
/// something has left, it has left. An affordance suggesting otherwise would
/// be the most consequential false promise MCF could make, because a person
/// would rely on it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contribution {
    rows: Vec<Row>,
}

/// The terms, in the words a person reads before deciding (B-310, §3.20).
pub const TERMS: &str = "What leaves is outcomes only: scores, classifications, conditions and \
                         effect sizes. No prompt, no completion, no task, no fixture and no \
                         file leaves — there is nowhere in the format to put one. Publication \
                         cannot be undone: MCF offers no retraction, because there is no such \
                         act (D21, B63).";

impl Contribution {
    /// Nothing yet.
    #[must_use]
    pub const fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    /// Adds a comparison.
    ///
    /// # Errors
    ///
    /// [`NotContributable::WorkloadIsCustom`] where the workload was the
    /// operator's own — the marking travels from production, so this refusal
    /// is a consequence of what the row already says rather than a judgement
    /// made at the boundary (B-203).
    pub fn and_comparison(mut self, compared: Comparison) -> Result<Self, NotContributable> {
        if !compared.workload.is_contributable() {
            return Err(NotContributable::WorkloadIsCustom);
        }
        self.rows.push(Row::Compared(Box::new(compared)));
        Ok(self)
    }

    /// Adds an absolute reading, which had to be constructible to exist.
    #[must_use]
    pub fn and_absolute(mut self, measured: Absolute) -> Self {
        self.rows.push(Row::Measured(Box::new(measured)));
        self
    }

    /// Every row.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The terms somebody is agreeing to.
    #[must_use]
    pub const fn terms() -> &'static str {
        TERMS
    }
}

impl fmt::Display for Comparison {
    /// The row itself: both arms, the direction, the size, and what it rests
    /// on.
    ///
    /// A24 requires that a person be shown **the rows that leave** rather than
    /// a description of them, so a comparison has to be able to render itself.
    /// The effect is a ratio in per cent because that is what travels (§3.27),
    /// and the conditions are named rather than counted: a reader deciding
    /// whether to publish is entitled to see what they would be publishing.
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (quicker, slower) = if self.left_quicker {
            (&self.left, &self.right)
        } else {
            (&self.right, &self.left)
        };
        write!(
            form,
            "comparison · {} quicker than {} by {}, over {} pair(s) · workload {} · under {}",
            quicker.as_str(),
            slower.as_str(),
            per_cent(self.effect),
            self.pairs,
            self.workload,
            self.conditions
        )
    }
}

impl fmt::Display for Absolute {
    /// The row itself, for the same reason.
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "absolute · {} took {} ns · workload {} · under {}",
            self.arm.as_str(),
            self.nanoseconds,
            self.workload,
            self.conditions
        )
    }
}

impl fmt::Display for Row {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compared(compared) => compared.fmt(form),
            Self::Measured(measured) => measured.fmt(form),
        }
    }
}

/// A ratio as a person reads it, from integers (A6).
#[allow(
    clippy::integer_division,
    reason = "a percentage to one decimal place, from parts per million, as every other ratio \
              in this workspace is rendered"
)]
fn per_cent(ppm: PartsPerMillion) -> String {
    let held = ppm.0;
    format!("{}.{}%", held / 10_000, (held % 10_000) / 1_000)
}

impl fmt::Display for Contribution {
    /// **Every row, in full, and the count after them** (A24, B-160).
    ///
    /// This was a count and nothing else — *2 row(s): 1 comparison(s) and 1
    /// absolute(s)* — which is precisely the description A24 forbids being
    /// shown *instead of* the rows. A person deciding whether to publish
    /// something that cannot be unpublished is entitled to read what it says,
    /// and a summary is what they would have been given.
    ///
    /// There is no other rendering. A surface that wanted the count alone would
    /// have to count the rows itself, which is a thing somebody has to write
    /// and a reviewer can see.
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.rows {
            writeln!(form, "{row}")?;
        }
        let compared = self
            .rows
            .iter()
            .filter(|row| matches!(row, Row::Compared(_)))
            .count();
        write!(
            form,
            "{} row(s): {compared} comparison(s) and {} absolute(s)",
            self.rows.len(),
            self.rows.len().saturating_sub(compared)
        )
    }
}

#[cfg(test)]
mod tests;

/// What arrives from somewhere else, and what it is worth here
/// (B-166, B-172, B33, §3.21, §6.29, §6.3, §3.4, A21).
///
/// **An import is a claim, not a measurement.** Somebody else's machine
/// produced these numbers, and A21's line — *declared is not verified* — is
/// exactly the line an import crosses if nothing stops it. So an [`Imported`]
/// carries the configuration as **declared** and the numbers as
/// [`FromCorpus`], which cannot be rendered as MCF's own and cannot back a
/// recommendation (B-167). Verification does not convert it; it *replaces* it
/// with a local measurement and records what the two said.
///
/// **And the divergence is the finding, not the error** (B-172). Two machines
/// running the same identifier and getting different numbers is a fact about
/// how far a result travels, which is the most valuable thing a corpus can
/// learn about itself. Recording it as a failure would throw away the one
/// observation nobody else is positioned to make.
///
/// [`FromCorpus`]: crate::origin::FromCorpus
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported<T> {
    identifier: String,
    claimed: crate::origin::FromCorpus<T>,
}

impl<T> Imported<T> {
    /// Takes in somebody else's figure, as theirs.
    ///
    /// Deliberately verbose at the boundary: this is the one place a foreign
    /// number enters, and it should look like it.
    #[must_use]
    pub fn new(identifier: impl Into<String>, claimed: crate::origin::FromCorpus<T>) -> Self {
        Self {
            identifier: identifier.into(),
            claimed,
        }
    }

    /// Which configuration this is about.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// What was claimed, which may advise and may not decide (B43).
    #[must_use]
    pub const fn claimed(&self) -> &crate::origin::FromCorpus<T> {
        &self.claimed
    }
}

/// What happened when MCF tried to reproduce an imported result (B-172).
///
/// **Every one of these is an outcome and none is an error** (A9, §6.3). *This
/// identifier needs 48 GiB and you have 24* is a complete answer to *can I run
/// this*, and storing it as a failure would make the register unable to say
/// what this machine has been told it cannot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reproduction<T> {
    /// It was measured here, and the two figures are recorded side by side.
    ///
    /// **Both, always.** Keeping only the local one throws away the
    /// comparison; keeping only the difference throws away what was compared.
    Measured {
        /// What they said.
        theirs: crate::origin::FromCorpus<T>,
        /// What this machine found.
        ours: crate::origin::LocallyMeasured<T>,
    },
    /// The configuration cannot run here, and why.
    ///
    /// A complete answer rather than a refusal to answer (§6.3).
    WillNotFitHere {
        /// What it would need, in the words of whatever judged it.
        needs: String,
        /// What this machine has.
        has: String,
    },
    /// Nothing was tried yet.
    ///
    /// The state an import is in the moment it arrives, and the reason
    /// [`Imported`] renders as *declared*: unattempted is not agreement (A21,
    /// A7).
    NotAttempted,
}

impl<T: fmt::Display> fmt::Display for Reproduction<T> {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measured { theirs, ours } => write!(
                form,
                "measured here: {ours}, against {theirs} — a difference between two machines \
                 running one identifier is evidence about how far a result travels, not an \
                 error (B-172, §6.29)"
            ),
            Self::WillNotFitHere { needs, has } => write!(
                form,
                "will not run here: it needs {needs} and this machine has {has}. That is a \
                 complete answer (§6.3, A9)"
            ),
            Self::NotAttempted => form.write_str(
                "declared, and not verified here: nothing has been measured, which is not \
                 agreement (A21, A7)",
            ),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Imported<T> {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{} — DECLARED elsewhere: {}. `mcf probe` and `mcf bench` are what would make it \
             a measurement here (A21, §3.21)",
            self.identifier, self.claimed
        )
    }
}
