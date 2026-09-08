use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::integrity::checksum_of;

use crate::inspect;
use crate::reference::Reference;
use crate::source::{Entry, Source};

const WHERE: Subsystem = Subsystem::new("mcf-hub::fetch");

pub const ATTEMPTS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    Digest { digest: String },
    LengthOnly { digest: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acquired {
    pub path: PathBuf,
    pub bytes: u64,
    pub verification: Verification,
    pub attempts: usize,
    pub resumed: bool,
}

pub fn acquire(
    source: &dyn Source,
    reference: &Reference,
    entry: &Entry,
    into: &Path,
) -> Result<Acquired> {
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

        if held == 0 {
            let _delivered = source.fetch(reference, entry, &partial)?;
        } else {
            match source.fetch_from(reference, entry, held, &partial) {
                Ok(_delivered) => {
                    resumed = true;
                }
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
            let _discarded = std::fs::remove_file(&partial);
            return Err(failure);
        }
        if arrived < entry.size {
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
