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

#[must_use]
pub fn failure_said(value: &Value) -> Option<String> {
    let detail = value.get("detail")?.as_text()?;
    let mut lines = vec![detail.to_owned()];
    if let Some(Value::Map(context)) = value.get("context") {
        for (key, held) in context {
            if let Some(text) = held.as_text() {
                lines.push(format!("  {key}: {text}"));
            }
        }
    }
    if let Some(cause) = value.get("caused_by").and_then(failure_said) {
        lines.push(format!("caused by: {cause}"));
    }
    Some(lines.join("\n"))
}

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
        reuse: condition(value, "reuse")?,
    })
}

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

#[must_use]
pub fn conditions(
    value: &Value,
    read_by: mcf_core::build_identity::BuildIdentity,
) -> Option<Conditions> {
    Some(Conditions::new(read_by, floor(value)?))
}

fn condition(value: &Value, question: &str) -> Option<Attested<ConditionValue>> {
    match value.get(question)? {
        Value::Null => Some(Attested::Unknown),
        Value::Text(text) => Some(Attested::Known(ConditionValue::text(text.clone()))),
        Value::Integer(number) => Some(Attested::Known(ConditionValue::integer(*number))),
        Value::Bool(_) | Value::ForeignNumber(_) | Value::List(_) | Value::Map(_) => None,
    }
}

const WHERE: Subsystem = Subsystem::new("mcf-record::decode");

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

pub fn provenance(value: &Value) -> Result<Provenance> {
    let origin = origin(value.get("origin").ok_or_else(|| missing("origin"))?)?;
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

fn known(value: Option<&Value>) -> Option<&Value> {
    match value {
        Some(Value::Null) | None => None,
        Some(found) => Some(found),
    }
}

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

pub fn transformation(value: &Value) -> Result<Transformation> {
    let kind = value
        .get("kind")
        .and_then(Value::as_text)
        .ok_or_else(|| missing("transformation.kind"))?;
    let kind = match kind {
        "quantization" => TransformationKind::Quantization,
        "requantization" => TransformationKind::Requantization,
        "format_conversion" => TransformationKind::FormatConversion,
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

pub fn timestamp(value: &Value) -> Result<Timestamp> {
    let found = value
        .get("utc_nanos")
        .ok_or_else(|| missing("timestamp.utc_nanos"))?;
    let nanos: i128 = match found {
        Value::Integer(nanos) => i128::from(*nanos),
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
