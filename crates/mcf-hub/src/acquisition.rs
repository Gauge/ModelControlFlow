//! Acquiring one published file, and writing down where it came from.
//!
//! **One path, two surfaces.** The command line fetches a model and so does
//! the daemon, on behalf of the window. An acquisition that recorded its
//! provenance differently depending on which asked for it would make the
//! record a fact about the surface rather than about the model, which is the
//! opposite of what a record is for (A24, B-072).
//!
//! **The rendering is not here.** What comes back is what happened; how a
//! terminal prints it and how a window draws it are each their own.

use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::provenance::{
    Checksum, Origin, Provenance, Repository, Revision, Transformation, TransformationKind,
};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

use crate::client::Hub;
use crate::fetch::{Acquired, Verification, acquire};
use crate::reference::Reference;
use crate::source::{Entry, Listing, Source as _};
use crate::store;

/// What an acquisition left behind.
#[derive(Debug)]
pub struct Done {
    /// The file, and how its arrival was established.
    pub acquired: Acquired,
    /// Where the provenance was written beside it.
    pub sidecar: PathBuf,
    /// What MCF knows about where it came from.
    pub provenance: Provenance,
    /// Where the journal entry went, or why it could not be written.
    pub recorded: Result<PathBuf, Failure>,
}

/// Fetches one published file and writes down everything about it.
///
/// # Errors
///
/// What the transfer said, or what writing the provenance beside it said. A
/// model MCF is holding and cannot account for is worse than one it does not
/// hold, so a sidecar that will not write is a failure and not a warning
/// (A24, §3.11).
pub fn one(hub: &Hub, listing: &Listing, entry: &Entry, root: &Path) -> Result<Done, Failure> {
    let into = destination(root, &listing.reference, &entry.path);
    if let Some(parent) = into.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            Failure::new(
                Category::ArtifactUnreadable,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-hub::acquisition"),
                "the store's directory could not be made",
            )
            .with_context("directory", parent.display().to_string())
            .with_context("said", error.to_string())
        })?;
    }
    let acquired = acquire(hub, &listing.reference, entry, &into)?;
    let at = Timestamp::now();
    let provenance = provenance_of(listing, &acquired, at);
    let sidecar = store::record_provenance(&into, &provenance)?;
    let recorded = record(hub, listing, entry, &acquired, at);
    Ok(Done {
        acquired,
        sidecar,
        provenance,
        recorded,
    })
}

/// Where the bytes are while they are arriving, so that something watching can
/// say how far along a transfer is without the transfer having to report it.
#[must_use]
pub fn arriving_at(root: &Path, listing: &Listing, entry: &Entry) -> PathBuf {
    crate::fetch::partial_path(&destination(root, &listing.reference, &entry.path))
}

/// Where an artifact lives: under the store, by the repository that published
/// it, at the path the repository gave it.
///
/// The revision is not in the path. A pin belongs in the provenance beside the
/// file, and a directory named for a commit would make the ordinary case — one
/// model, acquired once — unreadable to a person looking for it.
#[must_use]
pub fn destination(root: &Path, reference: &Reference, path: &str) -> PathBuf {
    root.join(&reference.owner).join(&reference.name).join(path)
}

/// What MCF knows about where this came from.
///
/// Everything read, nothing assumed: the revision is what the hub said the
/// listing was of, and where it said nothing the field stays unknown rather
/// than becoming the branch that was asked for (A7, B-019).
pub fn provenance_of(listing: &Listing, acquired: &Acquired, at: Timestamp) -> Provenance {
    let mut provenance = Provenance::acquired(
        Origin::hub(
            Repository::new(listing.reference.repository()),
            listing.revision.as_deref().map(Revision::new),
        ),
        at,
    );
    if let Verification::Digest { digest } | Verification::LengthOnly { digest } =
        &acquired.verification
        && let Some(checksum) = Checksum::sha256(digest)
    {
        provenance = provenance.with_integrity(checksum);
    }
    if let Some(declared) = listing.declared_licence.as_deref()
        && let Some(licence) = crate::licence::recognize(declared)
    {
        provenance = provenance.with_licence(licence);
    }
    if let Some(lineage) = &listing.lineage {
        // §XII's hard case, written down: what these bytes were made from, what
        // was done to them, and by whom — as far as the publisher said, and no
        // further. The tool and the moment are the publisher's pipeline, which
        // is not MCF's to interrogate, so they stay unknown rather than being
        // filled with this machine's clock (A7, B-019).
        provenance = provenance
            .transformed(Transformation::new(
                kind_of(lineage.relation.as_deref()),
                match &lineage.relation {
                    Some(relation) => Attested::Known(format!(
                        "the publisher calls it {relation}, from {}",
                        lineage.base
                    )),
                    None => Attested::Known(format!(
                        "the publisher names {} as the base and does not say what was done",
                        lineage.base
                    )),
                },
                Attested::Unknown,
                Attested::Unknown,
            ))
            .derived_from(Provenance::known_of(Origin::hub(
                Repository::new(lineage.base.clone()),
                // Which revision of the upstream this was made from is a thing
                // the publisher does not say, and MCF will not guess at: an
                // unpinned base is exactly the break in the chain §XII is
                // about.
                None,
            )));
    }
    provenance
}

/// What the publisher's own word maps to.
///
/// `Other` keeps the word where MCF has no kind for it, which is A7's habit
/// applied to somebody else's vocabulary: a relation filed under the nearest
/// known kind would make the record say something nobody claimed.
fn kind_of(relation: Option<&str>) -> TransformationKind {
    match relation {
        Some("quantized") => TransformationKind::Quantization,
        Some("finetune" | "merge" | "adapter") | None => {
            TransformationKind::Other(relation.unwrap_or("derived from").to_owned())
        }
        Some(other) => TransformationKind::Other(other.to_owned()),
    }
}

/// Writes the acquisition to the journal.
///
/// Returns where it was written, or the failure. A failure here does not undo
/// the acquisition — the model is on the disk and its provenance is beside it —
/// so it is reported rather than propagated: A4's shape, and A2's requirement
/// that it be said rather than swallowed.
fn record(
    hub: &Hub,
    listing: &Listing,
    entry: &Entry,
    acquired: &Acquired,
    at: Timestamp,
) -> Result<PathBuf, Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::failure::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-cli::pull"),
            "there is nowhere to record the acquisition",
        ));
    };
    let mut journal = Journal::open(&path)?;
    journal.append(&Record::new(
        EntryKind::ArtifactAcquired,
        at,
        Value::map([
            ("repository", Value::text(listing.reference.repository())),
            (
                "revision",
                match listing.revision.as_deref() {
                    Some(revision) => Value::text(revision),
                    None => Value::Null,
                },
            ),
            ("file", Value::text(entry.path.clone())),
            ("source", Value::text(hub.describe())),
            ("identity", Value::text(hub.identity().to_string())),
            ("path", Value::text(acquired.path.display().to_string())),
            (
                "bytes",
                Value::Integer(i64::try_from(acquired.bytes).unwrap_or(i64::MAX)),
            ),
            (
                "verification",
                match &acquired.verification {
                    Verification::Digest { digest } => Value::map([
                        ("state", Value::text("verified")),
                        ("digest", Value::text(digest.clone())),
                    ]),
                    Verification::LengthOnly { digest } => Value::map([
                        ("state", Value::text("held_unverified")),
                        ("digest", Value::text(digest.clone())),
                    ]),
                },
            ),
            (
                "attempts",
                Value::Integer(i64::try_from(acquired.attempts).unwrap_or(i64::MAX)),
            ),
            ("resumed", Value::Bool(acquired.resumed)),
        ]),
    ))?;
    Ok(path)
}
