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

use mcf_core::capability::Capability;
use mcf_core::provenance::Licence;

use crate::source::{Entry, Listing};

const WHERE: Subsystem = Subsystem::new("mcf-hub::inspect");

/// What a repository says its weights are, checked against what they are.
///
/// `declared` is the architecture from the repository's own metadata — a model
/// card, a config, the file name. `found` is what a reader of the weights
/// says. They agree, or they do not, and the second is a finding about the
/// What a repository says its model is, against what its weights say.
///
/// A [`Capability`] rather than an enum of its own: *declared*, *verified* and
/// *diverged* are not special to architectures, and B-050 makes them a type so
/// that a declaration cannot be read as an observation anywhere (§3.18, A21).
///
/// The four situations it can be in are the four that exist. A card and weights
/// that agree is a verified architecture with a declaration behind it. Weights
/// alone is verified with nothing declared — plenty of repositories carry no
/// card, and A7 makes that a state rather than a defect. A card whose weights
/// MCF has not read is *declared*, which is a thing MCF knows and must not
/// discard. And a card that disagrees with the weights is the divergence
/// [`deception`] turns into a failure.
#[must_use]
pub fn architecture(declared: Option<&str>, found: Option<&str>) -> Capability<String> {
    let mut known = Capability::unknown();
    if let Some(declared) = declared {
        known = known.and_declared(declared.to_owned());
    }
    if let Some(found) = found {
        known = known.and_verified(found.to_owned());
    }
    known
}

/// The failure a divergence is, if this is one.
///
/// Returned rather than raised: whether a deceptive card stops an acquisition
/// is the caller's decision (a user may want the weights anyway, knowing), and
/// B7 makes *a defined outcome* the commitment rather than a refusal.
#[must_use]
pub fn deception(architecture: &Capability<String>) -> Option<Failure> {
    let (declared, found) = architecture.divergence()?;
    Some(
        Failure::new(
            Category::HubMetadataDeceptive,
            Attribution::Artifact,
            Disposition::Refused,
            WHERE,
            "the repository declares an architecture its weights are not",
        )
        .with_context("declared", declared.clone())
        .with_context("found_in_the_weights", found.clone()),
    )
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
