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
use mcf_core::measurement::{Conditions, Measurement, Quantity};

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
