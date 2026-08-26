//! Getting a file onto this machine without ever leaving something that looks
//! like it succeeded (B-021, §III, §3.7).
//!
//! **The failure this module exists to prevent** is a half-downloaded model
//! sitting at the path a whole one would occupy. Everything downstream — a
//! checksum re-verification (B-301), a fitment plan, an engine loading weights —
//! reads that path and has no way to know. So the rule is structural rather
//! than careful: **bytes accumulate in a `.partial` file and the artifact's own
//! name is only ever given to something that has been verified.** A crash at
//! any moment leaves a partial file, which is a state MCF can see, resume and
//! name (A4, A27).
//!
//! **Resumption is an optimization; verification is not.** A transfer that
//! stops at ninety per cent continues from ninety per cent where the source
//! supports it and starts again where it does not — and either way the finished
//! file is checked before it is named. A source that cannot resume is a fact
//! about the source that gets recorded, not a failure of the acquisition.
//!
//! **A file that changed under the transfer is a classified failure, not a
//! corrupt artifact.** Resuming assumes the bytes already held are a prefix of
//! the bytes still coming, and a repository that repointed a tag between the
//! first attempt and the second breaks that assumption silently. The digest
//! catches it at the end; what MCF must not do is keep the mixture, because
//! half of one file and half of another is the one thing worse than no file.
//!
//! **What MCF verifies against, and what it does when it cannot.** A hub that
//! declares a digest can be checked against; a hub that declares none cannot,
//! and the artifact is then held as *unverified* rather than as verified by
//! omission (A7, A21). That state travels with it: an unverified artifact is a
//! condition of everything measured on it, not a footnote.

use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::integrity::checksum_of;

use crate::inspect;
use crate::reference::Reference;
use crate::source::{Entry, Source};

const WHERE: Subsystem = Subsystem::new("mcf-hub::fetch");

/// How many times a transfer is continued before MCF stops trying.
///
/// A bound rather than a policy: §III's *actionable outcome* for a source that
/// keeps stopping is "it kept stopping, here is how far it got", and a fetcher
/// that retried for ever would never produce it. Five is enough to carry a
/// transfer across a handful of interruptions and few enough to end.
pub const ATTEMPTS: usize = 5;

/// What holding an artifact was established by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    /// The hub declared a digest and what arrived matches it.
    Digest {
        /// The digest, which is both what was declared and what was computed.
        digest: String,
    },
    /// The hub declared no digest, so the file's length is all that could be
    /// checked. The artifact is *held*, and it is not verified.
    ///
    /// A distinct state rather than a weaker kind of success: A21 keeps
    /// declared, verified and unknown apart, and an artifact nobody could check
    /// is a condition of every measurement taken on it.
    LengthOnly {
        /// The digest MCF computed, which nothing was available to compare
        /// against. It is what a later re-verification will use (B-301).
        digest: String,
    },
}

/// A file that is now on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acquired {
    /// Where it is.
    pub path: PathBuf,
    /// How many bytes.
    pub bytes: u64,
    /// What its being there was established by.
    pub verification: Verification,
    /// How many transfers it took, counting the first.
    ///
    /// One is the ordinary case. More is a fact about the source worth keeping:
    /// a hub that stops halfway three times is a condition of the acquisition
    /// even when the result is identical.
    pub attempts: usize,
    /// Whether the source could continue rather than start again.
    pub resumed: bool,
}

/// Fetches a file, resuming where possible, and names it only once it is
/// verified.
///
/// # Errors
///
/// `resource.disk.exhausted` when the filesystem the artifact would land on has
/// less room than the hub says the file needs — refused *before* the transfer,
/// because §3.11 makes a full disk a decision rather than a surprise and
/// finding out at ninety per cent is the surprise. Where MCF cannot read the
/// room, it says nothing and proceeds: an unknown is not a refusal (A7), and
/// the write itself classifies exhaustion if it happens anyway.
/// `artifact.incomplete` when the transfer never finished within [`ATTEMPTS`],
/// naming how far it got; `artifact.corrupt` when what arrived does not match
/// the digest the hub declared — which is either damage or a file that changed
/// under the transfer, and both are refusals; `hub.metadata.deceptive` when
/// more bytes arrived than were promised; whatever the source failed with
/// otherwise.
pub fn acquire(
    source: &dyn Source,
    reference: &Reference,
    entry: &Entry,
    into: &Path,
) -> Result<Acquired> {
    // Already held, and still what it should be. A re-acquisition that
    // re-downloads a verified artifact is bandwidth spent to learn nothing.
    if into.exists() {
        let held = checksum_of(into)?;
        if let Some(declared) = entry.digest.as_deref()
            && held.hex() == declared
        {
            return Ok(Acquired {
                path: into.to_path_buf(),
                bytes: size_of(into)?,
                verification: Verification::Digest {
                    digest: held.hex().to_owned(),
                },
                attempts: 0,
                resumed: false,
            });
        }
    }

    there_is_room_for(entry, into)?;

    let partial = partial_path(into);
    let mut attempts = 0;
    let mut resumed = false;

    while attempts < ATTEMPTS {
        attempts += 1;
        let held = if partial.exists() {
            size_of(&partial)?
        } else {
            0
        };

        // What one attempt delivered is not what decides anything — the file on
        // the disk is. A source that reports more than it wrote, or less, is
        // exactly the kind of claim §3.7 says not to take on trust.
        if held == 0 {
            let _delivered = source.fetch(reference, entry, &partial)?;
        } else {
            match source.fetch_from(reference, entry, held, &partial) {
                Ok(_delivered) => {
                    resumed = true;
                }
                // A source that cannot continue is a fact about the source.
                // Starting again is the honest response — and the bytes already
                // held are discarded rather than kept, because a fetch that
                // began at zero will write them again.
                Err(failure) if failure.category() == Category::HubUnreachable => {
                    let _discarded = std::fs::remove_file(&partial);
                    let _delivered = source.fetch(reference, entry, &partial)?;
                }
                Err(failure) => return Err(failure),
            }
        }

        let arrived = size_of(&partial)?;
        if let Err(failure) = inspect::arrived_as_promised(entry, arrived)
            && failure.category() != Category::ArtifactIncomplete
        {
            // More than promised: the listing MCF planned against was wrong,
            // and nothing here should be kept (§3.11 — a full disk is a
            // decision, not a surprise). A *short* transfer is the other branch
            // and is not a failure yet: it is what the next attempt continues.
            let _discarded = std::fs::remove_file(&partial);
            return Err(failure);
        }
        if arrived < entry.size {
            // Short. Go round again; the partial file stays, which is what
            // makes the next attempt a continuation.
            continue;
        }

        return finish(&partial, into, entry, attempts, resumed);
    }

    let reached = if partial.exists() {
        size_of(&partial)?
    } else {
        0
    };
    Err(Failure::new(
        Category::ArtifactIncomplete,
        Attribution::Machine,
        Disposition::Partial,
        WHERE,
        "the transfer did not finish, and this is how far it got",
    )
    .with_context("file", entry.path.clone())
    .with_context("promised", entry.size.to_string())
    .with_context("reached", reached.to_string())
    .with_context("attempts", attempts.to_string())
    .with_context("partial_file", partial.display().to_string()))
}

/// Verifies a completed transfer and gives it the artifact's name.
///
/// The rename is the last thing that happens and it happens only here, which is
/// what makes the artifact's own path mean *verified* rather than *present*.
fn finish(
    partial: &Path,
    into: &Path,
    entry: &Entry,
    attempts: usize,
    resumed: bool,
) -> Result<Acquired> {
    let computed = checksum_of(partial)?;

    let verification = match entry.digest.as_deref() {
        Some(declared) if declared == computed.hex() => Verification::Digest {
            digest: computed.hex().to_owned(),
        },
        Some(declared) => {
            // Damage, or a file that changed under the transfer. Either way the
            // bytes held are not a prefix of anything MCF wants: a mixture of
            // two files would resume into a plausible, wrong artifact, so it
            // goes rather than being kept as evidence nobody can use.
            let _discarded = std::fs::remove_file(partial);
            return Err(Failure::new(
                Category::ArtifactCorrupt,
                Attribution::Artifact,
                Disposition::Refused,
                WHERE,
                "what arrived does not match the digest the repository declared — \
                 the transfer was damaged, or the file changed under it",
            )
            .with_context("file", entry.path.clone())
            .with_context("declared", declared.to_owned())
            .with_context("computed", computed.hex().to_owned())
            .with_context("attempts", attempts.to_string())
            .with_context("resumed", resumed.to_string()));
        }
        None => Verification::LengthOnly {
            digest: computed.hex().to_owned(),
        },
    };

    std::fs::rename(partial, into).map_err(|error| {
        Failure::new(
            Category::ArtifactUnreadable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the verified file could not be given the artifact's name",
        )
        .with_context("from", partial.display().to_string())
        .with_context("to", into.display().to_string())
        .with_context("os_error", error.to_string())
    })?;

    Ok(Acquired {
        path: into.to_path_buf(),
        bytes: size_of(into)?,
        verification,
        attempts,
        resumed,
    })
}

/// Whether the filesystem this would land on has room for it.
///
/// The hub's declared size against what the kernel says is available, both
/// stated in the refusal so that an operator can see the arithmetic rather than
/// be told a verdict (A6). What is *already* on the disk under the partial name
/// counts as room, because a resumed transfer needs only the rest of it.
///
/// # Errors
///
/// `resource.disk.exhausted`, naming both numbers and the filesystem. Nothing
/// when MCF cannot read the room: `Unknown` is not `no` (A7).
fn there_is_room_for(entry: &Entry, into: &Path) -> Result<()> {
    let directory = into.parent().unwrap_or_else(|| Path::new("."));
    let Attested::Known(space) = mcf_core::hardware::space_on(directory) else {
        return Ok(());
    };
    let already = size_of(&partial_path(into)).unwrap_or(0);
    let wanted = entry.size.saturating_sub(already);
    if wanted <= space.available.0 {
        return Ok(());
    }

    Err(Failure::new(
        Category::ResourceDiskExhausted,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "the filesystem this would be written to has less room than the file needs",
    )
    .with_context("file", entry.path.clone())
    .with_context("needs_bytes", wanted.to_string())
    .with_context("available_bytes", space.available.0.to_string())
    .with_context(
        "short_by_bytes",
        wanted.saturating_sub(space.available.0).to_string(),
    )
    .with_context("where", directory.display().to_string())
    .with_context(
        "storage",
        match mcf_core::hardware::storage_of(directory) {
            Attested::Known(storage) => storage.to_string(),
            Attested::Unknown => "a filesystem MCF could not name".to_owned(),
        },
    )
    .with_context(
        "what_to_do",
        "this is refused before the transfer rather than discovered during it (§3.11): free \
         the room, or acquire somewhere else",
    ))
}

/// Where a transfer accumulates before it has earned the artifact's name.
#[must_use]
pub fn partial_path(into: &Path) -> PathBuf {
    let mut name = into.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    into.with_file_name(name)
}

fn size_of(path: &Path) -> Result<u64> {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|error| {
            Failure::new(
                Category::ArtifactUnreadable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "a file MCF is holding could not be measured",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string())
        })
}

#[cfg(test)]
mod tests;
