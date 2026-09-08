use std::path::{Path, PathBuf};
use std::process::Command;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-serve::engines");

const OVERHEAD: u64 = 512 << 20;

#[expect(
    clippy::integer_division,
    reason = "half of a byte count, floored, which is what a memory allowance is"
)]
pub(crate) fn overhead_for(weights: u64) -> u64 {
    OVERHEAD.max(weights / 2)
}

pub const HEADROOM_VARIABLE: &str = "MCF_MEMORY_HEADROOM";

const HEADROOM_DEFAULT: u64 = 100;
const HEADROOM_DENOMINATOR: u64 = 100;

#[must_use]
pub fn headroom_percent() -> u64 {
    std::env::var(HEADROOM_VARIABLE)
        .ok()
        .and_then(|set| set.trim().parse::<u64>().ok())
        .filter(|percent| (1..=HEADROOM_DENOMINATOR).contains(percent))
        .unwrap_or(HEADROOM_DEFAULT)
}

const SMALLEST_CONTEXT: u64 = 512;

const SMALLEST_HOLD: u64 = 4_096;

#[must_use]
pub fn held_at(largest: u64, weights: u64, cache_per_token: u64) -> u64 {
    if cache_per_token == 0 || largest <= SMALLEST_HOLD {
        return largest;
    }
    let mut context = SMALLEST_HOLD;
    let mut best = SMALLEST_HOLD;
    while context <= largest {
        if context.saturating_mul(cache_per_token) <= weights {
            best = context;
        } else {
            break;
        }
        context = context.saturating_mul(2);
    }
    best
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub kind: Kind,
    pub name: String,
    pub free: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engine {
    pub name: String,
    pub prefix: PathBuf,
    pub commit: String,
}

impl Engine {
    #[must_use]
    pub fn tool(&self, name: &str) -> Option<PathBuf> {
        let path = self.prefix.join("build").join("bin").join(name);
        path.is_file().then_some(path)
    }

    pub fn devices(&self, cpu_free: Option<u64>) -> Result<Vec<Device>> {
        let Some(server) = self.tool("llama-server") else {
            return Err(Failure::new(
                Category::EngineSpawnNotFound,
                Attribution::Machine,
                Disposition::Aborted,
                WHERE,
                "the provisioned engine has no server to ask what devices it has",
            )
            .with_context("engine", self.name.clone()));
        };
        let mut found = vec![Device {
            kind: Kind::Cpu,
            name: "CPU".to_owned(),
            free: cpu_free,
        }];
        let Ok(spoke) = Command::new(&server).arg("--list-devices").output() else {
            return Ok(found);
        };
        let said = String::from_utf8_lossy(&spoke.stdout);
        for line in said.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.eq_ignore_ascii_case("(none)")
                || line.to_ascii_lowercase().starts_with("available devices")
            {
                continue;
            }
            if let Some(device) = parse_device(line) {
                found.push(device);
            }
        }
        Ok(found)
    }
}

fn parse_device(line: &str) -> Option<Device> {
    let (_tag, rest) = line.split_once(':')?;
    let rest = rest.trim();
    let (name, detail) = rest
        .split_once('(')
        .map_or((rest, ""), |(n, d)| (n.trim(), d));
    let free = detail
        .split(',')
        .find(|part| part.contains("free"))
        .and_then(|part| {
            let mebibytes: u64 = part
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())?;
            Some(mebibytes << 20)
        });
    Some(Device {
        kind: Kind::Gpu,
        name: name.to_owned(),
        free,
    })
}

#[must_use]
pub fn discover(mcf_home: &Path) -> Vec<Engine> {
    let Ok(entries) = std::fs::read_dir(mcf_home.join("provisioned")) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let prefix = entry.path();
        let Ok(text) = std::fs::read_to_string(prefix.join("mcf-provenance.json")) else {
            continue;
        };
        let Ok(value) = json::parse(text.trim()) else {
            continue;
        };
        let Some(name) = value.get("component").and_then(Value::as_text) else {
            continue;
        };
        let engine = Engine {
            name: name.to_owned(),
            commit: value
                .get("commit")
                .and_then(Value::as_text)
                .unwrap_or("unstated")
                .to_owned(),
            prefix,
        };
        if engine.tool("llama-server").is_some() {
            found.push(engine);
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

const NO_GROWING_CACHE: &[&str] = &[
    "mamba",
    "mamba2",
    "rwkv",
    "rwkv6",
    "rwkv7",
    "falcon_mamba",
    "jamba",
];

#[must_use]
pub fn shape_of(model: &mcf_standin::gguf::Model) -> Option<mcf_hub::fitment::Shape> {
    let body = mcf_standin::anatomy::of(model);
    match mcf_standin::anatomy::work::of(model, &body).cache {
        mcf_standin::anatomy::work::Cache::Sized {
            key_heads,
            per_head,
            attending,
            ..
        } => Some(mcf_hub::fitment::Shape {
            blocks: attending.0,
            key_value_heads: key_heads,
            per_head,
            bytes_per_element: 2,
        }),
        mcf_standin::anatomy::work::Cache::Unsized(_) => None,
    }
}

#[must_use]
pub fn cache_bytes_per_token(model: &mcf_standin::gguf::Model) -> Option<u64> {
    let architecture = model.architecture()?;
    if NO_GROWING_CACHE.contains(&architecture) {
        return Some(0);
    }
    let census = mcf_standin::anatomy::blocks::of(model);
    if census.attending == 0 && census.recurrent > 0 {
        return Some(0);
    }
    shape_of(model)?.bytes_per_token()
}

#[must_use]
pub fn largest_context(weights: u64, cache_per_token: u64, free: u64, trained: u64) -> u64 {
    let ceiling = free
        .saturating_mul(headroom_percent())
        .checked_div(HEADROOM_DENOMINATOR)
        .unwrap_or(0);
    let Some(budget) = ceiling.checked_sub(overhead_for(weights).saturating_add(weights)) else {
        return 0;
    };
    if cache_per_token == 0 {
        return trained;
    }
    let mut context = SMALLEST_CONTEXT;
    let mut best = 0;
    while context <= trained {
        if context.saturating_mul(cache_per_token) <= budget {
            best = context;
        } else {
            break;
        }
        context = context.saturating_mul(2);
    }
    best
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub engine: String,
    pub device: Device,
    pub context: u64,
    pub across: Vec<Device>,
}

impl Choice {
    #[must_use]
    pub fn on_one_device(engine: String, device: Device, context: u64) -> Self {
        Self {
            engine,
            device,
            context,
            across: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_spread(&self) -> bool {
        self.across.len() > 1
    }

    #[must_use]
    pub fn split(&self) -> Vec<u64> {
        self.across
            .iter()
            .map(|device| device.free.unwrap_or(0))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    NoEngine,
    DoesNotFit { largest_device: u64, needs: u64 },
    HeaderIncomplete,
}

impl Refused {
    #[must_use]
    pub fn says(&self) -> String {
        match self {
            Self::NoEngine => "no engine is installed yet — MCF can build one for you".to_owned(),
            Self::DoesNotFit {
                largest_device,
                needs,
            } => format!(
                "this model needs about {}, and the largest single device here has {} free \
                 ({}% of it, {}, is what MCF plans to). MCF does not yet spread one model \
                 across devices, so it does not fit on any one of them. Raise or lower the \
                 share with {}",
                gigabytes(*needs),
                gigabytes(*largest_device),
                headroom_percent(),
                gigabytes(
                    largest_device
                        .saturating_mul(headroom_percent())
                        .checked_div(HEADROOM_DENOMINATOR)
                        .unwrap_or(0)
                ),
                HEADROOM_VARIABLE
            ),
            Self::HeaderIncomplete => {
                "this model's file does not say how it is shaped, so MCF cannot tell whether \
                 it runs"
                    .to_owned()
            }
        }
    }
}

fn gigabytes(bytes: u64) -> String {
    #[allow(
        clippy::integer_division,
        reason = "exact: a float would round a byte count before printing it"
    )]
    {
        let whole = bytes / 1_000_000_000;
        let hundredths = (bytes % 1_000_000_000) / 10_000_000;
        format!("{whole}.{hundredths:02} GB")
    }
}

#[must_use]
pub fn accelerator_driver_present() -> bool {
    std::fs::read_dir("/proc/driver/nvidia/gpus").is_ok_and(|mut cards| cards.next().is_some())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cuda,
    Vulkan,
    None,
}

#[must_use]
pub fn backend_present() -> Backend {
    backend_from(
        accelerator_driver_present(),
        Path::new("/sys/class/drm"),
        Path::new("/usr/share/vulkan/icd.d"),
    )
}

#[must_use]
pub fn backend_from(nvidia: bool, drm: &Path, icds: &Path) -> Backend {
    if nvidia {
        return Backend::Cuda;
    }
    let vulkan = card_drivers(drm).iter().any(|driver| {
        let icd = match driver.as_str() {
            "amdgpu" => "radeon_icd.json",
            "i915" | "xe" => "intel_icd.json",
            _ => return false,
        };
        std::fs::metadata(icds.join(icd)).is_ok()
    });
    if vulkan {
        Backend::Vulkan
    } else {
        Backend::None
    }
}

#[must_use]
pub fn card_drivers(drm: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("card") && !name.contains('-'))
        })
        .collect();
    cards.sort();
    cards
        .iter()
        .filter_map(|card| {
            let driver = std::fs::read_link(card.join("device").join("driver")).ok()?;
            driver
                .file_name()
                .and_then(|held| held.to_str())
                .map(str::to_owned)
        })
        .collect()
}

#[must_use]
pub fn card_driver_present() -> Option<String> {
    card_drivers(Path::new("/sys/class/drm")).into_iter().next()
}

#[must_use]
pub fn card_memory_used() -> Option<u64> {
    card_memory_used_under(Path::new("/sys/class/drm"))
}

#[must_use]
pub fn card_memory_used_under(drm: &Path) -> Option<u64> {
    let entries = std::fs::read_dir(drm).ok()?;
    let mut total: Option<u64> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let device = entry.path().join("device");
        for file in ["mem_info_vram_used", "mem_info_gtt_used"] {
            if let Some(used) = std::fs::read_to_string(device.join(file))
                .ok()
                .and_then(|text| text.trim().parse::<u64>().ok())
            {
                total = Some(total.unwrap_or(0).saturating_add(used));
            }
        }
    }
    total
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardSensors {
    pub temperature_millic: Option<i64>,
    pub clock_hz: Option<i64>,
    pub power_uw: Option<i64>,
    pub power_label: Option<String>,
}

impl CardSensors {
    #[must_use]
    pub fn any(&self) -> bool {
        self.temperature_millic.is_some() || self.clock_hz.is_some() || self.power_uw.is_some()
    }

    #[must_use]
    pub fn power_is(&self) -> &'static str {
        match self.power_label.as_deref() {
            Some("PPT") => "the whole processor package, graphics and processor together",
            Some(_) | None => "the graphics device",
        }
    }

    #[must_use]
    pub fn power_named(&self) -> &'static str {
        match self.power_label.as_deref() {
            Some("PPT") => "package",
            Some(_) | None => "card",
        }
    }
}

#[must_use]
pub fn card_sensors() -> CardSensors {
    card_sensors_under(Path::new("/sys/class/drm"))
}

#[must_use]
pub fn card_sensors_under(drm: &Path) -> CardSensors {
    let Ok(entries) = std::fs::read_dir(drm) else {
        return CardSensors::default();
    };
    let mut cards: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("card") && !name.contains('-')
        })
        .map(|entry| entry.path())
        .collect();
    cards.sort();
    for card in cards {
        let Ok(monitors) = std::fs::read_dir(card.join("device").join("hwmon")) else {
            continue;
        };
        for monitor in monitors.flatten() {
            let read = |file: &str| {
                std::fs::read_to_string(monitor.path().join(file))
                    .ok()
                    .and_then(|text| text.trim().parse::<i64>().ok())
            };
            let label = |file: &str| {
                std::fs::read_to_string(monitor.path().join(file))
                    .ok()
                    .map(|text| text.trim().to_owned())
                    .filter(|text| !text.is_empty())
            };
            let sensors = CardSensors {
                temperature_millic: read("temp1_input"),
                clock_hz: read("freq1_input"),
                power_uw: read("power1_average").or_else(|| read("power1_input")),
                power_label: label("power1_label"),
            };
            if sensors.any() {
                return sensors;
            }
        }
    }
    CardSensors::default()
}

#[must_use]
pub fn card_memory_free() -> Option<u64> {
    card_memory_free_under(Path::new("/sys/class/drm"))
}

#[must_use]
pub fn card_memory_free_under(drm: &Path) -> Option<u64> {
    let device = first_card_under(drm)?.join("device");
    let read = |file: &str| {
        std::fs::read_to_string(device.join(file))
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok())
    };
    Some(read("mem_info_vram_total")?.saturating_sub(read("mem_info_vram_used")?))
}

#[must_use]
pub fn card_memory_is_the_hosts() -> bool {
    card_memory_is_the_hosts_under(Path::new("/sys/class/drm"))
}

#[must_use]
pub fn card_memory_is_the_hosts_under(drm: &Path) -> bool {
    let Some(card) = first_card_under(drm) else {
        return true;
    };
    std::fs::read_to_string(card.join("device").join("mem_info_vram_total"))
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .is_none_or(|total| total < CARVE_OUT_AT_MOST)
}

const CARVE_OUT_AT_MOST: u64 = 4 * 1024 * 1024 * 1024;

fn first_card_under(drm: &Path) -> Option<PathBuf> {
    let mut cards: Vec<PathBuf> = std::fs::read_dir(drm)
        .ok()?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("card") && !name.contains('-')
        })
        .map(|entry| entry.path())
        .collect();
    cards.sort();
    cards.into_iter().next()
}

#[must_use]
pub fn would_not_fit(model: &Path, context: u64, gpu_layers: u32) -> Option<(u64, u64)> {
    let bytes = std::fs::metadata(model).ok()?.len();
    let cache = crate::declared::header(model)
        .and_then(|file| cache_bytes_per_token(&file))
        .map_or(0, |per_token| per_token.saturating_mul(context));
    let needs = bytes.saturating_add(cache);
    let host = mcf_core::hardware::memory_available_now()?;
    let card = if gpu_layers > 0 && !card_memory_is_the_hosts() {
        card_memory_free().unwrap_or(0)
    } else {
        0
    };
    let available = host.saturating_add(card);
    (needs > available).then_some((needs, available))
}

#[must_use]
pub fn wanted_for(backend: Backend) -> Option<&'static mcf_core::component::Component> {
    let wanted = match backend {
        Backend::Cuda => "llama.cpp-cuda",
        Backend::Vulkan => "llama.cpp-vulkan",
        Backend::None => return None,
    };
    mcf_core::component::COMPONENTS
        .iter()
        .find(|component| component.name == wanted)
}

#[must_use]
pub fn required(backend: Backend) -> Option<&'static mcf_core::component::Component> {
    let wanted = match backend {
        Backend::Cuda => "llama.cpp-cuda",
        Backend::Vulkan => "llama.cpp-vulkan",
        Backend::None => "llama.cpp",
    };
    mcf_core::component::COMPONENTS
        .iter()
        .find(|component| component.name == wanted)
}

pub fn resolve(
    engines: &[(Engine, Vec<Device>)],
    weights: u64,
    cache_per_token: Option<u64>,
    trained: u64,
) -> core::result::Result<Choice, Refused> {
    if engines.is_empty() {
        return Err(Refused::NoEngine);
    }
    let Some(cache_per_token) = cache_per_token else {
        return Err(Refused::HeaderIncomplete);
    };
    let mut best: Option<Choice> = None;
    let mut largest_device = 0;
    for (engine, devices) in engines {
        for device in devices {
            let Some(free) = device.free else {
                continue;
            };
            largest_device = largest_device.max(free);
            let context = largest_context(weights, cache_per_token, free, trained);
            if context == 0 {
                continue;
            }
            let better = best.as_ref().is_none_or(|held| {
                context > held.context
                    || (context == held.context
                        && device.kind == Kind::Gpu
                        && held.device.kind == Kind::Cpu)
            });
            if better {
                best = Some(Choice::on_one_device(
                    engine.name.clone(),
                    device.clone(),
                    context,
                ));
            }
        }
    }
    if let Some(held) = best {
        return Ok(held);
    }
    if let Some(spread) = spread_across(engines, weights, cache_per_token, trained) {
        return Ok(spread);
    }
    Err(Refused::DoesNotFit {
        largest_device,
        needs: weights
            .saturating_add(overhead_for(weights))
            .saturating_add(SMALLEST_CONTEXT.saturating_mul(cache_per_token)),
    })
}

fn spread_across(
    engines: &[(Engine, Vec<Device>)],
    weights: u64,
    cache_per_token: u64,
    trained: u64,
) -> Option<Choice> {
    let mut best: Option<Choice> = None;
    for (engine, devices) in engines {
        let cards: Vec<Device> = devices
            .iter()
            .filter(|device| device.kind == Kind::Gpu && device.free.is_some_and(|free| free > 0))
            .cloned()
            .collect();
        if cards.len() < 2 {
            continue;
        }
        let together = cards.iter().fold(0_u64, |sum, device| {
            sum.saturating_add(device.free.unwrap_or(0))
        });
        let context = largest_context(weights, cache_per_token, together, trained);
        if context == 0 {
            continue;
        }
        let biggest = cards
            .iter()
            .max_by_key(|device| device.free.unwrap_or(0))?
            .clone();
        if best.as_ref().is_none_or(|held| context > held.context) {
            best = Some(Choice {
                engine: engine.name.clone(),
                device: biggest,
                context,
                across: cards,
            });
        }
    }
    best
}

#[cfg(test)]
mod tests;
