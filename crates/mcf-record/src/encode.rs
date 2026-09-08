use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::degradation::{Degradation, Degraded};
use mcf_core::failure::Failure;
use mcf_core::hardware::{Accelerator, Characterization, Machine};
use mcf_core::measurement::{ConditionValue, Conditions, Floor, Measurement, Quantity};
use mcf_core::provenance::{
    Checksum, Decay, Licence, Observation, Origin, Provenance, ToolIdentity, Transformation,
    TransformationKind,
};
use mcf_core::time::Timestamp;
use mcf_core::trial::{Draw, Series, Trial, Trials};

use crate::json::Value;

#[must_use]
pub fn build_identity(identity: BuildIdentity) -> Value {
    Value::map([
        ("version", Value::text(identity.version)),
        ("revision", attested_text(&identity.revision.to_string())),
        ("rustc", Value::text(identity.rustc)),
        ("target", Value::text(identity.target)),
        ("profile", Value::text(identity.profile)),
        (
            "instrument",
            match mcf_core::build_identity::instrument() {
                mcf_core::attested::Attested::Known(digest) => Value::text(digest.hex()),
                mcf_core::attested::Attested::Unknown => Value::Null,
            },
        ),
    ])
}

#[must_use]
pub fn failure(failure: &Failure) -> Value {
    Value::map([
        ("category", Value::text(failure.category().code())),
        ("meaning", Value::text(failure.category().meaning())),
        ("attribution", Value::text(failure.attribution().as_str())),
        ("disposition", Value::text(failure.disposition().as_str())),
        ("subsystem", Value::text(failure.subsystem().as_str())),
        ("detail", Value::text(failure.detail())),
        (
            "context",
            Value::map(
                failure
                    .context()
                    .iter()
                    .map(|entry| (entry.key, Value::text(entry.value.clone()))),
            ),
        ),
        (
            "caused_by",
            match failure.cause() {
                Some(cause) => self::failure(cause),
                None => Value::Null,
            },
        ),
    ])
}

#[must_use]
pub fn degradation(degradation: &Degradation) -> Value {
    Value::List(degradation.causes().iter().map(failure).collect())
}

#[must_use]
pub fn degraded<T>(value: &Degraded<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    Value::map([
        ("degraded", Value::Bool(true)),
        ("degradation", degradation(value.degradation())),
        ("value", encode(value.value())),
    ])
}

#[must_use]
pub fn provenance(provenance: &Provenance) -> Value {
    Value::map([
        ("origin", origin(provenance.origin())),
        (
            "retrieved_at",
            attested(provenance.retrieved_at(), |at| timestamp(*at)),
        ),
        ("integrity", attested(provenance.integrity(), checksum)),
        ("licence", attested(provenance.licence(), licence)),
        (
            "transformations",
            Value::List(
                provenance
                    .transformations()
                    .iter()
                    .map(transformation)
                    .collect(),
            ),
        ),
        (
            "observed",
            Value::List(provenance.observations().iter().map(observation).collect()),
        ),
        (
            "derived_from",
            match provenance.source() {
                Some(source) => self::provenance(source),
                None => Value::Null,
            },
        ),
    ])
}

#[must_use]
pub fn observation(observed: &Observation) -> Value {
    let mut fields = vec![
        ("looked_at", timestamp(observed.looked_at)),
        ("found", Value::text(observed.found.as_str())),
    ];
    match &observed.found {
        Decay::RevisionGone { revision } => fields.push(("revision", Value::text(revision))),
        Decay::Relicensed { was, now } => {
            fields.push(("was", Value::text(was)));
            fields.push(("now", Value::text(now)));
        }
        Decay::Gated { how } => fields.push(("how", Value::text(how))),
        Decay::Replaced { file, was, now } => {
            fields.push(("file", Value::text(file)));
            fields.push(("was", Value::text(was)));
            fields.push(("now", Value::text(now)));
        }
        Decay::Unreachable { said } => fields.push(("said", Value::text(said))),
        Decay::Unchanged | _ => {}
    }
    Value::map(fields)
}

#[must_use]
pub fn origin(origin: &Origin) -> Value {
    match origin {
        Origin::Hub {
            repository,
            revision,
        } => Value::map([
            ("kind", Value::text("hub")),
            ("repository", Value::text(repository.as_str())),
            (
                "revision",
                attested(revision, |revision| Value::text(revision.as_str())),
            ),
        ]),
        Origin::LocalFile { path } => Value::map([
            ("kind", Value::text("local_file")),
            ("path", Value::text(path.display().to_string())),
        ]),
        Origin::Unattributed => Value::map([("kind", Value::text("unattributed"))]),
        other => Value::map([
            ("kind", Value::text("unrecorded")),
            ("rendered", Value::text(other.to_string())),
        ]),
    }
}

#[must_use]
pub fn checksum(checksum: &Checksum) -> Value {
    Value::map([
        ("algorithm", Value::text(checksum.algorithm().as_str())),
        ("hex", Value::text(checksum.hex())),
    ])
}

#[must_use]
pub fn licence(licence: &Licence) -> Value {
    match licence {
        Licence::Spdx(identifier) => Value::map([
            ("state", Value::text("identified")),
            ("identifier", Value::text(identifier.clone())),
        ]),
        Licence::Stated => Value::map([("state", Value::text("stated_and_unmatched"))]),
        other => Value::map([
            ("state", Value::text("unrecorded")),
            ("rendered", Value::text(other.to_string())),
        ]),
    }
}

#[must_use]
pub fn transformation(transformation: &Transformation) -> Value {
    Value::map([
        ("kind", transformation_kind(transformation.kind())),
        (
            "detail",
            attested(transformation.detail(), |detail| {
                Value::text(detail.clone())
            }),
        ),
        (
            "performed_by",
            attested(transformation.performed_by(), tool_identity),
        ),
        (
            "performed_at",
            attested(transformation.performed_at(), |at| timestamp(*at)),
        ),
    ])
}

fn transformation_kind(kind: &TransformationKind) -> Value {
    match kind {
        TransformationKind::Quantization => Value::text("quantization"),
        TransformationKind::Requantization => Value::text("requantization"),
        TransformationKind::FormatConversion => Value::text("format_conversion"),
        TransformationKind::Other(name) => Value::text(name.clone()),
        other => Value::text(other.to_string()),
    }
}

fn tool_identity(tool: &ToolIdentity) -> Value {
    Value::map([
        ("name", Value::text(tool.name())),
        (
            "version",
            attested(tool.version(), |version| Value::text(version.clone())),
        ),
    ])
}

#[must_use]
pub fn timestamp(at: Timestamp) -> Value {
    let nanos = match i64::try_from(at.utc_nanos()) {
        Ok(nanos) => Value::Integer(nanos),
        Err(_) => Value::text(at.utc_nanos().to_string()),
    };
    Value::map([
        ("utc_nanos", nanos),
        (
            "offset_seconds_east",
            attested(&at.offset(), |offset| {
                Value::Integer(i64::from(offset.seconds_east()))
            }),
        ),
        ("rendered", Value::text(at.to_string())),
    ])
}

#[must_use]
pub fn conditions(conditions: &Conditions) -> Value {
    let mut floor: Vec<(String, Value)> = conditions
        .floor()
        .entries()
        .into_iter()
        .map(|(question, value)| {
            (
                question.to_owned(),
                match value {
                    Attested::Known(ConditionValue::Integer(number)) => Value::Integer(*number),
                    Attested::Known(value) => Value::text(value.to_string()),
                    Attested::Unknown => Value::Null,
                },
            )
        })
        .collect();
    floor.push(("mcf".to_owned(), build_identity(conditions.mcf())));
    Value::map(floor)
}

#[must_use]
pub fn conditions_from(value: &Value) -> Option<Conditions> {
    let identity = value.get("mcf")?;
    let read = |question: &str| match value.get(question) {
        Some(Value::Text(text)) => Attested::Known(ConditionValue::text(text.clone())),
        Some(Value::Integer(number)) => Attested::Known(ConditionValue::Integer(*number)),
        _ => Attested::Unknown,
    };
    Some(Conditions::new(
        BuildIdentity {
            version: leaked(identity.get("version")?.as_text()?),
            revision: match identity.get("revision").and_then(Value::as_text) {
                Some(revision) => mcf_core::build_identity::SourceRevision::Known(leaked(revision)),
                None => mcf_core::build_identity::SourceRevision::Unknown,
            },
            rustc: leaked(identity.get("rustc")?.as_text()?),
            target: leaked(identity.get("target")?.as_text()?),
            profile: leaked(identity.get("profile")?.as_text()?),
        },
        Floor {
            hardware_state: read("hardware_state"),
            thermal_state: read("thermal_state"),
            driver_versions: read("driver_versions"),
            runtime_versions: read("runtime_versions"),
            quantization: read("quantization"),
            context_length: read("context_length"),
            batch_shape: read("batch_shape"),
            mcf_configuration: read("mcf_configuration"),
            realized_placement: read("realized_placement"),
            instrumentation: read("instrumentation"),
            artifact_storage: read("artifact_storage"),
            seed_set: read("seed_set"),
            reuse: read("reuse"),
        },
    ))
}

fn leaked(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

#[must_use]
pub fn measurement<Q: Quantity>(measured: &Measurement<Q>, as_integer: impl Fn(Q) -> i64) -> Value {
    let spread = measured.spread();
    Value::map([
        ("unit", Value::text(Q::UNIT)),
        (
            "n",
            Value::Integer(i64::try_from(measured.n()).unwrap_or(i64::MAX)),
        ),
        (
            "trials",
            Value::List(
                measured
                    .samples()
                    .map(&as_integer)
                    .map(Value::Integer)
                    .collect(),
            ),
        ),
        (
            "spread",
            Value::map([
                ("minimum", Value::Integer(as_integer(spread.minimum))),
                ("p5", Value::Integer(as_integer(spread.p5))),
                ("median", Value::Integer(as_integer(spread.median))),
                ("p95", Value::Integer(as_integer(spread.p95))),
                ("maximum", Value::Integer(as_integer(spread.maximum))),
            ]),
        ),
        ("conditions", conditions(measured.conditions())),
    ])
}

#[must_use]
pub fn trial<Q: Quantity>(trial: &Trial<Q>, as_integer: impl Fn(Q) -> i64) -> Value {
    Value::map([
        ("value", Value::Integer(as_integer(trial.value()))),
        ("unit", Value::text(Q::UNIT)),
        ("arm", Value::text(trial.arm().as_str())),
        ("position", Value::Integer(i64::from(trial.position().0))),
        ("session", Value::text(trial.session().as_str())),
        ("drew", draw(trial.drew())),
    ])
}

#[must_use]
pub fn draw(drew: &Draw) -> Value {
    match drew {
        Draw::Seeded { seed, from } => Value::map([
            ("discipline", Value::text("seeded")),
            ("seed", Value::text(seed.to_string())),
            ("from", Value::text(from.clone())),
            ("tokens", Value::Null),
        ]),
        Draw::LengthPinned { seed, tokens } => Value::map([
            ("discipline", Value::text("length_pinned")),
            ("seed", Value::text(seed.to_string())),
            ("from", Value::Null),
            ("tokens", Value::Integer(i64::from(*tokens))),
        ]),
    }
}

#[must_use]
pub fn trials<Q: Quantity>(
    trials: &Trials<Q>,
    conditions: &Conditions,
    as_integer: impl Fn(Q) -> i64 + Copy,
) -> Value {
    Value::map([
        (
            "trials",
            Value::List(
                trials
                    .all()
                    .iter()
                    .map(|one| trial(one, as_integer))
                    .collect(),
            ),
        ),
        ("conditions", self::conditions(conditions)),
    ])
}

#[must_use]
pub fn series<Q: Quantity>(series: &Series<Q>, as_integer: impl Fn(Q) -> i64) -> Value {
    let (points, thinning) = series.points();
    Value::map([
        ("unit", Value::text(Q::UNIT)),
        (
            "thinning_factor",
            Value::Integer(i64::from(thinning.factor())),
        ),
        (
            "full_resolution",
            Value::Bool(thinning.is_full_resolution()),
        ),
        (
            "points",
            Value::List(
                points
                    .iter()
                    .map(|point| Value::Integer(as_integer(*point)))
                    .collect(),
            ),
        ),
    ])
}

#[must_use]
pub fn contention(held: &mcf_core::hardware::Snapshot) -> Value {
    let fraction = |value: Attested<u64>| match value {
        Attested::Known(held) => Value::Integer(i64::try_from(held).unwrap_or(i64::MAX)),
        Attested::Unknown => Value::Null,
    };
    Value::map([
        (
            "over_ms",
            Value::Integer(i64::try_from(mcf_core::hardware::OVER.as_millis()).unwrap_or(i64::MAX)),
        ),
        (
            "cores_taken_thousandths",
            Value::Integer(i64::try_from(held.cores_taken).unwrap_or(i64::MAX)),
        ),
        ("processor_pressure_ppm", fraction(held.processor_pressure)),
        ("memory_pressure_ppm", fraction(held.memory_pressure)),
        ("storage_pressure_ppm", fraction(held.storage_pressure)),
        (
            "load_average_thousandths",
            match held.load {
                Attested::Known(load) => Value::Integer(i64::try_from(load.0).unwrap_or(i64::MAX)),
                Attested::Unknown => Value::Null,
            },
        ),
        (
            "accelerator_occupancy",
            match &held.accelerator {
                Attested::Known(said) => Value::text(said.clone()),
                Attested::Unknown => Value::Null,
            },
        ),
        (
            "competitors",
            Value::List(
                held.competitors
                    .iter()
                    .map(|one| {
                        Value::map([
                            ("pid", Value::Integer(i64::from(one.pid))),
                            ("command", Value::text(one.command.clone())),
                            (
                                "cores_taken_thousandths",
                                Value::Integer(i64::try_from(one.cores_taken).unwrap_or(i64::MAX)),
                            ),
                            ("is_mcf", Value::Bool(one.is_mcf)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

#[must_use]
pub fn machine(machine: &Machine) -> Value {
    Value::map([
        (
            "processor",
            Value::map([
                (
                    "model",
                    attested(&machine.processor.model, |m| Value::text(m.clone())),
                ),
                (
                    "cores",
                    attested(&machine.processor.cores, |c| Value::Integer(i64::from(*c))),
                ),
                (
                    "threads",
                    attested(&machine.processor.threads, |t| {
                        Value::Integer(i64::from(*t))
                    }),
                ),
            ]),
        ),
        (
            "memory",
            Value::map([
                (
                    "total_bytes",
                    attested(&machine.memory.total, |b| {
                        Value::Integer(i64::try_from(b.0).unwrap_or(i64::MAX))
                    }),
                ),
                (
                    "available_bytes",
                    attested(&machine.memory.available, |b| {
                        Value::Integer(i64::try_from(b.0).unwrap_or(i64::MAX))
                    }),
                ),
            ]),
        ),
        (
            "accelerators",
            Value::List(machine.accelerators.iter().map(accelerator).collect()),
        ),
        (
            "power_profile",
            attested(&machine.power_profile, |p| Value::text(p.as_str())),
        ),
    ])
}

#[must_use]
pub fn accelerator(device: &Accelerator) -> Value {
    let reading = device.reading();
    let (state, missing) = match device.characterization() {
        Characterization::Characterized => ("characterized", Value::List(Vec::new())),
        Characterization::AttemptedUncharacterized { missing } => (
            "attempted_uncharacterized",
            Value::List(
                missing
                    .into_iter()
                    .map(|what| Value::text(what.as_str()))
                    .collect(),
            ),
        ),
    };
    Value::map([
        (
            "index",
            Value::Integer(i64::try_from(device.index()).unwrap_or(i64::MAX)),
        ),
        (
            "vendor",
            attested(&reading.vendor, |v| Value::text(v.clone())),
        ),
        (
            "model",
            attested(&reading.model, |v| Value::text(v.clone())),
        ),
        (
            "driver",
            attested(&reading.driver, |v| Value::text(v.clone())),
        ),
        (
            "runtime",
            attested(&reading.runtime, |v| Value::text(v.clone())),
        ),
        (
            "memory_total_bytes",
            attested(&reading.memory_total, |b| {
                Value::Integer(i64::try_from(b.0).unwrap_or(i64::MAX))
            }),
        ),
        (
            "memory_available_bytes",
            attested(&reading.memory_available, |b| {
                Value::Integer(i64::try_from(b.0).unwrap_or(i64::MAX))
            }),
        ),
        (
            "temperature_c",
            attested(&reading.temperature_c, |t| Value::Integer(i64::from(*t))),
        ),
        ("characterization", Value::text(state)),
        ("missing_readings", missing),
        (
            "routes",
            Value::List(device.routes().iter().map(|r| Value::text(*r)).collect()),
        ),
        (
            "routes_disagree_about",
            Value::List(
                device
                    .disagreements()
                    .iter()
                    .map(|field| Value::text(*field))
                    .collect(),
            ),
        ),
    ])
}

fn attested<T>(value: &Attested<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    match value {
        Attested::Known(value) => encode(value),
        Attested::Unknown => Value::Null,
    }
}

fn attested_text(rendered: &str) -> Value {
    if rendered == "unknown" {
        Value::Null
    } else {
        Value::text(rendered)
    }
}
