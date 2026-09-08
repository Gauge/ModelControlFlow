use std::collections::BTreeMap;
use std::path::Path;

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_hub::credentials::{self, Credential, Identity, Origin, Secret};
use mcf_hub::reference::Reference;
use mcf_hub::source::{Entry, Fetched, Listing, Source};

const WHERE: Subsystem = Subsystem::new("mcf-lab::hub");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Behaviour {
    WellFormed,
    NeedsCredentials,
    RejectsCredentials,
    Gated,
    RateLimited { retry_after: u64 },
    Truncates { after: u64 },
    ServesDifferentBytes,
    DeceptiveMetadata,
    HostileArchive,
    StopsEvery { bytes: u64 },
    NeverResumes,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub gated: Option<String>,
    pub files: BTreeMap<String, Vec<u8>>,
    pub declared_licence: Option<String>,
    pub lineage: Option<mcf_hub::source::Lineage>,
    pub revision: Option<String>,
    pub declares_digests: bool,
    pub behaviour: Behaviour,
}

impl Repository {
    #[must_use]
    pub fn holding(path: &str, bytes: &[u8]) -> Self {
        let mut files = BTreeMap::new();
        files.insert(path.to_owned(), bytes.to_vec());
        Self {
            files,
            gated: None,
            declared_licence: Some("apache-2.0".to_owned()),
            lineage: None,
            revision: Some("main".to_owned()),
            declares_digests: true,
            behaviour: Behaviour::WellFormed,
        }
    }

    #[must_use]
    pub fn without_digests(mut self) -> Self {
        self.declares_digests = false;
        self
    }

    #[must_use]
    pub fn behaving(mut self, behaviour: Behaviour) -> Self {
        self.behaviour = behaviour;
        self
    }

    #[must_use]
    pub fn derived_from(mut self, base: &str, relation: Option<&str>) -> Self {
        self.lineage = Some(mcf_hub::source::Lineage {
            base: base.to_owned(),
            relation: relation.map(str::to_owned),
        });
        self
    }

    #[must_use]
    pub fn without_licence(mut self) -> Self {
        self.declared_licence = None;
        self
    }
}

#[derive(Debug, Default)]
pub struct FakeHub {
    repositories: BTreeMap<String, Repository>,
    credential: Option<Credential>,
}

impl FakeHub {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with(mut self, repository_name: &str, repository: Repository) -> Self {
        self.repositories
            .insert(repository_name.to_owned(), repository);
        self
    }

    #[must_use]
    pub fn offered(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

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

    fn gate(&self, repository: &Repository, reference: &Reference) -> Option<Failure> {
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
            gated: repository.gated.clone(),
            declared_licence: repository.declared_licence.clone(),
            lineage: repository.lineage.clone(),
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
            Behaviour::Truncates { after } => bytes
                .get(..usize::try_from(*after).unwrap_or(0).min(bytes.len()))
                .unwrap_or(&[])
                .to_vec(),
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
