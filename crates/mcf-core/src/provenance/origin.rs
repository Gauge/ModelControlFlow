//! Where a set of bytes came from.

use core::fmt;

use crate::attested::Attested;

/// A repository reference, as the user wrote it.
///
/// Kept verbatim. B7 commits MCF to accepting any reference without
/// special-casing and reaching a defined outcome for every one, which starts
/// with not normalizing the input into something the user did not type — a
/// reference that failed is far easier to diagnose when the record holds what
/// was actually asked for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Repository(String);

impl Repository {
    /// A repository reference.
    #[must_use]
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    /// The reference, as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A pinned revision of a repository.
///
/// A revision is what makes an acquisition reproducible (§3.12, P3): a tag can
/// be repointed between resolve and fetch — `hub.ref.moved` — and a branch name
/// is not a revision at all. This type holds whatever the hub returned as the
/// immutable identity of what was fetched.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(String);

impl Revision {
    /// A revision.
    #[must_use]
    pub fn new(revision: impl Into<String>) -> Self {
        Self(revision.into())
    }

    /// The revision.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where an artifact came from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Origin {
    /// A model hub.
    Hub {
        /// The repository, as it was asked for.
        repository: Repository,
        /// The revision that was actually fetched, if the hub named one.
        revision: Attested<Revision>,
    },
    /// A file that was already on this machine.
    ///
    /// A real case — an operator who converted weights themselves — and an
    /// honest one: MCF knows where the bytes are and nothing about where they
    /// came from, which is what this variant says.
    LocalFile {
        /// Where it was found.
        path: std::path::PathBuf,
    },
    /// MCF does not know.
    ///
    /// Not a failure and not a placeholder for one. A9 makes "nobody can say
    /// where this came from" a result, and it is far more useful stated than
    /// guessed at.
    Unattributed,
}

impl Origin {
    /// An artifact from a hub.
    #[must_use]
    pub fn hub(repository: Repository, revision: Option<Revision>) -> Self {
        Self::Hub {
            repository,
            revision: match revision {
                Some(revision) => Attested::Known(revision),
                None => Attested::Unknown,
            },
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hub {
                repository,
                revision,
            } => write!(f, "{repository}@{revision}"),
            Self::LocalFile { path } => write!(f, "file {}", path.display()),
            Self::Unattributed => f.write_str("unattributed"),
        }
    }
}
