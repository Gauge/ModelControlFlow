use std::path::{Path, PathBuf};

use crate::attested::Attested;
use crate::measurement::Bytes;

use super::accelerator::{Missing, Reading, Route};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Amdgpu {
    pub drm: PathBuf,
    pub pci_ids: PathBuf,
    pub icds: PathBuf,
    pub kernel: PathBuf,
}

impl Default for Amdgpu {
    fn default() -> Self {
        Self {
            drm: PathBuf::from("/sys/class/drm"),
            pci_ids: PathBuf::from("/usr/share/misc/pci.ids"),
            icds: PathBuf::from("/usr/share/vulkan/icd.d"),
            kernel: PathBuf::from("/proc/sys/kernel/osrelease"),
        }
    }
}

impl Route for Amdgpu {
    fn name(&self) -> &'static str {
        "amdgpu-files"
    }

    fn covers(&self) -> &'static [Missing] {
        &[
            Missing::Identity,
            Missing::Versions,
            Missing::Memory,
            Missing::Thermal,
        ]
    }

    fn probe(&self) -> Vec<Reading> {
        let Ok(entries) = std::fs::read_dir(&self.drm) else {
            return Vec::new();
        };
        let mut cards: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("card") && !name.contains('-'))
            })
            .filter(|path| {
                std::fs::read_link(path.join("device").join("driver")).is_ok_and(|driver| {
                    driver.file_name().and_then(|name| name.to_str()) == Some("amdgpu")
                })
            })
            .collect();
        cards.sort();
        let kernel = read_trimmed(&self.kernel);
        let table = std::fs::read_to_string(&self.pci_ids).ok();
        let runtime = std::fs::metadata(self.icds.join("radeon_icd.json"))
            .is_ok()
            .then(|| "Vulkan, through the radeon driver the loader lists".to_owned());
        cards
            .iter()
            .map(|card| {
                let device = card.join("device");
                let vendor = read_trimmed(&device.join("vendor"));
                let product = read_trimmed(&device.join("device"));
                let model = match (&vendor, &product) {
                    (Some(vendor), Some(product)) => Some(
                        table
                            .as_deref()
                            .and_then(|table| named_in(table, vendor, product))
                            .unwrap_or_else(|| format!("PCI device {vendor}:{product}")),
                    ),
                    _ => None,
                };
                let (total, used) = memory_of(&device);
                Reading {
                    vendor: Attested::Known("AMD".to_owned()),
                    model: attest(model),
                    driver: attest(
                        kernel
                            .as_ref()
                            .map(|kernel| format!("amdgpu, kernel {kernel}")),
                    ),
                    runtime: attest(runtime.clone()),
                    memory_total: attest(total.map(Bytes)),
                    memory_available: attest(
                        total
                            .zip(used)
                            .map(|(total, used)| Bytes(total.saturating_sub(used))),
                    ),
                    temperature_c: attest(temperature_of(&device)),
                }
            })
            .collect()
    }
}

fn memory_of(device: &Path) -> (Option<u64>, Option<u64>) {
    let figure = |name: &str| read_trimmed(&device.join(name)).and_then(|text| text.parse().ok());
    let carves_out = std::fs::metadata(device.join("uma")).is_ok();
    if carves_out {
        (figure("mem_info_gtt_total"), figure("mem_info_gtt_used"))
    } else {
        (figure("mem_info_vram_total"), figure("mem_info_vram_used"))
    }
}

fn temperature_of(device: &Path) -> Option<u32> {
    let hwmon = std::fs::read_dir(device.join("hwmon")).ok()?;
    let mut sensors: Vec<PathBuf> = hwmon.flatten().map(|entry| entry.path()).collect();
    sensors.sort();
    sensors.into_iter().find_map(|sensor| {
        let millidegrees: u32 = read_trimmed(&sensor.join("temp1_input"))?.parse().ok()?;
        millidegrees.checked_div(1000)
    })
}

pub(super) fn named_in(table: &str, vendor: &str, product: &str) -> Option<String> {
    let vendor = vendor.trim_start_matches("0x").to_ascii_lowercase();
    let product = product.trim_start_matches("0x").to_ascii_lowercase();
    let mut inside = false;
    for line in table.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if !line.starts_with('\t') {
            inside = line
                .split_whitespace()
                .next()
                .is_some_and(|id| id.eq_ignore_ascii_case(&vendor));
            continue;
        }
        if inside && !line.starts_with("\t\t") {
            let mut parts = line.trim().splitn(2, "  ");
            let id = parts.next().unwrap_or_default();
            if id.eq_ignore_ascii_case(&product) {
                return parts.next().map(|name| name.trim().to_owned());
            }
        }
    }
    None
}

fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn attest<T>(value: Option<T>) -> Attested<T> {
    value.map_or(Attested::Unknown, Attested::Known)
}
