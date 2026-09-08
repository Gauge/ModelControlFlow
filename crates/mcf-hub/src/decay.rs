use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::provenance::{Decay, Observation, Origin, Provenance};
use mcf_core::time::Timestamp;

use crate::reference::Reference;
use crate::source::Source;

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
                Category::HubRefNotFound => match revision {
                    Attested::Known(pinned) => Decay::RevisionGone {
                        revision: pinned.as_str().to_owned(),
                    },
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
