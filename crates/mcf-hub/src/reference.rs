//! What a user means when they name a model, and every way of saying it.
//!
//! §III commits MCF to accepting **any** reference without special-casing, and
//! §6.3 is careful about what that means: a *defined, actionable outcome* for
//! every one, not success for every one (B7). So this is a total function.
//! Every string reaches a named result — a reference MCF can act on, or a
//! refusal that says what was wrong with it — and none of them hangs, panics or
//! leaves anything behind.
//!
//! **The shapes people actually type**, and each is here because it is written
//! in the wild rather than because it is tidy:
//!
//! | Written | Means |
//! |---|---|
//! | `owner/name` | the repository at its default revision |
//! | `owner/name@revision` | a branch, a tag or a commit |
//! | `owner/name:file.gguf` | one file inside it, which is how a quantization is named |
//! | `hf.co/owner/name`, `huggingface.co/owner/name` | the same, as people paste it |
//! | `https://huggingface.co/owner/name` | the same, from a browser |
//! | `https://huggingface.co/owner/name/blob/main/f.gguf` | one file, from a browser |
//! | `https://huggingface.co/owner/name/resolve/main/f.gguf` | the download link itself |
//!
//! **What it refuses, and why refusing is the point.** A reference is a thing
//! MCF will later turn into paths and network requests, so the syntax is the
//! first place a hostile input is stopped: a name containing `..` or a
//! separator would escape a directory, an empty owner would address the hub
//! itself, and a scheme MCF does not fetch over is a request nobody made.
//! Refusing them here means the rest of the acquisition path can be written for
//! references that are already known to be safe to join to a path (§3.7, A14's
//! habit).
//!
//! **It resolves nothing.** Whether the repository exists, whether the revision
//! is real, whether the file is there — all of that needs the network and is
//! B-021's. This is the part that can be total, deterministic and offline, and
//! keeping it separate is what lets the laboratory exercise every branch of it
//! without a hub (D26).

use core::fmt;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-hub::reference");

/// The hosts a reference may name, and the one MCF assumes when none is given.
const KNOWN_HOSTS: [&str; 2] = ["huggingface.co", "hf.co"];

/// What a user named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// Who publishes it.
    pub owner: String,
    /// What it is called.
    pub name: String,
    /// The branch, tag or commit, where one was named.
    ///
    /// `None` is *the repository's default*, which MCF cannot know without
    /// asking the hub — so it stays absent rather than becoming `"main"`, which
    /// is a guess that is wrong for every repository whose default is not
    /// (A7).
    pub revision: Option<String>,
    /// One file inside the repository, where one was named.
    pub file: Option<String>,
}

impl Reference {
    /// `owner/name`, which is how a repository is written everywhere.
    #[must_use]
    pub fn repository(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)?;
        if let Some(revision) = &self.revision {
            write!(f, "@{revision}")?;
        }
        if let Some(file) = &self.file {
            write!(f, ":{file}")?;
        }
        Ok(())
    }
}

/// Reads a reference.
///
/// Total: every input reaches this function's `Result`, and no input reaches a
/// panic, a hang or an allocation proportional to anything but its own length.
///
/// # Errors
///
/// `hub.ref.not_found` when the string does not name a repository at all —
/// which is the reference being wrong rather than the repository being absent,
/// and the two are told apart by *where* the answer came from: this one is
/// decided without asking anybody.
pub fn parse(written: &str) -> Result<Reference> {
    let trimmed = written.trim();
    if trimmed.is_empty() {
        return Err(refused("a reference cannot be empty", written));
    }
    // A reference is a line, and something with a newline in it is two things
    // or a paste that went wrong. Either way MCF is not being asked for one
    // model.
    if trimmed.contains(['\n', '\r']) {
        return Err(refused("a reference is one line", written));
    }

    let (rest, from_url) = strip_host(trimmed)?;
    if from_url {
        return from_hub_url(rest, written);
    }

    // `@` names a revision and `:` names a file. Both are optional; a string
    // with two of either is refused rather than read by a rule nobody would
    // guess — B7's *defined outcome* is a refusal, not a guess.
    let (rest, file) = split_once_exactly(rest, ':', "file", written)?;
    let (rest, revision) = split_once_exactly(rest, '@', "revision", written)?;

    let mut pieces = rest.split('/');
    let (Some(owner), Some(name), None) = (pieces.next(), pieces.next(), pieces.next()) else {
        return Err(refused(
            "a reference names an owner and a repository, separated by one slash",
            written,
        ));
    };

    Ok(Reference {
        owner: owner_component(owner, written)?,
        name: component(name, "repository", written)?,
        revision: match revision {
            Some(revision) => Some(component(&revision, "revision", written)?),
            None => None,
        },
        file: match file {
            Some(file) => Some(path_component(&file, written)?),
            None => None,
        },
    })
}

/// A reference that arrived as a browser URL, where the revision and the file
/// are written into the path instead of with `@` and `:`.
///
/// Two spellings of one thing, and neither is the general case — so each is
/// read where it is written rather than rewritten into the other.
fn from_hub_url(rest: &str, written: &str) -> Result<Reference> {
    let pieces: Vec<&str> = rest.split('/').filter(|piece| !piece.is_empty()).collect();
    match pieces.as_slice() {
        [owner, name] => Ok(Reference {
            owner: owner_component(owner, written)?,
            name: component(name, "repository", written)?,
            revision: None,
            file: None,
        }),
        [owner, name, kind, revision, file @ ..] => {
            if *kind != "blob" && *kind != "resolve" {
                return Err(refused(
                    "a hub URL addresses a file through /blob/ or /resolve/, and this is neither",
                    kind,
                ));
            }
            if file.is_empty() {
                return Err(refused("a hub URL names a file after its revision", rest));
            }
            Ok(Reference {
                owner: owner_component(owner, written)?,
                name: component(name, "repository", written)?,
                revision: Some(component(revision, "revision", written)?),
                file: Some(path_component(&file.join("/"), written)?),
            })
        }
        _ => Err(refused("a hub URL names an owner and a repository", rest)),
    }
}

/// Removes a scheme and a host, and says whether there was one.
///
/// A scheme MCF does not fetch over is refused here rather than later: `file://`
/// and `ftp://` are requests nobody made, and a hostname that is not the hub is
/// a reference to somewhere MCF does not go (A17's habit — nothing leaves the
/// machine for a destination the user did not choose).
fn strip_host(written: &str) -> Result<(&str, bool)> {
    let without_scheme = match written.split_once("://") {
        Some(("https" | "http", rest)) => rest,
        Some((scheme, _)) => {
            return Err(refused(
                "MCF fetches over https, and this names another scheme",
                scheme,
            ));
        }
        None => written,
    };

    for host in KNOWN_HOSTS {
        if let Some(rest) = without_scheme.strip_prefix(host)
            && let Some(rest) = rest.strip_prefix('/')
        {
            return Ok((rest, true));
        }
    }

    // No host at all is the ordinary case: `owner/name`. A host that is not the
    // hub is refused, and the test for "is a host" is that the first component
    // looks like one — a dot in it, which an owner may not have.
    let first = without_scheme.split('/').next().unwrap_or("");
    if first.contains('.') {
        return Err(refused(
            "MCF resolves references on the Hugging Face hub, and this names another host",
            first,
        ));
    }
    Ok((without_scheme, false))
}

/// Splits on a separator that must appear at most once.
///
/// Two `@` in a reference is not a revision containing an `@`; it is a string
/// MCF would have to guess about, and B7's *defined outcome* is a refusal
/// rather than a guess.
fn split_once_exactly<'a>(
    rest: &'a str,
    separator: char,
    what: &str,
    written: &str,
) -> Result<(&'a str, Option<String>)> {
    let count = rest.matches(separator).count();
    if count == 0 {
        return Ok((rest, None));
    }
    if count > 1 {
        return Err(refused(
            &format!("a reference names at most one {what}"),
            written,
        ));
    }
    let (before, after) = rest.split_once(separator).unwrap_or((rest, ""));
    Ok((before, Some(after.to_owned())))
}

/// An owner: a component, and additionally one with no dot in it.
///
/// A dot is what tells a host from an owner — `hf.co/owner/name` and
/// `owner/name` differ in exactly that — and Hugging Face owners are letters,
/// digits and hyphens. The fuzz tier found the consequence of leaving it
/// implicit: `hf.co/name.co/repo` was accepted with `name.co` as the owner and
/// rendered as `name.co/repo`, which reads back as a reference to another host.
/// Refusing it here makes the heuristic exact rather than approximate.
fn owner_component(piece: &str, written: &str) -> Result<String> {
    let owner = component(piece, "owner", written)?;
    if owner.contains('.') {
        return Err(refused(
            "an owner cannot contain a dot, which is what distinguishes one from a host",
            &owner,
        ));
    }
    Ok(owner)
}

/// An owner, a repository name or a revision: non-empty, no separators, and
/// nothing that would escape a directory when this is joined to a path.
fn component(piece: &str, what: &str, written: &str) -> Result<String> {
    if piece.is_empty() {
        return Err(refused(&format!("a reference names no {what}"), written));
    }
    if piece == "." || piece == ".." || piece.contains('/') || piece.contains('\\') {
        return Err(refused(
            &format!("a {what} that would escape a directory is not one"),
            piece,
        ));
    }
    if piece.contains(|character: char| character.is_control()) {
        return Err(refused(
            &format!("a {what} cannot contain a control character"),
            written,
        ));
    }
    if piece.contains([':', '@']) {
        return Err(refused(
            &format!("a {what} cannot contain ':' or '@'"),
            piece,
        ));
    }
    Ok(piece.to_owned())
}

/// A file inside a repository, which may contain slashes and may not climb out.
fn path_component(piece: &str, written: &str) -> Result<String> {
    if piece.is_empty() {
        return Err(refused("a reference names no file", written));
    }
    if piece.starts_with('/') || piece.contains('\\') {
        return Err(refused(
            "a file inside a repository is a relative path",
            piece,
        ));
    }
    for part in piece.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(refused(
                "a file that would escape the repository is not one",
                piece,
            ));
        }
        if part.contains(|character: char| character.is_control()) {
            return Err(refused(
                "a file name cannot contain a control character",
                written,
            ));
        }
    }
    // The written form separates a revision with `@` and a file with `:`, so a
    // file whose name contains either cannot be written back unambiguously.
    // The fuzz tier found this by accepting such a name out of a URL and then
    // failing to re-read what it had rendered: a reference MCF cannot write is
    // one it cannot record, and C5's habit is that a written identifier is one
    // somebody can type again.
    if piece.contains([':', '@']) {
        return Err(refused(
            "a file name containing ':' or '@' cannot be written as a reference",
            piece,
        ));
    }
    Ok(piece.to_owned())
}

fn refused(detail: &str, found: &str) -> Failure {
    Failure::new(
        Category::HubRefNotFound,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("reference", found.to_owned())
}

#[cfg(test)]
mod tests;
