use core::fmt;

use crate::measurement::{Conditions, PartsPerMillion};
use crate::trial::Arm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workload {
    Declared,
    Custom,
}

impl Workload {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub left: Arm,
    pub right: Arm,
    pub pairs: usize,
    pub effect: PartsPerMillion,
    pub left_quicker: bool,
    pub conditions: Conditions,
    pub workload: Workload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Absolute {
    arm: Arm,
    nanoseconds: u64,
    conditions: Conditions,
    workload: Workload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotContributable {
    ConditionsIncomplete { known: usize, of: usize },
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

    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    #[must_use]
    pub const fn nanoseconds(&self) -> u64 {
        self.nanoseconds
    }

    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }

    #[must_use]
    pub const fn workload(&self) -> Workload {
        self.workload
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Compared(Box<Comparison>),
    Measured(Box<Absolute>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contribution {
    rows: Vec<Row>,
}

pub const TERMS: &str = "What leaves is outcomes only: scores, classifications, conditions and \
                         effect sizes. No prompt, no completion, no task, no fixture and no \
                         file leaves — there is nowhere in the format to put one. Publication \
                         cannot be undone: MCF offers no retraction, because there is no such \
                         act (D21, B63).";

impl Contribution {
    #[must_use]
    pub const fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn and_comparison(mut self, compared: Comparison) -> Result<Self, NotContributable> {
        if !compared.workload.is_contributable() {
            return Err(NotContributable::WorkloadIsCustom);
        }
        self.rows.push(Row::Compared(Box::new(compared)));
        Ok(self)
    }

    #[must_use]
    pub fn and_absolute(mut self, measured: Absolute) -> Self {
        self.rows.push(Row::Measured(Box::new(measured)));
        self
    }

    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    #[must_use]
    pub const fn terms() -> &'static str {
        TERMS
    }
}

impl fmt::Display for Comparison {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported<T> {
    identifier: String,
    claimed: crate::origin::FromCorpus<T>,
}

impl<T> Imported<T> {
    #[must_use]
    pub fn new(identifier: impl Into<String>, claimed: crate::origin::FromCorpus<T>) -> Self {
        Self {
            identifier: identifier.into(),
            claimed,
        }
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    #[must_use]
    pub const fn claimed(&self) -> &crate::origin::FromCorpus<T> {
        &self.claimed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reproduction<T> {
    Measured {
        theirs: crate::origin::FromCorpus<T>,
        ours: crate::origin::LocallyMeasured<T>,
    },
    WillNotFitHere {
        needs: String,
        has: String,
    },
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
