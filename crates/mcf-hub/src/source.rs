//! Where a repository's contents come from, as an interface rather than as a
//! network client.
//!
//! **Why this exists before the network does.** B-028 asks for a complete,
//! deterministic simulated hub so that *every M1 test runs against it with no
//! network* — and a simulation is only possible if the thing being simulated
//! has a boundary. This is that boundary: the small set of questions MCF asks a
//! hub, stated as a trait, so the acquisition path is written once and exercised
//! against a laboratory's hub that behaves badly on purpose (D26, B19).
//!
//! **It is deliberately small.** Four questions — what does this repository
//! publish, what are its terms, give me this file, and who am I to you — and
//! each of them is something MCF already needs an answer to. A wider interface
//! would be a wider surface for a hostile hub to reach through (§3.7), and
//! §3.13 refuses generality nobody asked for.
//!
//! **What a source promises, and what it does not.** It promises to answer or
//! to fail in a classified way (A2): every method returns MCF's own failure
//! type, and there is no path through this interface that hangs or panics.
//! It promises nothing about *when* — a real hub is slow, throttled and
//! occasionally absent, and B7 makes each of those a defined outcome rather
//! than an exception.
//!
//! **Nothing here fetches yet.** [`Source::fetch`] exists so that the fake hub
//! can serve bytes to the tests that need them; the real client, resumption and
//! integrity checking are B-021, and the vendoring decision that a network
//! stack needs has not been made.

use std::path::Path;

use mcf_core::failure::Result;

use crate::credentials::Identity;
use crate::reference::Reference;

/// Whether MCF reads this format at all.
///
/// Case-insensitively, because a repository's file names are its own:
/// `.GGUF` is the same format and leaving it out would drop a variant from
/// the list without saying so (A1).
fn is_read_here(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
}

/// A published path with its `-00001-of-00004` taken off, so that the
/// parts of one quantization read as the one thing they are (B-597).
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

/// One thing a repository publishes that a person chooses between: a
/// quantization, whether it is one file or several (B-597).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The file to ask for. Where the variant is a set this is its first
    /// part, and asking for it fetches the whole (B-590).
    pub first: String,
    /// What to call it: the path with the part suffix taken off.
    pub name: String,
    /// What the whole variant weighs, in bytes.
    pub bytes: u64,
    /// How many files it is published as; one for an ordinary file.
    pub parts: u32,
    /// Whether every part the names declare is published. A variant that
    /// is not whole is one no engine can load, and a surface says so
    /// rather than offering it (A7).
    pub whole: bool,
    /// Whether the hub declares a digest for every file of it. A part
    /// nobody can check makes the whole variant unverifiable, and that is
    /// a condition of every measurement taken on it (A21).
    pub digested: bool,
}

/// The parts of a model published in several files (B-590).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set<'a> {
    /// The parts the repository publishes, in order.
    pub parts: Vec<&'a Entry>,
    /// How many the name says there are.
    pub of: u32,
}

impl Set<'_> {
    /// Whether every part the name declares is published.
    #[must_use]
    pub fn is_whole(&self) -> bool {
        usize::try_from(self.of).is_ok_and(|of| of == self.parts.len() && of > 0)
    }

    /// What the whole set is said to weigh, or `None` on an overflow.
    #[must_use]
    pub fn bytes(&self) -> Option<u64> {
        self.parts
            .iter()
            .try_fold(0_u64, |sum, part| sum.checked_add(part.size))
    }
}

/// A published path's set, if its name is `<prefix>-<at>-of-<of>.gguf` in
/// the shape the reference implementation reads: the directory and prefix
/// together, which part, and of how many.
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

/// One file a repository publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its path within the repository.
    pub path: String,
    /// How many bytes the hub says it is.
    ///
    /// The hub's claim, not a measurement: B-213 plans against it and B-021
    /// checks it, and the difference between the two is a finding rather than
    /// an error (A21's shape — declared and verified are different states).
    pub size: u64,
    /// The digest the hub declares, where it declares one.
    ///
    /// Also a claim. A hub that states a digest lets MCF verify what arrived
    /// against what was promised; a hub that states none leaves the artifact
    /// *unverified*, which is a state to record rather than a reason to trust
    /// it (A7, A21). It is never a substitute for the digest MCF computes,
    /// which is of what actually landed on this disk.
    pub digest: Option<String>,
}

impl Entry {
    /// An entry with no declared digest, which is the common case on a hub that
    /// publishes plain files.
    #[must_use]
    pub fn new(path: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            size,
            digest: None,
        }
    }

    /// The same, with the digest the hub declares.
    #[must_use]
    pub fn declaring(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }
}

/// What a repository publishes, and what MCF is allowed to do with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The reference this describes, with the revision resolved where the
    /// source could resolve it.
    pub reference: Reference,
    /// The revision the listing is of.
    ///
    /// A source that could not say leaves it absent rather than repeating what
    /// was asked for (A7): "the default branch, whatever it is" is not a
    /// revision anybody can pin.
    pub revision: Option<String>,
    /// Every file, in the order the source listed them.
    pub entries: Vec<Entry>,
    /// How the repository is gated, in the hub's own word, where it is gated
    /// at all.
    ///
    /// `None` means the source said nothing about a gate, which is not the same
    /// as *ungated* — a source that does not publish the field cannot be read
    /// as publishing `false` (A7). B-331 compares it over time: a repository
    /// that is gated now and was not when MCF acquired from it is one of the
    /// four decays §7.38 names, and the only one a hub announces before it
    /// bites ([findings.md](../../../doc/findings.md) F17).
    pub gated: Option<String>,
    /// The licence the repository declares, where it declares one.
    ///
    /// Declared, never verified: A21 keeps those apart, and B-023 is where the
    /// difference is surfaced to a user before they use an artifact.
    pub declared_licence: Option<String>,
    /// What the repository says these weights were made from.
    ///
    /// §XII's hard case: a GGUF conversion of somebody else's weights, where
    /// the interesting provenance is the *other* repository. A publisher who
    /// says so is telling MCF something it could not otherwise know, and a
    /// publisher who does not leaves this absent rather than unlinked-and-
    /// assumed-original (A7, B-019).
    pub lineage: Option<Lineage>,
}

/// What a repository says its weights were made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// The repository the weights came from, as the publisher wrote it.
    pub base: String,
    /// What was done to them, in the publisher's own word — `quantized`,
    /// `finetune`, `merge`, `adapter`.
    ///
    /// `None` where the publisher named a base and not a relation, which is a
    /// thing they do: the link is still worth having, and inventing a
    /// transformation for it would be MCF saying what happened (A7).
    pub relation: Option<String>,
}

impl Listing {
    /// The entry with this path, if the repository publishes one.
    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Every entry whose path ends with this suffix, which is how a
    /// quantization is chosen from a repository that publishes twenty.
    #[must_use]
    pub fn entries_ending(&self, suffix: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.path.ends_with(suffix))
            .collect()
    }

    /// The set a published part belongs to: every part of it, in the order
    /// the engine loads them, and how many the name says there are.
    ///
    /// **A model published in parts is one model** (B-590). `<name>-00001-of-00003.gguf`
    /// is the first of three files the reference implementation loads as one,
    /// from exactly this pattern in the name; asking for any part is asking
    /// for the model, and a store holding one part of three holds nothing
    /// an engine can load. `None` for a file that is not a part. The parts
    /// found may be fewer than the name declares, which a caller refuses
    /// rather than fetches — a repository missing a part publishes no model.
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

    /// What a person choosing from this repository is choosing between:
    /// one entry a quantization, however many files it is published as
    /// (B-597).
    ///
    /// **A model in four files is one thing to choose, not four.** A
    /// repository that publishes a 160-gigabyte variant as four parts
    /// listed four rows of forty gigabytes each, every one of them with
    /// its own button, none of them a model: an engine loads the set or
    /// nothing (B-590), so the set is what there is to want. Each variant
    /// names the file to ask for — the first part, which fetches the whole
    /// — and carries what the whole weighs.
    ///
    /// Only what MCF reads: a repository's `.gguf` files, in the order it
    /// lists them, a set appearing where its first part does.
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

    /// What the whole repository would cost to hold.
    ///
    /// `None` on an overflow rather than a wrapped total: a listing whose sizes
    /// do not add up is a listing MCF will not plan against (B-213).
    #[must_use]
    pub fn total_bytes(&self) -> Option<u64> {
        self.entries
            .iter()
            .try_fold(0_u64, |total, entry| total.checked_add(entry.size))
    }
}

/// What MCF asks a hub.
///
/// Implemented by the real client (B-021) and by the laboratory's simulated hub
/// (B-028). A caller written against this is a caller the laboratory can drive
/// through every failure in the `hub.*` taxonomy without a network.
pub trait Source {
    /// What this source is, for a record and for a message.
    ///
    /// Part of a measurement's conditions the moment an artifact acquired
    /// through it is measured (§3.4), and the difference between *the hub* and
    /// *a mirror somebody stood up* when a result is questioned.
    fn describe(&self) -> String;

    /// Who this caller is to this source.
    ///
    /// The fourth question, and the one that makes *this account cannot read
    /// it* distinguishable from *nobody can*. The default is
    /// [`Identity::Anonymous`], which is the truth for a source holding no
    /// credential; a source that holds one says so here, and a source that has
    /// been told an account name says that instead of guessing at one.
    ///
    /// It is a condition of anything acquired through this source (§3.4), so it
    /// is a question with an answer rather than a flag somewhere in a client.
    fn identity(&self) -> Identity {
        Identity::Anonymous
    }

    /// What a repository publishes.
    ///
    /// # Errors
    ///
    /// Any `hub.*` failure: the reference names nothing, the repository is
    /// gated, the account is throttled, the hub is unreachable.
    fn list(&self, reference: &Reference) -> Result<Listing>;

    /// Puts one file where the caller asked for it.
    ///
    /// The caller supplies the destination, so a source never decides where
    /// anything on this machine goes — §3.10's habit, and the reason a hostile
    /// source cannot choose a path.
    ///
    /// # Errors
    ///
    /// Any `hub.*` failure, and `artifact.incomplete` where the transfer ended
    /// early. What it must never do is leave a partial file looking like a
    /// whole one, which is B-021's condition and the thing the laboratory's hub
    /// exists to attempt.
    fn fetch(&self, reference: &Reference, entry: &Entry, into: &Path) -> Result<Fetched>;

    /// The same, continuing from a byte offset, appending to what is there.
    ///
    /// This is what makes a transfer resumable, and not every source can do it:
    /// a hub without range requests is a real thing, and the honest answer is
    /// to say so rather than to silently start again and report progress that
    /// did not happen. The default is exactly that answer.
    ///
    /// # Errors
    ///
    /// `hub.unreachable` by default — meaning *this source does not resume*,
    /// which a caller turns into a restart it records rather than into a
    /// failure of the acquisition.
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

/// What a completed fetch produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// How many bytes arrived.
    pub bytes: u64,
    /// The digest of what arrived, computed while it arrived.
    ///
    /// Computed by the *fetcher* rather than reported by the source: a checksum
    /// a hostile source supplies is a checksum of what it wishes it had sent
    /// (§3.7, B-021).
    pub digest: String,
}

#[cfg(test)]
mod tests;
