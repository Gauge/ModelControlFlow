//! Looking upstream at something already acquired (B-331, D37, §7.38).
//!
//! **What this asks and when.** Whether the repository an artifact came from
//! still says what it said. It runs when somebody asks — D37 forbids the timer
//! a watcher would need (B4, §3.13) — and it fetches no weights: a listing and
//! a model card.
//!
//! **What it can find** is [findings.md] F17's table and nothing beyond it. A
//! withdrawn revision is unambiguous. A gate that closed is visible in the card
//! while the file refuses. A relicensing and a replaced file refuse nothing at
//! all — a changed field beside a success — and are found only by comparing
//! against what was written down at acquisition, which is what makes B-006's
//! provenance load-bearing rather than decorative.
//!
//! **What it must not claim.** A hub answers the same way for a repository that
//! is private, one that was withdrawn and one that never existed. So a refusal
//! becomes [`Decay::Unreachable`] carrying what was observed, and
//! `Decay::is_a_change` is false for it: the hub declining to answer says
//! nothing about whether anything changed, and reporting an absence as an event
//! is what A7 forbids.
//!
//! **Nothing here invalidates anything.** The artifact is on the disk and its
//! digest still verifies against what was recorded (B-301). D37 is explicit
//! that a model vanishing from a hub says nothing about the bytes here; what is
//! lost is somebody else's ability to reproduce, which is a condition to state
//! rather than a result to withdraw.
//!
//! [findings.md]: ../../../doc/findings.md

use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::provenance::{Decay, Observation, Origin, Provenance};
use mcf_core::time::Timestamp;

use crate::reference::Reference;
use crate::source::Source;

/// Looks upstream at one artifact, and says what it found.
///
/// `None` when there is nothing to look at: an artifact whose origin is a local
/// file or unattributed has no upstream, and inventing one to check would be
/// worse than not checking (A7).
///
/// `file` is the artifact's own name in the repository, which is what makes a
/// replaced file findable; `None` compares everything except that.
#[must_use]
pub fn look(
    source: &dyn Source,
    provenance: &Provenance,
    file: Option<&str>,
    at: Timestamp,
) -> Option<Observation> {
    let Origin::Hub {
        repository,
        revision,
    } = provenance.origin()
    else {
        return None;
    };

    let (owner, name) = repository.as_str().split_once('/')?;
    let reference = Reference {
        owner: owner.to_owned(),
        name: name.to_owned(),
        file: None,
        revision: match revision {
            Attested::Known(pinned) => Some(pinned.as_str().to_owned()),
            Attested::Unknown => None,
        },
    };

    let listing = match source.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => {
            let found = match failure.category() {
                // The one unambiguous answer: the repository itself answered
                // and the revision did not (F17).
                Category::HubRefNotFound => match revision {
                    Attested::Known(pinned) => Decay::RevisionGone {
                        revision: pinned.as_str().to_owned(),
                    },
                    // Nothing was pinned, so nothing is gone: what the hub is
                    // saying is that the repository is not there, and that is
                    // the ambiguous answer rather than the clear one.
                    Attested::Unknown => Decay::Unreachable {
                        said: failure.category().code().to_owned(),
                    },
                },
                Category::HubAccessGated => Decay::Gated {
                    how: "the hub refused the repository to this identity".to_owned(),
                },
                other => Decay::Unreachable {
                    said: other.code().to_owned(),
                },
            };
            return Some(Observation::new(at, found));
        }
    };

    // A gate that closed. It is checked before the licence because a gated
    // repository's card is still readable (F17), so both would be visible and
    // the gate is the one that changes what an operator can do next.
    if let Some(how) = &listing.gated {
        return Some(Observation::new(at, Decay::Gated { how: how.clone() }));
    }

    if let (Attested::Known(had), Some(declares)) =
        (provenance.licence(), &listing.declared_licence)
    {
        let was = had.to_string();
        if !was.eq_ignore_ascii_case(declares) {
            return Some(Observation::new(
                at,
                Decay::Relicensed {
                    was,
                    now: declares.clone(),
                },
            ));
        }
    }

    if let (Some(name), Attested::Known(had)) = (file, provenance.integrity())
        && let Some(entry) = listing.entries.iter().find(|entry| entry.path == name)
        && let Some(declares) = &entry.digest
    {
        let was = had.hex().to_owned();
        if !was.eq_ignore_ascii_case(declares) {
            return Some(Observation::new(
                at,
                Decay::Replaced {
                    file: name.to_owned(),
                    was,
                    now: declares.clone(),
                },
            ));
        }
    }

    Some(Observation::new(at, Decay::Unchanged))
}

#[cfg(test)]
mod tests;
