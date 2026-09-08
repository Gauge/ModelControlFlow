use core::fmt;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-hub::reference");

const KNOWN_HOSTS: [&str; 2] = ["huggingface.co", "hf.co"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub owner: String,
    pub name: String,
    pub revision: Option<String>,
    pub file: Option<String>,
}

impl Reference {
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

pub fn parse(written: &str) -> Result<Reference> {
    let trimmed = written.trim();
    if trimmed.is_empty() {
        return Err(refused("a reference cannot be empty", written));
    }
    if trimmed.contains(['\n', '\r']) {
        return Err(refused("a reference is one line", written));
    }

    let (rest, from_url) = strip_host(trimmed)?;
    if from_url {
        return from_hub_url(rest, written);
    }

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

    let first = without_scheme.split('/').next().unwrap_or("");
    if first.contains('.') {
        return Err(refused(
            "MCF resolves references on the Hugging Face hub, and this names another host",
            first,
        ));
    }
    Ok((without_scheme, false))
}

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
