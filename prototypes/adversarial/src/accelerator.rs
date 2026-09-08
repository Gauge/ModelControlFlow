use std::path::Path;

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-prototype::accelerator");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) route: &'static str,
    pub(crate) vendor: Attested<String>,
    pub(crate) model: Attested<String>,
    pub(crate) driver: Attested<String>,
    pub(crate) memory_bytes: Attested<u64>,
    pub(crate) temperature_c: Attested<u32>,
}

impl Reading {
    fn nothing_known(route: &'static str) -> Self {
        Self {
            route,
            vendor: Attested::Unknown,
            model: Attested::Unknown,
            driver: Attested::Unknown,
            memory_bytes: Attested::Unknown,
            temperature_c: Attested::Unknown,
        }
    }

    #[must_use]
    pub(crate) fn entries(&self) -> [(&'static str, String); 5] {
        [
            ("vendor", self.vendor.to_string()),
            ("model", self.model.to_string()),
            ("driver", self.driver.to_string()),
            ("memory_bytes", self.memory_bytes.to_string()),
            ("temperature_c", self.temperature_c.to_string()),
        ]
    }

    #[must_use]
    pub(crate) fn answered(&self) -> usize {
        self.entries()
            .iter()
            .filter(|(_, value)| value != "unknown")
            .count()
    }
}

#[derive(Debug)]
pub(crate) enum Probe {
    Found(Reading),
    Nothing(Failure),
}

#[must_use]
pub(crate) fn by_file() -> Probe {
    const ROOT: &str = "/proc/driver/nvidia/gpus";

    let Ok(entries) = std::fs::read_dir(ROOT) else {
        return Probe::Nothing(
            Failure::new(
                Category::AccelAbsent,
                Attribution::Machine,
                Disposition::Degraded,
                WHERE,
                "no accelerator publishes state where this route looks",
            )
            .with_context("route", "file")
            .with_context("looked_in", ROOT),
        );
    };

    let Some(device) = entries.flatten().map(|entry| entry.path()).next() else {
        return Probe::Nothing(
            Failure::new(
                Category::AccelAbsent,
                Attribution::Machine,
                Disposition::Degraded,
                WHERE,
                "the driver is loaded and publishes no devices",
            )
            .with_context("route", "file")
            .with_context("looked_in", ROOT),
        );
    };

    let mut reading = Reading::nothing_known("file");
    reading.vendor = Attested::Known("NVIDIA".to_owned());
    if let Some(model) = field(&device.join("information"), "Model:") {
        reading.model = Attested::Known(model);
    }
    if let Some(version) = driver_version() {
        reading.driver = Attested::Known(version);
    }
    Probe::Found(reading)
}

fn field(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .map(|value| value.trim().to_owned())
}

fn driver_version() -> Option<String> {
    let text = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    let first = text.lines().next()?;
    first
        .split_whitespace()
        .find(|token| {
            token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.')
        })
        .map(str::to_owned)
}

#[must_use]
pub(crate) fn by_vendor_library() -> Probe {
    match crate::nvml::probe() {
        Ok(reading) => Probe::Found(reading),
        Err(failure) => Probe::Nothing(failure),
    }
}

#[cfg(test)]
mod tests {
    use super::{Probe, by_file, by_vendor_library};
    use mcf_core::failure::{Attribution, Category};

    fn assert_well_formed(probe: &Probe) {
        match probe {
            Probe::Found(reading) => {
                assert!(!reading.route.is_empty());
                assert!(
                    reading.answered() >= 1,
                    "a device was found and nothing was read about it"
                );
                for (question, value) in reading.entries() {
                    assert!(!value.is_empty(), "{question} rendered as nothing");
                }
            }
            Probe::Nothing(failure) => {
                assert_eq!(failure.attribution(), Attribution::Machine);
                assert!(
                    matches!(
                        failure.category(),
                        Category::AccelAbsent
                            | Category::AccelDriverAbsent
                            | Category::AccelDriverQueryFailed
                            | Category::AccelDriverVersionMismatch
                            | Category::AccelUnrecognized
                    ),
                    "an accelerator probe failed with {}",
                    failure.category()
                );
                assert!(
                    !failure.context().is_empty(),
                    "the failure carries nothing to reconstruct it from (B21)"
                );
            }
        }
    }

    #[test]
    fn the_file_route_reaches_a_defined_outcome() {
        assert_well_formed(&by_file());
    }

    #[test]
    fn the_vendor_library_route_reaches_a_defined_outcome() {
        assert_well_formed(&by_vendor_library());
    }

    #[test]
    fn the_file_route_does_not_invent_live_state() {
        if let Probe::Found(reading) = by_file() {
            assert!(
                !reading.memory_bytes.is_known(),
                "the file route claimed to know device memory"
            );
            assert!(
                !reading.temperature_c.is_known(),
                "the file route claimed to know device temperature"
            );
        }
    }

    #[test]
    fn the_two_routes_do_not_contradict_each_other() {
        let (Probe::Found(from_file), Probe::Found(from_library)) =
            (by_file(), by_vendor_library())
        else {
            return;
        };
        if from_file.model.is_known() && from_library.model.is_known() {
            assert_eq!(from_file.model, from_library.model);
        }
        if from_file.driver.is_known() && from_library.driver.is_known() {
            assert_eq!(from_file.driver, from_library.driver);
        }
    }
}
