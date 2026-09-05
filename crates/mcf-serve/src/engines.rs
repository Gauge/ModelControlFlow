//! Which engine runs a model, and on what — worked out rather than asked.
//!
//! **The question a person should never be asked.** An operator with a model
//! wants to know whether it runs and how fast, not which backend was compiled
//! with which flags. Everything here exists so that a surface can say *engine
//! llama.cpp, CUDA, on the GPU, up to 32 768 tokens* without anybody having
//! configured it.
//!
//! **Every provisioned engine, not one.** [`crate::adapters::provisioned_llama`] looks
//! for a single prefix named exactly `llama.cpp`, which was right when there was
//! one. A CUDA build provisioned beside it is called `llama.cpp-cuda`, so it was
//! filtered out and never seen — present on disk and invisible to everything
//! (F128, F130). [`discover`] reads every prefix that has provenance and tools,
//! and lets the caller choose between them on what they can do rather than on
//! what they are called.
//!
//! **What a device is, is verified, not declared.** A build's backends are a
//! property of how it was compiled, and asking it is the only honest way to
//! know: the CPU build answers `--list-devices` with nothing however many cards
//! are installed. So [`Engine::devices`] runs the binary and reads its answer
//! (A21) — declared support would be a claim about a compile nobody can see.
//!
//! **What fits is arithmetic, and it is exact.** The weights are a file size and
//! the cache is a size per token the header states, so the largest context a
//! device can hold is a division rather than an experiment. Powers of two only,
//! and never past what the model was trained for.

use std::path::{Path, PathBuf};
use std::process::Command;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-serve::engines");

/// Room for the engine's own buffers beside the weights and the cache.
///
/// Measured rather than guessed: a 0.6B model at a 512-token window held 187 MB
/// resident against 359 MB at 8 192, so the part that is neither weights nor
/// cache is a few hundred megabytes. Kept generous, because a window MCF
/// proposes and the engine then refuses is worse than one slightly smaller than
/// it had to be.
const OVERHEAD: u64 = 512 << 20;

/// What the engine holds beyond the weights and the cache, for a model of this
/// size.
///
/// **The flat constant above was measured on a model twenty times smaller than
/// the ones MCF is now asked about, and carried forward.** Half a gigabyte is
/// right for a 0.6B model and is not right for a 24B one: measured on
/// `llama-server`, a 14.5 GB model holds about 6.8 GB that is neither weights
/// nor cache, and a 17.5 GB model about 7.8 GB. Across four windows on those
/// two the figure barely moves with the window, so it is the model's size it
/// follows, not the context's (F144).
///
/// | weights | measured | this rule |
/// |---|---|---|
/// | 0.4 GB (the original reading) | 0.36 GB | 0.54 GB |
/// | 14.5 GB | 6.75 GB | 7.25 GB |
/// | 17.5 GB | 7.79 GB | 8.75 GB |
///
/// Half the weights, never below the old constant. Conservative on all three
/// readings, which is the direction the original comment asks for: a window
/// MCF proposes and the engine then refuses is worse than one slightly smaller
/// than it had to be. Left underestimating, MCF sized every window to fill the
/// memory it thought it had and the engine was killed at 40 GB and again at
/// 64 — raising the allowance only bought a larger window and the same death.
///
/// Three readings from two engines is not a law, and B-384 is where the
/// relationship gets measured properly rather than fitted to what was to hand.
#[expect(
    clippy::integer_division,
    reason = "half of a byte count, floored, which is what a memory allowance is"
)]
fn overhead_for(weights: u64) -> u64 {
    OVERHEAD.max(weights / 2)
}

/// The fraction of free memory a plan may take.
///
/// Not all of it: something else on the machine will want some, and a plan that
/// fills a device to the brim is a plan that fails on a machine doing anything
/// at all.
const HEADROOM_NUMERATOR: u64 = 85;
const HEADROOM_DENOMINATOR: u64 = 100;

/// The smallest window worth proposing.
const SMALLEST_CONTEXT: u64 = 512;

/// A kind of thing an engine can compute on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The processor MCF is running on.
    Cpu,
    /// A card.
    Gpu,
}

/// A device an engine says it can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Processor or card.
    pub kind: Kind,
    /// What the engine called it.
    pub name: String,
    /// Free memory, where the engine or the platform says. `None` is unknown,
    /// which is not zero (A7).
    pub free: Option<u64>,
}

/// An engine on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engine {
    /// The component name from its provenance — `llama.cpp`, `llama.cpp-cuda`.
    pub name: String,
    /// Where it lives.
    pub prefix: PathBuf,
    /// The source commit it was built from.
    pub commit: String,
}

impl Engine {
    /// One of its tools, if the build produced it.
    #[must_use]
    pub fn tool(&self, name: &str) -> Option<PathBuf> {
        let path = self.prefix.join("build").join("bin").join(name);
        path.is_file().then_some(path)
    }

    /// What this build can actually compute on, by asking it.
    ///
    /// The CPU is always among them: a build that runs at all runs on the
    /// processor. Cards are whatever the binary reports, which is a fact about
    /// how it was compiled and not about the machine — this is the whole reason
    /// to ask rather than to assume (A21).
    ///
    /// # Errors
    ///
    /// `engine.spawn.not_found` where the build has no server to ask.
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
            // It could not be asked. That is not the same as having no cards,
            // and reporting none would be reporting a measurement (A7).
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

/// One line of `--list-devices`, which looks like
/// `CUDA0: NVIDIA GeForce RTX 5080 (15877 MiB, 13773 MiB free)`.
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

/// Every engine provisioned under this data home.
///
/// Read from the disk rather than from a list, and by shape rather than by
/// name: a prefix with provenance and a server in it is an engine, whatever the
/// component happens to be called. The name that was hard-coded is exactly what
/// made a second build invisible.
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

/// Architectures that keep no cache that grows with the context.
///
/// A state-space or recurrent model carries a fixed state rather than a key and
/// value per token, so its window costs nothing beyond the weights and its
/// header states no attention geometry at all. Without this they read as
/// *the file does not say how it is shaped*, which is true of the fields and
/// false about the model — it runs perfectly well.
const NO_GROWING_CACHE: &[&str] = &[
    "mamba",
    "mamba2",
    "rwkv",
    "rwkv6",
    "rwkv7",
    "falcon_mamba",
    "jamba",
];

/// How a model is shaped, from its own GGUF header and tensor directory.
///
/// **Because most repositories publish no configuration.** `config.json` is
/// where `mcf_hub::offer` looks first and it is absent from nearly every
/// repository that publishes GGUFs — so *will this run here* came back as *MCF
/// cannot say* for almost everything somebody would try to download. The
/// header carries the same numbers, and the hub serves ranges, so a few
/// megabytes of prefix answers it without acquiring the model (B-413, PR3).
///
/// **One arithmetic.** The shape is the one `mcf explain` shows, read by
/// [`mcf_standin::anatomy::work`]: the blocks that hold keys rather than the
/// header's block count — a hybrid's cache was stated at four times its size
/// here (F150) — and, for a latent-attention model, the latent's width with
/// no value cache beside it, which had been stated at nearly twice (F151).
/// What the report says and what placement is resolved from cannot differ,
/// because they are one reading (B-072).
///
/// `None` where the header does not name the heads and widths that size a
/// cache — which is not a zero (A7) — and for a model whose every block keeps
/// a recurrent state, which has no growing cache to shape.
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
            // Half precision, as every engine caches by default. The same
            // parameter `Shape::from_configuration` is given.
            bytes_per_element: 2,
        }),
        mcf_standin::anatomy::work::Cache::Unsized(_) => None,
    }
}

/// How many bytes of cache one token of context costs.
///
/// [`shape_of`]'s product, at two bytes an element — the engine's cache is
/// sixteen-bit unless it is told otherwise, and that is a stated condition
/// rather than a constant hidden in a product.
///
/// `None` where the header does not say, which is not a zero: a header that
/// simply omits its attention geometry has no such number (A7). Zero for an
/// architecture that keeps no growing cache — a state-space model — whether
/// its name is on the list or its directory shows every block keeping a
/// recurrent state.
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

/// The largest power-of-two context this much memory can hold.
///
/// A power of two because that is the only shape MCF samples at, and no larger
/// than the model was trained for because a window it was never trained for is
/// not a window a reading may describe.
#[must_use]
pub fn largest_context(weights: u64, cache_per_token: u64, free: u64, trained: u64) -> u64 {
    let ceiling = free
        .saturating_mul(HEADROOM_NUMERATOR)
        .checked_div(HEADROOM_DENOMINATOR)
        .unwrap_or(0);
    let Some(budget) = ceiling.checked_sub(overhead_for(weights).saturating_add(weights)) else {
        return 0;
    };
    if cache_per_token == 0 {
        // No growing cache: the window costs nothing beyond the weights, so the
        // model's own limit is the only one.
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

/// What MCF worked out about running one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// The engine, by component name.
    pub engine: String,
    /// The device it would run on.
    pub device: Device,
    /// The largest window this pair can hold, in tokens.
    pub context: u64,
}

/// Why a model will not run, in words a person can act on.
///
/// No rule numbers and no clause references: a citation is for the record,
/// where it can be followed, and on a screen it is noise nobody can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Nothing is provisioned.
    NoEngine,
    /// Engines exist, but none has room for this model.
    DoesNotFit {
        /// The largest free memory any device had.
        largest_device: u64,
        /// What the model needs at the smallest window worth running.
        needs: u64,
    },
    /// The header does not say enough to work it out.
    HeaderIncomplete,
}

impl Refused {
    /// One sentence, for a person.
    #[must_use]
    pub fn says(&self) -> String {
        match self {
            Self::NoEngine => "no engine is installed yet — MCF can build one for you".to_owned(),
            Self::DoesNotFit {
                largest_device,
                needs,
            } => format!(
                // **The ceiling is named, not just the two numbers.** Without
                // it this said *needs 111.92 GB, has 129.15 GB free* and then
                // refused, which reads as MCF contradicting itself: the reader
                // does the subtraction, gets a positive number, and concludes
                // the refusal is a bug. What decides it is that MCF plans to a
                // fraction of free memory rather than all of it, so that is
                // the sentence's subject (F138, §3.15).
                "this model needs about {}, and MCF plans to at most {}% of what a device has \
                 free — {} of the {} here — so it does not fit",
                gigabytes(*needs),
                HEADROOM_NUMERATOR,
                gigabytes(
                    largest_device
                        .saturating_mul(HEADROOM_NUMERATOR)
                        .checked_div(HEADROOM_DENOMINATOR)
                        .unwrap_or(0)
                ),
                gigabytes(*largest_device)
            ),
            Self::HeaderIncomplete => {
                "this model's file does not say how it is shaped, so MCF cannot tell whether \
                 it runs"
                    .to_owned()
            }
        }
    }
}

/// A size a person reads, to two places and without a float.
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

/// Whether an accelerator's driver is loaded on this machine.
///
/// A file read, not a process: the kernel lists the cards its driver holds
/// under `/proc/driver/nvidia/gpus`, one directory each, and a directory
/// there is a card a CUDA build could use. Nothing is asked of the card.
#[must_use]
pub fn accelerator_driver_present() -> bool {
    std::fs::read_dir("/proc/driver/nvidia/gpus").is_ok_and(|mut cards| cards.next().is_some())
}

/// The back end a card on this machine is driven through, read from what the
/// kernel and the system installed — not from the card's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// NVIDIA's driver is loaded.
    Cuda,
    /// A card the kernel drives through a driver mesa has a Vulkan driver
    /// for, and the loader's table lists that driver.
    Vulkan,
    /// No card, or one nothing here can drive.
    None,
}

/// What this machine's card is driven through, where it has one.
///
/// CUDA first where NVIDIA's driver is loaded, which is the build that reads
/// that card best; otherwise Vulkan where a card the kernel drives through
/// `amdgpu`, `i915` or `xe` has its Vulkan driver installed. Both halves are
/// checked: a card without its Vulkan driver is a card an engine built for
/// Vulkan would start and not find, and saying so here is cheaper than
/// building one to find out.
#[must_use]
pub fn backend_present() -> Backend {
    backend_from(
        accelerator_driver_present(),
        Path::new("/sys/class/drm"),
        Path::new("/usr/share/vulkan/icd.d"),
    )
}

/// The same choice, over the directories it reads (B-015).
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

/// The kernel driver behind each card the DRM class lists, by card, read
/// from the driver link the kernel publishes and nothing else.
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

/// The kernel driver behind the first card here, where there is one.
#[must_use]
pub fn card_driver_present() -> Option<String> {
    card_drivers(Path::new("/sys/class/drm")).into_iter().next()
}

/// What the cards here hold in their memory right now, summed, as the
/// driver publishes it — the card's own memory and the system memory it
/// addresses, since on a chip that carves its memory out of the system's the
/// second is where a model goes. `None` where no card publishes the figure.
///
/// This is what a load onto a card shows: the engine's own resident memory
/// does not grow with weights that went to the card, and a load watched by
/// that figure alone read *0.1 GiB of 16 read* for its whole length (A7).
#[must_use]
pub fn card_memory_used() -> Option<u64> {
    card_memory_used_under(Path::new("/sys/class/drm"))
}

/// The same, over the directory it reads (B-015).
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

/// The component this machine's card wants, where it has one and an engine
/// for it is not already provisioned.
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

/// The component a model on this machine would run on: the accelerator build
/// where the driver for one is loaded, the processor build otherwise.
///
/// **Decided from the driver, not the card.** A CUDA build on a machine whose
/// kernel has no NVIDIA driver finds no device and runs on the processor
/// anyway, slower to build and no faster to run — so what decides is whether
/// the driver is there, which is the one thing a build could use. A machine
/// that gains a card later gains the other build the next time a model is
/// held with nothing to run it on; nothing here removes the one it has.
///
/// `None` only if MCF's own component table has lost the entry, which a test
/// forbids; a caller says so rather than building something else.
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

/// Picks the engine and device that give this model the largest window.
///
/// Largest window rather than fastest device, because speed is a measurement
/// MCF may not have and capacity is arithmetic it always has. A surface that
/// wants the faster one asks after the diagnostics have run.
///
/// # Errors
///
/// A [`Refused`] saying, in a sentence, why nothing here runs it.
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
                    // A tie goes to the card: it is the same window and it will
                    // be quicker, which is the one thing MCF can say about
                    // speed without having measured anything.
                    || (context == held.context
                        && device.kind == Kind::Gpu
                        && held.device.kind == Kind::Cpu)
            });
            if better {
                best = Some(Choice {
                    engine: engine.name.clone(),
                    device: device.clone(),
                    context,
                });
            }
        }
    }
    best.ok_or(Refused::DoesNotFit {
        largest_device,
        needs: weights
            .saturating_add(overhead_for(weights))
            .saturating_add(SMALLEST_CONTEXT.saturating_mul(cache_per_token)),
    })
}

#[cfg(test)]
mod tests;
