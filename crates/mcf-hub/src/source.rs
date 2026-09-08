use std::path::Path;

use mcf_core::failure::Result;

use crate::credentials::Identity;
use crate::reference::Reference;

fn is_read_here(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
}

#[must_use]
pub fn without_the_part(path: &str) -> String {
    let Some(stem) = path.strip_suffix(".gguf") else {
        return path.to_owned();
    };
    let Some((before, of)) = stem.rsplit_once("-of-") else {
        return path.to_owned();
    };
    let Some((prefix, at)) = before.rsplit_once('-') else {
        return path.to_owned();
    };
    if at.is_empty()
        || of.is_empty()
        || !at.chars().all(|c| c.is_ascii_digit())
        || !of.chars().all(|c| c.is_ascii_digit())
    {
        return path.to_owned();
    }
    format!("{prefix}.gguf")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub first: String,
    pub name: String,
    pub bytes: u64,
    pub parts: u32,
    pub whole: bool,
    pub digested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set<'a> {
    pub parts: Vec<&'a Entry>,
    pub of: u32,
}

impl Set<'_> {
    #[must_use]
    pub fn is_whole(&self) -> bool {
        usize::try_from(self.of).is_ok_and(|of| of == self.parts.len() && of > 0)
    }

    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        self.parts
            .iter()
            .try_fold(0_u64, |sum, part| sum.checked_add(part.size))
    }
}

fn part_name(path: &str) -> Option<(String, u32, u32)> {
    let (directory, name) = path.rsplit_once('/').unwrap_or(("", path));
    let stem = name.strip_suffix(".gguf")?;
    let (before, of) = stem.rsplit_once("-of-")?;
    let (prefix, at) = before.rsplit_once('-')?;
    if of.is_empty() || at.is_empty() || prefix.is_empty() {
        return None;
    }
    if !of.chars().all(|c| c.is_ascii_digit()) || !at.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (at, of): (u32, u32) = (at.parse().ok()?, of.parse().ok()?);
    (at >= 1 && at <= of).then(|| (format!("{directory}/{prefix}"), at, of))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    pub digest: Option<String>,
}

impl Entry {
    #[must_use]
    pub fn new(path: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            size,
            digest: None,
        }
    }

    #[must_use]
    pub fn declaring(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub reference: Reference,
    pub revision: Option<String>,
    pub entries: Vec<Entry>,
    pub gated: Option<String>,
    pub declared_licence: Option<String>,
    pub lineage: Option<Lineage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    pub base: String,
    pub relation: Option<String>,
}

impl Listing {
    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    #[must_use]
    pub fn entries_ending(&self, suffix: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.path.ends_with(suffix))
            .collect()
    }

    #[must_use]
    pub fn parts_of(&self, path: &str) -> Option<Set<'_>> {
        let (prefix, _, of) = part_name(path)?;
        let mut parts: Vec<(u32, &Entry)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let (other, at, count) = part_name(&entry.path)?;
                (other == prefix && count == of).then_some((at, entry))
            })
            .collect();
        parts.sort_by_key(|(at, _)| *at);
        parts.dedup_by_key(|(at, _)| *at);
        Some(Set {
            parts: parts.into_iter().map(|(_, entry)| entry).collect(),
            of,
        })
    }

    #[must_use]
    pub fn variants(&self) -> Vec<Variant> {
        let mut found: Vec<Variant> = Vec::new();
        for entry in self
            .entries
            .iter()
            .filter(|entry| is_read_here(&entry.path))
        {
            let Some(set) = self.parts_of(&entry.path) else {
                found.push(Variant {
                    first: entry.path.clone(),
                    name: entry.path.clone(),
                    bytes: entry.size,
                    parts: 1,
                    whole: true,
                    digested: entry.digest.is_some(),
                });
                continue;
            };
            let first = set
                .parts
                .first()
                .map_or_else(|| entry.path.clone(), |part| part.path.clone());
            if found.iter().any(|held| held.first == first) {
                continue;
            }
            found.push(Variant {
                name: without_the_part(&first),
                bytes: set.bytes().unwrap_or(0),
                parts: u32::try_from(set.parts.len()).unwrap_or(u32::MAX),
                whole: set.is_whole(),
                digested: set.parts.iter().all(|part| part.digest.is_some()),
                first,
            });
        }
        found
    }

    #[must_use]
    pub fn total_bytes(&self) -> Option<u64> {
        self.entries
            .iter()
            .try_fold(0_u64, |total, entry| total.checked_add(entry.size))
    }
}

pub trait Source {
    fn describe(&self) -> String;

    fn identity(&self) -> Identity {
        Identity::Anonymous
    }

    fn list(&self, reference: &Reference) -> Result<Listing>;

    fn fetch(&self, reference: &Reference, entry: &Entry, into: &Path) -> Result<Fetched>;

    fn fetch_from(
        &self,
        _reference: &Reference,
        _entry: &Entry,
        _from: u64,
        _into: &Path,
    ) -> Result<Fetched> {
        Err(mcf_core::failure::Failure::new(
            mcf_core::failure::Category::HubUnreachable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-hub::source"),
            "this source cannot continue a transfer from an offset",
        )
        .with_context("source", self.describe()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub bytes: u64,
    pub digest: String,
}

#[cfg(test)]
mod tests;
