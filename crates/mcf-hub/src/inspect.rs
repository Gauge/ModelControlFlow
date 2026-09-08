use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use mcf_core::capability::Capability;
use mcf_core::provenance::Licence;

use crate::source::{Entry, Listing};

const WHERE: Subsystem = Subsystem::new("mcf-hub::inspect");

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

pub const TOLERATED_OVERRUN: u64 = 0;

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

#[must_use]
pub const fn ceiling_for(entry: &Entry) -> u64 {
    entry.size
}

#[cfg(test)]
mod tests;
