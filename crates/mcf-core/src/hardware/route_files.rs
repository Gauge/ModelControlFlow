use std::path::Path;

use crate::attested::Attested;

use super::accelerator::{Missing, Reading, Route};

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
