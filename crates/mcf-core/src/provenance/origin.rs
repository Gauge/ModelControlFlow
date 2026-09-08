use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Repository(String);

impl Repository {
    #[must_use]
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(String);

impl Revision {
    #[must_use]
    pub fn new(revision: impl Into<String>) -> Self {
        Self(revision.into())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Origin {
    Hub {
        repository: Repository,
        revision: Attested<Revision>,
    },
    LocalFile {
        path: std::path::PathBuf,
    },
    Unattributed,
}

impl Origin {
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
