use std::path::{Path, PathBuf};

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

use crate::reference::Reference;

const WHERE: Subsystem = Subsystem::new("mcf-hub::credentials");

pub const KNOWN_VARIABLES: &[&str] = &["HF_TOKEN", "HUGGING_FACE_HUB_TOKEN", "HUGGINGFACE_TOKEN"];

#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    token: String,
    fingerprint: String,
}

impl Secret {
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        let fingerprint = fingerprint_of(&token);
        Self { token, fingerprint }
    }

    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.token
    }
}

impl core::fmt::Debug for Secret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Secret({}, redacted)", self.fingerprint)
    }
}

fn fingerprint_of(token: &str) -> String {
    let digest = sha256(token.as_bytes()).hex();
    let short: String = digest.chars().take(12).collect();
    format!("sha256:{short}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Supplied,
    Environment { variable: String },
    File { path: PathBuf },
}

impl core::fmt::Display for Origin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Supplied => f.write_str("supplied directly"),
            Self::Environment { variable } => write!(f, "the environment variable {variable}"),
            Self::File { path } => write!(f, "the file {}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    secret: Secret,
    origin: Origin,
}

impl Credential {
    #[must_use]
    pub fn new(secret: Secret, origin: Origin) -> Self {
        Self { secret, origin }
    }

    #[must_use]
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    #[must_use]
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    #[must_use]
    pub fn describe(&self) -> String {
        format!("{} from {}", self.secret.fingerprint(), self.origin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    pub origin: Origin,
    pub fingerprint: String,
}

impl Sighting {
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "a credential is present in {} ({}); MCF has not used it",
            self.origin, self.fingerprint
        )
    }
}

pub fn sightings(
    look_up: &dyn Fn(&str) -> Option<String>,
    token_file: Option<&Path>,
    read_file: &dyn Fn(&Path) -> Option<String>,
) -> Vec<Sighting> {
    let mut seen = Vec::new();
    for variable in KNOWN_VARIABLES {
        if let Some(value) = look_up(variable) {
            let token = value.trim();
            if !token.is_empty() {
                seen.push(Sighting {
                    origin: Origin::Environment {
                        variable: (*variable).to_owned(),
                    },
                    fingerprint: fingerprint_of(token),
                });
            }
        }
    }
    if let Some((path, contents)) = token_file.and_then(|path| Some((path, read_file(path)?))) {
        let token = contents.trim();
        if !token.is_empty() {
            seen.push(Sighting {
                origin: Origin::File {
                    path: path.to_path_buf(),
                },
                fingerprint: fingerprint_of(token),
            });
        }
    }
    seen
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    Anonymous,
    Offered {
        fingerprint: String,
    },
    Confirmed {
        account: String,
        fingerprint: String,
    },
}

impl core::fmt::Display for Identity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Anonymous => f.write_str("anonymous — no credential was offered"),
            Self::Offered { fingerprint } => {
                write!(f, "a credential ({fingerprint}) is offered and unconfirmed")
            }
            Self::Confirmed {
                account,
                fingerprint,
            } => write!(f, "{account}, by the credential {fingerprint}"),
        }
    }
}

#[must_use]
pub fn missing(reference: &Reference, source: &str) -> Failure {
    Failure::new(
        Category::HubAuthRequired,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "the hub would not say whether this repository exists without a credential, and MCF \
         was given none",
    )
    .with_context("repository", reference.repository())
    .with_context("source", source.to_owned())
    .with_context("identity", Identity::Anonymous.to_string())
    .with_context(
        "what_this_does_not_say",
        "a hub answers the same way for a repository that is private, one that has been \
         withdrawn, and one that never existed (F17). MCF reports what it observed rather \
         than guessing which of the three this is",
    )
    .with_context(
        "what_to_do",
        format!(
            "if {} is private to you, supply a hub access token with read scope for it — MCF \
             does not take one from the environment on its own, and will say so if one is \
             sitting there. If it is not, check the name: a repository that is gone and a \
             repository misspelled look identical from here",
            reference.repository()
        ),
    )
}

#[must_use]
pub fn rejected(reference: &Reference, source: &str, identity: &Identity) -> Failure {
    Failure::new(
        Category::HubAuthRejected,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "the hub refused the credential MCF offered",
    )
    .with_context("repository", reference.repository())
    .with_context("source", source.to_owned())
    .with_context("identity", identity.to_string())
    .with_context(
        "what_to_do",
        "the credential is not accepted here — check that it has not expired and that it has \
         read scope, then supply it again",
    )
}

#[must_use]
pub fn gated(reference: &Reference, source: &str, identity: &Identity) -> Failure {
    Failure::new(
        Category::HubAccessGated,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "the credential was accepted and this repository's terms have not been for this account",
    )
    .with_context("repository", reference.repository())
    .with_context("source", source.to_owned())
    .with_context("identity", identity.to_string())
    .with_context(
        "what_to_do",
        format!(
            "accept the terms for {} on the page the hub publishes them, with the account this \
             credential belongs to; no other credential will help",
            reference.repository()
        ),
    )
}

#[cfg(test)]
mod tests;
