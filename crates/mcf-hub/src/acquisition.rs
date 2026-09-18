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

#[derive(Debug)]
pub struct Done {
    pub acquired: Acquired,
    pub sidecar: PathBuf,
    pub provenance: Provenance,
    pub recorded: Result<PathBuf, Failure>,
}

pub fn one(
    hub: &Hub,
    listing: &Listing,
    entry: &Entry,
    root: &Path,
    stopping: &crate::stopping::Stopping,
) -> Result<Done, Failure> {
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
    let acquired = acquire(hub, &listing.reference, entry, &into, stopping)?;
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

#[must_use]
pub fn arriving_at(root: &Path, listing: &Listing, entry: &Entry) -> PathBuf {
    crate::fetch::partial_path(&destination(root, &listing.reference, &entry.path))
}

#[must_use]
pub fn destination(root: &Path, reference: &Reference, path: &str) -> PathBuf {
    root.join(&reference.owner).join(&reference.name).join(path)
}

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
                None,
            )));
    }
    provenance
}

fn kind_of(relation: Option<&str>) -> TransformationKind {
    match relation {
        Some("quantized") => TransformationKind::Quantization,
        Some("finetune" | "merge" | "adapter") | None => {
            TransformationKind::Other(relation.unwrap_or("derived from").to_owned())
        }
        Some(other) => TransformationKind::Other(other.to_owned()),
    }
}

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
