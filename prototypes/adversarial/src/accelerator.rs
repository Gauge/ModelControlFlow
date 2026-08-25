//! Interrogating an accelerator, twice, by two different routes.
//!
//! §7.19 asks whether the substrate is right, and D4's third argument is that
//! "hardware probing, accelerator interrogation and driving inference engines
//! are constant C-ABI work, and Rust pays no tax at that boundary". A
//! prototype that only read files would not have tested that claim, so this
//! module probes the same device both ways and reports what each route yields:
//!
//! * [`by_file`] reads what the driver publishes under `/proc` and `/sys`. No
//!   `unsafe`, no vendor library, no linkage — and no live state: the files
//!   name the device and the driver, not what the device is doing.
//! * [`by_vendor_library`] loads the vendor's management library at runtime
//!   and asks it. This is the C-ABI boundary D4 is arguing about, and it is
//!   where live memory and thermal state actually come from.
//!
//! **Neither is adopted here.** B-013 builds the profiler, DEC-008 decides
//! which hardware is characterized versus attempted-and-uncharacterized, and
//! this prototype exists to give both of those something measured to reason
//! from. What it produces is evidence, and every reading is `Attested` (A7).
//!
//! **The vendor library is loaded, never shipped.** D23's second tier —
//! platform-provided — was deferred in intent v23 for *inference*, on the
//! ground that an unpinned runtime is an unpinned variable in every result
//! taken through it. Reading a driver's own report of its own state is a
//! different act from computing through it: nothing measured *passes through*
//! this library, so there is no result for it to be an unpinned variable in.
//! Whether that distinction survives is a question for DEC-008, and it is
//! recorded here rather than assumed.

use std::path::Path;

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-prototype::accelerator");

/// What one probe route learned about one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    /// How this was obtained.
    pub(crate) route: &'static str,
    /// The vendor, if the route says.
    pub(crate) vendor: Attested<String>,
    /// The model, if the route says.
    pub(crate) model: Attested<String>,
    /// The driver version, if the route says.
    pub(crate) driver: Attested<String>,
    /// Total device memory in bytes, if the route says.
    pub(crate) memory_bytes: Attested<u64>,
    /// Device temperature in degrees Celsius, if the route says.
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

    /// Each field, paired with the question it answers.
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

    /// How many of the five questions this route answered.
    #[must_use]
    pub(crate) fn answered(&self) -> usize {
        self.entries()
            .iter()
            .filter(|(_, value)| value != "unknown")
            .count()
    }
}

/// What a probe route returned.
#[derive(Debug)]
pub(crate) enum Probe {
    /// The route found a device and read something about it.
    Found(Reading),
    /// The route ran and found nothing, or could not run.
    ///
    /// A9: this is a result. "No accelerator is present" and "the driver is
    /// there and would not answer" are both things MCF should be able to say.
    Nothing(Failure),
}

/// Reads what the driver publishes as files.
///
/// The NVIDIA driver publishes one directory per device under
/// `/proc/driver/nvidia/gpus/`, whose `information` file names the model, the
/// bus location and the firmware. The driver's own version is in
/// `/proc/driver/nvidia/version`.
///
/// Only this vendor is read, and that is stated rather than hidden: the
/// prototype's job is to find out what one route costs, not to be a profiler.
/// B-013 generalizes, DEC-008 decides what "characterized" means, and until
/// then an unrecognized device is `accel.unrecognized` rather than a guess
/// (A7).
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
    // Memory and temperature are deliberately left unknown. They are not in
    // these files, and A7 forbids the plausible substitute — which is the
    // finding this route exists to produce.
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
    // `NVRM version: NVIDIA UNIX x86_64 Kernel Module  610.57.04  …`
    let first = text.lines().next()?;
    first
        .split_whitespace()
        .find(|token| {
            token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.')
        })
        .map(str::to_owned)
}

/// Asks the vendor's management library, over the C ABI.
///
/// See [`crate::nvml`] for the boundary itself. This wrapper exists so that
/// the two routes have the same shape and the same failure vocabulary, which
/// is what lets the prototype's report compare them.
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

    /// B19: the suite runs on a machine with no accelerator. Both outcomes are
    /// correct answers, and what is asserted is that whichever one occurs is
    /// well-formed — A9 makes "there is no accelerator here" a result rather
    /// than a gap in the test.
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

    /// A7: what a route cannot read stays unknown. The file route publishes no
    /// live state, and the honest reading of that is four fields of which two
    /// are `unknown` — never a plausible substitute for memory or temperature.
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

    /// The two routes agree about what they both claim to know. A disagreement
    /// would be a finding about one of them, and it is worth failing on rather
    /// than reporting quietly (A8: two readings of one thing that differ are
    /// not two data points).
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
