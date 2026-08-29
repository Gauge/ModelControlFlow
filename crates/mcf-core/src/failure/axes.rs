//! The two axes that are not *what failed*.
//!
//! The taxonomy separates three questions because a single tree would have to
//! encode their product. [`Attribution`] answers §3.8's question — is this
//! model slow, or was this machine busy — and B24 makes
//! [`Attribution::Unattributable`] a verdict rather than a gap.
//! [`Disposition`] answers §3.1's and §3.2's: partial success is a real
//! outcome, and degradation is a marked one.

use core::fmt;

/// Whose failure it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Attribution {
    /// MCF itself.
    Mcf,
    /// Something MCF supervises: a download, a conversion, an engine process.
    Managed,
    /// This machine — its memory, its disk, its accelerator, its clock.
    Machine,
    /// The model source.
    Hub,
    /// The artifact: its bytes, its format, its metadata.
    Artifact,
    /// The operator, who asked for something that cannot be done.
    User,
    /// The model under test, whose behaviour is a measurement rather than a
    /// defect (§6.17).
    ModelUnderTest,
    /// Nobody, determinedly.
    ///
    /// B24: this is a verdict, not a gap. MCF says it when it cannot tell
    /// whether a result belongs to the model or to a busy machine, and a
    /// contended trial is excluded from a claim rather than averaged into one.
    Unattributable,
}

use super::category::Branch;

impl Attribution {
    /// Every attribution, in the taxonomy's own order.
    pub const ALL: [Self; 8] = [
        Self::Mcf,
        Self::Managed,
        Self::Machine,
        Self::Hub,
        Self::Artifact,
        Self::User,
        Self::ModelUnderTest,
        Self::Unattributable,
    ];

    /// The value as it appears in the taxonomy and in a record.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mcf => "mcf",
            Self::Managed => "managed",
            Self::Machine => "machine",
            Self::Hub => "hub",
            Self::Artifact => "artifact",
            Self::User => "user",
            Self::ModelUnderTest => "model-under-test",
            Self::Unattributable => "unattributable",
        }
    }

    /// The attribution a written value names, if it names one.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.as_str() == value)
    }
}

impl fmt::Display for Attribution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What MCF did about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Disposition {
    /// MCF declined to proceed, and nothing was started.
    Refused,
    /// MCF continued with reduced capability, and said so at full volume
    /// (A5, §3.2).
    Degraded,
    /// Some of the work completed and is kept (A4).
    Partial,
    /// MCF recovered and the work completed.
    ///
    /// A recovery is never invisible: B2 makes a retried attempt a different
    /// set of conditions from a first one, so what recovered is recorded
    /// rather than smoothed away (P1).
    Recovered,
    /// The work stopped and what it had produced is not kept.
    Aborted,
    /// The work completed and its result is not sound — a clock jump mid-run,
    /// conditions that moved. §3.4's honest answer when isolation was lost.
    Invalidated,
}

impl Disposition {
    /// Every disposition, in the taxonomy's own order.
    pub const ALL: [Self; 6] = [
        Self::Refused,
        Self::Degraded,
        Self::Partial,
        Self::Recovered,
        Self::Aborted,
        Self::Invalidated,
    ];

    /// The value as it appears in the taxonomy and in a record.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Degraded => "degraded",
            Self::Partial => "partial",
            Self::Recovered => "recovered",
            Self::Aborted => "aborted",
            Self::Invalidated => "invalidated",
        }
    }

    /// The disposition a written value names, if it names one.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.as_str() == value)
    }
}

impl fmt::Display for Disposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Attribution {
    /// Which of B-233's four branches this attribution belongs to.
    ///
    /// **The grouping B-233 asks for**: every failure of a yielding run
    /// classifies to one branch or the other, never ambiguously. `Attribution`
    /// already made that unambiguous — every failure supplies one and there is
    /// no default — and this is the coarser reading of it, which is what a
    /// surface needs when the question is *is this evidence about the model*.
    ///
    /// **`Unattributable` is its own answer and not a fifth branch.** B24
    /// makes *MCF cannot tell* a real result, and folding it into any of the
    /// four would be the attribution MCF refused to make, made anyway. So it
    /// returns `None`.
    #[must_use]
    pub const fn branch(self) -> Option<Branch> {
        match self {
            Self::ModelUnderTest => Some(Branch::TheModel),
            Self::Machine | Self::Hub => Some(Branch::TheEnvironment),
            Self::Artifact => Some(Branch::TheArtifact),
            Self::Mcf | Self::Managed | Self::User => Some(Branch::McfItself),
            Self::Unattributable => None,
        }
    }
}
