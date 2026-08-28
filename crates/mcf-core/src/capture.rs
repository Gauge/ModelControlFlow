//! Filling in the §3.3 floor from a live machine.
//!
//! §3.3 fixes what varies and could change a result, and calls it a floor that
//! *does not shrink under §VII*. [`Floor`] is that list as a type; this module
//! is what fills it in from what MCF can actually read at the moment a
//! measurement is taken.
//!
//! **Captured at measurement time, never harvested from a stream.** §3.3 is
//! explicit: the floor is *captured deliberately at measurement time rather
//! than harvested from a continuous stream*, which is what lets B4 refuse
//! ambient telemetry without costing §II anything. So this is a function
//! somebody calls when they take a reading, and there is nothing here that
//! runs on its own.
//!
//! **What is read and what stays unknown.** Four of the eleven come from the
//! machine profile and are as good as the profiler is (B-013, D25). One is
//! MCF's own configuration, which MCF always knows, and one is the storage the
//! artifact under measurement was read from (B-193). The remaining
//! four — quantization, context length, batch shape, realized placement —
//! describe a *model being run*, and at M0 nothing runs a model, so they stay
//! [`Attested::Unknown`] rather than being filled with something plausible
//! (A7). A measurement taken now says so, which is a weak claim honestly made
//! rather than a strong one that lies.

use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::configuration::Configuration;
use crate::hardware::{Characterization, Machine, storage_of};
use crate::measurement::{ConditionValue, Conditions, Floor};

/// Captures the floor from a machine and, where there is one, a configuration.
///
/// `configuration` is what is being run. At M0 there is never one; the argument
/// exists because the four fields it supplies are part of the floor whether or
/// not anything can supply them yet, and a capture function that grew the
/// argument later would be a capture function that silently omitted them until
/// then.
///
/// `artifact` is the file the measurement is *about* — at M0 that is MCF's own
/// binary, since nothing else is measured yet, and from M1 it is the model. The
/// storage it lives on is a condition because F5 measured it changing a figure
/// by three orders of magnitude (B-193). `None` where the caller does not know,
/// which stays unknown rather than becoming the current directory's storage.
#[must_use]
pub fn floor(
    machine: &Machine,
    configuration: Option<&Configuration>,
    mcf_configuration: &str,
    instrumentation: &str,
    artifact: Option<&std::path::Path>,
) -> Floor {
    Floor {
        hardware_state: known(describe_hardware(machine)),
        thermal_state: known(describe_thermal(machine)),
        driver_versions: known(versions(machine, |reading| reading.driver.clone())),
        runtime_versions: known(versions(machine, |reading| reading.runtime.clone())),
        quantization: configuration
            .map_or(Attested::Unknown, |c| attest_text(c.quantization.known())),
        context_length: configuration.map_or(Attested::Unknown, |c| {
            match c.context_length.known() {
                Some(length) => Attested::Known(ConditionValue::integer(i64::from(*length))),
                None => Attested::Unknown,
            }
        }),
        // Batch shape is not part of a configuration's identity (D17) and is
        // set by whatever drives the engine, which does not exist yet.
        batch_shape: Attested::Unknown,
        mcf_configuration: Attested::Known(ConditionValue::text(mcf_configuration)),
        // The realized layout is what the machine did with the declared intent
        // (intent v16), and nothing has realized one yet.
        realized_placement: Attested::Unknown,
        instrumentation: Attested::Known(ConditionValue::text(instrumentation)),
        artifact_storage: artifact.map_or(Attested::Unknown, |path| match storage_of(path) {
            Attested::Known(storage) => Attested::Known(ConditionValue::text(storage.to_string())),
            Attested::Unknown => Attested::Unknown,
        }),
        // Which seed set a run drew from is a property of the run, not of the
        // machine, and this function reads the machine. A run that took seeded
        // trials fills it in from its own `SeedSet`; a timing laboratory has
        // no answer to give, because D19 has it hold the seed still and pin the
        // generation length instead (B-290).
        seed_set: Attested::Unknown,
        // What a measurement reused is a property of the run, not of the
        // machine, and this function reads the machine. A benchmark fills it
        // in from what the engine said about each trial (B-081, §6.13).
        reuse: Attested::Unknown,
    }
}

/// The conditions in force now: the floor, bound to the instrument.
#[must_use]
pub fn conditions(
    machine: &Machine,
    configuration: Option<&Configuration>,
    mcf_configuration: &str,
    instrumentation: &str,
    artifact: Option<&std::path::Path>,
) -> Conditions {
    Conditions::new(
        BuildIdentity::current(),
        floor(
            machine,
            configuration,
            mcf_configuration,
            instrumentation,
            artifact,
        ),
    )
}

/// What the machine is, in one line a record can hold and a person can read.
///
/// Every accelerator's characterization is in here, because D25 makes that a
/// per-run verdict: a measurement taken while a device was uncharacterized is
/// not comparable with one taken while it was, and the condition is where that
/// becomes visible.
fn describe_hardware(machine: &Machine) -> Option<String> {
    let processor = machine.processor.model.known()?;
    let mut parts = vec![format!(
        "{processor}, {} threads",
        machine.processor.threads
    )];
    if machine.accelerators.is_empty() {
        parts.push("no accelerator".to_owned());
    }
    for device in &machine.accelerators {
        let state = match device.characterization() {
            Characterization::Characterized => "characterized".to_owned(),
            Characterization::AttemptedUncharacterized { missing } => {
                let names: Vec<&str> = missing.iter().map(|m| m.as_str()).collect();
                format!("uncharacterized, missing {}", names.join("/"))
            }
        };
        parts.push(format!(
            "accel#{} {} ({state})",
            device.index(),
            device.reading().model
        ));
    }
    parts.push(format!("load {}", machine.load));
    Some(parts.join(", "))
}

/// The thermal state, which is only what the accelerators report.
///
/// The processor's own temperature is not read: no route supplies it yet, and
/// A7 forbids reporting the accelerator's as though it were the machine's.
fn describe_thermal(machine: &Machine) -> Option<String> {
    let described: Vec<String> = machine
        .accelerators
        .iter()
        .filter_map(|device| {
            device
                .reading()
                .temperature_c
                .known()
                .map(|celsius| format!("accel#{} {celsius} °C", device.index()))
        })
        .collect();
    if described.is_empty() {
        None
    } else {
        Some(described.join(", "))
    }
}

fn versions(
    machine: &Machine,
    of: impl Fn(&crate::hardware::Reading) -> Attested<String>,
) -> Option<String> {
    let described: Vec<String> = machine
        .accelerators
        .iter()
        .enumerate()
        .filter_map(|(index, device)| {
            of(device.reading())
                .known()
                .map(|version| format!("accel#{index} {version}"))
        })
        .collect();
    if described.is_empty() {
        None
    } else {
        Some(described.join(", "))
    }
}

fn known(described: Option<String>) -> Attested<ConditionValue> {
    match described {
        Some(text) => Attested::Known(ConditionValue::text(text)),
        None => Attested::Unknown,
    }
}

fn attest_text<T: core::fmt::Display>(value: Option<&T>) -> Attested<ConditionValue> {
    match value {
        Some(value) => Attested::Known(ConditionValue::text(value.to_string())),
        None => Attested::Unknown,
    }
}

#[cfg(test)]
mod tests;
