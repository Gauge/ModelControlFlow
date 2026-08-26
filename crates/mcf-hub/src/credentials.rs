//! A credential MCF holds because somebody handed it over (B-024, §III, §3.10).
//!
//! **Never a silent prerequisite.** The failure this module exists to prevent
//! is the ordinary one: a tool reads `HF_TOKEN` out of the environment, works
//! on the machine where that variable happens to be set, and fails somewhere
//! else for a reason nobody can see. Worse for MCF than for most tools, because
//! §3.4 makes the conditions of a measurement part of the measurement: an
//! artifact acquired with a credential and one acquired without it were
//! acquired under different conditions, and a credential picked up invisibly is
//! a condition nobody recorded.
//!
//! So nothing here reads the environment. [`sightings`] is given a way to look
//! and reports what is *there*, which is a different act from using it — the
//! operator is told a token exists and chooses. `checks/tests/
//! a_credential_is_never_picked_up.rs` holds that line structurally: the names
//! of the variables appear in this file and nowhere else in the workspace, and
//! this file contains no environment access at all.
//!
//! **The secret is not the record.** A1 wants everything kept and §3.10 wants a
//! credential to stay the operator's, and both are satisfied by keeping a
//! *fingerprint* — the first bytes of the digest of the token — rather than the
//! token. It answers the question a record actually has to answer, which is
//! *was this the same credential as that one*, and answers nothing else.
//! [`Secret`] has no `Display`, its `Debug` shows the fingerprint, and the only
//! way to the bytes is [`Secret::reveal`], named so that every use site reads as
//! a decision.

use std::path::{Path, PathBuf};

use mcf_core::digest::sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

use crate::reference::Reference;

const WHERE: Subsystem = Subsystem::new("mcf-hub::credentials");

/// The environment variables a hub token turns up in.
///
/// Listed so MCF can *tell an operator one is set*. Reading this list is not
/// reading the environment: [`sightings`] is handed a way to look, and the
/// caller that hands it one is the caller that decided to.
///
/// The first two are what the ecosystem's own client reads. The third is not:
/// it is the name people set when they are guessing, and a survey that stayed
/// silent about it would tell somebody with a token sitting right there that
/// there is no credential anywhere — which is worse than useless, because they
/// would believe it. Reporting it costs nothing: a sighting is a report, and
/// nothing here uses what it finds.
pub const KNOWN_VARIABLES: &[&str] = &["HF_TOKEN", "HUGGING_FACE_HUB_TOKEN", "HUGGINGFACE_TOKEN"];

/// A token, held so that it cannot be printed by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    token: String,
    fingerprint: String,
}

impl Secret {
    /// Takes a token.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        let token = token.into();
        let fingerprint = fingerprint_of(&token);
        Self { token, fingerprint }
    }

    /// Enough of the digest to tell one credential from another, and not enough
    /// to be one.
    ///
    /// This is what goes in a record and in a message. It is a digest of the
    /// token, so two records naming the same fingerprint were made with the
    /// same credential — which is the question provenance has to answer — and
    /// nothing in it shortens the work of guessing the token.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The token itself.
    ///
    /// Named to be conspicuous: a call to this is a place where a secret leaves
    /// its box, and there should be few enough of them to read in one sitting.
    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.token
    }
}

/// Shows the fingerprint, never the token.
///
/// `Debug` is what a panic message, a log line and a derived `Debug` on some
/// enclosing struct all reach for, so it is the one that must be safe. There is
/// deliberately no `Display`: a type with no `Display` cannot be interpolated
/// into a message by somebody in a hurry.
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

/// Where a credential came from.
///
/// Part of the conditions of anything acquired with it (§3.4): *the operator
/// typed it* and *it was sitting in the environment* are different provenances
/// for the same bytes, and a record that flattened them would be a record that
/// could not explain a difference between two machines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Handed to MCF directly by whoever is running it.
    Supplied,
    /// Read from an environment variable, because the operator said to.
    Environment {
        /// Which variable.
        variable: String,
    },
    /// Read from a file, because the operator said to.
    File {
        /// Which file.
        path: PathBuf,
    },
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

/// A credential and the story of where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    secret: Secret,
    origin: Origin,
}

impl Credential {
    /// Holds a token that came from somewhere stated.
    #[must_use]
    pub fn new(secret: Secret, origin: Origin) -> Self {
        Self { secret, origin }
    }

    /// The token.
    #[must_use]
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    /// Where it came from.
    #[must_use]
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// What a record says about it: which credential, and how MCF came to have
    /// it. Never the token.
    #[must_use]
    pub fn describe(&self) -> String {
        format!("{} from {}", self.secret.fingerprint(), self.origin)
    }
}

/// A credential MCF can see and has not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    /// Where it is.
    pub origin: Origin,
    /// Which credential it is, as far as anything is ever told.
    pub fingerprint: String,
}

impl Sighting {
    /// What an operator is shown: this exists, here, and MCF has not used it.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "a credential is present in {} ({}); MCF has not used it",
            self.origin, self.fingerprint
        )
    }
}

/// What credentials are lying around, without using any of them.
///
/// `look_up` is how to read an environment variable and `token_file` is a file
/// to check — both supplied by the caller, because a module that went looking
/// on its own would be the silent prerequisite this one exists to refuse. A
/// caller with nothing to offer passes a lookup that always answers `None`.
///
/// Blank values are not sightings: a variable set to the empty string is a
/// variable somebody unset, and reporting it as a credential would send an
/// operator looking for something that is not there.
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

/// Who a source says this caller is.
///
/// The fourth of the questions MCF asks a hub, and the one that makes *the
/// account could not read it* distinguishable from *nobody could*. Both are
/// conditions of an acquisition (§3.4) and neither is inferable from the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// No credential was offered, so the hub is answering the public.
    Anonymous,
    /// A credential was offered and the source has not confirmed who it is.
    ///
    /// The honest state for a source that has no way to ask — offering a token
    /// is not the same as being told it works, and a source that reported an
    /// account it had not been given would be inventing one.
    Offered {
        /// Which credential, as far as anything is ever told.
        fingerprint: String,
    },
    /// The hub said who this is.
    Confirmed {
        /// The account name the hub gave.
        account: String,
        /// Which credential it recognized.
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

/// The three refusals a hub makes about who is asking, written once.
///
/// Written here rather than in each source because B-024's condition is that
/// the outcome *name exactly what is missing*, and three sources phrasing that
/// three ways would be three answers to one question (A6's habit). Every one of
/// them carries the repository, what MCF was to the hub at the time, and the
/// one thing an operator would do next — a refusal that leaves somebody
/// guessing has failed at the only job a refusal has (§3.1).
///
/// What none of them carries is the token. The identity's fingerprint says
/// *which* credential was refused, which is what a second attempt needs to
/// know, and says nothing that would let a reader be that credential.
///
/// This is the one where no credential was offered at all.
///
/// **It names the ambiguity, because the hub will not.**
/// [findings.md](../../../doc/findings.md) F17 measured what a hub says about a
/// repository that does not exist: `401`, with the body *Invalid username or
/// password* — the same answer it gives for a private repository and for one
/// that has been withdrawn. Telling an anonymous caller that a private
/// repository exists would be a leak, so this is deliberate on the hub's part
/// and permanent from MCF's.
///
/// What that costs is the advice. *Supply a credential* is right for a private
/// repository, useless for a withdrawn one and misleading for a typo — offered
/// as though it could work when for two of the three it cannot. So the refusal
/// says what was observed and names the question it is not answering, which is
/// what D33 does for a missing network and D37 for a decayed pin.
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

/// A credential was offered and the hub would not have it.
///
/// Attributed to the user rather than to the machine: an expired or wrong-scope
/// token is a state of the operator's account, and a refusal that blamed the
/// hub would send somebody to look in the wrong place.
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

/// The credential is fine and the terms have not been accepted.
///
/// The distinction B-024 exists for: *the account cannot read this* and *nobody
/// can read this* are different worlds, and the second is not fixed by a better
/// token.
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
