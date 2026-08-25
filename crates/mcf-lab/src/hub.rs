//! A hub that behaves badly on purpose (B-028, §3.17, D26).
//!
//! B-028: *a complete, deterministic simulated Hugging Face — well-formed,
//! malformed, gated, hostile, truncated, mutating*, so that **every M1 test
//! runs against it with no network** (B19). This is that hub.
//!
//! **It simulates what MCF observes, never what causes it** (D26). There is no
//! model of a rate limiter here, no queue and no clock: there is a source that
//! *answers* "throttled", because what MCF has to get right is what it does
//! with that answer. A simulator that modelled the cause would be a second,
//! unvalidated implementation of somebody else's server, and A12 would then
//! have MCF believing it over the world.
//!
//! **Every behaviour is declared, not random.** A repository in this hub is a
//! constant: these files, this licence, and this way of misbehaving. §3.17's
//! requirement is that a failure found once reproduces exactly, forever, and a
//! hub that decided at random when to fail would make every scenario a
//! different scenario each run.
//!
//! **Bytes are real where bytes matter.** A truncated transfer writes the file
//! it truncated: B-021 has to detect a partial artifact on the disk, and a
//! simulation that returned an error without writing anything would be testing
//! the wrong half.

use std::collections::BTreeMap;
use std::path::Path;

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_hub::reference::Reference;
use mcf_hub::source::{Entry, Fetched, Listing, Source};

const WHERE: Subsystem = Subsystem::new("mcf-lab::hub");

/// How a repository in this hub behaves.
///
/// One variant per thing a real hub does that MCF must survive. Adding one is
/// the same decision B32 governs for scenarios: it earns its place by making a
/// claim MCF cannot otherwise make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Behaviour {
    /// Lists and serves what it says it has.
    WellFormed,
    /// Exists, and will not be listed without credentials.
    NeedsCredentials,
    /// Exists, credentials accepted, terms not agreed for this account.
    Gated,
    /// Throttled, with the hint a hub usually gives.
    RateLimited {
        /// How many seconds it suggests waiting.
        retry_after: u64,
    },
    /// Lists, and serves fewer bytes than it promised.
    ///
    /// The file that arrives is written: B-021 has to find a partial artifact
    /// on the disk rather than an absence.
    Truncates {
        /// How many bytes of the file actually arrive.
        after: u64,
    },
    /// Lists one thing and serves another, which is the mutation-under-us case.
    ServesDifferentBytes,
    /// Publishes metadata that disagrees with the weights it publishes.
    DeceptiveMetadata,
    /// Publishes an archive whose members climb out of the extraction root.
    HostileArchive,
}

/// One repository this hub publishes.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Its files, and the bytes each holds.
    pub files: BTreeMap<String, Vec<u8>>,
    /// The licence it declares, where it declares one.
    pub declared_licence: Option<String>,
    /// The revision it answers with.
    pub revision: Option<String>,
    /// How it misbehaves.
    pub behaviour: Behaviour,
}

impl Repository {
    /// A repository that publishes one file and behaves.
    #[must_use]
    pub fn holding(path: &str, bytes: &[u8]) -> Self {
        let mut files = BTreeMap::new();
        files.insert(path.to_owned(), bytes.to_vec());
        Self {
            files,
            declared_licence: Some("apache-2.0".to_owned()),
            revision: Some("main".to_owned()),
            behaviour: Behaviour::WellFormed,
        }
    }

    /// The same, misbehaving in a stated way.
    #[must_use]
    pub fn behaving(mut self, behaviour: Behaviour) -> Self {
        self.behaviour = behaviour;
        self
    }

    /// The same, declaring no licence — which is a real state and a common one.
    #[must_use]
    pub fn without_licence(mut self) -> Self {
        self.declared_licence = None;
        self
    }
}

/// A hub, holding whichever repositories a scenario put in it.
#[derive(Debug, Default)]
pub struct FakeHub {
    repositories: BTreeMap<String, Repository>,
    /// Whether a caller has presented credentials.
    ///
    /// A flag rather than a token: what MCF does differently is *ask again with
    /// credentials*, and modelling a token's format would be modelling the
    /// cause (D26).
    credentials: bool,
}

impl FakeHub {
    /// An empty hub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a repository at `owner/name`.
    #[must_use]
    pub fn with(mut self, repository_name: &str, repository: Repository) -> Self {
        self.repositories
            .insert(repository_name.to_owned(), repository);
        self
    }

    /// The same hub, with credentials presented.
    #[must_use]
    pub fn authenticated(mut self) -> Self {
        self.credentials = true;
        self
    }

    fn find(&self, reference: &Reference) -> Result<&Repository> {
        self.repositories
            .get(&reference.repository())
            .ok_or_else(|| {
                failure(
                    Category::HubRefNotFound,
                    Attribution::User,
                    "no repository of that name is published here",
                    &reference.repository(),
                )
            })
    }

    /// The failure a repository's behaviour produces before anything else
    /// happens, if it produces one.
    fn gate(&self, repository: &Repository, named: &str) -> Option<Failure> {
        match &repository.behaviour {
            Behaviour::NeedsCredentials if !self.credentials => Some(failure(
                Category::HubAuthRequired,
                Attribution::User,
                "this repository is not readable without credentials",
                named,
            )),
            Behaviour::Gated => Some(
                failure(
                    Category::HubAccessGated,
                    Attribution::User,
                    "this repository's terms have not been accepted for this account",
                    named,
                )
                .with_context("what_to_do", "accept the terms on the repository's page"),
            ),
            Behaviour::RateLimited { retry_after } => Some(
                failure(
                    Category::HubRateLimited,
                    Attribution::Machine,
                    "this account is throttled",
                    named,
                )
                .with_context("retry_after_seconds", retry_after.to_string()),
            ),
            Behaviour::NeedsCredentials
            | Behaviour::WellFormed
            | Behaviour::Truncates { .. }
            | Behaviour::ServesDifferentBytes
            | Behaviour::DeceptiveMetadata
            | Behaviour::HostileArchive => None,
        }
    }
}

impl Source for FakeHub {
    fn describe(&self) -> String {
        format!(
            "the laboratory's simulated hub, {} repositories, credentials {}",
            self.repositories.len(),
            if self.credentials {
                "presented"
            } else {
                "absent"
            }
        )
    }

    fn list(&self, reference: &Reference) -> Result<Listing> {
        let repository = self.find(reference)?;
        if let Some(failure) = self.gate(repository, &reference.repository()) {
            return Err(failure);
        }

        let entries = repository
            .files
            .iter()
            .map(|(path, bytes)| Entry {
                path: path.clone(),
                // The size it *claims*. `ServesDifferentBytes` is the case where
                // this and what arrives disagree, which is the whole point of
                // keeping the claim and the measurement apart (A21).
                size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            })
            .collect();

        Ok(Listing {
            reference: reference.clone(),
            revision: repository.revision.clone(),
            entries,
            declared_licence: repository.declared_licence.clone(),
        })
    }

    fn fetch(&self, reference: &Reference, entry: &Entry, into: &Path) -> Result<Fetched> {
        let repository = self.find(reference)?;
        if let Some(failure) = self.gate(repository, &reference.repository()) {
            return Err(failure);
        }
        let bytes = repository.files.get(&entry.path).ok_or_else(|| {
            failure(
                Category::HubRefNotFound,
                Attribution::User,
                "this repository does not publish that file",
                &entry.path,
            )
        })?;

        let served: Vec<u8> = match &repository.behaviour {
            // Fewer bytes than promised, *written*: B-021 has to find a partial
            // artifact on the disk rather than an absence.
            Behaviour::Truncates { after } => bytes
                .get(..usize::try_from(*after).unwrap_or(0).min(bytes.len()))
                .unwrap_or(&[])
                .to_vec(),
            // The same length, different content: the file changed under the
            // fetch, which a size check cannot see and a digest can.
            Behaviour::ServesDifferentBytes => bytes.iter().map(|byte| byte ^ 0xFF).collect(),
            Behaviour::WellFormed
            | Behaviour::NeedsCredentials
            | Behaviour::Gated
            | Behaviour::RateLimited { .. }
            | Behaviour::DeceptiveMetadata
            | Behaviour::HostileArchive => bytes.clone(),
        };

        if let Some(parent) = into.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return Err(failure(
                Category::ArtifactUnreadable,
                Attribution::Machine,
                "the destination could not be created",
                &error.to_string(),
            ));
        }
        if let Err(error) = std::fs::write(into, &served) {
            return Err(failure(
                Category::ArtifactUnreadable,
                Attribution::Machine,
                "the fetched bytes could not be written",
                &error.to_string(),
            ));
        }

        Ok(Fetched {
            bytes: u64::try_from(served.len()).unwrap_or(u64::MAX),
            digest: sha256(&served).hex(),
        })
    }
}

fn failure(category: Category, attribution: Attribution, detail: &str, found: &str) -> Failure {
    Failure::new(category, attribution, Disposition::Refused, WHERE, detail)
        .with_context("found", found.to_owned())
}

#[cfg(test)]
mod tests;
