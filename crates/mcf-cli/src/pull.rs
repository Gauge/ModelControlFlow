//! `mcf pull`: a model enters this machine, with its provenance (B-029, §III).
//!
//! **What it does, in order.** Read the reference. Ask the hub what the
//! repository publishes and at which revision. If no file was named, say what
//! is there and stop — choosing a quantization for somebody is choosing what
//! they will measure, and §3.13 refuses generality nobody asked for as firmly
//! as A7 refuses a plausible default. If one was named, acquire it: bytes
//! accumulate under a `.partial` name, the digest the hub declared is checked
//! against the bytes that arrived, and the artifact's own name is given only to
//! something verified (B-021).
//!
//! **Then two records, which are not the same record.** The provenance goes in
//! a sidecar beside the artifact, because that is what travels with the file
//! (§3.6); the acquisition goes in the journal, because that is what happened on
//! this machine (A1). An artifact somebody later moves by hand keeps the first
//! and cannot alter the second.
//!
//! **What it will not do yet.** Reach an `https` hub. MCF has no TLS stack —
//! [findings.md](../../../doc/findings.md) F9 measured what admitting one costs
//! and B-322 is where it happens — so a request for one is refused in as many
//! words rather than attempted and failed obscurely. `--from` points at an
//! `http` mirror, which is what the laboratory uses and what an operator on a
//! closed network has.

use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::failure::Failure;
use mcf_core::hardware::Machine;
use mcf_core::measurement::Bytes;
use mcf_core::provenance::{
    Checksum, Origin, Provenance, Repository, Revision, Transformation, TransformationKind,
};
use mcf_core::time::Timestamp;
use mcf_hub::client::Hub;
use mcf_hub::credentials::{self, Credential, Origin as Held, Secret};
use mcf_hub::fetch::{Acquired, Verification, acquire};
use mcf_hub::fitment::{self, Requirement, Shape, Verdict};
use mcf_hub::http::Url;
use mcf_hub::reference::{self, Reference};
use mcf_hub::source::{Entry, Listing, Source as _};
use mcf_hub::store;
use mcf_hub::wire::{Tcp, Tls, Wire};
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

use crate::Response;
use crate::models;

/// Where MCF looks for models when nobody says otherwise.
pub(crate) const DEFAULT_HUB: &str = "https://huggingface.co/";

/// Where a credential came from, as the operator said.
///
/// There is no fourth option and no default. B-024's whole claim is that MCF
/// never picks one up on its own: an operator either hands one over or names
/// exactly where MCF may read it from, and either way what happened is in the
/// provenance of the acquisition (§3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Offered<'a> {
    /// None, which is how most of the hub is read.
    Nothing,
    /// This file holds one.
    File(&'a str),
    /// This environment variable holds one, and the operator said so.
    Variable(&'a str),
}

/// Acquires a model, or says what would be acquired.
pub(crate) fn run(asked_for: &str, from: Option<&str>, offered: Offered<'_>) -> Response {
    let reference = match reference::parse(asked_for) {
        Ok(reference) => reference,
        Err(failure) => return refused("that is not a reference MCF can resolve", &failure),
    };
    let base = match Url::parse(from.unwrap_or(DEFAULT_HUB)) {
        Ok(base) => base,
        Err(failure) => return refused("that is not a hub MCF can reach", &failure),
    };
    let Some(root) = models::default_root() else {
        return Response {
            text: "mcf: there is nowhere to keep a model — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };

    let wire = match wire_for(&base) {
        Ok(wire) => wire,
        Err(failure) => return refused("nothing was acquired", &failure),
    };
    let hub = match credential(offered, &environment) {
        Ok(None) => Hub::at(base, wire),
        Ok(Some(credential)) => Hub::at(base, wire).offering(credential),
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => {
            let mut response = refused("nothing was acquired", &failure);
            if failure.category() == mcf_core::failure::Category::HubAuthRequired {
                response.text.push('\n');
                response.text.push_str(&what_is_lying_around(&environment));
            }
            return response;
        }
    };

    let Some(wanted) = reference.file.clone() else {
        // The plan is what an operator is really asking for when they name a
        // repository and no file: not *what is published* but *which of these
        // will run here* (PR3, B-213).
        let planned = plan_for(&hub, &listing);
        return Response {
            text: offer(&listing, planned.as_deref()),
            served: true,
        };
    };
    let Some(entry) = listing.entry(&wanted).cloned() else {
        return Response {
            text: format!(
                "mcf: {} publishes no file called {wanted}\n{}",
                listing.reference.repository(),
                offer(&listing, None)
            ),
            served: false,
        };
    };

    acquire_one(&hub, &listing, &entry, &root)
}

/// Fetches one file and writes down everything about it.
fn acquire_one(hub: &Hub, listing: &Listing, entry: &Entry, root: &Path) -> Response {
    let into = destination(root, &listing.reference, &entry.path);
    if let Some(parent) = into.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return Response {
            text: format!(
                "mcf: nothing was acquired — {} could not be made\n  {error}",
                parent.display()
            ),
            served: false,
        };
    }

    let acquired = match acquire(hub, &listing.reference, entry, &into) {
        Ok(acquired) => acquired,
        Err(failure) => return refused("nothing was acquired", &failure),
    };

    let at = Timestamp::now();
    let provenance = provenance_of(listing, &acquired, at);
    let sidecar = match store::record_provenance(&into, &provenance) {
        Ok(sidecar) => sidecar,
        Err(failure) => {
            return refused(
                "the model was acquired and its provenance could not be written beside it, so \
                 MCF is holding a file it cannot account for",
                &failure,
            );
        }
    };

    let recorded = record(hub, listing, entry, &acquired, at);
    Response {
        text: render(&acquired, listing, &sidecar, &provenance, recorded.as_ref()),
        served: true,
    }
}

/// Where an artifact lives: under the store, by the repository that published
/// it, at the path the repository gave it.
///
/// The revision is not in the path. A pin belongs in the provenance beside the
/// file, and a directory named for a commit would make the ordinary case — one
/// model, acquired once — unreadable to a person looking for it.
fn destination(root: &Path, reference: &Reference, path: &str) -> PathBuf {
    root.join(&reference.owner).join(&reference.name).join(path)
}

/// What MCF knows about where this came from.
///
/// Everything read, nothing assumed: the revision is what the hub said the
/// listing was of, and where it said nothing the field stays unknown rather
/// than becoming the branch that was asked for (A7, B-019).
fn provenance_of(listing: &Listing, acquired: &Acquired, at: Timestamp) -> Provenance {
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
        && let Some(licence) = mcf_hub::licence::recognize(declared)
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
    let sequence = journal.appended();
    journal.append(&Record::new(
        EntryKind::ArtifactAcquired,
        at,
        sequence,
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

/// How MCF reaches this hub.
///
/// TLS where the hub is encrypted, a plain socket where it is not. Chosen from
/// the URL rather than configured, because *which one* is not a preference: a
/// wire that cannot keep a secret refuses to carry one, and an `https` request
/// over a plain socket is refused before it is opened (B-024, B-322).
fn wire_for(base: &Url) -> Result<Box<dyn Wire>, mcf_core::failure::Failure> {
    if base.scheme() == "https" {
        Ok(Box::new(Tls::new()?))
    } else {
        Ok(Box::new(Tcp::default()))
    }
}

/// The one place in this surface that reads the environment.
///
/// A function rather than a call at each site, so that everything below is
/// *handed* a way to look and the reading happens where a reader can see it —
/// the same discipline `mcf_hub::credentials` keeps, for the same reason
/// (B-024).
fn environment(variable: &str) -> Option<String> {
    std::env::var(variable).ok()
}

/// The credential the operator named, read from where they said it was.
///
/// Reading a file or an environment variable *because somebody named it* is not
/// the silent pickup B-024 forbids: what makes it deliberate is that the name
/// came from the command line, and what makes it accountable is that the origin
/// travels with the credential into the record (§3.4).
fn credential(
    offered: Offered<'_>,
    look_up: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<Credential>, String> {
    let (token, origin) = match offered {
        Offered::Nothing => return Ok(None),
        Offered::File(path) => {
            let read = std::fs::read_to_string(path).map_err(|error| {
                format!("mcf: the credential file could not be read\n  {path}: {error}")
            })?;
            (
                read.trim().to_owned(),
                Held::File {
                    path: PathBuf::from(path),
                },
            )
        }
        Offered::Variable(name) => (
            look_up(name)
                .ok_or_else(|| {
                    format!(
                        "mcf: {name} is not set, so there is no credential to offer\n  MCF reads \
                         an environment variable only when it is named, and this one holds nothing"
                    )
                })?
                .trim()
                .to_owned(),
            Held::Environment {
                variable: name.to_owned(),
            },
        ),
    };
    if token.is_empty() {
        return Err(format!(
            "mcf: {origin} holds nothing\n  an empty credential is not a credential, and \
             offering one would produce a refusal nobody could explain"
        ));
    }
    Ok(Some(Credential::new(Secret::new(token), origin)))
}

/// What credentials are sitting on this machine, unused.
///
/// Shown only when the hub has just said it needs one. MCF has looked and used
/// nothing: the whole point of B-024 is that finding a token is not permission
/// to spend it, and the operator is told the name to pass rather than having
/// the decision made for them.
fn what_is_lying_around(look_up: &dyn Fn(&str) -> Option<String>) -> String {
    let seen = credentials::sightings(look_up, None, &|_| None);
    if seen.is_empty() {
        return "  MCF looked for a credential on this machine and found none.\n  Offer one with \
                --token-from <file> or --token-from-env <VARIABLE>."
            .to_owned();
    }
    let mut lines = vec!["  MCF has looked, and used nothing:".to_owned()];
    for sighting in &seen {
        lines.push(format!("    {}", sighting.describe()));
        if let Held::Environment { variable } = &sighting.origin {
            lines.push(format!("    offer it with --token-from-env {variable}"));
        }
    }
    lines.join("\n")
}

/// How much context a plan is made at when nobody has said.
///
/// Four thousand and ninety-six tokens: the length most engines default to, and
/// a number stated here rather than buried, because the answer *this fits*
/// means nothing without the context it fits at (A6, §3.4).
pub(crate) const PLANNING_CONTEXT: u64 = 4096;

/// How many bytes one cached element takes.
///
/// Two, for the half-precision caches engines use by default. A parameter of
/// the run rather than a fact about the model, and the reason [`Shape`] takes
/// it rather than reading it.
const CACHE_ELEMENT: u64 = 2;

/// Which of the variants a repository publishes will run on this machine.
///
/// `None` when the plan cannot be made: no configuration published, a shape MCF
/// will not guess at, or a machine that will not say how much memory it has.
/// Each of those is a *reason there is no plan* rather than a plan with holes
/// in it (A7), and [`offer`] says which.
fn plan_for(hub: &Hub, listing: &Listing) -> Option<Vec<String>> {
    let configuration = hub.configuration(listing).ok().flatten()?;
    let shape = Shape::from_configuration(&configuration, CACHE_ELEMENT)?;
    let available = match Machine::read().memory.available {
        Attested::Known(available) => available,
        Attested::Unknown => return None,
    };

    let requirements: Vec<Requirement> = listing
        .entries
        .iter()
        // Case-insensitively, because a repository's file names are its own:
        // `.GGUF` is the same format and a plan that skipped it would leave a
        // variant out of the list without saying so (A1).
        .filter(|entry| {
            std::path::Path::new(&entry.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
        })
        .map(|entry| Requirement {
            name: entry.path.clone(),
            weights: Bytes(entry.size),
            shape,
        })
        .collect();
    if requirements.is_empty() {
        return None;
    }

    let verdicts = fitment::plan(&requirements, PLANNING_CONTEXT, available).ok()?;
    Some(
        verdicts
            .iter()
            .map(|(name, verdict)| match verdict {
                Verdict::Fits { needs, headroom } => format!(
                    "  {name} — fits: needs {} of {} usable, {} left",
                    needs.0, available.0, headroom.0
                ),
                Verdict::FitsWithoutContextHeadroom {
                    needs,
                    longest_context,
                } => format!(
                    "  {name} — fits at a shorter context: {} at {PLANNING_CONTEXT} tokens is \
                     more than this machine has; {longest_context} tokens would fit",
                    needs.0
                ),
                Verdict::DoesNotFit { needs, short_by } => format!(
                    "  {name} — does NOT fit: needs {}, which is {} more than this machine has",
                    needs.0, short_by.0
                ),
            })
            .collect(),
    )
}

/// What a repository publishes, when nobody has said which file they want.
///
/// Choosing for an operator would be choosing what they measure. What MCF can
/// do is put the choice in front of them with the sizes, which is the question
/// they are actually asking.
fn offer(listing: &Listing, planned: Option<&[String]>) -> String {
    let mut lines = vec![format!(
        "{} publishes {} file(s) at {}",
        listing.reference.repository(),
        listing.entries.len(),
        listing
            .revision
            .as_deref()
            .unwrap_or("a revision the hub did not name")
    )];
    lines.push(mcf_hub::licence::describe(
        listing
            .declared_licence
            .as_deref()
            .and_then(mcf_hub::licence::recognize)
            .as_ref(),
    ));
    for entry in &listing.entries {
        lines.push(format!(
            "  {} — {} bytes{}",
            entry.path,
            entry.size,
            match entry.digest {
                Some(_) => "",
                None => " (the hub declares no digest for this one)",
            }
        ));
    }
    match planned {
        Some(plan) => {
            lines.push(format!(
                "\nat {PLANNING_CONTEXT} tokens of context, on this machine:"
            ));
            lines.extend(plan.iter().cloned());
        }
        None => lines.push(
            "\nMCF cannot say which of these would run here: that needs the model's own \
             configuration\nand this machine's free memory, and one of them could not be read \
             (A7)"
                .to_owned(),
        ),
    }
    lines.push(format!(
        "\nnothing was acquired: name the file you want, as\n  mcf pull {}:<file>",
        listing.reference.repository()
    ));
    lines.join("\n")
}

/// What an operator is told when a model has arrived.
fn render(
    acquired: &Acquired,
    listing: &Listing,
    sidecar: &Path,
    provenance: &Provenance,
    recorded: Result<&PathBuf, &Failure>,
) -> String {
    let verification = match &acquired.verification {
        Verification::Digest { digest } => format!("verified against the hub's digest: {digest}"),
        Verification::LengthOnly { digest } => format!(
            "HELD, NOT VERIFIED: the hub declared no digest, so all MCF can say is that {} bytes \
             arrived and their digest is {digest}",
            acquired.bytes
        ),
    };
    let mut lines = vec![
        format!("acquired {}", acquired.path.display()),
        format!("  {} bytes, {verification}", acquired.bytes),
        format!(
            "  from {} at {}",
            listing.reference.repository(),
            listing
                .revision
                .as_deref()
                .unwrap_or("a revision the hub did not name")
        ),
        format!(
            "  {}",
            mcf_hub::licence::describe(provenance.licence().known())
        ),
        format!("  provenance beside it: {}", sidecar.display()),
    ];
    if let Some(source) = provenance.source() {
        lines.push(format!(
            "  made from {}, which MCF has not fetched and cannot vouch for",
            source.origin()
        ));
        for transformation in provenance.transformations() {
            lines.push(format!("    {transformation}"));
        }
    }
    if acquired.attempts > 1 {
        lines.push(format!(
            "  it took {} transfers{}",
            acquired.attempts,
            if acquired.resumed {
                ", continuing where each stopped"
            } else {
                ", starting again each time"
            }
        ));
    }
    match recorded {
        Ok(path) => lines.push(format!("  recorded in {}", path.display())),
        Err(failure) => lines.push(format!(
            "  NOT recorded: {failure}\n   the model is on the disk with its provenance; what is \
             missing is the record that it arrived"
        )),
    }
    lines.join("\n")
}

/// A refusal, with the context that makes it actionable.
fn refused(what: &str, failure: &Failure) -> Response {
    let mut lines = vec![format!("mcf: {what}"), format!("  {failure}")];
    for entry in failure.context() {
        lines.push(format!("    {}: {}", entry.key, entry.value));
    }
    Response {
        text: lines.join("\n"),
        served: false,
    }
}

/// Reads the licence a listing declares, for a surface that has one.
#[cfg(test)]
pub(crate) fn licence_of(listing: &Listing) -> Option<mcf_core::provenance::Licence> {
    listing
        .declared_licence
        .as_deref()
        .and_then(mcf_hub::licence::recognize)
}

#[cfg(test)]
mod tests;
