use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tenths(pub u32);

impl Tenths {
    #[must_use]
    pub fn whole(self) -> u32 {
        #[allow(clippy::integer_division, reason = "tenths to whole, exactly")]
        {
            self.0 / 10
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Processor {
    pub load: Option<Tenths>,
    pub temperature: Option<i32>,
    pub clock: Option<u32>,
    pub cores: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Memory {
    pub total: Option<u64>,
    pub available: Option<u64>,
}

impl Memory {
    #[must_use]
    pub fn used(&self) -> Option<u64> {
        self.total?.checked_sub(self.available?)
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub load: Option<Tenths>,
    pub temperature: Option<i32>,
    pub power: Option<u32>,
    pub used: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Disk {
    pub name: String,
    pub read: Option<u64>,
    pub written: Option<u64>,
    pub temperature: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct Reading {
    pub processor: Processor,
    pub memory: Memory,
    pub cards: Vec<Card>,
    pub disks: Vec<Disk>,
}

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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            previous_cpu: None,
            previous_disks: Vec::new(),
            previous_at: None,
        }
    }

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

fn cpu_totals() -> Option<(u64, u64)> {
    let text = read_to_string("/proc/stat")?;
    let line = text.lines().next()?;
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|held| held.parse().ok())
        .collect();
    let total: u64 = fields.iter().sum();
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

fn disk_totals() -> Vec<(String, u64, u64)> {
    let Some(text) = read_to_string("/proc/diskstats") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let Some(name) = fields.get(2) else { continue };
        if !std::path::Path::new(&format!("/sys/block/{name}")).exists() {
            continue;
        }
        let sectors = |at: usize| -> u64 {
            fields
                .get(at)
                .and_then(|held| held.parse::<u64>().ok())
                .unwrap_or(0)
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

#[allow(
    clippy::integer_division,
    reason = "whole degrees is the unit shown; the truncation is the point"
)]
const fn whole_degrees(millidegrees: i64) -> i64 {
    millidegrees / 1000
}

#[allow(
    clippy::integer_division,
    reason = "whole watts is the unit shown; the truncation is the point"
)]
const fn whole_watts(microwatts: u64) -> u64 {
    microwatts / 1_000_000
}

#[derive(Debug, Clone)]
struct Vendor {
    bus: String,
    card: Card,
}

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
            let number = |at: usize| fields.get(at).and_then(|held| held.parse::<f64>().ok());
            let whole = |at: usize| -> Option<u64> {
                let held = number(at)?;
                if !held.is_finite() || held < 0.0 {
                    return None;
                }
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

fn card_slot(card: &str) -> Option<String> {
    let at = std::path::Path::new("/sys/class/drm")
        .join(card)
        .join("device");
    Some(
        std::fs::canonicalize(at)
            .ok()?
            .file_name()?
            .to_string_lossy()
            .into_owned(),
    )
}

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

fn number_at(at: &std::path::Path) -> Option<u64> {
    std::fs::read_to_string(at).ok()?.trim().parse::<u64>().ok()
}

fn card_number(card: &str, leaf: &str) -> Option<u64> {
    number_at(
        &std::path::Path::new("/sys/class/drm")
            .join(card)
            .join("device")
            .join(leaf),
    )
}

fn card_memory(card: &str, driver: &str) -> (Option<u64>, Option<u64>) {
    match driver {
        // Asked of the pool the card actually draws on, which on a card that carves its
        // memory out of system memory is not the dedicated one. Reading the carve-out had
        // this machine's Strix Halo reported as having half a gigabyte of graphics memory,
        // all of it spoken for, while ninety-five gigabytes of model sat in the pool
        // nobody was asking about.
        "amdgpu" | "radeon" => {
            let device = std::path::Path::new("/sys/class/drm")
                .join(card)
                .join("device");
            let (total, used) = mcf_core::hardware::route_amdgpu::memory_of(&device);
            (used, total)
        }
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
