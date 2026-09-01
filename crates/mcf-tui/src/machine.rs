//! What the machine is doing, read without privilege.
//!
//! **Everything here is a file the operator can already read.** Loads and
//! memory come from `/proc`, clocks and temperatures from `/sys`, and the card
//! from whatever the vendor tool will say. Nothing needs root, because a
//! console that asked for privileges to draw a number would be asking for
//! trouble in exchange for a screen.
//!
//! **A reading MCF cannot take is `None`, and `None` is not zero** (A7). CPU
//! package power needs a counter that is root-only on this platform, so it is
//! absent rather than reported as nothing — a screen that printed `0 W` there
//! would be inventing a measurement, and the whole point of the console is that
//! it does not.
//!
//! **Not portable, and it says so.** These are Linux paths. On a platform that
//! has none of them every field reads unknown, which is the honest answer and
//! not a crash: the console still draws, and every row says what could not be
//! found.

use std::time::Instant;

/// A percentage, as tenths, so it can be rendered without a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tenths(pub u32);

impl Tenths {
    /// Renders as a whole percentage.
    #[must_use]
    pub fn whole(self) -> u32 {
        #[allow(clippy::integer_division, reason = "tenths to whole, exactly")]
        {
            self.0 / 10
        }
    }
}

/// The processor.
#[derive(Debug, Clone, Default)]
pub struct Processor {
    /// Busy fraction since the previous reading.
    pub load: Option<Tenths>,
    /// The hottest core-complex temperature, in whole degrees.
    pub temperature: Option<i32>,
    /// The fastest core's clock, in megahertz.
    pub clock: Option<u32>,
    /// How many the machine reports.
    pub cores: Option<usize>,
}

/// System memory, in bytes.
#[derive(Debug, Clone, Default)]
pub struct Memory {
    /// Total.
    pub total: Option<u64>,
    /// Free for something new.
    pub available: Option<u64>,
}

impl Memory {
    /// Total minus available, which is what is in use by everything.
    #[must_use]
    pub fn used(&self) -> Option<u64> {
        self.total?.checked_sub(self.available?)
    }
}

/// A graphics card.
#[derive(Debug, Clone)]
pub struct Card {
    /// What it calls itself.
    pub name: String,
    /// Busy fraction.
    pub load: Option<Tenths>,
    /// Degrees.
    pub temperature: Option<i32>,
    /// Watts.
    pub power: Option<u32>,
    /// Bytes in use.
    pub used: Option<u64>,
    /// Bytes it has.
    pub total: Option<u64>,
}

/// A disk.
#[derive(Debug, Clone)]
pub struct Disk {
    /// The kernel's name for it.
    pub name: String,
    /// Bytes read since the previous reading, per second.
    pub read: Option<u64>,
    /// Bytes written since the previous reading, per second.
    pub written: Option<u64>,
    /// Degrees, where the drive reports them.
    pub temperature: Option<i32>,
}

/// One look at the machine.
#[derive(Debug, Clone, Default)]
pub struct Reading {
    /// The processor.
    pub processor: Processor,
    /// System memory.
    pub memory: Memory,
    /// Every card found.
    pub cards: Vec<Card>,
    /// Every disk with traffic worth showing.
    pub disks: Vec<Disk>,
}

/// Takes readings, holding what it needs to turn counters into rates.
///
/// A load and a transfer rate are both differences between two looks, so the
/// first reading has neither and says so rather than reporting a total as
/// though it were a rate.
#[derive(Debug)]
#[allow(
    clippy::struct_field_names,
    reason = "each field holds the same thing from the previous look, which is what they are"
)]
pub struct Sampler {
    previous_cpu: Option<(u64, u64)>,
    previous_disks: Vec<(String, u64, u64)>,
    previous_at: Option<Instant>,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    /// A sampler that has not looked yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            previous_cpu: None,
            previous_disks: Vec::new(),
            previous_at: None,
        }
    }

    /// Looks at the machine.
    pub fn read(&mut self) -> Reading {
        let now = Instant::now();
        let elapsed = self.previous_at.map(|then| now.duration_since(then));
        self.previous_at = Some(now);
        Reading {
            processor: self.processor(),
            memory: memory(),
            cards: cards(),
            disks: self.disks(elapsed.map(|held| held.as_millis()).unwrap_or_default()),
        }
    }

    fn processor(&mut self) -> Processor {
        let mut processor = Processor {
            temperature: hottest_core_complex(),
            clock: fastest_clock(),
            cores: core_count(),
            load: None,
        };
        if let Some((busy, total)) = cpu_totals() {
            if let Some((was_busy, was_total)) = self.previous_cpu {
                let moved = total.saturating_sub(was_total);
                let worked = busy.saturating_sub(was_busy);
                if let Some(scaled) = worked.saturating_mul(1000).checked_div(moved) {
                    let tenths = u32::try_from(scaled).unwrap_or(1000);
                    processor.load = Some(Tenths(tenths.min(1000)));
                }
            }
            self.previous_cpu = Some((busy, total));
        }
        processor
    }

    fn disks(&mut self, elapsed_ms: u128) -> Vec<Disk> {
        let now = disk_totals();
        let mut found = Vec::new();
        for (name, read, written) in &now {
            let was = self
                .previous_disks
                .iter()
                .find(|(held, _, _)| held == name)
                .map(|(_, r, w)| (*r, *w));
            let rate = |then: u64, held: u64| -> Option<u64> {
                let moved = u128::from(held.saturating_sub(then));
                let per_second = moved.saturating_mul(1000).checked_div(elapsed_ms)?;
                u64::try_from(per_second).ok()
            };
            found.push(Disk {
                name: name.clone(),
                read: was.and_then(|(r, _)| rate(r, *read)),
                written: was.and_then(|(_, w)| rate(w, *written)),
                temperature: drive_temperature(),
            });
        }
        self.previous_disks = now;
        found
    }
}

fn read_to_string(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Busy and total jiffies from `/proc/stat`'s first line.
fn cpu_totals() -> Option<(u64, u64)> {
    let text = read_to_string("/proc/stat")?;
    let line = text.lines().next()?;
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|held| held.parse().ok())
        .collect();
    let total: u64 = fields.iter().sum();
    // The fourth is idle and the fifth is waiting for a disk; neither is work.
    let idle: u64 = fields.iter().skip(3).take(2).sum();
    Some((total.saturating_sub(idle), total))
}

fn memory() -> Memory {
    let mut memory = Memory::default();
    let Some(text) = read_to_string("/proc/meminfo") else {
        return memory;
    };
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let key = fields.next().unwrap_or_default();
        let Some(kibibytes) = fields.next().and_then(|held| held.parse::<u64>().ok()) else {
            continue;
        };
        match key {
            "MemTotal:" => memory.total = Some(kibibytes * 1024),
            "MemAvailable:" => memory.available = Some(kibibytes * 1024),
            _ => {}
        }
    }
    memory
}

fn core_count() -> Option<usize> {
    std::thread::available_parallelism().ok().map(Into::into)
}

fn fastest_clock() -> Option<u32> {
    let mut fastest = 0_u64;
    for index in 0..1024 {
        let path = format!("/sys/devices/system/cpu/cpu{index}/cpufreq/scaling_cur_freq");
        let Some(text) = read_to_string(&path) else {
            break;
        };
        if let Ok(kilohertz) = text.trim().parse::<u64>() {
            fastest = fastest.max(kilohertz);
        }
    }
    #[allow(clippy::integer_division, reason = "kilohertz to megahertz")]
    (fastest > 0).then(|| u32::try_from(fastest / 1000).unwrap_or(u32::MAX))
}

/// The hottest core complex, which is the one that matters under load.
fn hottest_core_complex() -> Option<i32> {
    let mut hottest: Option<i32> = None;
    for hwmon in 0..32 {
        for sensor in 1..12 {
            let label = format!("/sys/class/hwmon/hwmon{hwmon}/temp{sensor}_label");
            let Some(text) = read_to_string(&label) else {
                continue;
            };
            let text = text.trim();
            if !(text.starts_with("Tccd") || text == "Tctl" || text.starts_with("Package")) {
                continue;
            }
            let input = label.replace("_label", "_input");
            if let Some(millidegrees) =
                read_to_string(&input).and_then(|held| held.trim().parse::<i32>().ok())
            {
                #[allow(clippy::integer_division, reason = "millidegrees to degrees")]
                let degrees = millidegrees / 1000;
                hottest = Some(hottest.map_or(degrees, |held: i32| held.max(degrees)));
            }
        }
    }
    hottest
}

fn drive_temperature() -> Option<i32> {
    for hwmon in 0..32 {
        let name = format!("/sys/class/hwmon/hwmon{hwmon}/name");
        if read_to_string(&name)?.trim() != "nvme" {
            continue;
        }
        let input = format!("/sys/class/hwmon/hwmon{hwmon}/temp1_input");
        if let Some(millidegrees) =
            read_to_string(&input).and_then(|held| held.trim().parse::<i32>().ok())
        {
            #[allow(clippy::integer_division, reason = "millidegrees to degrees")]
            return Some(millidegrees / 1000);
        }
    }
    None
}

/// Read and written bytes per disk, from `/proc/diskstats`.
fn disk_totals() -> Vec<(String, u64, u64)> {
    let Some(text) = read_to_string("/proc/diskstats") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(name) = fields.get(2) else { continue };
        // Whole devices only, decided by the kernel: `/sys/block` holds the
        // disks and not their partitions. Guessing from the name got `sda1`
        // wrong, which double-counted `sda`.
        if !std::path::Path::new(&format!("/sys/block/{name}")).exists() {
            continue;
        }
        let sectors = |at: usize| -> u64 {
            fields
                .get(at)
                .and_then(|held| held.parse::<u64>().ok())
                .unwrap_or(0)
                // A sector is 512 bytes in this file whatever the drive uses.
                .saturating_mul(512)
        };
        let (read, written) = (sectors(5), sectors(9));
        if read == 0 && written == 0 {
            continue;
        }
        found.push(((*name).to_owned(), read, written));
    }
    found
}

/// Millidegrees as whole degrees.
///
/// The rounding is toward zero and it is stated here rather than left inside
/// an expression: a screen shows whole degrees, and a tenth of a degree is not
/// a thing this row has room to say.
#[allow(
    clippy::integer_division,
    reason = "whole degrees is the unit shown; the truncation is the point"
)]
const fn whole_degrees(millidegrees: i64) -> i64 {
    millidegrees / 1000
}

/// Microwatts as whole watts, for the same reason.
#[allow(
    clippy::integer_division,
    reason = "whole watts is the unit shown; the truncation is the point"
)]
const fn whole_watts(microwatts: u64) -> u64 {
    microwatts / 1_000_000
}

/// One card as NVIDIA's own tool reports it, with the bus it sits on.
///
/// The bus id travels because it is the only thing that ties a row of this
/// tool's output to a `card0` in sysfs: the tool prints a marketing name and
/// sysfs prints an index, and neither is the other. Matching on order instead
/// gave every NVIDIA card in a machine the first one's numbers.
#[derive(Debug, Clone)]
struct Vendor {
    /// `domain:bus:device.function`, as the tool spells it.
    bus: String,
    /// What the tool measured.
    card: Card,
}

/// Every row NVIDIA's tool prints.
///
/// Only NVIDIA publishes occupancy and memory through a tool rather than
/// sysfs, so this is an enrichment and never the discovery: a machine without
/// the tool still lists every card it has.
fn nvidia_details() -> Vec<Vendor> {
    let Ok(spoke) = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=pci.bus_id,name,utilization.gpu,temperature.gpu,power.draw,\
             memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&spoke.stdout)
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            let bus = (*fields.first()?).to_owned();
            let name = (*fields.get(1)?).to_owned();
            // The tool prints whole numbers with an occasional decimal on the
            // power. Rounded rather than cast, and a value that will not fit is
            // dropped rather than wrapped into a plausible wrong one.
            let number = |at: usize| fields.get(at).and_then(|held| held.parse::<f64>().ok());
            let whole = |at: usize| -> Option<u64> {
                let held = number(at)?;
                if !held.is_finite() || held < 0.0 {
                    return None;
                }
                // Through a string, because a float cast to an integer is a
                // truncation the lints refuse and a rounding nobody stated.
                format!("{held:.0}").parse::<u64>().ok()
            };
            let mebibytes = |at: usize| whole(at).map(|held| held << 20);
            Some(Vendor {
                bus,
                card: Card {
                    name,
                    load: whole(2)
                        .and_then(|held| u32::try_from(held.saturating_mul(10)).ok().map(Tenths)),
                    temperature: whole(3).and_then(|held| i32::try_from(held).ok()),
                    power: whole(4).and_then(|held| u32::try_from(held).ok()),
                    used: mebibytes(5),
                    total: mebibytes(6),
                },
            })
        })
        .collect()
}

/// Whether two spellings name the same slot on the bus.
///
/// `nvidia-smi` writes `00000000:C2:00.0` and sysfs writes `0000:c2:00.0`:
/// the same address, a different number of leading zeros and a different case.
/// Compared field by field, with the domain read as a number, so neither
/// spelling has to be the canonical one.
fn same_slot(one: &str, two: &str) -> bool {
    let parts = |held: &str| -> Option<(u64, String)> {
        let (domain, rest) = held.split_once(':')?;
        Some((
            u64::from_str_radix(domain.trim(), 16).ok()?,
            rest.to_ascii_lowercase(),
        ))
    };
    match (parts(one), parts(two)) {
        (Some(one), Some(two)) => one == two,
        _ => false,
    }
}

/// Where this card's PCI address is, as sysfs spells it.
fn card_slot(card: &str) -> Option<String> {
    let at = std::path::Path::new("/sys/class/drm").join(card).join("device");
    Some(
        std::fs::canonicalize(at)
            .ok()?
            .file_name()?
            .to_string_lossy()
            .into_owned(),
    )
}

/// This card's own hwmon directory.
///
/// Scanned rather than assumed: the number in `hwmon3` is whatever the kernel
/// handed out at boot, so a hard-coded `hwmon0` reads a different device's
/// sensors or nothing at all. Under the card's own directory, so two cards
/// never share a reading — which is what matching sensors by driver name did.
fn card_hwmon(card: &str) -> Option<std::path::PathBuf> {
    let at = std::path::Path::new("/sys/class/drm")
        .join(card)
        .join("device")
        .join("hwmon");
    let mut found: Vec<std::path::PathBuf> = std::fs::read_dir(at)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("hwmon"))
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// A whole number a sysfs file holds.
fn number_at(at: &std::path::Path) -> Option<u64> {
    std::fs::read_to_string(at).ok()?.trim().parse::<u64>().ok()
}

/// A number under this card's device directory.
fn card_number(card: &str, leaf: &str) -> Option<u64> {
    number_at(
        &std::path::Path::new("/sys/class/drm")
            .join(card)
            .join("device")
            .join(leaf),
    )
}

/// What a card is using of its own memory, and how much it has.
///
/// Three shapes, because three vendors publish it three ways. An integrated
/// card has no separate memory at all and answers `None` for both — which is
/// the truth about it, and not a zero.
fn card_memory(card: &str, driver: &str) -> (Option<u64>, Option<u64>) {
    match driver {
        // AMD publishes bytes outright.
        "amdgpu" | "radeon" => (
            card_number(card, "mem_info_vram_used"),
            card_number(card, "mem_info_vram_total"),
        ),
        // Intel's discrete cards publish local memory; the integrated ones
        // publish neither file, which is how an integrated card says it has
        // no memory of its own.
        "i915" | "xe" => {
            let total = card_number(card, "lmem_total_bytes");
            let available = card_number(card, "lmem_avail_bytes");
            let used = total
                .zip(available)
                .map(|(total, available)| total.saturating_sub(available));
            (used, total)
        }
        _ => (None, None),
    }
}

/// Every accelerator this machine has, whoever made it and however many.
///
/// **Discovery is sysfs, for every vendor at once.** This used to shell out to
/// `nvidia-smi` and take its silence for an absence, so a machine with an AMD
/// or Intel card reported having none — the card was there, the driver was
/// loaded, and the monitor said nothing at all, while MCF's own `doctor` had
/// been reading it from sysfs the whole time. Two readers, disagreeing (the
/// shape of F127).
///
/// `mcf_core::hardware::utilisation::accelerators` walks `/sys/class/drm`, so
/// one card, four cards, or a mixed set of vendors all arrive the same way,
/// and both surfaces now read the one enumeration `doctor` reads.
///
/// **What is read per card, and never across cards.** Temperature and power
/// come from the card's own hwmon directory; memory from its own device
/// directory. Reading them by driver name instead gave two cards of the same
/// make one card's numbers twice. NVIDIA publishes occupancy through NVML
/// rather than sysfs, so its rows are merged in from the vendor tool, matched
/// on the bus address; where that tool is absent the card is still listed with
/// what sysfs gave and the rest unknown (A7).
fn cards() -> Vec<Card> {
    let details = nvidia_details();
    mcf_core::hardware::utilisation::accelerators()
        .into_iter()
        .map(|busy| {
            let slot = card_slot(&busy.card);
            let vendor = slot.as_ref().and_then(|slot| {
                details
                    .iter()
                    .find(|held| same_slot(&held.bus, slot))
                    .map(|held| held.card.clone())
            });
            let hwmon = card_hwmon(&busy.card);
            let sensed = |leaf: &str| hwmon.as_ref().and_then(|at| number_at(&at.join(leaf)));
            let (used, total) = card_memory(&busy.card, &busy.driver);
            Card {
                // The vendor's own name where there is one, because "GeForce
                // RTX 5090" tells a person more than "card1" does. The sysfs
                // name otherwise, which always exists.
                name: vendor.as_ref().map_or_else(
                    || format!("{} ({})", busy.card, busy.driver),
                    |held| held.name.clone(),
                ),
                load: busy
                    .percent
                    .known()
                    .and_then(|percent| u32::try_from(percent.saturating_mul(10)).ok().map(Tenths))
                    .or_else(|| vendor.as_ref().and_then(|held| held.load)),
                temperature: sensed("temp1_input")
                    .and_then(|milli| i64::try_from(milli).ok())
                    .and_then(|milli| i32::try_from(whole_degrees(milli)).ok())
                    .or_else(|| vendor.as_ref().and_then(|held| held.temperature)),
                power: sensed("power1_average")
                    .and_then(|micro| u32::try_from(whole_watts(micro)).ok())
                    .or_else(|| vendor.as_ref().and_then(|held| held.power)),
                used: used.or_else(|| vendor.as_ref().and_then(|held| held.used)),
                total: total.or_else(|| vendor.and_then(|held| held.total)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
