//! Reading what a repository claims about itself, and refusing to believe it
//! on its own word (B-022, §3.7).
//!
//! §3.7 makes every byte from the hub untrusted, and §6.4 is the hard version
//! of that: MCF's own integrity does not depend on anything a repository
//! contains behaving. This module is where a repository's *claims* are turned
//! into MCF's *findings*, and the difference between the two is the whole
//! subject (A21 — declared, verified, unknown, never a fourth state).
//!
//! **Three claims a repository makes, and what MCF does with each.**
//!
//! | It claims | MCF | Failure when it is wrong |
//! |---|---|---|
//! | this is the licence | records it as *declared*, and never as verified | `hub.metadata.absent` when there is none |
//! | this file is *n* bytes | plans against it, then checks what arrived | `artifact.incomplete` |
//! | these weights are architecture *x* | reads the weights and compares | `hub.metadata.deceptive` |
//!
//! The third is the interesting one, and it is only possible because MCF has a
//! second reader for the weights themselves (D31): a repository can say
//! anything in a model card, and the tensors either have the shape that
//! architecture implies or they do not.
//!
//! **What "enormous" means, and why it is a rule rather than a judgement.** A
//! hostile repository's cheapest attack is a file that costs more to receive
//! than to send — a listing that says four gigabytes and a stream that never
//! ends. So a fetch is bounded by what the listing claimed, and a source that
//! sends more than it promised is refused at the moment it exceeds it rather
//! than after the disk fills (§3.11).
//!
//! **Nothing here executes anything.** A15 keeps repository code from ever
//! running implicitly, and B-025 is where running it deliberately is built.
//! Reading a tensor directory is not running a model, which is exactly why the
//! deceptive-metadata check can be made without deciding anything about trust.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use mcf_core::provenance::Licence;

use crate::source::{Entry, Listing};

const WHERE: Subsystem = Subsystem::new("mcf-hub::inspect");

/// What a repository says its weights are, checked against what they are.
///
/// `declared` is the architecture from the repository's own metadata — a model
/// card, a config, the file name. `found` is what a reader of the weights
/// says. They agree, or they do not, and the second is a finding about the
/// repository rather than an error in MCF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Architecture {
    /// The repository declared one and the weights agree.
    Agreed {
        /// What both say.
        architecture: String,
    },
    /// The repository declared one and the weights are something else.
    ///
    /// `hub.metadata.deceptive`, and the most useful thing MCF can say about
    /// such a repository (A21's divergence, which is often the most useful
    /// output there is).
    Diverged {
        /// What the repository said.
        declared: String,
        /// What the weights say.
        found: String,
    },
    /// The repository declared nothing, and the weights say what they say.
    ///
    /// Not a failure: plenty of repositories carry no card at all, and A7 makes
    /// the absence a state rather than a defect.
    OnlyTheWeights {
        /// What the weights say.
        found: String,
    },
    /// Neither says anything MCF can read.
    Unknown,
}

impl Architecture {
    /// Compares a declaration with what a reader of the weights found.
    #[must_use]
    pub fn compare(declared: Option<&str>, found: Option<&str>) -> Self {
        match (declared, found) {
            (Some(declared), Some(found)) if declared == found => Self::Agreed {
                architecture: found.to_owned(),
            },
            (Some(declared), Some(found)) => Self::Diverged {
                declared: declared.to_owned(),
                found: found.to_owned(),
            },
            (None, Some(found)) => Self::OnlyTheWeights {
                found: found.to_owned(),
            },
            (Some(_) | None, None) => Self::Unknown,
        }
    }

    /// The failure a divergence is, if this is one.
    ///
    /// Returned rather than raised: whether a deceptive card stops an
    /// acquisition is the caller's decision (a user may want the weights
    /// anyway, knowing), and B7 makes *a defined outcome* the commitment rather
    /// than a refusal.
    #[must_use]
    pub fn divergence(&self) -> Option<Failure> {
        match self {
            Self::Diverged { declared, found } => Some(
                Failure::new(
                    Category::HubMetadataDeceptive,
                    Attribution::Artifact,
                    Disposition::Refused,
                    WHERE,
                    "the repository declares an architecture its weights are not",
                )
                .with_context("declared", declared.clone())
                .with_context("found_in_the_weights", found.clone()),
            ),
            Self::Agreed { .. } | Self::OnlyTheWeights { .. } | Self::Unknown => None,
        }
    }
}

/// Whether a listing says enough about itself to be acted on.
///
/// Returns the licence as one of [`Licence`]'s states: an identifier MCF
/// recognizes, or terms that are present and unmatched. The second is not a
/// failure — a licence MCF cannot name is still a licence, and the answer *the
/// terms are there and you must read them* is one a reader can act on
/// ([`crate::licence`] is where that matching lives, and where MCF's refusal to
/// summarize terms is argued).
///
/// # Errors
///
/// `hub.metadata.absent` when the repository declares no licence: §III makes a
/// licence something MCF surfaces *before* use (B-023), and a repository that
/// declares none is a repository whose terms nobody can read — which is a state
/// to report, not one to fill in. A declaration that is blank declares nothing
/// and is reported the same way.
pub fn terms_are_legible(listing: &Listing) -> Result<Licence> {
    if let Some(licence) = listing
        .declared_licence
        .as_deref()
        .and_then(crate::licence::recognize)
    {
        return Ok(licence);
    }
    Err(Failure::new(
        Category::HubMetadataAbsent,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the repository declares no licence, so its terms cannot be surfaced before use",
    )
    .with_context("repository", listing.reference.repository()))
}

/// How much more than its stated size a file may arrive as before the transfer
/// is refused.
///
/// Zero. A source that sends more than it promised is either broken or hostile,
/// and neither is a reason to keep reading: the listing is the contract, and
/// §3.11 makes a full disk a decision rather than a surprise. A tolerance here
/// would be a number chosen to avoid an argument with a hub, which is how a
/// bound stops being one.
pub const TOLERATED_OVERRUN: u64 = 0;

/// Judges a transfer against what the listing promised.
///
/// # Errors
///
/// `artifact.incomplete` when fewer bytes arrived than were promised —
/// truncation, which is B-021's case to resume from; `hub.metadata.deceptive`
/// when more arrived, because a file larger than its listing is a repository
/// lying about a number MCF plans with.
pub fn arrived_as_promised(entry: &Entry, arrived: u64) -> Result<()> {
    if arrived < entry.size {
        return Err(Failure::new(
            Category::ArtifactIncomplete,
            Attribution::Machine,
            Disposition::Partial,
            WHERE,
            "fewer bytes arrived than the listing promised",
        )
        .with_context("file", entry.path.clone())
        .with_context("promised", entry.size.to_string())
        .with_context("arrived", arrived.to_string()));
    }
    if arrived > entry.size.saturating_add(TOLERATED_OVERRUN) {
        return Err(Failure::new(
            Category::HubMetadataDeceptive,
            Attribution::Artifact,
            Disposition::Refused,
            WHERE,
            "more bytes arrived than the listing promised",
        )
        .with_context("file", entry.path.clone())
        .with_context("promised", entry.size.to_string())
        .with_context("arrived", arrived.to_string()));
    }
    Ok(())
}

/// The most bytes MCF will accept for a file it was told the size of.
///
/// The listing's figure exactly. A fetcher reads to this and stops: a stream
/// that does not end is the cheapest attack a hostile source has, and the only
/// defence that works is refusing to keep reading (§3.7, §3.11).
#[must_use]
pub const fn ceiling_for(entry: &Entry) -> u64 {
    entry.size
}

#[cfg(test)]
mod tests;
