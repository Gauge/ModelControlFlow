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
use mcf_hub::credentials::{self, Credential, Identity, Origin, Secret};
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
    /// Exists, and refuses the credential it is offered.
    ///
    /// A different world from [`Self::NeedsCredentials`]: something *was*
    /// offered and the hub would not have it — expired, revoked, or scoped for
    /// something else — and a better-worded request will not help.
    RejectsCredentials,
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
    /// Serves in pieces: every transfer stops after this many bytes, wherever
    /// it started.
    ///
    /// Distinct from [`Behaviour::Truncates`], which always stops at the same
    /// absolute offset and therefore never finishes. This one is a source that
    /// keeps stopping and can be carried across by resuming — which is B-021's
    /// case, and the only way to test that a resumption actually continues
    /// rather than starting again.
    StopsEvery {
        /// How many bytes each attempt delivers.
        bytes: u64,
    },
    /// Cannot continue from an offset at all, which is a real kind of hub.
    NeverResumes,
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
    /// Whether its listings carry a digest per file.
    ///
    /// Both states are real: a hub that publishes plain files often declares
    /// none, and an artifact acquired from one is *held* rather than verified
    /// (A21). A scenario picks which world it is testing.
    pub declares_digests: bool,
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
            declares_digests: true,
            behaviour: Behaviour::WellFormed,
        }
    }

    /// The same, declaring no digest — which is what a hub publishing plain
    /// files usually does, and the state an acquisition cannot verify against.
    #[must_use]
    pub fn without_digests(mut self) -> Self {
        self.declares_digests = false;
        self
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
    /// The credential a caller has offered, where one has been.
    ///
    /// The real type rather than a flag, so what the scenarios drive is what
    /// the acquisition path will hold (B-024). What the hub does *not* do is
    /// check the token's shape: which tokens a hub accepts is the cause, and
    /// D26 keeps causes out of here — a repository declares that it refuses
    /// what it is offered, and that is the observation.
    credential: Option<Credential>,
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

    /// The same hub, offered a credential.
    ///
    /// The lab holds the real type rather than a boolean, so what the scenarios
    /// exercise is what the acquisition path will hold: a secret that redacts
    /// itself and an origin that is part of the conditions (B-024).
    #[must_use]
    pub fn offered(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// The same hub, offered the laboratory's credential.
    ///
    /// Which token it is does not matter to any scenario — that a credential
    /// was offered does — so the scenarios say the shorter thing.
    #[must_use]
    pub fn authenticated(self) -> Self {
        self.offered(Credential::new(
            Secret::new("hf_the-laboratory's-credential"),
            Origin::Supplied,
        ))
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
    fn gate(&self, repository: &Repository, reference: &Reference) -> Option<Failure> {
        // The refusals are `mcf_hub::credentials`' own, so a scenario asserts
        // the words MCF will really say rather than words a simulator invented
        // (D26: the observation is simulated, the response is not).
        match &repository.behaviour {
            Behaviour::NeedsCredentials if self.credential.is_none() => {
                Some(credentials::missing(reference, &self.describe()))
            }
            Behaviour::RejectsCredentials => Some(credentials::rejected(
                reference,
                &self.describe(),
                &self.identity(),
            )),
            Behaviour::Gated => Some(credentials::gated(
                reference,
                &self.describe(),
                &self.identity(),
            )),
            Behaviour::RateLimited { retry_after } => Some(
                failure(
                    Category::HubRateLimited,
                    Attribution::Machine,
                    "this account is throttled",
                    &reference.repository(),
                )
                .with_context("retry_after_seconds", retry_after.to_string()),
            ),
            Behaviour::NeedsCredentials
            | Behaviour::WellFormed
            | Behaviour::Truncates { .. }
            | Behaviour::StopsEvery { .. }
            | Behaviour::NeverResumes
            | Behaviour::ServesDifferentBytes
            | Behaviour::DeceptiveMetadata
            | Behaviour::HostileArchive => None,
        }
    }
}

impl Source for FakeHub {
    fn identity(&self) -> Identity {
        // *Offered*, never *confirmed*: this hub has no account directory, and
        // a simulator that named an account MCF was never told would be
        // inventing the one thing identity exists to establish.
        match &self.credential {
            None => Identity::Anonymous,
            Some(credential) => Identity::Offered {
                fingerprint: credential.secret().fingerprint().to_owned(),
            },
        }
    }

    fn describe(&self) -> String {
        format!(
            "the laboratory's simulated hub, {} repositories, credentials {}",
            self.repositories.len(),
            if self.credential.is_some() {
                "presented"
            } else {
                "absent"
            }
        )
    }

    fn list(&self, reference: &Reference) -> Result<Listing> {
        let repository = self.find(reference)?;
        if let Some(failure) = self.gate(repository, reference) {
            return Err(failure);
        }

        let entries = repository
            .files
            .iter()
            .map(|(path, bytes)| {
                // The size and digest it *claims*. `ServesDifferentBytes` is the
                // case where these and what arrives disagree, which is the whole
                // point of keeping the claim and the measurement apart (A21).
                let entry =
                    Entry::new(path.clone(), u64::try_from(bytes.len()).unwrap_or(u64::MAX));
                if repository.declares_digests {
                    entry.declaring(sha256(bytes).hex())
                } else {
                    entry
                }
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
        if let Some(failure) = self.gate(repository, reference) {
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
            Behaviour::StopsEvery { bytes: limit } => bytes
                .get(..usize::try_from(*limit).unwrap_or(0).min(bytes.len()))
                .unwrap_or(&[])
                .to_vec(),
            Behaviour::WellFormed
            | Behaviour::NeedsCredentials
            | Behaviour::RejectsCredentials
            | Behaviour::Gated
            | Behaviour::RateLimited { .. }
            | Behaviour::DeceptiveMetadata
            | Behaviour::HostileArchive
            | Behaviour::NeverResumes => bytes.clone(),
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

    fn fetch_from(
        &self,
        reference: &Reference,
        entry: &Entry,
        from: u64,
        into: &Path,
    ) -> Result<Fetched> {
        let repository = self.find(reference)?;
        if let Some(failure) = self.gate(repository, reference) {
            return Err(failure);
        }
        if repository.behaviour == Behaviour::NeverResumes {
            // The default answer the trait gives, said by a source that means
            // it: this hub has no ranges. A caller turns that into a restart it
            // records rather than a failure.
            return Err(failure(
                Category::HubUnreachable,
                Attribution::Machine,
                "this source cannot continue a transfer from an offset",
                &self.describe(),
            ));
        }
        let bytes = repository.files.get(&entry.path).ok_or_else(|| {
            failure(
                Category::HubRefNotFound,
                Attribution::User,
                "this repository does not publish that file",
                &entry.path,
            )
        })?;

        let start = usize::try_from(from).unwrap_or(usize::MAX).min(bytes.len());
        let rest = bytes.get(start..).unwrap_or(&[]);
        let served: Vec<u8> = match &repository.behaviour {
            Behaviour::Truncates { after } => {
                // Always stops at the same absolute offset, so a resumption
                // past it delivers nothing at all — which is what a source that
                // is simply broken looks like.
                let stop = usize::try_from(*after).unwrap_or(0);
                if start >= stop {
                    Vec::new()
                } else {
                    bytes.get(start..stop).unwrap_or(&[]).to_vec()
                }
            }
            Behaviour::StopsEvery { bytes: limit } => rest
                .get(..usize::try_from(*limit).unwrap_or(0).min(rest.len()))
                .unwrap_or(&[])
                .to_vec(),
            Behaviour::ServesDifferentBytes => rest.iter().map(|byte| byte ^ 0xFF).collect(),
            Behaviour::WellFormed
            | Behaviour::NeedsCredentials
            | Behaviour::RejectsCredentials
            | Behaviour::Gated
            | Behaviour::RateLimited { .. }
            | Behaviour::DeceptiveMetadata
            | Behaviour::HostileArchive
            | Behaviour::NeverResumes => rest.to_vec(),
        };

        // Appended, because that is what continuing a transfer is.
        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(into)
        {
            Ok(file) => file,
            Err(error) => {
                return Err(failure(
                    Category::ArtifactUnreadable,
                    Attribution::Machine,
                    "the partial file could not be opened to continue",
                    &error.to_string(),
                ));
            }
        };
        if let Err(error) = std::io::Write::write_all(&mut file, &served) {
            return Err(failure(
                Category::ArtifactUnreadable,
                Attribution::Machine,
                "the continued bytes could not be written",
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
