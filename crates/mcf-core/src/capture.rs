use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::configuration::Configuration;
use crate::hardware::{Characterization, Machine, storage_of};
use crate::measurement::{ConditionValue, Conditions, Floor};

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
        batch_shape: Attested::Unknown,
        mcf_configuration: Attested::Known(ConditionValue::text(mcf_configuration)),
        realized_placement: Attested::Unknown,
        instrumentation: Attested::Known(ConditionValue::text(instrumentation)),
        artifact_storage: artifact.map_or(Attested::Unknown, |path| match storage_of(path) {
            Attested::Known(storage) => Attested::Known(ConditionValue::text(storage.to_string())),
            Attested::Unknown => Attested::Unknown,
        }),
        seed_set: Attested::Unknown,
        reuse: Attested::Unknown,
    }
}

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

fn describe_thermal(machine: &Machine) -> Option<String> {
    let sensors = crate::hardware::thermal::sensors();
    let mut described: Vec<String> = Vec::new();
    if let Some(found) = crate::hardware::thermal::processor(&sensors) {
        described.push(format!("processor {found}"));
    }
    described.extend(machine.accelerators.iter().filter_map(|device| {
        device
            .reading()
            .temperature_c
            .known()
            .map(|celsius| format!("accel#{} {celsius} °C", device.index()))
    }));
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
