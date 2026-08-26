//! Turning what MCF knows into what the record holds.
//!
//! The encoders live here rather than beside the types they encode, and the
//! layering is the reason: `mcf-core` holds the types every rule is enforced
//! through and depends on nothing, so it cannot know about a serialization.
//! The record crate is the one that knows how to *record* things, which is what
//! it is for.
//!
//! Two rules shape every encoder below.
//!
//! **Unknown is `null`, never a substitute** (A7). A condition MCF could not
//! read, a device whose memory it could not query, a licence it did not find —
//! each is `null` in the record, and a reader can tell the difference between
//! *nothing was there* and *nothing was looked for* only because MCF never
//! writes a plausible value into either.
//!
//! **A measurement never loses its conditions** (A6). There is no encoder that
//! writes a value without them; the only way to put a measured quantity in the
//! record is through [`measurement`], and it writes the samples, the count, the
//! spread and the condition set together.

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::degradation::{Degradation, Degraded};
use mcf_core::failure::Failure;
use mcf_core::hardware::{Accelerator, Characterization, Machine};
use mcf_core::measurement::{ConditionValue, Conditions, Measurement, Quantity};
use mcf_core::provenance::{
    Checksum, Licence, Origin, Provenance, ToolIdentity, Transformation, TransformationKind,
};
use mcf_core::time::Timestamp;
use mcf_core::trial::{Series, Trial, Trials};

use crate::json::Value;

/// What built the running binary (§3.4, §3.12).
#[must_use]
pub fn build_identity(identity: BuildIdentity) -> Value {
    Value::map([
        ("version", Value::text(identity.version)),
        ("revision", attested_text(&identity.revision.to_string())),
        ("rustc", Value::text(identity.rustc)),
        ("target", Value::text(identity.target)),
        ("profile", Value::text(identity.profile)),
    ])
}

/// A classified failure, with everything the laboratory needs to rebuild it
/// (A2, B21).
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

/// What was lost, and why (A5).
#[must_use]
pub fn degradation(degradation: &Degradation) -> Value {
    Value::List(degradation.causes().iter().map(failure).collect())
}

/// A degraded value, with its mark attached to it in the record as well as in
/// the type.
///
/// The mark is a sibling key rather than a wrapper, so no reader can take the
/// value without seeing it — a nested value could be lifted out by a query that
/// did not know to look one level up.
#[must_use]
pub fn degraded<T>(value: &Degraded<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    Value::map([
        ("degraded", Value::Bool(true)),
        ("degradation", degradation(value.degradation())),
        ("value", encode(value.value())),
    ])
}

/// Where an artifact came from, and everything that happened to it since
/// (§3.6, B-006).
///
/// The chain is written whole rather than summarized. §XII's hard case is a
/// requantization of somebody else's weights, and a record that kept only the
/// nearest repository would be a record that cannot answer *what were these
/// originally* — which is the question the chain exists for (A1).
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
            "derived_from",
            match provenance.source() {
                Some(source) => self::provenance(source),
                None => Value::Null,
            },
        ),
    ])
}

/// Where bytes came from.
///
/// The variant is named in the record rather than inferred from which fields
/// are present: *a local file* and *nobody can say* are different answers, and
/// a reader that had to deduce which one it was holding would deduce wrongly
/// the first time a field went missing for another reason (A7, A9).
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
        // `Origin` is non-exhaustive; anything added later is recorded as what
        // MCF can say about it rather than silently as `unattributed`, which
        // would be a claim.
        Origin::Unattributed => Value::map([("kind", Value::text("unattributed"))]),
        other => Value::map([
            ("kind", Value::text("unrecorded")),
            ("rendered", Value::text(other.to_string())),
        ]),
    }
}

/// A digest of an artifact's bytes, and what computed it.
#[must_use]
pub fn checksum(checksum: &Checksum) -> Value {
    Value::map([
        ("algorithm", Value::text(checksum.algorithm().as_str())),
        ("hex", Value::text(checksum.hex())),
    ])
}

/// What an artifact's terms are, in the three states B-023 keeps apart.
#[must_use]
pub fn licence(licence: &Licence) -> Value {
    match licence {
        Licence::Spdx(identifier) => Value::map([
            ("state", Value::text("identified")),
            ("identifier", Value::text(identifier.clone())),
        ]),
        // Terms are present and MCF could not name them. Distinct from the
        // absent case, which is `null` because the whole field is `Unknown`.
        Licence::Stated => Value::map([("state", Value::text("stated_and_unmatched"))]),
        other => Value::map([
            ("state", Value::text("unrecorded")),
            ("rendered", Value::text(other.to_string())),
        ]),
    }
}

/// One thing that was done to an artifact.
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
        // The operator's own words, kept as they were given: a kind MCF has no
        // name for is recorded as what it was called (A7).
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

/// A moment, as nanoseconds and as something a person can read.
///
/// Both, because they answer different questions and neither is derivable in
/// this record's absence: the integer is what a reader compares and the
/// rendering is what a person checks against their own memory of the day. The
/// integer is text when it does not fit in one — a `Timestamp` holds more range
/// than JSON's integers do, and A1 puts the whole value above the tidier type.
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

/// The conditions a measurement is bound to (§3.4).
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
                    // A condition keeps the shape it was read in. Rendering
                    // an integer through `Display` was the first defect the
                    // property tier found (B-191): a context length written as
                    // `"4096"` read back as text, so B-007's *round-trips
                    // losslessly* held for every floor question but the one
                    // that is naturally a number. §3.3 asks the
                    // record be machine-readable first, and a number a reader
                    // has to re-parse from a string is not that.
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

/// A measurement: its trials, its count, its spread and its conditions.
///
/// B56 keeps the trials and derives the summary, and this writes both — the
/// trials because they are the record, and the spread because a reader that
/// had to recompute it might compute a different one. The summary is written
/// *beside* the trials it came from and never instead of them, which is the
/// distinction B56 draws.
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

/// One trial: the row D16 makes the record.
///
/// Everything a later question needs, and nothing derived. B56's violation is a
/// stored mean; the shape that prevents it is that this is what gets stored and
/// a summary is projected from a set of these when somebody asks.
#[must_use]
pub fn trial<Q: Quantity>(trial: &Trial<Q>, as_integer: impl Fn(Q) -> i64) -> Value {
    Value::map([
        ("value", Value::Integer(as_integer(trial.value()))),
        ("unit", Value::text(Q::UNIT)),
        ("arm", Value::text(trial.arm().as_str())),
        ("position", Value::Integer(i64::from(trial.position().0))),
        ("session", Value::text(trial.session().as_str())),
    ])
}

/// A session's trials, and the conditions they were taken under.
///
/// The conditions are written once for the set rather than repeated on every
/// row: they are conditions *of the session*, and repeating them would invite
/// a reader to believe two rows could disagree about them.
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

/// Interior detail, with what was done to it (B-271).
///
/// The thinning factor is a sibling of the points rather than a wrapper around
/// them, for the reason [`degraded`] gives about a degradation mark: a nested
/// value can be lifted out by a query that did not know to look one level up,
/// and a series read without its factor is a resolution claim nobody made.
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

/// The machine, as read at this moment (§3.8).
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

/// One accelerator, with D25's verdict and what it rests on.
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

/// An attested value, or `null` (A7).
fn attested<T>(value: &Attested<T>, encode: impl FnOnce(&T) -> Value) -> Value {
    match value {
        Attested::Known(value) => encode(value),
        Attested::Unknown => Value::Null,
    }
}

/// Text that may itself read as `unknown`, written as `null` when it does.
fn attested_text(rendered: &str) -> Value {
    if rendered == "unknown" {
        Value::Null
    } else {
        Value::text(rendered)
    }
}
