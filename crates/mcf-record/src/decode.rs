//! Reading back what the record holds.
//!
//! B-007's condition is that *the §3.4 floor is captured from a live machine
//! and round-trips through the record store losslessly*, and a round trip needs
//! both directions. [`encode`] writes; this reads.
//!
//! **What losslessly means here, and what it does not.** A condition MCF wrote
//! comes back as what MCF wrote; a condition MCF could not read comes back as
//! [`Attested::Unknown`] and not as the word *unknown*. Those two are the whole
//! of the property: a floor whose unknowns came back as strings would compare
//! equal to a floor that had read something, which is exactly A7's substitution
//! arriving through the back door of a decoder.
//!
//! **A line this version cannot understand is not decoded into a guess.** The
//! floor's decoder returns `None` on a shape it does not recognize, and the
//! caller decides whether that is `record.schema.unknown` or a corrupt line —
//! the same discipline `EntryKind::parse` follows, for §7.30's reason.
//!
//! **Two shapes of answer, for two different questions.** That yes-or-no is
//! right for a journal line, where only the caller knows what it was doing when
//! it found one it could not read. It is wrong for a provenance: there is an
//! artifact on the disk and somebody is asking where it came from (B-029), and
//! *no* is not an answer they can act on. So [`provenance`] refuses with a
//! classified failure that names the field, and refuses the whole chain when a
//! link in it is unreadable — a chain with an invented link is worse than no
//! chain (A1, §XII).
//!
//! [`encode`]: crate::encode

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::measurement::{ConditionValue, Conditions, Floor};
use mcf_core::provenance::{
    Checksum, Decay, DigestAlgorithm, Licence, Observation, Origin, Provenance, Repository,
    Revision, ToolIdentity, Transformation, TransformationKind,
};
use mcf_core::time::{Timestamp, UtcOffset};
use mcf_core::trial::Draw;

use crate::json::Value;

/// Reads a condition floor back.
///
/// Returns `None` when a question the floor asks is missing from the record
/// entirely — which is a *different* thing from a question that was asked and
/// not answered. The first means this line was not written by a version that
/// asks the same questions; the second is `null`, and comes back as
/// [`Attested::Unknown`].
#[must_use]
pub fn floor(value: &Value) -> Option<Floor> {
    Some(Floor {
        hardware_state: condition(value, "hardware_state")?,
        thermal_state: condition(value, "thermal_state")?,
        driver_versions: condition(value, "driver_versions")?,
        runtime_versions: condition(value, "runtime_versions")?,
        quantization: condition(value, "quantization")?,
        context_length: condition(value, "context_length")?,
        batch_shape: condition(value, "batch_shape")?,
        mcf_configuration: condition(value, "mcf_configuration")?,
        realized_placement: condition(value, "realized_placement")?,
        instrumentation: condition(value, "instrumentation")?,
        artifact_storage: condition(value, "artifact_storage")?,
        seed_set: condition(value, "seed_set")?,
    })
}

/// Reads what a trial drew back (B-290).
///
/// `None` where the record does not say — an entry from a version that did not
/// ask, which is a different thing from a trial that drew nothing. Deciding
/// what to do about that belongs to the reader with the context (§7.30), and a
/// decoder that invented a seed would be inventing a condition.
#[must_use]
pub fn draw(value: &Value) -> Option<Draw> {
    let held = value.get("drew")?;
    let seed = held.get("seed")?.as_text()?.parse::<u64>().ok()?;
    match held.get("discipline")?.as_text()? {
        "seeded" => Some(Draw::Seeded {
            seed,
            from: held.get("from")?.as_text()?.to_owned(),
        }),
        "length_pinned" => Some(Draw::LengthPinned {
            seed,
            tokens: u32::try_from(held.get("tokens")?.as_integer()?).ok()?,
        }),
        _ => None,
    }
}

/// Reads a condition set back, instrument included.
///
/// The build identity is *not* reconstructed from the record: [`Conditions`]
/// binds a floor to the instrument that read it, and the instrument a decoder
/// could offer is the one running now, not the one that wrote the line. The
/// caller supplies it, which forces the question of whose instrument this is to
/// be answered at the call site rather than assumed by a parser (§3.4).
#[must_use]
pub fn conditions(
    value: &Value,
    read_by: mcf_core::build_identity::BuildIdentity,
) -> Option<Conditions> {
    Some(Conditions::new(read_by, floor(value)?))
}

/// One condition: present and readable, present and null, or absent.
fn condition(value: &Value, question: &str) -> Option<Attested<ConditionValue>> {
    match value.get(question)? {
        Value::Null => Some(Attested::Unknown),
        Value::Text(text) => Some(Attested::Known(ConditionValue::text(text.clone()))),
        Value::Integer(number) => Some(Attested::Known(ConditionValue::integer(*number))),
        // A condition written as a boolean, a list, an object or a number this
        // format does not carry is a shape this version does not ask for. It is
        // not decoded into text, because a decoder that coerced would make a
        // record say something nobody wrote.
        Value::Bool(_) | Value::ForeignNumber(_) | Value::List(_) | Value::Map(_) => None,
    }
}

const WHERE: Subsystem = Subsystem::new("mcf-record::decode");

/// Something the record had to say and did not.
fn missing(what: &str) -> Failure {
    Failure::new(
        Category::ArtifactProvenanceIncomplete,
        Attribution::Mcf,
        Disposition::Refused,
        WHERE,
        "a provenance record does not state something it must",
    )
    .with_context("wanted", what.to_owned())
}

/// Something the record said that cannot be read.
fn unreadable(what: &str, found: &Value) -> Failure {
    Failure::new(
        Category::ArtifactProvenanceIncomplete,
        Attribution::Mcf,
        Disposition::Refused,
        WHERE,
        "a provenance record states something MCF cannot read",
    )
    .with_context("field", what.to_owned())
    .with_context("found", found.to_line())
}

/// Where an artifact came from, and everything that happened to it since.
///
/// # Errors
///
/// `artifact.provenance.incomplete` naming the field, for anything absent or
/// unreadable. The chain is read whole: a source whose own provenance is
/// unreadable refuses the derivative too, because a chain with an invented link
/// is worse than no chain (A1, §XII).
pub fn provenance(value: &Value) -> Result<Provenance> {
    let origin = origin(value.get("origin").ok_or_else(|| missing("origin"))?)?;
    // A retrieval time that is `null` is a link MCF never fetched, which is a
    // state the type has (`known_of`) rather than a field to fill in — the
    // upstream half of §XII's chain (A7).
    let mut read = match known(value.get("retrieved_at")) {
        Some(at) => Provenance::acquired(origin, timestamp(at)?),
        None => Provenance::known_of(origin),
    };

    if let Some(found) = known(value.get("integrity")) {
        read = read.with_integrity(checksum(found)?);
    }
    if let Some(found) = known(value.get("licence")) {
        read = read.with_licence(licence(found)?);
    }
    if let Some(list) = value.get("transformations") {
        let entries = list
            .as_list()
            .ok_or_else(|| unreadable("transformations", list))?;
        for entry in entries {
            read = read.transformed(transformation(entry)?);
        }
    }
    if let Some(list) = value.get("observed") {
        let entries = list.as_list().ok_or_else(|| unreadable("observed", list))?;
        for entry in entries {
            read = read.observed(observation(entry)?);
        }
    }
    if let Some(found) = known(value.get("derived_from")) {
        read = read.derived_from(self::provenance(found)?);
    }
    Ok(read)
}

/// What MCF found upstream when it looked (B-331, D37).
///
/// A finding this version does not know is `record.schema.unknown` rather than
/// a shrug: an observation read as *unchanged* when it said something else
/// would be a record that lies in the safe-sounding direction (§7.30, A7).
fn observation(value: &Value) -> Result<Observation> {
    let looked_at = timestamp(value.get("looked_at").ok_or_else(|| missing("looked_at"))?)?;
    let name = value
        .get("found")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("found"))?;
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_text)
            .map(str::to_owned)
            .ok_or_else(|| missing(key))
    };
    let found = match name {
        "unchanged" => Decay::Unchanged,
        "revision_gone" => Decay::RevisionGone {
            revision: text("revision")?,
        },
        "relicensed" => Decay::Relicensed {
            was: text("was")?,
            now: text("now")?,
        },
        "gated" => Decay::Gated { how: text("how")? },
        "replaced" => Decay::Replaced {
            file: text("file")?,
            was: text("was")?,
            now: text("now")?,
        },
        "unreachable" => Decay::Unreachable {
            said: text("said")?,
        },
        other => {
            return Err(Failure::new(
                Category::RecordSchemaUnknown,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the record names a finding this build does not know",
            )
            .with_context("found", other.to_owned()));
        }
    };
    Ok(Observation::new(looked_at, found))
}

/// A field that is present and is not `null`.
///
/// `null` is *unknown* in a record (A7), and a field that is absent altogether
/// is the same absence written by an older writer — both are read as unknown
/// rather than as a reason to refuse, because a provenance that says *MCF did
/// not read the licence* is a complete provenance.
fn known(value: Option<&Value>) -> Option<&Value> {
    match value {
        Some(Value::Null) | None => None,
        Some(found) => Some(found),
    }
}

/// Where bytes came from.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the kind is absent, unreadable, or one
/// this build does not know — the last of which is a record from a later MCF,
/// and inventing an origin for it would be worse than saying so (§7.30).
pub fn origin(value: &Value) -> Result<Origin> {
    let kind = value
        .get("kind")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("origin.kind"))?;
    match kind {
        "hub" => {
            let repository = value
                .get("repository")
                .and_then(Value::as_text)
                .ok_or_else(|| missing("origin.repository"))?;
            let revision = known(value.get("revision"))
                .map(|found| {
                    found
                        .as_text()
                        .map(Revision::new)
                        .ok_or_else(|| unreadable("origin.revision", found))
                })
                .transpose()?;
            Ok(Origin::hub(Repository::new(repository), revision))
        }
        "local_file" => {
            let path = value
                .get("path")
                .and_then(Value::as_text)
                .ok_or_else(|| missing("origin.path"))?;
            Ok(Origin::LocalFile {
                path: std::path::PathBuf::from(path),
            })
        }
        "unattributed" => Ok(Origin::Unattributed),
        _ => Err(unreadable("origin.kind", value)),
    }
}

/// A digest of an artifact's bytes.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the algorithm is one this build does
/// not compute, or the digest is not one: a checksum MCF cannot check is not a
/// checksum, and recording it as though it were would make an unverifiable
/// artifact look verified (A21).
pub fn checksum(value: &Value) -> Result<Checksum> {
    let algorithm = value
        .get("algorithm")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("integrity.algorithm"))?;
    let hex = value
        .get("hex")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("integrity.hex"))?;
    match algorithm {
        "sha256" => Checksum::new(DigestAlgorithm::Sha256, hex)
            .ok_or_else(|| unreadable("integrity.hex", value)),
        _ => Err(unreadable("integrity.algorithm", value)),
    }
}

/// What an artifact's terms are.
///
/// # Errors
///
/// `artifact.provenance.incomplete` for a state this build does not know.
pub fn licence(value: &Value) -> Result<Licence> {
    let state = value
        .get("state")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("licence.state"))?;
    match state {
        "identified" => value
            .get("identifier")
            .and_then(Value::as_text)
            .map(Licence::spdx)
            .ok_or_else(|| missing("licence.identifier")),
        "stated_and_unmatched" => Ok(Licence::Stated),
        _ => Err(unreadable("licence.state", value)),
    }
}

/// One thing that was done to an artifact.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the kind is absent or unreadable.
pub fn transformation(value: &Value) -> Result<Transformation> {
    let kind = value
        .get("kind")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("transformation.kind"))?;
    let kind = match kind {
        "quantization" => TransformationKind::Quantization,
        "requantization" => TransformationKind::Requantization,
        "format_conversion" => TransformationKind::FormatConversion,
        // Anything else is what whoever did it called it, which is exactly how
        // it was written.
        other => TransformationKind::Other(other.to_owned()),
    };

    let detail = match known(value.get("detail")) {
        Some(found) => Attested::Known(
            found
                .as_text()
                .ok_or_else(|| unreadable("transformation.detail", found))?
                .to_owned(),
        ),
        None => Attested::Unknown,
    };
    let performed_by = match known(value.get("performed_by")) {
        Some(found) => Attested::Known(tool_identity(found)?),
        None => Attested::Unknown,
    };
    let performed_at = match known(value.get("performed_at")) {
        Some(found) => Attested::Known(timestamp(found)?),
        None => Attested::Unknown,
    };
    Ok(Transformation::new(
        kind,
        detail,
        performed_by,
        performed_at,
    ))
}

/// What performed a transformation.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the tool has no name.
pub fn tool_identity(value: &Value) -> Result<ToolIdentity> {
    let name = value
        .get("name")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("performed_by.name"))?;
    let version = match known(value.get("version")) {
        Some(found) => Some(
            found
                .as_text()
                .ok_or_else(|| unreadable("performed_by.version", found))?
                .to_owned(),
        ),
        None => None,
    };
    Ok(ToolIdentity::new(name, version))
}

/// A moment.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the nanoseconds are absent or are not
/// a number. The rendering is not read back: it is for a person, and a reader
/// that trusted it over the integer would be trusting a formatting decision.
pub fn timestamp(value: &Value) -> Result<Timestamp> {
    let found = value
        .get("utc_nanos")
        .ok_or_else(|| missing("timestamp.utc_nanos"))?;
    let nanos: i128 = match found {
        Value::Integer(nanos) => i128::from(*nanos),
        // Written as text where the value does not fit an integer, which is how
        // the whole range survives a format that has less of one (A1).
        Value::Text(written) => written
            .parse()
            .map_err(|_| unreadable("timestamp.utc_nanos", found))?,
        _ => return Err(unreadable("timestamp.utc_nanos", found)),
    };
    let offset = match known(value.get("offset_seconds_east")) {
        Some(found) => {
            let seconds = found
                .as_integer()
                .and_then(|seconds| i32::try_from(seconds).ok())
                .and_then(UtcOffset::from_seconds_east)
                .ok_or_else(|| unreadable("timestamp.offset_seconds_east", found))?;
            Attested::Known(seconds)
        }
        None => Attested::Unknown,
    };
    Ok(Timestamp::from_utc_nanos(nanos, offset))
}

#[cfg(test)]
mod tests;
