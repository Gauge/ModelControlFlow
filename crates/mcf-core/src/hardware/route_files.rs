//! The route that reads what a driver publishes as files.
//!
//! No `unsafe`, no linkage, no vendor library — and, as F1 established, no
//! live state. This route supplies a device's identity and its driver version
//! and stops, which under D25 means a machine on which it is the only working
//! route reports every device as *attempted, uncharacterized*. That is the
//! honest outcome and it is stated in [`Route::covers`] rather than discovered
//! from what the route happens to return.
//!
//! One vendor is read, and the narrowness is deliberate rather than hidden:
//! adding a vendor is adding a route (D25), and a route that pretended to be
//! general by guessing at unfamiliar files would be inventing readings, which
//! A7 forbids.
//!
//! **Cross-checked by:** the vendor-library route in `nvml`, which reads the
//! same devices by another means; `accelerator` compares them and records
//! where they differ.

use std::path::Path;

use crate::attested::Attested;

use super::accelerator::{Missing, Reading, Route};

/// Reads `/proc/driver/nvidia`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Files;

impl Route for Files {
    fn name(&self) -> &'static str {
        "driver-files"
    }

    fn covers(&self) -> &'static [Missing] {
        &[Missing::Identity, Missing::Versions]
    }

    fn probe(&self) -> Vec<Reading> {
        const ROOT: &str = "/proc/driver/nvidia/gpus";
        let Ok(entries) = std::fs::read_dir(ROOT) else {
            return Vec::new();
        };
        let driver = driver_version();

        let mut devices: Vec<std::path::PathBuf> =
            entries.flatten().map(|entry| entry.path()).collect();
        // The directory order is the filesystem's, which is not stable. Sorted
        // by bus location so that device #0 is the same device between runs —
        // an index that moved would make two readings of one machine look like
        // readings of two.
        devices.sort();

        devices
            .into_iter()
            .map(|device| Reading {
                vendor: Attested::Known("NVIDIA".to_owned()),
                model: attest(field(&device.join("information"), "Model:")),
                driver: attest(driver.clone()),
                ..Reading::default()
            })
            .collect()
    }
}

fn field(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix(key))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// `NVRM version: NVIDIA UNIX x86_64 Kernel Module  610.57.04  …`
fn driver_version() -> Option<String> {
    let text = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    text.lines().next()?.split_whitespace().find_map(|token| {
        let looks_like_a_version = token.contains('.')
            && token.chars().all(|c| c.is_ascii_digit() || c == '.')
            && token.chars().next().is_some_and(|c| c.is_ascii_digit());
        looks_like_a_version.then(|| token.to_owned())
    })
}

fn attest<T>(value: Option<T>) -> Attested<T> {
    match value {
        Some(value) => Attested::Known(value),
        None => Attested::Unknown,
    }
}
