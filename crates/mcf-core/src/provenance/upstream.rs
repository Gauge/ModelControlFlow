//! What MCF later found where an artifact came from (B-331, D37, §7.38).
//!
//! **Why this is not a change to the provenance.** A [`Provenance`] records
//! what was true when an artifact was acquired, and that does not stop being
//! true because a hub changed afterwards. §3.6 makes provenance a *history*
//! rather than a status, so a decay finding is appended beside it and never
//! written over it — the same shape a transformation has, pointed upstream
//! instead of at the file.
//!
//! **What can be found, and what cannot.** [findings.md] F17 measured a hub's
//! answers: a withdrawn revision is unambiguous, a gate that closed is visible
//! in the card while the file refuses, and a relicensing or a repointed tag
//! refuses nothing at all — a changed field beside a success. A repository that
//! does not answer is indistinguishable from a private one and from one that
//! never existed, which is why [`Decay::Unreachable`] says what was observed
//! rather than which of the three it was.
//!
//! **None of this invalidates anything.** D37 is explicit: the artifact is
//! here, its digest is verifiable, and a tool that retracted its own
//! measurements because somebody else deleted something would be destroying
//! evidence for a reason that is not scientific. What decay costs is
//! reproducibility by a third party, and that is a condition to state rather
//! than a result to withdraw.
//!
//! [`Provenance`]: super::Provenance
//! [findings.md]: ../../../../doc/findings.md

use core::fmt;

use crate::time::Timestamp;

/// What MCF found when it looked upstream, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// When MCF looked. Not when the change happened — a hub does not say that,
    /// and a time MCF inferred would be a condition it invented (A7).
    pub looked_at: Timestamp,
    /// What it found.
    pub found: Decay,
}

impl Observation {
    /// Records a look upstream.
    #[must_use]
    pub const fn new(looked_at: Timestamp, found: Decay) -> Self {
        Self { looked_at, found }
    }
}

impl fmt::Display for Observation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {}", self.looked_at, self.found)
    }
}

/// What a look upstream found.
///
/// *Unchanged* is a variant rather than an absence: a check that ran and found
/// nothing wrong is a fact worth having, and reading it as *never checked*
/// would lose the difference (A7, A1).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Decay {
    /// Everything MCF compared still matches what it recorded.
    Unchanged,
    /// The pinned revision is not there any more.
    ///
    /// The one unambiguous answer a hub gives (F17): a 404 against a repository
    /// that itself answers.
    RevisionGone {
        /// The revision MCF pinned.
        revision: String,
    },
    /// The repository now declares a different licence.
    ///
    /// Refuses nothing and breaks nothing; it changes what an operator may do
    /// with what they already have, which is why §III makes it legible at all.
    Relicensed {
        /// What it declared when MCF acquired the artifact.
        was: String,
        /// What it declares now.
        now: String,
    },
    /// The repository is gated now and was not before.
    Gated {
        /// How the hub describes the gate, in its own word.
        how: String,
    },
    /// The file is still published and is not the file MCF has.
    ///
    /// A repository may replace a file under one name. The local copy is
    /// unaffected — its digest still verifies against what was recorded — but
    /// somebody following the same reference now gets different bytes, which is
    /// the whole of what a pin was supposed to prevent (§3.6).
    Replaced {
        /// The file.
        file: String,
        /// The digest MCF recorded.
        was: String,
        /// The digest the hub declares now.
        now: String,
    },
    /// MCF did not get an answer about it.
    ///
    /// Two quite different situations wear this one name, and the rendering
    /// keeps them apart. A hub that **refused** — `hub.auth.*` — is the case
    /// F17 measured: withdrawn, made private and never-there are one answer, so
    /// MCF says what it observed and names the question it is not answering
    /// (D33's habit, D37). A hub that could not be **reached** at all is a fact
    /// about a network, and saying *this could mean the repository is private*
    /// about a refused connection would be inventing a possibility the
    /// observation does not support.
    Unreachable {
        /// What was observed, in MCF's own classification.
        said: String,
    },
}

impl Decay {
    /// Whether anything is different from what was recorded.
    ///
    /// [`Decay::Unreachable`] is **not** a change: the hub declining to answer
    /// says nothing about whether anything changed, and calling it decay would
    /// be reporting an absence as an event (A7).
    #[must_use]
    pub const fn is_a_change(&self) -> bool {
        matches!(
            self,
            Self::RevisionGone { .. }
                | Self::Relicensed { .. }
                | Self::Gated { .. }
                | Self::Replaced { .. }
        )
    }

    /// The finding's name, as a record writes it. Stable for life (C5).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::RevisionGone { .. } => "revision_gone",
            Self::Relicensed { .. } => "relicensed",
            Self::Gated { .. } => "gated",
            Self::Replaced { .. } => "replaced",
            Self::Unreachable { .. } => "unreachable",
        }
    }
}

impl fmt::Display for Decay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => f.write_str("nothing MCF compared has changed"),
            Self::RevisionGone { revision } => {
                write!(f, "the pinned revision {revision} is not there any more")
            }
            Self::Relicensed { was, now } => {
                write!(f, "the declared licence was {was} and is now {now}")
            }
            Self::Gated { how } => write!(f, "the repository is gated now ({how})"),
            Self::Replaced { file, was, now } => write!(
                f,
                "{file} is published with a different digest: was {was}, now {now}"
            ),
            Self::Unreachable { said } if said.starts_with("hub.auth") => write!(
                f,
                "the hub would not say: {said}. That is the same answer it gives for a \
                 repository that is private and one that never existed, so this is what was \
                 observed rather than what happened"
            ),
            Self::Unreachable { said } => write!(
                f,
                "MCF got no answer about it: {said}. That is a fact about reaching the hub \
                 and says nothing about whether anything there has changed"
            ),
        }
    }
}

#[cfg(test)]
mod tests;
