//! The route that reads what the `amdgpu` driver publishes as files.
//!
//! **A second vendor, as D25 said one is added: as a route.** The first two
//! routes read NVIDIA, and a machine with a Radeon reported *accelerators:
//! none present* while reading that same Radeon's temperature two lines
//! down — the sensor module knew the card and the profiler did not. What was
//! wrong was not a guess but a silence, and A7 forbids the silence as much as
//! the guess: a card the kernel names is a card MCF has to name.
//!
//! **Everything here is a file the driver wrote.** Identity is the PCI id the
//! kernel publishes and the name the system's own `pci.ids` gives that id —
//! a lookup, not an inference; a machine without the table gets the id.
//! Memory is what the driver reports for the card's own memory and for the
//! system memory it may address, and which of the two a model lands in is
//! the driver's `uma` group to say: a chip that carves its memory out of the
//! system's has the larger pool, and one with memory of its own has that.
//! Thermal is the driver's hwmon. The runtime is the Vulkan driver the
//! system installed for it, where the loader's table names one — which is
//! also what an engine built with a Vulkan back end will find at start.
//!
//! **Rooted rather than absolute**, so a test can lay out the files a driver
//! writes and read them back on a machine with no card at all (B-015).
//!
//! **Cross-checked by test:** `a_radeons_temperature_agrees_with_the_thermal_module`
//! in the instrument tier, which on a machine with the card reads the same
//! sensor through `thermal`, by the hwmon class rather than the card's
//! directory, and holds the two to a degree of each other; and, on any
//! machine, `a_radeon_is_read_from_the_files_its_driver_writes` reads back
//! the files laid out as the kernel lays them, field by field.

use std::path::{Path, PathBuf};

use crate::attested::Attested;
use crate::measurement::Bytes;

use super::accelerator::{Missing, Reading, Route};

/// Reads `/sys/class/drm/card*/device` where the driver is `amdgpu`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Amdgpu {
    /// Where the kernel's DRM class lives.
    pub drm: PathBuf,
    /// The system's PCI id table, where it has one.
    pub pci_ids: PathBuf,
    /// The Vulkan loader's driver table.
    pub icds: PathBuf,
    /// The running kernel's release, which is the driver's version: `amdgpu`
    /// ships in the kernel and has no version of its own.
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
        // The cards only: `card0` and not its connectors (`card0-DP-1`) nor
        // its render node, which are the same device under other names and
        // would be counted as three.
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
        // The directory order is the filesystem's, which is not stable.
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

/// What the card can hold, and what it holds: its own memory, or — where the
/// driver says the chip carves its memory out of the system's — the system
/// memory it may address, which is where a model on such a chip goes.
fn memory_of(device: &Path) -> (Option<u64>, Option<u64>) {
    let figure = |name: &str| read_trimmed(&device.join(name)).and_then(|text| text.parse().ok());
    let carves_out = std::fs::metadata(device.join("uma")).is_ok();
    if carves_out {
        (figure("mem_info_gtt_total"), figure("mem_info_gtt_used"))
    } else {
        (figure("mem_info_vram_total"), figure("mem_info_vram_used"))
    }
}

/// The driver's first temperature sensor, in whole degrees.
fn temperature_of(device: &Path) -> Option<u32> {
    let hwmon = std::fs::read_dir(device.join("hwmon")).ok()?;
    let mut sensors: Vec<PathBuf> = hwmon.flatten().map(|entry| entry.path()).collect();
    sensors.sort();
    sensors.into_iter().find_map(|sensor| {
        let millidegrees: u32 = read_trimmed(&sensor.join("temp1_input"))?.parse().ok()?;
        millidegrees.checked_div(1000)
    })
}

/// The name the PCI id table gives a device, where it lists it.
///
/// The table's shape: a vendor line `1002  Advanced Micro Devices…` at the
/// margin, then its devices one tab in, `\t1586  Strix Halo…`. Ids arrive as
/// the kernel writes them, `0x1002`, and are matched without the prefix.
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
