pub mod chart;
pub mod font;
pub mod job;
pub mod paint;
pub mod paper;
pub mod sdl;
pub mod ui;
pub mod view;
pub mod words;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_record::json::Value;

use crate::job::refused_because;
use mcf_serve::control::{Answer, Request};

#[derive(Debug, Clone, Copy)]
pub struct Action {
    pub key: &'static str,
    pub does: &'static str,
    pub reaches: Option<&'static str>,
}

pub const ACTIONS: &[Action] = &[
    Action {
        key: "click a model",
        does: "open what is known about it",
        reaches: Some("Holding"),
    },
    Action {
        key: "click Your computer",
        does: "show what this machine can run",
        reaches: Some("Status"),
    },
    Action {
        key: "open Your computer",
        does: "show the record's newest classified failures, each with its context",
        reaches: Some("Failures"),
    },
    Action {
        key: "scroll",
        does: "move through a long list",
        reaches: None,
    },
    Action {
        key: "r",
        does: "ask MCF again",
        reaches: Some("Status"),
    },
    Action {
        key: "click Components",
        does: "show what MCF can build and what is already here",
        reaches: Some("Components"),
    },
    Action {
        key: "click What is in it",
        does: "count what the chosen model's file holds, header against directory",
        reaches: Some("Anatomy"),
    },
    Action {
        key: "click Vocabulary",
        does: "count the chosen model's token list and chat template, from What is in it",
        reaches: Some("Anatomy"),
    },
    Action {
        key: "Ctrl+V",
        does: "paste a reference into the field being typed into",
        reaches: None,
    },
    Action {
        key: "q or Escape",
        does: "close the window",
        reaches: None,
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct Hosted {
    pub model: String,
    pub address: String,
    pub since: String,
    pub context: Option<u64>,
    pub projector: Option<String>,
    pub takes: Option<mcf_serve::takes::Takes>,
    pub api_key: bool,
    pub network_address: Option<String>,
    pub in_use: Option<Use>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Use {
    pub generated: Option<u64>,
    pub prompted: Option<u64>,
    pub generated_live: Option<u64>,
    pub generated_per_second: Option<f32>,
    pub rate_over_seconds: Option<f32>,
    pub card_power_watts: Option<f32>,
    pub power_named: Option<String>,
    pub power_is: Option<String>,
    pub card_energy_joules: Option<f32>,
    pub card_energy_over_seconds: Option<f32>,
    pub prompted_per_second: Option<f32>,
    pub cache_used: Option<f32>,
    pub processing: Option<u64>,
    pub queued: Option<u64>,
    pub resident: Option<u64>,
    pub card: Option<u64>,
    pub uptime_seconds: Option<u64>,
    pub cache_tokens: Option<u64>,
    pub decodes: Option<u64>,
}

impl Use {
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let count = |key: &str| match value.get(key) {
            Some(Value::Integer(held)) => u64::try_from(*held).ok(),
            Some(Value::Text(text)) => text.trim().parse::<f32>().ok().map(|held| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a count the engine wrote as a float"
                )]
                let whole = held.max(0.0) as u64;
                whole
            }),
            _ => None,
        };
        let rate = |key: &str| match value.get(key) {
            Some(Value::Text(text)) => text.trim().parse::<f32>().ok(),
            Some(Value::Integer(held)) => {
                #[expect(clippy::cast_precision_loss, reason = "a rate, shown to one decimal")]
                let held = *held as f32;
                Some(held)
            }
            _ => None,
        };
        Self {
            generated: count("generated_tokens"),
            prompted: count("prompted_tokens"),
            generated_live: count("generated_tokens_live"),
            generated_per_second: rate("generated_tokens_per_second"),
            rate_over_seconds: rate("rate_over_seconds"),
            card_power_watts: rate("card_power_watts"),
            power_named: value
                .get("power_named")
                .and_then(Value::as_text)
                .map(str::to_owned),
            power_is: value
                .get("power_is")
                .and_then(Value::as_text)
                .map(str::to_owned),
            card_energy_joules: rate("card_energy_joules"),
            card_energy_over_seconds: rate("card_energy_over_seconds"),
            prompted_per_second: rate("prompt_tokens_per_second"),
            cache_used: rate("cache_used_ratio"),
            processing: count("requests_processing"),
            queued: count("requests_queued"),
            resident: count("resident_bytes"),
            card: count("card_bytes"),
            uptime_seconds: count("uptime_seconds"),
            cache_tokens: count("cache_tokens"),
            decodes: count("decodes"),
        }
    }
}

impl Hosted {
    #[must_use]
    pub fn name(&self) -> String {
        self.model
            .rsplit('/')
            .next()
            .unwrap_or(&self.model)
            .trim_end_matches(".gguf")
            .to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastHold {
    pub model: String,
    pub device: String,
    pub engine: String,
    pub stopped: bool,
    pub ago_seconds: Option<u64>,
}

impl LastHold {
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        Some(Self {
            model: text("model")?,
            device: text("device").unwrap_or_else(|| "?".to_owned()),
            engine: text("engine").unwrap_or_else(|| "?".to_owned()),
            stopped: !matches!(value.get("until"), None | Some(Value::Null)),
            ago_seconds: value
                .get("ago_seconds")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
        })
    }

    #[must_use]
    pub fn name(&self) -> String {
        self.model
            .rsplit('/')
            .next()
            .unwrap_or(&self.model)
            .trim_end_matches(".gguf")
            .to_owned()
    }

    #[must_use]
    pub fn said(&self) -> String {
        let ago = self.ago_seconds.map_or_else(String::new, |seconds| {
            format!(", {} ago", ago_said(seconds))
        });
        format!(
            "Last served: {} on {}, {}{ago}",
            self.name(),
            self.device,
            if self.stopped {
                "stopped"
            } else {
                "ran until MCF stopped"
            }
        )
    }
}

#[must_use]
pub fn ago_said(seconds: u64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "whole minutes and hours, and the rest"
    )]
    let (hours, minutes, rest) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    match (hours, minutes) {
        (0, 0) => format!("{rest} s"),
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub on: String,
    pub engine: String,
    pub device: String,
    pub gpu_layers: u32,
    pub free: Option<u64>,
}

impl Placement {
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        Some(Self {
            on: text("on")?,
            engine: text("engine")?,
            device: text("device")?,
            gpu_layers: value
                .get("gpu_layers")
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok())?,
            free: value
                .get("free_bytes")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
        })
    }
}

#[must_use]
pub fn will_take(
    held: &Model,
    settings: &mcf_serve::hosting::Hosting,
    placements: &[Placement],
) -> Option<(String, bool)> {
    let (_, total) = view::reserve_of(held, settings.context)?;
    let total = total?;
    let free = placements
        .iter()
        .find(|placement| placement.device == settings.device)
        .and_then(|placement| placement.free)
        .or(held.device_free)?;
    let fits = total <= free;
    Some((
        if fits {
            format!(
                "Memory required: {} of {} free on {}",
                view::gigabytes(total),
                view::gigabytes(free),
                settings.device
            )
        } else {
            format!(
                "Won't fit: {} needed, {} free on {}",
                view::gigabytes(total),
                view::gigabytes(free),
                settings.device
            )
        },
        fits,
    ))
}

#[must_use]
pub fn loading_said(read: u64, on_card: bool, of: Option<u64>, seconds: u64) -> String {
    let where_ = if on_card { " to GPU" } else { "" };
    match of {
        Some(of) if read >= of => format!(
            "Loading{where_}: {} so far — {} of weights on, KV cache and buffers next · {seconds} s",
            view::gigabytes(read),
            view::gigabytes(of)
        ),
        Some(of) => {
            let left = if read.saturating_mul(20) >= of && seconds > 0 && read > 0 {
                #[expect(
                    clippy::integer_division,
                    reason = "whole seconds left at the rate so far; the remainder is under a second"
                )]
                let eta = (of - read).saturating_mul(seconds) / read;
                if eta == 0 {
                    " · nearly done".to_owned()
                } else {
                    format!(" · ~{eta} s left")
                }
            } else {
                String::new()
            };
            format!(
                "Loading{where_}: {} of {} weights · {seconds} s{left}",
                view::gigabytes(read),
                view::gigabytes(of)
            )
        }
        None => format!("Loading{where_}: {} · {seconds} s", view::gigabytes(read)),
    }
}

fn said_of(version: &str, revision: &str) -> String {
    let short: String = revision.chars().take(7).collect();
    if short.is_empty() || short == "unknown" {
        version.to_owned()
    } else {
        format!("{version} · {short}")
    }
}

pub const FAULTS_SHOWN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fault {
    pub at: String,
    pub category: String,
    pub meaning: String,
    pub attribution: String,
    pub disposition: String,
    pub subsystem: String,
    pub detail: String,
    pub context: Vec<(String, String)>,
    pub caused_by: Option<Box<Fault>>,
    pub asked: String,
}

#[must_use]
pub fn fault_from(entry: &Value) -> Fault {
    let body = entry.get("body").unwrap_or(entry);
    let mut fault = fault_body(body);
    fault.at = entry
        .get("recorded_at")
        .and_then(Value::as_text)
        .map(|at| at.chars().take(19).collect())
        .unwrap_or_default();
    fault
}

fn fault_body(body: &Value) -> Fault {
    let text = |key: &str| {
        body.get(key)
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let context = match body.get("context") {
        Some(Value::Map(fields)) => fields
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value
                        .as_text()
                        .map_or_else(|| value.to_line(), str::to_owned),
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    let caused_by = match body.get("caused_by") {
        Some(cause @ Value::Map(_)) => Some(Box::new(fault_body(cause))),
        _ => None,
    };
    Fault {
        at: String::new(),
        category: text("category"),
        meaning: text("meaning"),
        attribution: text("attribution"),
        disposition: text("disposition"),
        subsystem: text("subsystem"),
        detail: text("detail"),
        context,
        caused_by,
        asked: text("asked"),
    }
}

#[must_use]
pub fn fault_lines(fault: &Fault) -> Vec<String> {
    let mut lines = vec![
        format!("{} — {}", fault.category, fault.meaning),
        format!(
            "{}; {}; in {}",
            attribution_said(&fault.attribution),
            disposition_said(&fault.disposition),
            fault.subsystem
        ),
    ];
    if !fault.detail.is_empty() {
        lines.push(fault.detail.clone());
    }
    for (key, value) in &fault.context {
        lines.push(format!("{key}: {value}"));
    }
    if !fault.asked.is_empty() {
        lines.push(format!("asked: {}", fault.asked));
    }
    if let Some(cause) = &fault.caused_by {
        lines.push(format!("because: {} — {}", cause.category, cause.detail));
    }
    lines
}

fn attribution_said(held: &str) -> String {
    match held {
        "machine" => "the machine's doing".to_owned(),
        "user" => "the operator's doing".to_owned(),
        "mcf" => "MCF's own doing".to_owned(),
        "managed" => "something MCF manages".to_owned(),
        "hub" => "the hub's doing".to_owned(),
        "artifact" => "the artifact's own".to_owned(),
        "" => "unattributed".to_owned(),
        other => format!("attributed to {other}"),
    }
}

fn disposition_said(held: &str) -> String {
    match held {
        "refused" => "refused".to_owned(),
        "degraded" => "went on degraded".to_owned(),
        "partial" => "partly done".to_owned(),
        "recovered" => "recovered".to_owned(),
        "" => "no disposition".to_owned(),
        other => other.to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub commit: String,
    pub role: String,
    pub image: String,
    pub provisioned: bool,
    pub present: bool,
    pub usable_engine: bool,
    pub prefix: String,
}

#[must_use]
pub fn component_from(held: &Value) -> Component {
    let text = |key: &str| {
        held.get(key)
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let flag = |key: &str| matches!(held.get(key), Some(Value::Bool(true)));
    Component {
        name: text("name"),
        commit: text("commit").chars().take(12).collect(),
        role: text("role"),
        image: text("image"),
        provisioned: flag("provisioned"),
        present: flag("present"),
        usable_engine: flag("usable_engine"),
        prefix: text("prefix"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Caret {
    Setting,
    #[default]
    Document,
    Temperature,
    System,
    Effort,
    Picture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Monitor,
    Host,
    Diagnostics,
    Models,
    Components,
    Settings,
    Exit,
    Adding,
    Hosting,
    Prompt,
    Anatomy,
    Vocabulary,
}

impl Page {
    pub const MENU: &'static [(Self, &'static str)] = &[
        (Self::Monitor, "System"),
        (Self::Models, "Models"),
        (Self::Hosting, "Server"),
        (Self::Diagnostics, "Diagnostics"),
        (Self::Exit, "Exit"),
    ];

    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Host | Self::Adding | Self::Anatomy | Self::Vocabulary | Self::Models => {
                Self::Models
            }
            Self::Hosting => Self::Hosting,
            Self::Diagnostics | Self::Prompt => Self::Diagnostics,
            Self::Monitor | Self::Components | Self::Settings => Self::Monitor,
            Self::Exit => Self::Exit,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Finding {
    pub name: String,
    pub at: Option<String>,
    pub engine: Option<String>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub name: String,
    pub path: String,
    pub bytes: Option<u64>,
    pub architecture: Option<String>,
    pub trained: Option<u64>,
    pub context: Option<u64>,
    pub cache_per_token: Option<u64>,
    pub engine: Option<String>,
    pub device: Option<String>,
    pub device_free: Option<u64>,
    pub measured_body: Option<Value>,
    pub measured_at: Option<String>,
    pub cross_checked: Vec<String>,
    pub cross_checked_at: Option<String>,
    pub prompt_reported: bool,
    pub prompt_reported_at: Option<String>,
    pub applied_addressing: Option<String>,
    pub applied_budget: Option<String>,
    pub probed: Vec<Finding>,
    pub readings_at: std::collections::BTreeMap<String, String>,
    pub repository: Option<String>,
    pub file: String,
    pub on_a_card: bool,
    pub refused: Option<String>,
    pub speed: Option<f64>,
    pub start_up: Option<String>,
    pub fastest: Option<f64>,
    pub slowest: Option<f64>,
    pub ladder: Vec<crate::chart::Reading>,
}

pub use mcf_tui::screens::diagnostics::Test;

pub use mcf_tui::screens::diagnostics::Run;

#[must_use]
pub fn tests() -> Vec<Test> {
    mcf_tui::screens::diagnostics::tests()
}

impl Model {
    #[must_use]
    pub fn will_run(&self) -> bool {
        self.refused.is_none() && self.engine.is_some()
    }

    #[must_use]
    pub fn where_it_runs(&self) -> String {
        if let Some(why) = &self.refused {
            return why.clone();
        }
        if self.on_a_card {
            "Ready to run on your graphics card.".to_owned()
        } else if self.engine.is_some() {
            "Ready to run on your processor.".to_owned()
        } else {
            "MCF has not worked out how to run this one.".to_owned()
        }
    }

    #[must_use]
    pub fn in_a_sentence(&self) -> String {
        if let Some(why) = &self.refused {
            return why.clone();
        }
        let place = if self.on_a_card {
            "your graphics card"
        } else {
            "your processor"
        };
        match words::speed_in_words(self.speed) {
            Some(speed) => format!("Runs on {place} at about {speed}."),
            None => format!("Will run on {place}. MCF has not timed it on this computer yet."),
        }
    }

    #[must_use]
    pub fn memory_sentence(&self) -> String {
        words::size_in_words(self.bytes).map_or_else(
            || words::UNMEASURED.to_owned(),
            |size| format!("Uses {size}"),
        )
    }

    #[must_use]
    pub fn speed_at_512(&self) -> String {
        self.fastest.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |ms| format!("{ms:.2} ms/token"),
        )
    }

    #[must_use]
    pub fn speed_at_window(&self) -> String {
        self.slowest.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |ms| format!("{ms:.2} ms/token"),
        )
    }

    #[must_use]
    pub fn speed_rows(&self) -> [(String, String); 2] {
        let at = |reading: Option<&crate::chart::Reading>, or: &str| {
            reading.map_or_else(
                || or.to_owned(),
                |held| format!("at {} tokens", words::grouped(held.depth)),
            )
        };
        [
            (
                at(self.ladder.first(), "at 512 tokens"),
                self.speed_at_512(),
            ),
            (
                at(self.ladder.last(), "at the deepest rung"),
                self.speed_at_window(),
            ),
        ]
    }

    #[must_use]
    pub fn start_up(&self) -> String {
        self.start_up
            .as_ref()
            .map_or_else(|| crate::view::UNKNOWN.to_owned(), |ms| format!("{ms} ms"))
    }

    #[must_use]
    pub fn measured(&self) -> bool {
        self.fastest.is_some() || self.slowest.is_some() || self.start_up.is_some()
    }

    #[must_use]
    pub fn technical(&self) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        let unknown = || words::UNMEASURED.to_owned();
        rows.push((
            "Architecture".to_owned(),
            self.architecture.clone().unwrap_or_else(unknown),
        ));
        rows.push((
            "File".to_owned(),
            self.bytes
                .map_or_else(unknown, |bytes| format!("{} bytes", words::grouped(bytes))),
        ));
        rows.push((
            "Trained context".to_owned(),
            self.trained
                .map_or_else(unknown, |held| format!("{} tokens", words::grouped(held))),
        ));
        rows.push((
            "Largest window here".to_owned(),
            self.context
                .map_or_else(unknown, |held| format!("{} tokens", words::grouped(held))),
        ));
        rows.push((
            "Engine".to_owned(),
            match (&self.engine, &self.device) {
                (Some(engine), Some(device)) => format!("{engine} · {device}"),
                (Some(engine), None) => engine.clone(),
                _ => unknown(),
            },
        ));
        rows.push((
            "Speed".to_owned(),
            self.speed
                .map_or_else(unknown, |rate| format!("{rate:.1} tokens a second")),
        ));
        rows.push(("Path".to_owned(), self.path.clone()));
        rows
    }
}

fn repository_of(held: &Value) -> Option<String> {
    held.get("provenance")
        .and_then(|provenance| provenance.get("origin"))
        .filter(|origin| origin.get("kind").and_then(Value::as_text) == Some("hub"))
        .and_then(|origin| origin.get("repository"))
        .and_then(Value::as_text)
        .map(str::to_owned)
}

fn readings_at_of(held: &Value) -> std::collections::BTreeMap<String, String> {
    match held.get("readings_at") {
        Some(Value::Map(entries)) => entries
            .iter()
            .filter_map(|(method, at)| Some((method.clone(), at.as_text()?.to_owned())))
            .collect(),
        _ => std::collections::BTreeMap::new(),
    }
}

fn probed_of(held: &Value) -> Vec<Finding> {
    held.get("probed")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(|found| {
            let text = |key: &str| found.get(key).and_then(Value::as_text).map(str::to_owned);
            Some(Finding {
                name: text("method")?,
                at: text("at"),
                engine: text("engine"),
                lines: vec![text("said")?],
            })
        })
        .collect()
}

fn model_from(held: &Value) -> Model {
    let text = |key: &str| held.get(key).and_then(Value::as_text).map(str::to_owned);
    let path = text("path").unwrap_or_default();
    let file = path
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or_default()
        .to_owned();
    let name = file.trim_end_matches(".gguf").to_owned();
    let name = if name.is_empty() {
        "a model".to_owned()
    } else {
        name
    };
    let bytes = held
        .get("bytes")
        .and_then(Value::as_integer)
        .and_then(|number| u64::try_from(number).ok());

    let runs = held.get("runs");
    let from_runs = |key: &str| {
        runs.as_ref()
            .and_then(|runs| runs.get(key))
            .and_then(Value::as_text)
            .map(str::to_owned)
    };
    let number_from_runs = |key: &str| {
        runs.as_ref()
            .and_then(|runs| runs.get(key))
            .and_then(Value::as_integer)
            .and_then(|number| u64::try_from(number).ok())
    };
    let measured = runs
        .as_ref()
        .and_then(|runs| runs.get("measured"))
        .map(measured_ends);
    let results = results_of(runs);
    let resolved = runs.as_ref().and_then(|runs| runs.get("resolved"));
    let known = matches!(
        resolved.as_ref().and_then(|resolved| resolved.get("known")),
        Some(Value::Bool(true))
    );
    let resolved_text = |key: &str| {
        resolved
            .as_ref()
            .and_then(|resolved| resolved.get(key))
            .and_then(Value::as_text)
            .map(str::to_owned)
    };

    Model {
        name,
        path,
        bytes,
        architecture: from_runs("architecture"),
        trained: number_from_runs("trained_context"),
        context: resolved
            .as_ref()
            .and_then(|resolved| resolved.get("context"))
            .and_then(Value::as_integer)
            .and_then(|number| u64::try_from(number).ok()),
        engine: known.then(|| resolved_text("engine")).flatten(),
        device: known.then(|| resolved_text("device")).flatten(),
        device_free: resolved
            .as_ref()
            .and_then(|resolved| resolved.get("device_free_bytes"))
            .and_then(Value::as_integer)
            .and_then(|number| u64::try_from(number).ok()),
        measured_at: at_in(results.0.as_ref()),
        measured_body: results.0,
        cross_checked: results.1,
        cross_checked_at: results.3,
        prompt_reported: results.2,
        prompt_reported_at: results.4,
        // **What the daemon says of a model's runs is under `runs`.** The
        applied_addressing: runs
            .and_then(|runs| runs.get("configured"))
            .and_then(|applied| applied.get("addressing"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        applied_budget: runs
            .and_then(|runs| runs.get("configured"))
            .and_then(|applied| applied.get("budget"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        repository: repository_of(held),
        file,
        probed: runs.map_or_else(Vec::new, probed_of),
        readings_at: runs.map_or_else(std::collections::BTreeMap::new, readings_at_of),
        on_a_card: resolved_text("device_kind").as_deref() == Some("gpu"),
        cache_per_token: number_from_runs("cache_bytes_per_token"),
        refused: if known { None } else { resolved_text("why") },
        speed: measured
            .as_ref()
            .and_then(|held| held.fastest)
            .map(per_second),
        start_up: measured.as_ref().and_then(|held| held.start_up.clone()),
        fastest: measured.as_ref().and_then(|held| held.fastest),
        slowest: measured.as_ref().and_then(|held| held.slowest),
        ladder: measured.map(|held| held.ladder).unwrap_or_default(),
    }
}

fn at_in(body: Option<&Value>) -> Option<String> {
    body.and_then(|body| body.get("at"))
        .and_then(Value::as_text)
        .map(str::to_owned)
}

fn results_of(
    runs: Option<&Value>,
) -> (
    Option<Value>,
    Vec<String>,
    bool,
    Option<String>,
    Option<String>,
) {
    let measured = runs
        .and_then(|runs| runs.get("measured"))
        .filter(|held| !matches!(held, Value::Null))
        .cloned();
    let cross_checked = runs
        .and_then(|runs| runs.get("cross_checked"))
        .and_then(|held| held.get("said"))
        .and_then(Value::as_list)
        .map(|said| {
            said.iter()
                .filter_map(Value::as_text)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let prompt_reported = runs
        .and_then(|runs| runs.get("prompt_reported"))
        .is_some_and(|held| !matches!(held, Value::Null));
    let at_of = |key: &str| at_in(runs.and_then(|runs| runs.get(key)));
    (
        measured,
        cross_checked,
        prompt_reported,
        at_of("cross_checked"),
        at_of("prompt_reported"),
    )
}

#[must_use]
pub fn held_mark(held: Option<&Value>, depth: i64) -> String {
    let Some(held) = held.filter(|held| !matches!(held, Value::Null)) else {
        return "—".to_owned();
    };
    match held.get("first_rank").and_then(Value::as_integer) {
        Some(rank) => rank.to_string(),
        None => format!(">{depth}"),
    }
}

#[must_use]
pub fn open_mark(held: Option<&Value>) -> String {
    let Some(held) = held.filter(|held| !matches!(held, Value::Null)) else {
        return "—".to_owned();
    };
    let count = |key: &str| held.get(key).and_then(Value::as_integer).unwrap_or(0);
    format!("{}/{}", count("kept"), count("of"))
}

#[must_use]
pub fn answer_marks(body: &Value) -> Vec<String> {
    let mut marks = Vec::new();
    if let Some(tokens) = body.get("answer_tokens").and_then(Value::as_integer) {
        marks.push(if tokens == 1 {
            "1 token".to_owned()
        } else {
            format!("{tokens} tokens")
        });
    }
    if let Some(stopped) = body.get("answer_stopped").and_then(Value::as_text) {
        marks.push(match stopped {
            "stop_token" => "ended at its stop token".to_owned(),
            "limit" => "ran to the cap".to_owned(),
            "stop_word" => "ended at a stop word".to_owned(),
            other => format!("ended: {other}"),
        });
    }
    marks
}

pub const NOTHING_WRITTEN: &str = "nothing written";

#[must_use]
pub fn pair_mark(at: usize) -> String {
    format!("{}&{}", at.saturating_add(1), at.saturating_add(2))
}

#[must_use]
pub fn expected_mark(grouped: Option<&Value>, at: usize) -> Option<String> {
    let part = grouped?.get("parts")?.as_list()?.get(at)?;
    let first = part.get("first_choice").and_then(Value::as_integer)?;
    let tokens = part.get("tokens").and_then(Value::as_integer)?;
    Some(format!("{first}/{tokens}"))
}

fn per_second(ms: f64) -> f64 {
    if ms > 0.0 { 1000.0 / ms } else { 0.0 }
}

#[derive(Debug, Default)]
struct Measured {
    fastest: Option<f64>,
    slowest: Option<f64>,
    ladder: Vec<crate::chart::Reading>,
    start_up: Option<String>,
}

fn measured_ends(held: &Value) -> Measured {
    let mut ends = Measured {
        start_up: held
            .get("first_token")
            .filter(|figure| matches!(figure.get("measured"), Some(Value::Bool(true))))
            .and_then(|figure| figure.get("ms"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        ..Measured::default()
    };
    let Some(readings) = held.get("readings").and_then(Value::as_list) else {
        return ends;
    };
    for reading in readings {
        if !matches!(reading.get("measured"), Some(Value::Bool(true))) {
            continue;
        }
        let Some(ms) = reading
            .get("ms_per_token")
            .and_then(Value::as_text)
            .and_then(|held| held.parse::<f64>().ok())
        else {
            continue;
        };
        if ends.fastest.is_none() {
            ends.fastest = Some(ms);
        }
        ends.slowest = Some(ms);
        if let Some(depth) = reading
            .get("depth")
            .and_then(Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
        {
            ends.ladder.push(crate::chart::Reading { depth, ms });
        }
    }
    ends
}

fn ask(socket: &Path, request: &Request) -> Result<Answer, String> {
    ask_within(socket, request, std::time::Duration::from_secs(30))
}

const POLL: std::time::Duration = std::time::Duration::from_secs(5);

fn ask_within(
    socket: &Path,
    request: &Request,
    deadline: std::time::Duration,
) -> Result<Answer, String> {
    let mut connection = UnixStream::connect(socket)
        .map_err(|_| "MCF is not answering on this computer".to_owned())?;
    let _deadline = connection.set_read_timeout(Some(deadline));
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("the request could not be sent: {error}"))?;
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|error| format!("MCF did not answer: {error}"))?;
    Answer::read(line.trim_end()).map_err(|failure| failure.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Model,
    Window,
    On,
    Placement,
    Rope,
    Quantization,
    Architecture,
    Fits,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    Library,
    Page,
    Hub,
    Diagnostics,
    DiagnosticsPage,
    Checks,
    Server,
    Prompt,
    Monitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Splitter {
    Side,
    List,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Splits {
    pub side: f32,
    pub list: f32,
    pub diagnostics: f32,
}

impl Default for Splits {
    fn default() -> Self {
        Self {
            side: 168.0,
            list: 250.0,
            diagnostics: 300.0,
        }
    }
}

fn as_points(whole: i32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a point on the screen, exact in f32 at any window size"
    )]
    let at = whole as f32;
    at
}

const PAGE_PAD: f32 = 26.0;

impl Splits {
    pub fn set(&mut self, splitter: Splitter, to: f32) {
        match splitter {
            Splitter::Side => self.side = to.clamp(120.0, 320.0),
            Splitter::List => {
                self.list = (to - self.side - PAGE_PAD).clamp(180.0, 520.0);
            }
            Splitter::Diagnostics => {
                self.diagnostics = (to - self.side - PAGE_PAD).clamp(220.0, 520.0);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filters {
    pub open: bool,
    pub architecture: Option<String>,
    pub fits: Option<bool>,
    pub size: Option<u64>,
}

impl Filters {
    #[must_use]
    pub const fn any_set(&self) -> bool {
        self.architecture.is_some() || self.fits.is_some() || self.size.is_some()
    }
}

pub const SIZE_CHOICES: [Option<u64>; 4] = [
    None,
    Some(8_000_000_000),
    Some(20_000_000_000),
    Some(50_000_000_000),
];

pub const FITS_CHOICES: [Option<bool>; 3] = [None, Some(true), Some(false)];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubList {
    pub query: String,
    pub repositories: Vec<HubRepo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubRepo {
    pub id: String,
    pub downloads: Option<u64>,
}

impl HubRepo {
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            id: value.get("id")?.as_text()?.to_owned(),
            downloads: value
                .get("downloads")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedFile {
    pub file: String,
    pub bytes: Option<u64>,
    pub fits: Option<bool>,
}

impl OfferedFile {
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            file: value.get("file")?.as_text()?.to_owned(),
            bytes: value
                .get("bytes")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            fits: match value.get("fits") {
                Some(Value::Bool(fits)) => Some(*fits),
                _ => None,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub repository: String,
    pub file: String,
    pub bytes: Option<u64>,
    pub fits: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quant {
    pub file: String,
    pub bytes: Option<u64>,
    pub here: Option<usize>,
    pub fits: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub repository: Option<String>,
    pub members: Vec<usize>,
}

impl Group {
    #[must_use]
    pub fn name(&self, models: &[Model]) -> String {
        match &self.repository {
            Some(repository) => repository
                .rsplit('/')
                .next()
                .unwrap_or(repository)
                .to_owned(),
            None => self
                .members
                .first()
                .and_then(|at| models.get(*at))
                .map_or_else(|| "a model".to_owned(), |held| held.name.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    Throughput,
    CrossCheck,
    Capabilities,
    Performance,
    Fidelity,
    Behaviour,
    Prompt,
    Comparison,
    Coding,
}

impl Card {
    pub const ALL: [Self; 9] = [
        Self::Throughput,
        Self::CrossCheck,
        Self::Capabilities,
        Self::Performance,
        Self::Fidelity,
        Self::Behaviour,
        Self::Prompt,
        Self::Comparison,
        Self::Coding,
    ];

    #[must_use]
    pub fn measures(self) -> &'static [&'static str] {
        let family = match self {
            Self::Performance => 0,
            Self::Fidelity => 1,
            Self::Behaviour => 2,
            _ => return &[],
        };
        mcf_serve::examine::FAMILIES
            .get(family)
            .map_or(&[][..], |(_, members)| members)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Throughput => "Throughput",
            Self::CrossCheck => "Cross-check",
            Self::Capabilities => "Capabilities",
            Self::Performance => "Performance",
            Self::Fidelity => "Fidelity",
            Self::Behaviour => "Behaviour",
            Self::Prompt => "Prompt analysis",
            Self::Comparison => "Comparison",
            Self::Coding => "Coding",
        }
    }

    #[must_use]
    pub const fn answers(self) -> &'static str {
        match self {
            Self::Throughput => {
                "Generation speed at each depth, prefill rate, time to first token, KV cache memory"
            }
            Self::CrossCheck => {
                "Whether MCF's own engine agrees with the provisioned engine's tokens"
            }
            Self::Capabilities => {
                "Chat template, stop conditions, thinking, tool calls, context, language cost, vision"
            }
            Self::Performance => {
                "Offload curve, prefill saturation, prefix reuse, memory as predicted, concurrency, cold start"
            }
            Self::Fidelity => {
                "Agreement with a reference file, bits per byte, determinism, tokenizer round trip"
            }
            Self::Behaviour => "Retrieval by depth, degeneration, grammar cost, image cost",
            Self::Prompt => "What each part of a prompt does to the answer",
            Self::Comparison => "Two models on one question under one engine",
            Self::Coding => {
                "Twenty tasks in Python, JavaScript and Rust, edits, repairs and tests, every answer run in a container"
            }
        }
    }

    #[must_use]
    pub fn command(self, model: &str) -> Option<String> {
        match self {
            Self::Comparison => Some(format!("mcf bench {model} <other-model> --prompt \"…\"")),
            Self::Coding => Some(format!("mcf eval {model}")),
            Self::Throughput
            | Self::CrossCheck
            | Self::Capabilities
            | Self::Performance
            | Self::Fidelity
            | Self::Behaviour
            | Self::Prompt => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Diagnostic {
    Throughput,
    CrossCheck,
    Prompt,
    Comparison,
    Probe(usize),
    Measure(usize),
    Eval(usize),
}

pub const SUITES: [(&str, &str, &str); 7] = [
    ("challenges-easy", "Challenges: easy", "challenges-easy"),
    (
        "challenges-medium",
        "Challenges: medium",
        "challenges-medium",
    ),
    ("challenges-hard", "Challenges: hard", "challenges-hard"),
    (
        "challenges-expert",
        "Challenges: expert",
        "challenges-expert",
    ),
    ("editing", "Editing", "editing"),
    ("tests", "Test writing", "test-writing"),
    ("queries", "SQL and patterns", "queries"),
];

pub const RETRIES_DEFAULT: usize = 10;

pub const SMALLEST_WINDOW: u64 = 4096;

pub const LANGUAGE_NAMES: [&str; 4] = ["python", "javascript", "rust", "go"];

const SUITE_ANSWERS: [&str; 7] = [
    "The catalogue's fourteen easy challenges in Python, JavaScript, Rust and Go, up to ten attempts each: the attempt that solved it, the corrections, the tokens and the time",
    "The seventeen medium challenges — parsing, geometry, dynamic programming, bits — in the same four languages, with retries",
    "The ten hard challenges — graphs, search, caches, sequences — in the same four languages, with retries",
    "The three expert challenges — regular expressions, grid validity, knapsack — in the same four languages, with retries",
    "A whole file given and one change asked: the cases held, and every untouched function compared byte for byte",
    "Tests written for a stated function, run against a correct implementation and three broken ones",
    "SQL queries run against a fixed table beside the reference, and patterns run against match and no-match cases",
];

const PROBE_ANSWERS: [&str; 9] = [
    "How the model is addressed: which form of its template ends its turn",
    "How much of the declared context the engine accepts",
    "Whether the model ends its own turn, and how long its turns run",
    "Whether a well-formed tool call comes out when one is asked for",
    "Whether the shape asked for comes out as JSON, with the fields asked",
    "How many tokens go before the answer, inside the markers the file declares",
    "What a sentence costs in tokens, language by language",
    "Whether a vector comes out, its width, and whether it comes out the same twice",
    "Whether a picture reaches the model at all",
];

const MEASURE_ANSWERS: [&str; 47] = [
    "Tokens a second at nought, a quarter, half, three quarters and all of the layers on the card",
    "Prompt-reading tokens a second across batch sizes, and where reading more at once stops helping",
    "What a conversation pays for its history every turn: a kept prefix against the prompt read again",
    "The engine's resident bytes at each window against what MCF predicted from the header",
    "Aggregate and per-request tokens a second at one, two, four and eight requests at once",
    "The first token with the file evicted from the page cache, against the same with it there",
    "Agreement with the repository's most precise file here, position by position, and the bits spent",
    "The log-likelihood of a fixed text, per byte so vocabularies compare",
    "Whether the same prompt, seed and greedy draw produce the same tokens, and where they part",
    "Whether text survives being read and spelled back, and whether two tokenizers count it the same",
    "Whether a number planted in a long prompt comes back, by depth and by placement",
    "Where a long generation begins to repeat itself",
    "Valid JSON, tokens and time with and without a grammar constraint",
    "Tokens and prefill time a picture adds, at three sides",
    "Tool use over fixed tasks: the right tool, the arguments matched, the result carried, held back when nothing fits",
    "Chains of calls each from the last result, two calls at once, and an error to recover from: steps completed, results carried",
    "Dates, amounts, names and lists pulled from fixed texts into JSON, each field compared exactly",
    "Exactly so many words, no digits, capitals only, a list of a stated length, a stated ending: each a parser's check",
    "One question put six ways, greedy: how many answers agree, how many are right",
    "The same arithmetic and reading tasks in six languages, the same exact match",
    "Four prices planted through a long prompt, asked to list, to order and to sum, at each depth",
    "Tokens a second every thirty seconds over five minutes, with the card's temperature, clock and power beside each sample",
    "Microjoules a produced token and a prompt token, from the card's power summed over the time, with the idle draw beside them",
    "Every gap between one streamed piece and the next in a long generation, the longest stall and where it fell",
    "The same generation with and without the file's own draft head: tokens a second each way, and whether the outputs agree",
    "The key-value cache at 16, 8 and 4 bits: tokens a second, resident bytes, and where the output parts from the 16-bit run",
    "Two hundred requests through one server: which failed, what each took by hundred, resident bytes at the start and the end",
    "From a stop being raised to the engine idle again, at several depths of prompt in flight",
    "Rank agreement with the reference at a hundred, five hundred and a thousand tokens deep",
    "Every file of the repository here read against the most precise one, with its size and speed beside it",
    "Nested objects, arrays, enums and optional fields, free and under the schema constraint, each answer read for its shape",
    "How many distinct answers an exact question draws at five temperatures, and how many are right",
    "Byte fallbacks and unknown tokens over a mixed corpus of scripts, code and symbols, text by text",
    "Computed pictures with countable content — circles, a number, a colour, the larger — asked back exactly",
    "Sums, differences and products at two to twelve digits, exact",
    "Weekdays, days between dates, sorting and counting, each exact",
    "So many distinct items one per line: the count and the repeats read by a parser",
    "A fact asked back after two, five and ten turns, and a correction honoured later",
    "A checkable rule in the system turn held across five turns",
    "Questions a passage does not answer: stated absent or a figure invented",
    "A short program's printed number predicted, and a planted bug's line named",
    "Over fixed triples, whether the paraphrase sits nearer than the unrelated sentence by the model's own embedding",
    "An instruction planted in a document: followed, or the question answered",
    "The same exact questions with thinking on and off: right or not, and the tokens spent thinking",
    "From the server started to its first token at each share of the layers on the card",
    "Tokens a second on the processor at each thread count",
    "A planted number found at 32k, 64k and 128k where the window allows",
];

impl Diagnostic {
    #[must_use]
    pub fn all() -> Vec<Self> {
        Self::families()
            .into_iter()
            .flat_map(|(_, _, members)| members)
            .collect()
    }

    #[must_use]
    pub fn families() -> Vec<(&'static str, Option<Card>, Vec<Self>)> {
        let probes = (0..mcf_serve::probes::run::PROBES.len())
            .map(Self::Probe)
            .collect();
        let family = |at: usize, card: Card| {
            let members = mcf_serve::examine::FAMILIES
                .get(at)
                .map_or(&[][..], |(_, members)| members);
            let mut found = Vec::new();
            for name in members {
                if let Some(place) = mcf_serve::examine::MEASURES
                    .iter()
                    .position(|held| held == name)
                {
                    found.push(Self::Measure(place));
                }
            }
            (card.name(), Some(card), found)
        };
        vec![
            (
                "Runs",
                None,
                vec![
                    Self::Throughput,
                    Self::CrossCheck,
                    Self::Prompt,
                    Self::Comparison,
                ],
            ),
            ("Probes", Some(Card::Capabilities), probes),
            family(0, Card::Performance),
            family(1, Card::Fidelity),
            family(2, Card::Behaviour),
            (
                "Coding",
                Some(Card::Coding),
                (0..SUITES.len()).map(Self::Eval).collect(),
            ),
        ]
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Throughput => "Throughput",
            Self::CrossCheck => "Cross-check",
            Self::Prompt => "Prompt analysis",
            Self::Comparison => "Comparison",
            Self::Probe(at) => mcf_serve::probes::run::PROBES
                .get(at)
                .copied()
                .unwrap_or("?"),
            Self::Measure(at) => mcf_serve::examine::MEASURES.get(at).copied().unwrap_or("?"),
            Self::Eval(at) => SUITES.get(at).map_or("?", |(_, name, _)| name),
        }
    }

    #[must_use]
    pub fn suite(self) -> Option<&'static str> {
        match self {
            Self::Eval(at) => SUITES.get(at).map(|(suite, _, _)| *suite),
            _ => None,
        }
    }

    #[must_use]
    pub fn answers(self) -> &'static str {
        match self {
            Self::Throughput => Card::Throughput.answers(),
            Self::CrossCheck => Card::CrossCheck.answers(),
            Self::Prompt => Card::Prompt.answers(),
            Self::Comparison => Card::Comparison.answers(),
            Self::Probe(at) => PROBE_ANSWERS.get(at).copied().unwrap_or(""),
            Self::Measure(at) => MEASURE_ANSWERS.get(at).copied().unwrap_or(""),
            Self::Eval(at) => SUITE_ANSWERS.get(at).copied().unwrap_or(""),
        }
    }

    #[must_use]
    pub fn card(self) -> Card {
        match self {
            Self::Throughput => Card::Throughput,
            Self::CrossCheck => Card::CrossCheck,
            Self::Prompt => Card::Prompt,
            Self::Comparison => Card::Comparison,
            Self::Probe(_) => Card::Capabilities,
            Self::Eval(_) => Card::Coding,
            Self::Measure(at) => {
                let name = mcf_serve::examine::MEASURES.get(at).copied().unwrap_or("");
                [Card::Performance, Card::Fidelity, Card::Behaviour]
                    .into_iter()
                    .find(|card| card.measures().contains(&name))
                    .unwrap_or(Card::Performance)
            }
        }
    }

    #[must_use]
    pub fn method(self) -> Option<&'static str> {
        match self {
            Self::Probe(_) | Self::Measure(_) => Some(self.name()),
            _ => None,
        }
    }

    #[must_use]
    pub fn readings_method(self) -> Option<&'static str> {
        match self {
            Self::Throughput => Some("throughput"),
            Self::CrossCheck => Some("cross-check"),
            Self::Prompt | Self::Comparison => None,
            Self::Probe(at) => mcf_serve::probes::run::RECORDED.get(at).copied(),
            Self::Measure(_) => Some(self.name()),
            Self::Eval(at) => SUITES.get(at).map(|(_, _, method)| *method),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Configure,
    Statistics,
    Contents,
}

impl Tab {
    pub const ALL: [Self; 3] = [Self::Configure, Self::Statistics, Self::Contents];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Configure => "Configure",
            Self::Statistics => "Statistics",
            Self::Contents => "Contents",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Context,
    Threads,
    Batch,
    Port,
    ApiKey,
    RopeFactor,
    Retries,
    Window,
    Languages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    FlashAttention,
    KeepResident,
    DraftHead,
    Projector,
    Open,
}

pub const ROPE_CHOICES: [Option<mcf_serve::declared::Scaling>; 4] = [
    None,
    Some(mcf_serve::declared::Scaling::Off),
    Some(mcf_serve::declared::Scaling::Linear),
    Some(mcf_serve::declared::Scaling::Yarn),
];

#[must_use]
pub const fn windows() -> [u64; 7] {
    [1_024, 2_048, 4_096, 8_192, 16_384, 32_768, 65_536]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    Without(usize),
    Alone(usize),
    Prefix(usize),
    Swap(usize),
    Form(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    Go(Page),
    CycleThinking,
    LookUp,
    Download { reference: String, file: String },
    Measure { deepest: u64 },
    Run(Card),
    SearchHub,
    Scroll(Region, i32),
    Split(Splitter, i32),
    ToggleFilters,
    SetArchitecture(usize),
    SetFits(usize),
    SetSize(usize),
    Quantization(usize),
    PickOffered(usize),
    DownloadThen(std::boxed::Box<Act>),
    PickHub(usize),
    ApplyProbes,
    Show(Diagnostic),
    RunOne(Diagnostic),
    RunAll,
    SeeStatistics,
    Stop,
    HostAgain,
    Pick(String),
    Tab(Tab),
    Contents(Page),
    Edit(Field),
    Switch(Switch),
    Place(usize),
    Rope(usize),
    Copy(String),
    SetOn(Option<mcf_serve::control::On>),
    Open(Picker),
    ShowWithout(usize),
    ShowAlone(usize),
    ShowPrefix(usize),
    ShowSwap(usize),
    ShowForm(usize),
    Shut,
    SetWindow(u64),
    Cycle(usize),
    Recommended,
    LastSettings,
    HostIt,
    Build(String),
    StopHosting,
    Close,
    ReportPrompt,
    Focus(Caret),
    MostParts(usize),
    TakeApartBy(Option<mcf_serve::prompt::Unit>),
    Extra(mcf_serve::prompt::Extra, bool),
    Ask { at: usize },
    Choose(usize),
    Clear,
    Dismiss,
}

#[derive(Debug)]
pub enum Doing {
    Nothing,
    Hosting(job::Job),
    Provisioning(job::Job),
    Listing(job::Job),
    Downloading(job::Job),
    Measuring(job::Job),
    CrossChecking(job::Job),
    Answering(job::Job),
    Reporting(job::Job),
    Probing(job::Job),
    Examining(job::Job),
    Evaluating(job::Job),
}

impl Doing {
    #[must_use]
    pub fn job(&self) -> Option<&job::Job> {
        match self {
            Self::Nothing => None,
            Self::Reporting(job)
            | Self::Listing(job)
            | Self::Downloading(job)
            | Self::Measuring(job)
            | Self::CrossChecking(job)
            | Self::Answering(job)
            | Self::Provisioning(job)
            | Self::Probing(job)
            | Self::Examining(job)
            | Self::Evaluating(job)
            | Self::Hosting(job) => Some(job),
        }
    }

    #[must_use]
    pub fn busy(&self) -> bool {
        self.job().is_some_and(|job| !job.finished)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnderTest {
    pub model: String,
    pub engine: String,
    pub window: Option<u64>,
    pub in_use: Use,
}

impl UnderTest {
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        Self {
            model: value
                .get("model")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned(),
            engine: value
                .get("engine")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned(),
            window: value
                .get("window")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            in_use: value.get("use").map(Use::from_value).unwrap_or_default(),
        }
    }

    #[must_use]
    pub fn name(&self) -> String {
        std::path::Path::new(&self.model).file_stem().map_or_else(
            || self.model.clone(),
            |stem| stem.to_string_lossy().into_owned(),
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spent {
    pub millijoules: u64,
    pub seconds: u64,
    pub tokens_at_start: Option<u64>,
    pub tokens_now: Option<u64>,
}

impl Spent {
    #[must_use]
    pub fn tokens(&self) -> Option<u64> {
        Some(self.tokens_now?.saturating_sub(self.tokens_at_start?))
    }

    #[must_use]
    #[expect(
        clippy::integer_division,
        reason = "whole tokens a kilojoule is the figure"
    )]
    pub fn tokens_per_kilojoule(&self) -> Option<u64> {
        let tokens = self.tokens()?;
        if self.millijoules == 0 {
            return None;
        }
        Some(tokens.saturating_mul(1_000_000) / self.millijoules)
    }
}

#[derive(Debug)]
pub struct Desk {
    socket: std::path::PathBuf,
    pub page: Page,
    pub models: Vec<Model>,
    pub shown: Option<Shown>,
    pub scrolls: std::collections::BTreeMap<Region, f32>,
    pub splits: Splits,
    pub grabbed: Option<Splitter>,
    pub reading: mcf_tui::machine::Reading,
    pub refusal: Option<String>,
    pub busy: bool,
    pub typed: String,
    pub filter: String,
    pub hub: Option<HubList>,
    pub hub_chosen: Option<usize>,
    pub offered: std::collections::BTreeMap<String, Vec<OfferedFile>>,
    pub pending: Option<Pending>,
    pub after_download: Option<Act>,
    pub filters: Filters,
    pub temperature: String,
    pub system: String,
    pub effort: String,
    pub thinking: Option<bool>,
    pub picture: String,
    pub caret: Caret,
    pub most: Option<usize>,
    pub by: Option<mcf_serve::prompt::Unit>,
    pub extras: mcf_serve::prompt::Extras,
    pub chosen: Option<usize>,
    pub doing: Doing,
    pub said: String,
    pub tests: Vec<Test>,
    pub probes_apply: bool,
    pub diagnostic: Diagnostic,
    pub evaluating: Option<usize>,
    evaluation_kept: bool,
    pub readings: Option<(String, Vec<Value>)>,
    pub queued: std::collections::VecDeque<Card>,
    pub queued_of: usize,
    pub components: Vec<Component>,
    pub daemon_build: Option<String>,
    pub faults: Vec<Fault>,
    pub faults_in_record: usize,
    faults_read: Option<std::time::Instant>,
    pub settings: Option<mcf_serve::hosting::Hosting>,
    pub recommended: Option<mcf_serve::hosting::Hosting>,
    pub no_settings: Option<String>,
    pub last_settings: Option<(mcf_serve::hosting::Hosting, String)>,
    pub needs_engine: Option<String>,
    pub card_unused: Option<(String, String)>,
    pub placements: Vec<Placement>,
    pub on: Option<mcf_serve::control::On>,
    pub tab: Tab,
    pub contents: Page,
    pub editing: Option<(Field, String)>,
    pub edit_refused: Option<String>,
    pub retries: usize,
    pub challenge_window: Option<u64>,
    pub challenge_languages: Option<String>,
    pub declared: Option<mcf_serve::declared::Declared>,
    host_after: Option<String>,
    pub building: Option<String>,
    pub build_failed: Option<(String, String)>,
    pub hosted: Option<Hosted>,
    pub under_test: Option<UnderTest>,
    pub spent: Spent,
    pub last_hold: Option<LastHold>,
    pub rates: std::collections::VecDeque<f32>,
    pub host_refused: Option<String>,
    pub freed: Option<String>,
    pub anatomy: Option<mcf_serve::anatomy::Said>,
    pub no_anatomy: Option<String>,
    pub window: u64,
    pub open: Option<Picker>,
    sampler: mcf_tui::machine::Sampler,
}

impl Desk {
    #[must_use]
    pub fn new(socket: std::path::PathBuf) -> Self {
        Self {
            socket,
            page: Page::Models,
            models: Vec::new(),
            card_unused: None,
            placements: Vec::new(),
            on: None,
            tab: Tab::default(),
            contents: Page::Anatomy,
            editing: None,
            edit_refused: None,
            retries: RETRIES_DEFAULT,
            challenge_window: None,
            challenge_languages: None,
            declared: None,
            shown: None,
            scrolls: std::collections::BTreeMap::new(),
            splits: Splits::default(),
            grabbed: None,
            reading: mcf_tui::machine::Reading::default(),
            refusal: None,
            busy: false,
            typed: String::new(),
            temperature: String::new(),
            caret: Caret::Document,
            system: String::new(),
            effort: String::new(),
            thinking: None,
            picture: String::new(),
            most: None,
            by: None,
            extras: mcf_serve::prompt::Extras::NONE,
            chosen: None,
            doing: Doing::Nothing,
            said: String::new(),
            tests: tests(),
            diagnostic: Diagnostic::Throughput,
            evaluating: None,
            evaluation_kept: true,
            readings: None,
            queued: std::collections::VecDeque::new(),
            queued_of: 0,
            probes_apply: false,
            components: Vec::new(),
            daemon_build: None,
            faults: Vec::new(),
            faults_in_record: 0,
            faults_read: None,
            settings: None,
            recommended: None,
            no_settings: None,
            last_settings: None,
            filter: String::new(),
            hub: None,
            hub_chosen: None,
            offered: std::collections::BTreeMap::new(),
            pending: None,
            after_download: None,
            filters: Filters::default(),
            needs_engine: None,
            host_after: None,
            building: None,
            build_failed: None,
            hosted: None,
            under_test: None,
            spent: Spent::default(),
            last_hold: None,
            rates: std::collections::VecDeque::new(),
            host_refused: None,
            freed: None,
            anatomy: None,
            no_anatomy: None,
            window: 8192,
            open: None,
            sampler: mcf_tui::machine::Sampler::new(),
        }
    }

    #[must_use]
    pub fn takes_typing(&self) -> bool {
        matches!(
            self.page,
            Page::Adding | Page::Hosting | Page::Prompt | Page::Models
        ) || (matches!(self.page, Page::Host | Page::Diagnostics) && self.editing.is_some())
    }

    const PASTE_LIMIT: usize = 512;

    pub const PROMPT_LIMIT: usize = 65_536;

    pub fn paste(&mut self, text: &str) {
        if self.page == Page::Prompt && self.caret == Caret::Document {
            let kept: String = text
                .replace("\r\n", "\n")
                .chars()
                .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                .collect();
            let room = Self::PROMPT_LIMIT.saturating_sub(self.typed.chars().count());
            self.typed.extend(kept.chars().take(room));
            return;
        }
        let first = text.lines().next().unwrap_or_default();
        let kept: String = first
            .chars()
            .filter(|c| !c.is_control())
            .take(Self::PASTE_LIMIT)
            .collect();
        let kept = kept.trim();
        if kept.is_empty() {
            return;
        }
        let into = self.typing();
        let room = Self::PASTE_LIMIT.saturating_sub(into.chars().count());
        if room == 0 {
            return;
        }
        into.extend(kept.chars().take(room));
    }

    pub fn typing(&mut self) -> &mut String {
        if let (Page::Models | Page::Host | Page::Diagnostics, Some((_, typed))) =
            (self.page, self.editing.as_mut())
        {
            return typed;
        }
        match (self.page, self.caret) {
            (Page::Prompt, Caret::Temperature) => &mut self.temperature,
            (Page::Hosting, Caret::System) => &mut self.system,
            (Page::Hosting, Caret::Effort) => &mut self.effort,
            (Page::Hosting, Caret::Picture) => &mut self.picture,
            (Page::Models, _) => &mut self.filter,
            _ => &mut self.typed,
        }
    }

    #[must_use]
    pub fn being_typed(&self) -> &str {
        if let (Page::Models | Page::Host | Page::Diagnostics, Some((_, typed))) =
            (self.page, self.editing.as_ref())
        {
            return typed;
        }
        match (self.page, self.caret) {
            (Page::Prompt, Caret::Temperature) => &self.temperature,
            (Page::Hosting, Caret::System) => &self.system,
            (Page::Hosting, Caret::Effort) => &self.effort,
            (Page::Hosting, Caret::Picture) => &self.picture,
            (Page::Models, _) => &self.filter,
            _ => &self.typed,
        }
    }

    pub fn settle(&self) -> Result<Option<mcf_core::configuration::Thousandths>, &str> {
        let typed = self.temperature.trim();
        if typed.is_empty() {
            return Ok(None);
        }
        match typed.parse::<mcf_core::configuration::Thousandths>() {
            Ok(held) if held.0 > 0 => Ok(Some(held)),
            _ => Err(typed),
        }
    }

    #[must_use]
    pub fn taken(&self) -> mcf_serve::prompt::Taken<'_> {
        mcf_serve::prompt::Taken {
            text: self.typed.trim(),
            by: self.by,
            most: self.most,
            extras: self.extras,
        }
    }

    pub fn returned(&mut self, with_control: bool) {
        if self.page == Page::Prompt && !with_control && self.caret == Caret::Document {
            if self.typed.chars().count() < Self::PROMPT_LIMIT {
                self.typed.push('\n');
            }
            return;
        }
        self.entered();
    }

    pub fn entered(&mut self) {
        match self.page {
            Page::Models if self.editing.is_some() => self.apply_edit(),
            Page::Models => {
                if !self.filter.trim().is_empty() && self.library().is_empty() {
                    self.search_hub();
                }
            }
            Page::Host | Page::Diagnostics => self.apply_edit(),
            Page::Adding => self.look_up(),
            Page::Prompt => self.report_prompt(),
            Page::Hosting => {
                if let Some(at) = self.chosen {
                    self.ask(at);
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines, reason = "one arm a kind of job, each named")]
    pub fn hear(&mut self) -> bool {
        let before = self.doing.job().map_or(0, |job| job.answers.len());
        let heard = match &mut self.doing {
            Doing::Nothing => false,
            Doing::Listing(job)
            | Doing::Downloading(job)
            | Doing::Measuring(job)
            | Doing::CrossChecking(job)
            | Doing::Answering(job)
            | Doing::Reporting(job)
            | Doing::Probing(job)
            | Doing::Examining(job)
            | Doing::Evaluating(job)
            | Doing::Provisioning(job)
            | Doing::Hosting(job) => job.drain(),
        };
        if !heard {
            self.start_the_next_queued();
            return false;
        }
        if let Doing::Evaluating(job) = &self.doing
            && job
                .answers
                .get(before..)
                .unwrap_or(&[])
                .iter()
                .filter_map(|answer| answer.get("line").and_then(Value::as_text))
                .any(|line| line.starts_with("result: "))
        {
            self.fetch_readings();
        }
        if let Doing::Answering(job) = &self.doing {
            self.said = job
                .answers
                .iter()
                .filter_map(|answer| answer.get("token").and_then(Value::as_text))
                .collect::<Vec<_>>()
                .concat();
        }
        if let Doing::Downloading(job) = &self.doing
            && job.finished
            && job.refused.is_none()
        {
            self.refresh();
            self.settle_download();
        }
        if let Doing::Hosting(job) = &self.doing
            && job.finished
        {
            if let Some(why) = &job.refused {
                self.host_refused = Some(why.clone());
            }
            self.read_hosted();
        }
        if let Doing::Measuring(job) = &self.doing
            && job.finished
        {
            self.keep_the_run();
        }
        if let Doing::CrossChecking(job) = &self.doing
            && job.finished
        {
            self.keep_the_cross_check();
        }
        if let Doing::Reporting(job) = &self.doing
            && job.finished
        {
            self.refresh_readings_at();
        }
        if let Doing::Probing(job) = &self.doing
            && job.finished
        {
            self.keep_the_probes();
        }
        if let Doing::Examining(job) = &self.doing
            && job.finished
        {
            self.keep_the_examination();
        }
        if let Doing::Evaluating(job) = &self.doing
            && job.finished
            && !self.evaluation_kept
        {
            self.keep_the_evaluation();
        }
        self.start_the_next_queued();
        if let Doing::Listing(job) = &self.doing
            && job.finished
        {
            self.keep_the_hub();
            self.keep_the_files();
        }
        if let Doing::Provisioning(job) = &self.doing
            && job.finished
        {
            let refused = job.refused.clone();
            let wanted = self.host_after.take();
            let built = self.building.take();
            self.read_components();
            self.read_settings();
            let still_chosen = self
                .chosen
                .and_then(|at| self.models.get(at))
                .is_some_and(|held| Some(&held.path) == wanted.as_ref());
            if refused.is_none() && still_chosen {
                self.host_it();
            } else if let Some(why) = refused {
                if wanted.is_some() {
                    self.no_settings = Some(format!("the engine could not be built: {why}"));
                }
                self.build_failed = built.map(|name| (name, why));
            }
        }
        true
    }

    fn keep_the_run(&mut self) {
        let Doing::Measuring(job) = &self.doing else {
            return;
        };
        mcf_tui::screens::diagnostics::keep_the_ladder(&mut self.tests, job);
        self.refresh_readings_at();
    }

    fn keep_the_cross_check(&mut self) {
        let Doing::CrossChecking(job) = &self.doing else {
            return;
        };
        mcf_tui::screens::diagnostics::keep_the_cross_check(&mut self.tests, job);
        self.refresh_readings_at();
    }

    fn show(&mut self, shown: Shown) {
        self.shown = if self.shown == Some(shown) {
            None
        } else {
            Some(shown)
        };
    }

    pub fn act(&mut self, act: Act) {
        match act {
            Act::Go(page) => {
                if page != self.page {
                    self.scrolls.clear();
                    if page.section() == Page::Monitor {
                        self.sample();
                    }
                    if matches!(page, Page::Anatomy | Page::Vocabulary) {
                        self.read_anatomy();
                    }
                    self.page = page;
                }
            }
            Act::LookUp => self.look_up(),
            Act::Download { reference, file } => self.download(&reference, &file),
            Act::Stop => self.stop_run(),
            Act::HostAgain => self.host_again(),
            Act::Pick(repository) => {
                self.typed = repository;
                self.look_up();
            }
            Act::Tab(_) | Act::Edit(_) | Act::Switch(_) | Act::Place(_) | Act::Rope(_) => {
                self.configure(&act);
            }
            Act::Contents(page) => self.contents = page,
            Act::SetOn(on) => {
                self.on = on;
                self.open = None;
            }
            Act::Measure { deepest } => {
                if let Some(at) = self.chosen {
                    self.page = Page::Diagnostics;
                    self.measure(at, deepest);
                }
            }
            Act::Run(card) => self.run_card(card),
            Act::SearchHub => self.search_hub(),
            Act::Scroll(region, to) => {
                let _was = self.scrolls.insert(region, as_points(to.max(0)));
            }
            Act::Split(splitter, to) => self.split(splitter, to),
            Act::ToggleFilters | Act::SetArchitecture(_) | Act::SetFits(_) | Act::SetSize(_) => {
                self.filter_act(&act);
            }
            Act::Quantization(at) => self.pick_quantization(at),
            Act::PickOffered(at) => self.pick_offered(at),
            Act::DownloadThen(then) => self.download_then(*then),
            Act::PickHub(at) => self.pick_hub(at),
            Act::ApplyProbes => self.probes_apply = !self.probes_apply,
            Act::Show(diagnostic) => self.show_diagnostic(diagnostic),
            Act::RunOne(diagnostic) => self.run_one(diagnostic),
            Act::RunAll => self.run_all(),
            Act::SeeStatistics => {
                self.page = Page::Models;
                self.tab = Tab::Statistics;
            }
            Act::Open(picker) => self.open_picker(picker),
            Act::Shut => self.open = None,
            Act::SetWindow(window) => {
                self.window = window;
                self.open = None;
            }
            Act::Cycle(at) => self.cycle(at),
            Act::Recommended => self.settings.clone_from(&self.recommended),
            Act::LastSettings => {
                if let Some((last, _)) = &self.last_settings {
                    self.settings = Some(last.clone());
                }
            }
            Act::HostIt => self.host_it(),
            Act::Build(name) => self.build(&name),
            Act::StopHosting => self.stop_hosting(),
            Act::Close | Act::Copy(_) => {}
            Act::ShowWithout(at) => self.show(Shown::Without(at)),
            Act::ShowAlone(at) => self.show(Shown::Alone(at)),
            Act::ShowPrefix(at) => self.show(Shown::Prefix(at)),
            Act::ShowSwap(at) => self.show(Shown::Swap(at)),
            Act::ShowForm(at) => self.show(Shown::Form(at)),
            Act::Ask { at } => self.ask(at),
            Act::Choose(at) => {
                self.chosen = Some(at);
                self.open = None;
                self.tab = Tab::Configure;
                self.pending = None;
                self.hub_chosen = None;
                let _was = self.scrolls.remove(&Region::Page);
                self.read_settings();
            }
            Act::ReportPrompt => self.report_prompt(),
            Act::Focus(caret) => self.caret = caret,
            Act::CycleThinking => self.cycle_thinking(),
            Act::MostParts(most) => self.most = Some(most.max(1)),
            Act::TakeApartBy(by) => self.by = by,
            Act::Extra(extra, asked) => self.extras = self.extras.with(extra, asked),
            Act::Clear => self.typed.clear(),
            Act::Dismiss => self.doing = Doing::Nothing,
        }
    }

    pub fn read_settings(&mut self) {
        self.settings = None;
        self.recommended = None;
        self.no_settings = None;
        self.last_settings = None;
        self.needs_engine = None;
        self.placements.clear();
        self.editing = None;
        self.edit_refused = None;
        self.declared = None;
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        let asked = Request::Settings {
            model: held.path.clone(),
        };
        match ask(&self.socket, &asked) {
            Ok(answer) if answer.served => {
                let recommended = answer.body.get("recommended").map(|held| {
                    mcf_serve::hosting::Hosting::from_value(
                        held,
                        &mcf_serve::hosting::Hosting::recommended(
                            "", "", false, 0, None, false, None,
                        ),
                    )
                });
                self.settings.clone_from(&recommended);
                self.last_settings = answer
                    .body
                    .get("last")
                    .filter(|held| !matches!(held, Value::Null))
                    .and_then(|last| {
                        let against = recommended.as_ref()?;
                        let held =
                            mcf_serve::hosting::Hosting::from_value(last.get("settings")?, against);
                        let since = last
                            .get("since")
                            .and_then(|at| mcf_record::decode::timestamp(at).ok())
                            .map(|at| at.to_string())
                            .map_or_else(
                                || "?".to_owned(),
                                |at| at.get(..19).map_or(at.clone(), |head| format!("{head}Z")),
                            );
                        Some((held, since))
                    });
                self.recommended = recommended;
                self.declared = answer
                    .body
                    .get("declares")
                    .map(mcf_serve::declared::Declared::from_value);
                self.placements = answer
                    .body
                    .get("placements")
                    .and_then(Value::as_list)
                    .map(|listed| listed.iter().filter_map(Placement::from_value).collect())
                    .unwrap_or_default();
            }
            Ok(answer) => {
                self.no_settings = Some(refused_because(&answer.body));
                self.needs_engine = answer
                    .body
                    .get("context")
                    .and_then(|context| context.get("needs_component"))
                    .and_then(Value::as_text)
                    .map(str::to_owned);
            }
            Err(why) => self.no_settings = Some(why),
        }
    }

    pub fn read_anatomy(&mut self) {
        self.anatomy = None;
        self.no_anatomy = None;
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.no_anatomy = Some("Choose a model on the Models screen first.".to_owned());
            return;
        };
        let asked = Request::Anatomy {
            model: held.path.clone(),
        };
        match ask(&self.socket, &asked) {
            Ok(answer) if answer.served => {
                self.anatomy = mcf_serve::anatomy::Said::from_value(&answer.body);
                if self.anatomy.is_none() {
                    self.no_anatomy = Some(
                        "MCF answered, but not in the shape this window reads — the daemon and \
                         the window are not the same build."
                            .to_owned(),
                    );
                }
            }
            Ok(answer) => self.no_anatomy = Some(refused_because(&answer.body)),
            Err(why) => self.no_anatomy = Some(why),
        }
    }

    pub fn read_hosted(&mut self) {
        let answered = ask_within(&self.socket, &Request::Hosted, POLL).ok();
        if let Some(answer) = answered.as_ref().filter(|answer| answer.served) {
            self.under_test = answer
                .body
                .get("under_test")
                .filter(|held| !matches!(held, Value::Null))
                .map(UnderTest::from_value);
        }
        let read =
            match ask_within(&self.socket, &Request::Hosted, POLL) {
                Ok(answer) if answer.served => answer
                    .body
                    .get("hosting")
                    .and_then(Value::as_text)
                    .map(|model| Hosted {
                        model: model.to_owned(),
                        address: answer
                            .body
                            .get("address")
                            .and_then(Value::as_text)
                            .unwrap_or_default()
                            .to_owned(),
                        since: answer
                            .body
                            .get("since")
                            .and_then(Value::as_text)
                            .unwrap_or_default()
                            .to_owned(),
                        context: answer
                            .body
                            .get("settings")
                            .and_then(|settings| settings.get("context"))
                            .and_then(Value::as_integer)
                            .and_then(|context| u64::try_from(context).ok()),
                        projector: answer
                            .body
                            .get("settings")
                            .and_then(|settings| settings.get("projector"))
                            .and_then(Value::as_text)
                            .map(|path| path.rsplit('/').next().unwrap_or(path).to_owned()),
                        takes: answer
                            .body
                            .get("takes")
                            .filter(|takes| !matches!(takes, Value::Null))
                            .map(mcf_serve::takes::Takes::from_value),
                        api_key: answer
                            .body
                            .get("settings")
                            .and_then(|settings| settings.get("api_key"))
                            .is_some_and(|key| !matches!(key, Value::Null)),
                        network_address: answer
                            .body
                            .get("network_address")
                            .and_then(Value::as_text)
                            .map(str::to_owned),
                        in_use: answer.body.get("use").map(Use::from_value),
                    }),
                Ok(answer) if answer.served => None,
                _ => {
                    self.busy = true;
                    return;
                }
            };
        self.busy = false;
        match (&self.hosted, &read) {
            (Some(was), Some(now)) if was.model == now.model => {}
            _ => self.rates.clear(),
        }
        if let Some(rate) = read
            .as_ref()
            .and_then(|hosting| hosting.in_use.as_ref())
            .and_then(|in_use| in_use.generated_per_second)
        {
            self.rates.push_back(rate);
            while self.rates.len() > 120 {
                self.rates.pop_front();
            }
        }
        self.hosted = read;
        self.last_hold = answered
            .as_ref()
            .and_then(|answer| answer.body.get("last"))
            .and_then(LastHold::from_value);
    }

    pub fn host_it(&mut self) {
        self.apply_edit();
        if let Some(why) = &self.edit_refused {
            self.host_refused = Some(why.clone());
            return;
        }
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.no_settings = Some("choose a model first".to_owned());
            return;
        };
        if self
            .settings
            .as_ref()
            .is_some_and(|settings| settings.open && settings.api_key.is_none())
        {
            self.host_refused = Some(
                "Reachable from the network is on, so the hold needs an API key: type one in \
                 the API key field above, then Start server; or turn the switch off to keep \
                 the hold on this computer"
                    .to_owned(),
            );
            return;
        }
        self.host_refused = None;
        self.freed = None;
        self.page = Page::Hosting;
        if self.settings.is_none()
            && let Some(engine) = self.needs_engine.clone()
        {
            self.host_after = Some(held.path.clone());
            self.building = Some(engine.clone());
            self.doing = Doing::Provisioning(job::Job::start(
                &self.socket,
                Request::Provision { component: None },
                format!("building {engine} so that {} can be held", held.name),
            ));
            return;
        }
        let Some(settings) = self.settings.clone() else {
            self.no_settings = Some(self.no_settings.clone().unwrap_or_else(|| {
                "MCF has not said what this model would run under, so there is nothing to \
                     hold it under"
                    .to_owned()
            }));
            return;
        };
        self.doing = Doing::Hosting(job::Job::start(
            &self.socket,
            Request::Host {
                model: held.path.clone(),
                settings: settings.to_request(),
            },
            format!("holding {}", held.name),
        ));
    }

    pub fn build(&mut self, name: &str) {
        if self.doing.job().is_some_and(|job| !job.finished) {
            return;
        }
        self.host_after = None;
        self.build_failed = None;
        self.building = Some(name.to_owned());
        self.doing = Doing::Provisioning(job::Job::start(
            &self.socket,
            Request::Provision {
                component: Some(name.to_owned()),
            },
            format!("building {name}"),
        ));
    }

    #[must_use]
    pub fn to_let_go(&self) -> Option<String> {
        self.hosted.as_ref().map(Hosted::name)
    }

    pub fn stop_hosting(&mut self) {
        let answered = ask(&self.socket, &Request::Unhost);
        let was = self.hosted.take();
        self.freed = answered.ok().map(|answer| {
            let figure = |key: &str| {
                answer
                    .body
                    .get(key)
                    .and_then(Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
                    .map(view::gigabytes)
            };
            let name = was.map_or_else(|| "it".to_owned(), |held| held.name());
            match (figure("freed_bytes"), figure("freed_card_bytes")) {
                (Some(memory), Some(card)) => {
                    format!("Server stopped: {name} — freed {memory} RAM, {card} VRAM")
                }
                (Some(memory), None) => format!("Server stopped: {name} — freed {memory} RAM"),
                (None, Some(card)) => {
                    format!("Server stopped: {name} — freed {card} VRAM")
                }
                (None, None) => format!("Server stopped: {name}"),
            }
        });
    }

    #[must_use]
    pub fn loading_line(&self) -> Option<String> {
        let Doing::Hosting(job) = &self.doing else {
            return None;
        };
        if job.finished {
            return None;
        }
        let latest = job
            .answers
            .iter()
            .rev()
            .find_map(|body| body.get("loading"));
        let figure = |key: &str| {
            latest
                .and_then(|loading| loading.get(key))
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        let read = match figure("card_bytes") {
            Some(on_card) => Some((on_card, true)),
            None => figure("resident_bytes").map(|resident| (resident, false)),
        };
        Some(match read {
            Some((read, on_card)) => loading_said(read, on_card, figure("of_bytes"), job.ran()),
            None => format!("Loading · {} s — no progress reported yet", job.ran()),
        })
    }

    #[must_use]
    pub fn hosted_model(&self) -> Option<&Model> {
        let hosting = self.hosted.as_ref()?;
        self.models.iter().find(|held| held.path == hosting.model)
    }

    #[must_use]
    pub fn second_copy(&self) -> Option<String> {
        let chosen = self.chosen.and_then(|at| self.models.get(at))?;
        let hosted = self.hosted_model()?;
        if hosted.path != chosen.path {
            return Some(format!(
                "{} is hosted, and one model runs at a time: diagnostics run on the hosted model. \
                 Let it go on the Server page, or host {} first",
                hosted.name, chosen.name
            ));
        }
        Some(
            "hosted through the window: a measurement lets the hold go for its run and hosts it \
             again after, so that one copy of the model is resident throughout"
                .to_owned(),
        )
    }

    pub fn cycle(&mut self, at: usize) {
        let (Some(settings), Some(recommended)) =
            (self.settings.as_mut(), self.recommended.as_ref())
        else {
            return;
        };
        match at {
            0 => {
                settings.context = if settings.context >= recommended.context {
                    512
                } else {
                    settings.context.saturating_mul(2)
                };
            }
            1 => {
                if self.placements.is_empty() {
                    settings.gpu_layers = if settings.gpu_layers == 0 { 999 } else { 0 };
                } else {
                    let at = self
                        .placements
                        .iter()
                        .position(|held| {
                            held.engine == settings.engine
                                && held.device == settings.device
                                && held.gpu_layers == settings.gpu_layers
                        })
                        .map_or(0, |at| (at + 1) % self.placements.len());
                    if let Some(next) = self.placements.get(at) {
                        settings.engine.clone_from(&next.engine);
                        settings.device.clone_from(&next.device);
                        settings.gpu_layers = next.gpu_layers;
                    }
                }
            }
            4 => {
                settings.threads = match settings.threads {
                    held if held >= 64 => 1,
                    held => held.saturating_mul(2),
                };
            }
            5 => {
                settings.batch = match settings.batch {
                    held if held >= 4096 => 256,
                    held => held.saturating_mul(2),
                };
            }
            6 => settings.flash_attention = !settings.flash_attention,
            7 => settings.keep_resident = !settings.keep_resident,
            8 => settings.port = settings.port.saturating_add(1),
            _ => {}
        }
    }

    pub fn look_up(&mut self) {
        let asked = self.typed.trim().to_owned();
        if asked.is_empty() {
            return;
        }
        let is_reference = mcf_serve::control::is_reference(&asked);
        self.doing = Doing::Listing(job::Job::start(
            &self.socket,
            if is_reference {
                Request::Offered {
                    reference: asked.clone(),
                    from: None,
                    fresh: false,
                }
            } else {
                Request::Search {
                    query: asked.clone(),
                    from: None,
                    fresh: false,
                }
            },
            if is_reference {
                format!("looking up {asked}")
            } else {
                format!("searching the hub for {asked:?}")
            },
        ));
    }

    pub fn report_prompt(&mut self) {
        self.tally_afresh();
        let taken = self.taken();
        if taken.text.is_empty() {
            return;
        }
        let Ok(temperature) = self.settle() else {
            return;
        };
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        self.doing = Doing::Reporting(job::Job::start(
            &self.socket,
            Request::PromptReport {
                turn: None,
                model: held.path.clone(),
                prompt: taken.text.to_owned(),
                by: taken.by,
                most: taken.most,
                extras: taken.extras,
                temperature,
                seed: 41,
            },
            format!("taking the prompt apart on {}", held.name),
        ));
    }

    pub fn download(&mut self, reference: &str, file: &str) {
        self.doing = Doing::Downloading(job::Job::start(
            &self.socket,
            Request::Acquire {
                reference: reference.to_owned(),
                file: file.to_owned(),
                from: None,
            },
            format!("getting {file}"),
        ));
    }

    pub fn measure(&mut self, at: usize, deepest: u64) {
        self.tally_afresh();
        let Some(held) = self.models.get(at) else {
            return;
        };
        self.chosen = Some(at);
        self.doing = Doing::Measuring(job::Job::start(
            &self.socket,
            Request::Measure {
                started: mcf_serve::declared::Started::default(),
                model: held.path.clone(),
                engine: None,
                on: self.on,
                deepest,
            },
            format!("measuring {}", held.name),
        ));
    }

    pub fn run_card(&mut self, card: Card) {
        let Some(at) = self.chosen else {
            return;
        };
        self.run_card_on(at, card);
    }

    pub fn run_one(&mut self, diagnostic: Diagnostic) {
        let Some(at) = self.chosen else {
            return;
        };
        self.diagnostic = diagnostic;
        match diagnostic {
            Diagnostic::Probe(_) => {
                self.page = Page::Diagnostics;
                self.probe_only(at, vec![diagnostic.name().to_owned()]);
            }
            Diagnostic::Measure(_) => {
                self.page = Page::Diagnostics;
                self.examine_only(at, vec![diagnostic.name().to_owned()]);
            }
            Diagnostic::Eval(which) => self.evaluate(at, Some(which)),
            other => self.run_card_on(at, other.card()),
        }
    }

    pub fn run_all(&mut self) {
        if self.chosen.is_none() || self.doing.busy() {
            return;
        }
        self.queued = Self::EVERY_RUN.iter().copied().collect();
        self.queued_of = self.queued.len();
        self.start_the_next_queued();
    }

    pub const EVERY_RUN: [Card; 7] = [
        Card::Throughput,
        Card::CrossCheck,
        Card::Capabilities,
        Card::Performance,
        Card::Fidelity,
        Card::Behaviour,
        Card::Coding,
    ];

    fn start_the_next_queued(&mut self) {
        if self.queued.is_empty() {
            return;
        }
        if let Some(job) = self.doing.job() {
            if !job.finished {
                return;
            }
            if job.refused.is_some() {
                self.queued.clear();
                self.queued_of = 0;
                return;
            }
        }
        let (Some(at), Some(card)) = (self.chosen, self.queued.pop_front()) else {
            self.queued.clear();
            self.queued_of = 0;
            return;
        };
        self.run_card_on(at, card);
        if self.queued.is_empty() && !self.doing.busy() {
            self.queued_of = 0;
        }
    }

    #[must_use]
    pub fn run_fraction(&self) -> Option<f32> {
        let job = self.doing.job().filter(|job| !job.finished)?;
        let latest = job.latest();
        let of_step = || {
            let step = latest?.get("step")?;
            let count = step.get("count").and_then(Value::as_integer)?;
            let of = step.get("of").and_then(Value::as_integer)?;
            let lines = latest?
                .get("lines")
                .and_then(Value::as_list)
                .is_some_and(|lines| !lines.is_empty());
            let done = if lines { count } else { count - 1 };
            let whole = fraction_of(done.max(0), of)?;
            let within = latest?.get("progress").and_then(|progress| {
                let done = progress.get("done").and_then(Value::as_integer)?;
                let parts = progress.get("of").and_then(Value::as_integer)?;
                fraction_of(done, parts)
            });
            #[allow(clippy::cast_precision_loss, reason = "a step count")]
            let steps = of.max(1) as f32;
            Some(within.map_or(whole, |within| (whole + within / steps).min(1.0)))
        };
        match &self.doing {
            Doing::Probing(_) | Doing::Examining(_) => of_step(),
            Doing::Evaluating(_) => {
                let line = latest?.get("line").and_then(Value::as_text)?;
                let (done, of) = line.strip_prefix("progress: ")?.split_once('/')?;
                let of_word = of.split_whitespace().next()?;
                fraction_of(done.trim().parse().ok()?, of_word.parse().ok()?)
            }
            Doing::Measuring(_) => {
                let so_far = latest?.get("so_far").and_then(Value::as_integer);
                let of = latest?.get("of").and_then(Value::as_integer);
                match (so_far, of) {
                    (Some(so_far), Some(of)) => fraction_of(so_far, of),
                    _ => Some(0.0),
                }
            }
            Doing::CrossChecking(job) => Some(if job.answers.len() >= 2 { 0.5 } else { 0.05 }),
            _ => None,
        }
    }

    #[must_use]
    pub fn time_left(&self) -> Option<u64> {
        let job = self.doing.job().filter(|job| !job.finished)?;
        let fraction = self.run_fraction()?;
        let elapsed = job.ran();
        if fraction < 0.05 || elapsed < 15 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole seconds of an estimate"
        )]
        let left = (elapsed as f32 * (1.0 - fraction) / fraction).round() as u64;
        Some(left)
    }

    #[must_use]
    pub fn sequence_fraction(&self) -> Option<f32> {
        if self.queued_of == 0 {
            return None;
        }
        let going = usize::from(self.doing.busy());
        let done = self
            .queued_of
            .saturating_sub(self.queued.len())
            .saturating_sub(going);
        let own = self.run_fraction().unwrap_or(0.0);
        #[expect(clippy::cast_precision_loss, reason = "a count of six runs")]
        let whole = (done as f32 + own) / self.queued_of as f32;
        Some(whole.clamp(0.0, 1.0))
    }

    #[must_use]
    pub fn sequence_place(&self) -> Option<(usize, usize)> {
        if self.queued_of == 0 {
            return None;
        }
        let going = usize::from(self.doing.busy());
        let done = self
            .queued_of
            .saturating_sub(self.queued.len())
            .saturating_sub(going);
        Some((done.saturating_add(going).max(1), self.queued_of))
    }

    fn run_card_on(&mut self, at: usize, card: Card) {
        match card {
            Card::Throughput => {
                self.page = Page::Diagnostics;
                self.measure(at, self.window);
            }
            Card::CrossCheck => {
                self.page = Page::Diagnostics;
                self.cross_check(at);
            }
            Card::Prompt => self.page = Page::Prompt,
            Card::Capabilities => {
                self.page = Page::Diagnostics;
                self.probe(at);
            }
            Card::Performance | Card::Fidelity | Card::Behaviour => {
                self.page = Page::Diagnostics;
                self.examine(at, card);
            }
            Card::Coding => self.evaluate(at, None),
            Card::Comparison => {}
        }
    }

    #[must_use]
    pub fn scrolled(&self, region: Region) -> f32 {
        self.scrolls.get(&region).copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn library(&self) -> Vec<Group> {
        let wanted = self.filter.trim().to_lowercase();
        let words: Vec<&str> = wanted.split_whitespace().collect();
        let filters = &self.filters;
        let passes = |held: &Model| {
            if let Some(architecture) = &filters.architecture
                && held.architecture.as_deref().map(str::to_lowercase)
                    != Some(architecture.to_lowercase())
            {
                return false;
            }
            if let Some(fits) = filters.fits
                && held.refused.is_none() != fits
            {
                return false;
            }
            if let Some(size) = filters.size
                && held.bytes.is_none_or(|bytes| bytes > size)
            {
                return false;
            }
            if words.is_empty() {
                return true;
            }
            let haystack = format!(
                "{} {} {}",
                held.name,
                held.architecture.as_deref().unwrap_or(""),
                held.path
            )
            .to_lowercase();
            words.iter().all(|word| haystack.contains(word))
        };
        let mut groups: Vec<Group> = Vec::new();
        for (at, held) in self.models.iter().enumerate() {
            if !passes(held) {
                continue;
            }
            let joined = held.repository.as_ref().and_then(|repository| {
                groups
                    .iter_mut()
                    .find(|group| group.repository.as_ref() == Some(repository))
            });
            match joined {
                Some(group) => group.members.push(at),
                None => groups.push(Group {
                    repository: held.repository.clone(),
                    members: vec![at],
                }),
            }
        }
        groups
    }

    fn filter_act(&mut self, act: &Act) {
        match *act {
            Act::ToggleFilters => self.filters.open = !self.filters.open,
            Act::SetArchitecture(at) => {
                self.filters.architecture = at
                    .checked_sub(1)
                    .and_then(|at| self.architectures().get(at).cloned());
            }
            Act::SetFits(at) => self.filters.fits = FITS_CHOICES.get(at).copied().flatten(),
            Act::SetSize(at) => self.filters.size = SIZE_CHOICES.get(at).copied().flatten(),
            _ => {}
        }
        self.open = None;
    }

    #[must_use]
    pub fn architectures(&self) -> Vec<String> {
        let mut found: Vec<String> = self
            .models
            .iter()
            .filter_map(|held| held.architecture.clone())
            .collect();
        found.sort();
        found.dedup();
        found
    }

    #[must_use]
    pub fn quantizations(&self) -> Vec<Quant> {
        let repository = self.subject_repository();
        let mut listed: Vec<Quant> = Vec::new();
        for (at, held) in self.models.iter().enumerate() {
            let same = match (&repository, &held.repository) {
                (Some(wanted), Some(held)) => wanted == held,
                (None, _) => self.chosen == Some(at),
                _ => false,
            };
            if same {
                listed.push(Quant {
                    file: held.file.clone(),
                    bytes: held.bytes,
                    here: Some(at),
                    fits: None,
                });
            }
        }
        if let Some(offered) = repository.as_ref().and_then(|held| self.offered.get(held)) {
            for file in offered {
                if listed.iter().any(|quant| quant.file == file.file) {
                    continue;
                }
                listed.push(Quant {
                    file: file.file.clone(),
                    bytes: file.bytes,
                    here: None,
                    fits: file.fits,
                });
            }
        }
        listed
    }

    #[must_use]
    pub fn subject_repository(&self) -> Option<String> {
        if let Some(pending) = &self.pending {
            return Some(pending.repository.clone());
        }
        self.chosen
            .and_then(|at| self.models.get(at))
            .and_then(|held| held.repository.clone())
    }

    #[must_use]
    pub fn quantization_at(&self) -> Option<usize> {
        let listed = self.quantizations();
        match &self.pending {
            Some(pending) => listed.iter().position(|quant| quant.file == pending.file),
            None => listed.iter().position(|quant| quant.here == self.chosen),
        }
    }

    fn open_picker(&mut self, picker: Picker) {
        self.open = if self.open == Some(picker) {
            None
        } else {
            Some(picker)
        };
        if self.open == Some(Picker::Quantization) {
            self.look_up_files();
        }
    }

    pub fn pick_quantization(&mut self, at: usize) {
        self.open = None;
        let Some(quant) = self.quantizations().get(at).cloned() else {
            return;
        };
        match quant.here {
            Some(held) => {
                self.pending = None;
                self.act(Act::Choose(held));
            }
            None => {
                if let Some(repository) = self.subject_repository() {
                    self.pending = Some(Pending {
                        repository,
                        file: quant.file,
                        bytes: quant.bytes,
                        fits: quant.fits,
                    });
                }
            }
        }
    }

    pub fn pick_offered(&mut self, at: usize) {
        let Some(repository) = self
            .hub
            .as_ref()
            .zip(self.hub_chosen)
            .and_then(|(hub, chosen)| hub.repositories.get(chosen))
            .map(|found| found.id.clone())
        else {
            return;
        };
        let Some(file) = self
            .offered
            .get(&repository)
            .and_then(|files| files.get(at))
            .cloned()
        else {
            return;
        };
        self.pending = Some(Pending {
            repository,
            file: file.file,
            bytes: file.bytes,
            fits: file.fits,
        });
        self.hub_chosen = None;
        self.chosen = None;
    }

    pub fn download_then(&mut self, then: Act) {
        let Some(pending) = self.pending.clone() else {
            return;
        };
        if self.doing.busy() {
            return;
        }
        self.after_download = Some(then);
        self.download(&pending.repository, &pending.file);
    }

    pub fn settle_download(&mut self) {
        let Some(pending) = self.pending.clone() else {
            self.after_download = None;
            return;
        };
        let Some(at) = self.models.iter().position(|held| {
            held.file == pending.file && held.repository.as_deref() == Some(&pending.repository)
        }) else {
            return;
        };
        self.pending = None;
        self.hub_chosen = None;
        self.chosen = Some(at);
        self.read_settings();
        if let Some(then) = self.after_download.take() {
            self.act(then);
        }
    }

    pub fn look_up_files(&mut self) {
        let Some(repository) = self.subject_repository() else {
            return;
        };
        if self.offered.contains_key(&repository) || self.doing.busy() {
            return;
        }
        self.doing = Doing::Listing(job::Job::start(
            &self.socket,
            Request::Offered {
                reference: repository.clone(),
                from: None,
                fresh: false,
            },
            format!("looking up {repository}"),
        ));
    }

    fn keep_the_files(&mut self) {
        let Doing::Listing(job) = &self.doing else {
            return;
        };
        let Some(found) = job.conclusion().or_else(|| job.latest()) else {
            return;
        };
        let (Some(repository), Some(files)) = (
            found.get("repository").and_then(Value::as_text),
            found.get("files").and_then(Value::as_list),
        ) else {
            return;
        };
        let _replaced = self.offered.insert(
            repository.to_owned(),
            files.iter().filter_map(OfferedFile::from_value).collect(),
        );
    }

    #[must_use]
    pub fn hub_matches(&self) -> bool {
        self.hub
            .as_ref()
            .is_some_and(|hub| hub.query == self.filter.trim())
    }

    pub fn search_hub(&mut self) {
        let query = self.filter.trim().to_owned();
        if query.is_empty() || self.doing.busy() {
            return;
        }
        self.hub = None;
        self.hub_chosen = None;
        self.doing = Doing::Listing(job::Job::start(
            &self.socket,
            Request::Search {
                query: query.clone(),
                from: None,
                fresh: false,
            },
            format!("searching the hub for {query:?}"),
        ));
    }

    pub fn pick_hub(&mut self, at: usize) {
        let Some(id) = self
            .hub
            .as_ref()
            .and_then(|hub| hub.repositories.get(at))
            .map(|found| found.id.clone())
        else {
            return;
        };
        self.hub_chosen = Some(at);
        self.chosen = None;
        self.doing = Doing::Listing(job::Job::start(
            &self.socket,
            Request::Offered {
                reference: id.clone(),
                from: None,
                fresh: false,
            },
            format!("looking up {id}"),
        ));
    }

    pub fn hear_for_review(&mut self) {
        self.keep_the_hub();
        self.keep_the_files();
    }

    fn keep_the_hub(&mut self) {
        let Doing::Listing(job) = &self.doing else {
            return;
        };
        let Some(found) = job.conclusion().or_else(|| job.latest()) else {
            return;
        };
        let Some(listed) = found.get("repositories").and_then(Value::as_list) else {
            return;
        };
        let query = found
            .get("query")
            .and_then(Value::as_text)
            .unwrap_or("")
            .to_owned();
        self.hub = Some(HubList {
            query,
            repositories: listed.iter().filter_map(HubRepo::from_value).collect(),
        });
    }

    pub fn probe(&mut self, at: usize) {
        self.probe_only(at, Vec::new());
    }

    pub fn probe_only(&mut self, at: usize, only: Vec<String>) {
        self.tally_afresh();
        let Some(held) = self.models.get(at) else {
            return;
        };
        self.chosen = Some(at);
        self.doing = Doing::Probing(job::Job::start(
            &self.socket,
            Request::Probe {
                model: held.path.clone(),
                engine: None,
                apply: self.probes_apply,
                up_to: None,
                only,
            },
            format!("probing {}", held.name),
        ));
    }

    pub fn examine(&mut self, at: usize, card: Card) {
        let only = card
            .measures()
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<String>>();
        if only.is_empty() {
            return;
        }
        self.examine_only(at, only);
    }

    pub fn examine_only(&mut self, at: usize, only: Vec<String>) {
        self.tally_afresh();
        let Some(held) = self.models.get(at) else {
            return;
        };
        self.chosen = Some(at);
        self.doing = Doing::Examining(job::Job::start(
            &self.socket,
            Request::Examine {
                model: held.path.clone(),
                engine: None,
                only,
            },
            format!("examining {}", held.name),
        ));
    }

    #[must_use]
    pub fn running_diagnostic(&self) -> Option<Diagnostic> {
        let step_name = |job: &job::Job| {
            job.latest()
                .and_then(|answer| answer.get("step"))
                .and_then(|step| step.get("name"))
                .and_then(Value::as_text)
                .map(str::to_owned)
        };
        match &self.doing {
            Doing::Measuring(job) if !job.finished => Some(Diagnostic::Throughput),
            Doing::CrossChecking(job) if !job.finished => Some(Diagnostic::CrossCheck),
            Doing::Reporting(job) if !job.finished => Some(Diagnostic::Prompt),
            Doing::Probing(job) if !job.finished => {
                let name = step_name(job)?;
                mcf_serve::probes::run::PROBES
                    .iter()
                    .position(|held| *held == name)
                    .map(Diagnostic::Probe)
            }
            Doing::Examining(job) if !job.finished => {
                let name = step_name(job)?;
                mcf_serve::examine::MEASURES
                    .iter()
                    .position(|held| *held == name)
                    .map(Diagnostic::Measure)
            }
            Doing::Evaluating(job) if !job.finished => {
                Some(Diagnostic::Eval(self.evaluating.unwrap_or(0)))
            }
            _ => None,
        }
    }

    fn evaluate(&mut self, at: usize, suite: Option<usize>) {
        self.tally_afresh();
        let Some(held) = self.models.get(at) else {
            return;
        };
        if self.doing.busy() {
            return;
        }
        let Ok(own) = std::env::current_exe() else {
            self.refusal = Some("MCF cannot find its own binary to run the suite with".to_owned());
            return;
        };
        let mut command = std::process::Command::new(own);
        command.args(self.eval_arguments(&held.path, suite));
        let what = match suite.and_then(|at| SUITES.get(at)) {
            Some((_, name, _)) => format!("running the {name} suite on {}", held.name),
            None => format!("running every coding suite on {}", held.name),
        };
        self.page = Page::Diagnostics;
        self.evaluating = suite;
        self.evaluation_kept = false;
        self.doing = Doing::Evaluating(job::Job::spawned(command, what));
    }

    #[must_use]
    pub fn eval_arguments(&self, path: &str, suite: Option<usize>) -> Vec<String> {
        let mut arguments = vec!["eval".to_owned(), path.to_owned()];
        if let Some(name) = suite
            .and_then(|at| SUITES.get(at))
            .map(|(name, _, _)| *name)
        {
            arguments.push("--only".to_owned());
            match name.strip_prefix("challenges-") {
                Some(tier) => {
                    arguments.push("challenges".to_owned());
                    arguments.push("--tier".to_owned());
                    arguments.push(tier.to_owned());
                }
                None => arguments.push(name.to_owned()),
            }
        }
        if let Some(languages) = &self.challenge_languages {
            arguments.push("--languages".to_owned());
            arguments.push(languages.clone());
        }
        if suite.is_some_and(|at| self.resumable(Diagnostic::Eval(at))) {
            arguments.push("--resume".to_owned());
        }
        if self.retries != RETRIES_DEFAULT {
            arguments.push("--retries".to_owned());
            arguments.push(self.retries.to_string());
        }
        if let Some(window) = self.challenge_window {
            arguments.push("--window".to_owned());
            arguments.push(window.to_string());
        }
        arguments
    }

    #[must_use]
    pub fn resumable(&self, diagnostic: Diagnostic) -> bool {
        self.readings_of(diagnostic).is_some_and(|run| {
            mcf_record::readings::in_parts(run)
                && mcf_record::readings::ended_of(run).as_deref() != Some("finished")
        })
    }

    pub fn tally(&mut self) {
        if !self.doing.busy() {
            return;
        }
        let under = self.under_test.as_ref().map(|under| &under.in_use);
        if let Some(joules) = under.and_then(|in_use| in_use.card_energy_joules) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "joules the daemon wrote to three places, as millijoules"
            )]
            let millijoules = (joules.max(0.0) * 1_000.0) as u64;
            self.spent.millijoules = millijoules;
        }
        if let Some(seconds) = under.and_then(|in_use| in_use.card_energy_over_seconds) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "whole seconds, for the run's clock"
            )]
            let whole = seconds.max(0.0) as u64;
            self.spent.seconds = whole;
        } else {
            self.spent.seconds = self.spent.seconds.saturating_add(1);
        }
        let tokens = under.and_then(|in_use| in_use.generated_live.or(in_use.generated));
        if self.spent.tokens_at_start.is_none() {
            self.spent.tokens_at_start = tokens;
        }
        self.spent.tokens_now = tokens;
    }

    pub fn tally_afresh(&mut self) {
        self.spent = Spent::default();
    }

    #[must_use]
    pub fn results_so_far(&self) -> Vec<String> {
        let Doing::Evaluating(job) = &self.doing else {
            return Vec::new();
        };
        job.answers
            .iter()
            .filter_map(|answer| answer.get("line").and_then(Value::as_text))
            .filter_map(|line| line.strip_prefix("result: "))
            .map(str::to_owned)
            .collect()
    }

    fn keep_the_evaluation(&mut self) {
        self.evaluation_kept = true;
        self.fetch_readings();
        self.refresh_readings_at();
    }

    fn refresh_readings_at(&mut self) {
        let Some(at) = self.chosen else {
            return;
        };
        let Ok(answer) = ask_within(&self.socket, &Request::Holding, POLL) else {
            return;
        };
        if !answer.served {
            return;
        }
        let Some(path) = self.models.get(at).map(|held| held.path.clone()) else {
            return;
        };
        let fresh = answer
            .body
            .get("models")
            .and_then(Value::as_list)
            .and_then(|models| {
                models
                    .iter()
                    .find(|entry| entry.get("path").and_then(Value::as_text) == Some(path.as_str()))
            })
            .map(model_from);
        if let (Some(fresh), Some(held)) = (fresh, self.models.get_mut(at)) {
            held.readings_at = fresh.readings_at;
            held.measured_at = fresh.measured_at;
            held.measured_body = fresh.measured_body;
            held.cross_checked = fresh.cross_checked;
            held.cross_checked_at = fresh.cross_checked_at;
            held.prompt_reported = fresh.prompt_reported;
            held.prompt_reported_at = fresh.prompt_reported_at;
        }
    }

    #[must_use]
    pub fn finding_of(&self, diagnostic: Diagnostic) -> Option<&Finding> {
        let method = diagnostic.method()?;
        let recorded = diagnostic.readings_method();
        let held = self.chosen.and_then(|at| self.models.get(at))?;
        held.probed.iter().find(|finding| {
            finding.name == method || recorded.is_some_and(|name| finding.name == name)
        })
    }

    #[must_use]
    pub fn last_run(&self, diagnostic: Diagnostic) -> Option<(String, Option<String>)> {
        let held = self.chosen.and_then(|at| self.models.get(at))?;
        match diagnostic {
            Diagnostic::Throughput => {
                let engine = held
                    .measured_body
                    .as_ref()
                    .and_then(|body| body.get("conditions"))
                    .and_then(|conditions| conditions.get("engine_ran"))
                    .and_then(Value::as_text)
                    .map(str::to_owned);
                held.measured_at.clone().map(|at| (at, engine))
            }
            Diagnostic::CrossCheck => held.cross_checked_at.clone().map(|at| (at, None)),
            Diagnostic::Prompt => held.prompt_reported_at.clone().map(|at| (at, None)),
            Diagnostic::Comparison => None,
            Diagnostic::Probe(_) | Diagnostic::Measure(_) => {
                let finding = self.finding_of(diagnostic)?;
                finding.at.clone().map(|at| (at, finding.engine.clone()))
            }
            Diagnostic::Eval(_) => {
                let method = diagnostic.readings_method()?;
                held.readings_at.get(method).map(|at| (at.clone(), None))
            }
        }
    }

    fn keep_the_examination(&mut self) {
        let Doing::Examining(job) = &self.doing else {
            return;
        };
        let found = findings_of(job);
        if let Some(held) = self.chosen.and_then(|at| self.models.get_mut(at)) {
            keep_findings(&mut held.probed, found);
        }
        self.fetch_readings();
    }

    fn show_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostic = diagnostic;
        let _was = self.scrolls.insert(Region::Diagnostics, 0.0);
        self.read_readings();
    }

    pub fn read_readings(&mut self) {
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.readings = None;
            return;
        };
        if self
            .readings
            .as_ref()
            .is_some_and(|(path, _)| *path == held.path)
        {
            return;
        }
        self.fetch_readings();
    }

    fn fetch_readings(&mut self) {
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.readings = None;
            return;
        };
        let asked = Request::Readings {
            model: held.path.clone(),
            method: None,
        };
        let runs = match ask(&self.socket, &asked) {
            Ok(answer) if answer.served => answer
                .body
                .get("runs")
                .and_then(Value::as_list)
                .map(<[Value]>::to_vec)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        self.readings = Some((held.path.clone(), runs));
    }

    #[must_use]
    pub fn readings_of(&self, diagnostic: Diagnostic) -> Option<&Value> {
        let method = diagnostic.readings_method()?;
        let held = self.chosen.and_then(|at| self.models.get(at))?;
        let (path, runs) = self.readings.as_ref()?;
        if *path != held.path {
            return None;
        }
        runs.iter()
            .find(|run| run.get("method").and_then(Value::as_text) == Some(method))
    }

    fn keep_the_probes(&mut self) {
        let Doing::Probing(job) = &self.doing else {
            return;
        };
        let found = findings_of(job);
        if let Some(held) = self.chosen.and_then(|at| self.models.get_mut(at)) {
            keep_findings(&mut held.probed, found);
        }
        self.read_settings();
        self.fetch_readings();
    }

    fn configure(&mut self, act: &Act) {
        match *act {
            Act::Tab(tab) => {
                self.apply_edit();
                self.tab = tab;
                self.open = None;
                if tab == Tab::Contents && self.anatomy.is_none() {
                    self.read_anatomy();
                }
            }
            Act::Edit(field) => self.edit(field),
            Act::Switch(switch) => {
                self.apply_edit();
                self.flip(switch);
            }
            Act::Place(at) => {
                self.apply_edit();
                self.place(at);
                self.open = None;
            }
            Act::Rope(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut() {
                    settings.started.rope = ROPE_CHOICES.get(at).copied().flatten();
                    if settings.started.rope.is_none() {
                        settings.started.factor = None;
                    }
                }
                self.open = None;
            }
            _ => {}
        }
    }

    pub fn edit(&mut self, field: Field) {
        self.apply_edit();
        let card = match field {
            Field::Retries => Some(self.retries.to_string()),
            Field::Window => Some(
                self.challenge_window
                    .map_or_else(String::new, |window| window.to_string()),
            ),
            Field::Languages => Some(self.challenge_languages.clone().unwrap_or_default()),
            _ => None,
        };
        if let Some(now) = card {
            self.editing = Some((field, now));
            self.edit_refused = None;
            self.caret = Caret::Setting;
            return;
        }
        let Some(settings) = self.settings.as_ref() else {
            return;
        };
        let now = match field {
            Field::Context => settings.context.to_string(),
            Field::Threads => settings.threads.to_string(),
            Field::Batch => settings.batch.to_string(),
            Field::Port => settings.port.to_string(),
            Field::ApiKey => settings.api_key.clone().unwrap_or_default(),
            Field::RopeFactor => settings
                .started
                .factor
                .map_or_else(String::new, |factor| factor.to_string()),
            Field::Retries | Field::Window | Field::Languages => String::new(),
        };
        self.editing = Some((field, now));
        self.edit_refused = None;
        self.caret = Caret::Setting;
    }

    #[allow(clippy::too_many_lines, reason = "one arm a field, each named")]
    pub fn apply_edit(&mut self) {
        let Some((field, typed)) = self.editing.take() else {
            return;
        };
        let listed = typed.trim().to_owned();
        let typed = typed.trim().replace([',', '_'], "");
        let not_a_number = |what: &str| Some(format!("{what} wants a whole number, not {typed:?}"));
        match field {
            Field::Retries => {
                self.edit_refused = match typed.parse::<usize>() {
                    Ok(retries) if retries >= 1 => {
                        self.retries = retries;
                        None
                    }
                    Ok(_) => Some("a challenge needs at least one attempt".to_owned()),
                    Err(_) => not_a_number("the retries"),
                };
                return;
            }
            Field::Languages => {
                let named: Vec<&str> = listed
                    .split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .collect();
                let unknown = named.iter().find(|name| !LANGUAGE_NAMES.contains(name));
                self.edit_refused = if let Some(name) = unknown {
                    Some(format!(
                        "no language is called {name}; the catalogue runs in {}",
                        LANGUAGE_NAMES.join(", ")
                    ))
                } else {
                    self.challenge_languages = (!named.is_empty()).then(|| named.join(","));
                    None
                };
                return;
            }
            Field::Window => {
                self.edit_refused = if typed.is_empty() {
                    self.challenge_window = None;
                    None
                } else {
                    match typed.parse::<u64>() {
                        Ok(window) if window >= SMALLEST_WINDOW => {
                            self.challenge_window = Some(window);
                            None
                        }
                        Ok(_) => Some(format!(
                            "the window wants at least {SMALLEST_WINDOW} tokens"
                        )),
                        Err(_) => not_a_number("the window"),
                    }
                };
                return;
            }
            _ => {}
        }
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        self.edit_refused = match field {
            Field::Context => match typed.parse::<u64>() {
                Ok(tokens) if tokens >= 512 => {
                    settings.context = tokens;
                    None
                }
                Ok(_) => Some("the window wants at least 512 tokens".to_owned()),
                Err(_) => not_a_number("the context window"),
            },
            Field::Threads => match typed.parse::<u32>() {
                Ok(threads) if threads >= 1 => {
                    settings.threads = threads;
                    None
                }
                _ => not_a_number("threads"),
            },
            Field::Batch => match typed.parse::<u32>() {
                Ok(batch) if batch >= 1 => {
                    settings.batch = batch;
                    None
                }
                _ => not_a_number("the batch size"),
            },
            Field::Port => match typed.parse::<u16>() {
                Ok(port) if port >= 1024 => {
                    settings.port = port;
                    None
                }
                Ok(_) => Some("a port below 1024 needs rights MCF does not ask for".to_owned()),
                Err(_) => not_a_number("the port"),
            },
            Field::ApiKey => {
                settings.api_key = Some(typed).filter(|key| !key.is_empty());
                None
            }
            Field::RopeFactor => {
                if typed.is_empty() {
                    settings.started.factor = None;
                    None
                } else {
                    match typed.parse::<u32>() {
                        Ok(factor) if factor >= 1 => {
                            settings.started.factor = Some(factor);
                            None
                        }
                        _ => not_a_number("the rope factor"),
                    }
                }
            }
            Field::Retries | Field::Window | Field::Languages => None,
        };
    }

    pub fn flip(&mut self, switch: Switch) {
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        match switch {
            Switch::FlashAttention => settings.flash_attention = !settings.flash_attention,
            Switch::KeepResident => settings.keep_resident = !settings.keep_resident,
            Switch::Open => settings.open = !settings.open,
            Switch::DraftHead => settings.started.draft_head = !settings.started.draft_head,
            Switch::Projector => {
                settings.projector = if settings.projector.is_some() {
                    None
                } else {
                    self.recommended
                        .as_ref()
                        .and_then(|recommended| recommended.projector.clone())
                };
            }
        }
    }

    pub fn place(&mut self, at: usize) {
        let Some(placement) = self.placements.get(at) else {
            return;
        };
        if let Some(settings) = self.settings.as_mut() {
            settings.engine.clone_from(&placement.engine);
            settings.device.clone_from(&placement.device);
            settings.gpu_layers = placement.gpu_layers;
        }
    }

    #[must_use]
    pub fn placed_at(&self) -> Option<usize> {
        let settings = self.settings.as_ref()?;
        self.placements.iter().position(|held| {
            held.engine == settings.engine
                && held.device == settings.device
                && held.gpu_layers == settings.gpu_layers
        })
    }

    pub fn host_again(&mut self) {
        let Some(path) = self.last_hold.as_ref().map(|last| last.model.clone()) else {
            return;
        };
        let Some(at) = self.models.iter().position(|held| held.path == path) else {
            self.no_settings = Some(format!("{path} is not among the models here any more"));
            return;
        };
        self.chosen = Some(at);
        self.read_settings();
        self.host_it();
    }

    pub fn stop_run(&mut self) {
        self.queued.clear();
        self.queued_of = 0;
        match &mut self.doing {
            Doing::Measuring(job)
            | Doing::CrossChecking(job)
            | Doing::Reporting(job)
            | Doing::Probing(job)
            | Doing::Examining(job)
            | Doing::Evaluating(job) => {
                job.stop();
            }
            _ => {}
        }
    }

    pub fn cross_check(&mut self, at: usize) {
        self.tally_afresh();
        let Some(held) = self.models.get(at) else {
            return;
        };
        self.chosen = Some(at);
        self.doing = Doing::CrossChecking(job::Job::start(
            &self.socket,
            Request::CrossCheck {
                model: held.path.clone(),
            },
            format!("cross-checking {}", held.name),
        ));
    }

    #[must_use]
    pub fn asked_turn(&self) -> Option<mcf_serve::turn::Turn> {
        let word = |held: &str| {
            Some(held.trim())
                .filter(|typed| !typed.is_empty())
                .map(str::to_owned)
        };
        let turn = mcf_serve::turn::Turn {
            thinking: self.thinking,
            effort: word(&self.effort),
            system: word(&self.system),
        };
        turn.asks_anything().then_some(turn)
    }

    pub fn cycle_thinking(&mut self) {
        self.thinking = match self.thinking {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
    }

    pub fn ask(&mut self, at: usize) {
        let Some(held) = self.models.get(at) else {
            return;
        };
        let question = self.typed.trim().to_owned();
        if question.is_empty() {
            return;
        }
        self.said.clear();
        let turn = self.asked_turn();
        let picture = Some(self.picture.trim())
            .filter(|typed| !typed.is_empty())
            .map(str::to_owned);
        self.doing = Doing::Answering(job::Job::start(
            &self.socket,
            Request::Generate {
                model: held.path.clone(),
                prompt: question,
                limit: Some(256),
                seed: 0,
                tokens: None,
                pieces: None,
                engine: None,
                whose: mcf_record::content::Whose::User,
                pinned: false,
                turn: turn.clone(),
                image: picture,
                started: mcf_serve::declared::Started::default(),
            },
            format!("asking {}", held.name),
        ));
    }

    pub fn read_components(&mut self) {
        if let Ok(answer) = ask_within(&self.socket, &Request::Components, POLL)
            && answer.served
            && let Some(listed) = answer.body.get("components").and_then(Value::as_list)
        {
            self.components = listed.iter().map(component_from).collect();
        }
    }

    pub fn refresh(&mut self) {
        match ask_within(&self.socket, &Request::Holding, POLL) {
            Ok(answer) if answer.served => {
                self.refusal = None;
                self.busy = false;
                let mut read: Vec<Model> = answer
                    .body
                    .get("models")
                    .and_then(Value::as_list)
                    .map(|held| {
                        held.iter()
                            .filter(|entry| {
                                !matches!(entry.get("companion"), Some(Value::Bool(true)))
                            })
                            .map(model_from)
                            .collect()
                    })
                    .unwrap_or_default();
                read.sort_by(|one, two| one.name.cmp(&two.name));
                self.models = read;
                self.card_unused = answer.body.get("card_unused").and_then(|held| {
                    Some((
                        held.get("component").and_then(Value::as_text)?.to_owned(),
                        held.get("because").and_then(Value::as_text)?.to_owned(),
                    ))
                });
            }
            Ok(answer) => {
                self.refusal = Some(refused_because(&answer.body));
            }
            Err(why) if why.contains("not answering on this computer") => {
                self.busy = false;
                self.refusal = Some(why);
            }
            Err(_) => self.busy = true,
        }
    }

    pub fn sample(&mut self) {
        self.reading = self.sampler.read();
        if self.faults_read.is_none_or(|read| read.elapsed() >= POLL) {
            self.read_faults();
        }
    }

    #[must_use]
    pub fn build_said() -> String {
        said_of(
            mcf_core::build_identity::BuildIdentity::current().version,
            &mcf_core::build_identity::BuildIdentity::current()
                .revision
                .to_string(),
        )
    }

    pub fn read_build(&mut self) {
        if let Ok(answer) = ask_within(&self.socket, &Request::Status, POLL)
            && answer.served
            && let Some(build) = answer.body.get("build")
        {
            let text = |key: &str| build.get(key).and_then(Value::as_text).unwrap_or_default();
            self.daemon_build = Some(said_of(text("version"), text("revision")));
        }
    }

    pub fn read_faults(&mut self) {
        self.faults_read = Some(std::time::Instant::now());
        if let Ok(answer) = ask_within(
            &self.socket,
            &Request::Failures { last: FAULTS_SHOWN },
            POLL,
        ) && answer.served
            && let Some(listed) = answer.body.get("failures").and_then(Value::as_list)
        {
            self.faults = listed.iter().map(fault_from).collect();
            self.faults_in_record = answer
                .body
                .get("in_record")
                .and_then(Value::as_integer)
                .and_then(|held| usize::try_from(held).ok())
                .unwrap_or(self.faults.len());
        }
    }

    #[must_use]
    pub fn state_word(&self) -> String {
        if self.refusal.is_some() {
            "not answering".to_owned()
        } else if let Some(said) = self.under_way() {
            format!("working — {said}")
        } else {
            "MCF".to_owned()
        }
    }

    #[must_use]
    pub fn under_way(&self) -> Option<String> {
        let job = self.doing.job().filter(|job| !job.finished)?;
        Some(format!("{}, {} s so far", job.what, job.ran()))
    }

    #[must_use]
    pub fn state_line(&self) -> (String, String) {
        if let Some(why) = &self.refusal {
            return ("NOT UP".to_owned(), why.clone());
        }
        if self.busy
            && let Doing::Hosting(job) = &self.doing
            && !job.finished
        {
            return (
                "HOLDING".to_owned(),
                format!("{} — {}s so far", job.what, job.ran()),
            );
        }
        if self.busy
            && let Doing::Provisioning(job) = &self.doing
            && !job.finished
        {
            return (
                "BUILDING".to_owned(),
                format!("{} — {}s so far", job.what, job.ran()),
            );
        }
        if self.busy {
            return (
                "BUSY".to_owned(),
                "MCF is working on something and answers one thing at a time".to_owned(),
            );
        }
        match &self.doing {
            Doing::Nothing => (
                "IDLE".to_owned(),
                "nothing is being served — Models holds a model here".to_owned(),
            ),
            Doing::Listing(job)
            | Doing::Downloading(job)
            | Doing::Measuring(job)
            | Doing::CrossChecking(job)
            | Doing::Answering(job)
            | Doing::Reporting(job)
            | Doing::Probing(job)
            | Doing::Examining(job)
            | Doing::Evaluating(job)
            | Doing::Provisioning(job)
            | Doing::Hosting(job) => (
                if job.finished {
                    "IDLE".to_owned()
                } else {
                    "BUSY".to_owned()
                },
                job.what.clone(),
            ),
        }
    }

    #[must_use]
    pub fn ladder_line(&self) -> String {
        let mut depths = Vec::new();
        let mut depth = 512_u64;
        while depth <= self.window {
            depths.push(depth.to_string());
            depth = depth.saturating_mul(2);
        }
        depths.join(" · ")
    }

    #[must_use]
    pub fn quick_depth(&self) -> u64 {
        mcf_tui::screens::diagnostics::QUICK_DEPTH
    }

    #[must_use]
    pub fn estimate(&self, quick: bool) -> (u64, u64) {
        let seconds: u64 = if quick {
            mcf_tui::screens::diagnostics::quick_seconds(&self.tests)
        } else {
            self.seconds_of(Run::Ladder)
        };
        Self::spread(seconds)
    }

    #[must_use]
    pub fn cross_check_estimate(&self) -> (u64, u64) {
        Self::spread(self.seconds_of(Run::CrossCheck))
    }

    fn seconds_of(&self, run: Run) -> u64 {
        self.tests
            .iter()
            .find(|test| test.run == run)
            .and_then(|test| test.seconds)
            .unwrap_or(30)
    }

    fn spread(seconds: u64) -> (u64, u64) {
        #[expect(
            clippy::integer_division,
            reason = "a range in whole seconds; the remainder of a second is \
                      far inside the width of the range itself"
        )]
        let bounds = (
            seconds.saturating_mul(58) / 100,
            seconds.saturating_mul(142) / 100,
        );
        bounds
    }

    #[must_use]
    pub fn capability_sentence(&self) -> String {
        let card = self
            .reading
            .cards
            .iter()
            .filter_map(|card| Some((card.total?, card.used?)))
            .map(|(total, used)| total.saturating_sub(used))
            .max();
        match card {
            Some(free) if free > 0 => format!(
                "You have a graphics card with room for models up to about {}.",
                words::size_in_words(Some(free)).unwrap_or_default()
            ),
            _ => match self.reading.memory.available {
                Some(free) => format!(
                    "No graphics card was found, so models run on the processor. About {} is free.",
                    words::size_in_words(Some(free)).unwrap_or_default()
                ),
                None => "MCF has not been able to read this computer's memory.".to_owned(),
            },
        }
    }

    #[must_use]
    pub fn card_sentence(&self) -> String {
        let Some(card) = self.reading.cards.first() else {
            return "None found".to_owned();
        };
        match (card.total, card.used) {
            (Some(total), Some(used)) => format!(
                "{} · {} free",
                card.name,
                words::size_in_words(Some(total.saturating_sub(used))).unwrap_or_default()
            ),
            _ => card.name.clone(),
        }
    }

    #[must_use]
    pub fn memory_sentence(&self) -> String {
        match (self.reading.memory.available, self.reading.memory.total) {
            (Some(free), Some(total)) => format!(
                "{} of {} free",
                words::size_in_words(Some(free)).unwrap_or_default(),
                words::size_in_words(Some(total)).unwrap_or_default()
            ),
            _ => words::UNMEASURED.to_owned(),
        }
    }

    #[must_use]
    pub fn processor_sentence(&self) -> String {
        self.reading.processor.cores.map_or_else(
            || words::UNMEASURED.to_owned(),
            |cores| format!("{cores} cores"),
        )
    }

    #[must_use]
    pub fn doing_sentence(&self) -> String {
        if self.refusal.is_some() {
            "MCF is not answering".to_owned()
        } else {
            "Nothing running".to_owned()
        }
    }
}

#[allow(clippy::too_many_lines, reason = "the one loop, each event named")]
pub fn run(socket: std::path::PathBuf) -> Result<(), String> {
    let mut paint = paint::Painter::open("MCF", 1180, 760, paint::NIGHT)?;
    if let Some(window) = paint.window() {
        window.start_typing();
    }
    let mut desk = Desk::new(socket);
    desk.refresh();
    desk.sample();
    desk.read_hosted();
    desk.read_components();
    desk.read_faults();
    desk.read_build();

    let mut mouse = ui::Mouse::default();
    let mut last = std::time::Instant::now();
    let mut dirty = true;
    let mut waiting: Option<[u8; sdl::EVENT_BYTES]> = None;
    loop {
        mouse.settle();
        let mut acted = dirty;
        dirty = false;
        while let Some(event) = waiting
            .take()
            .or_else(|| paint.window().and_then(sdl::Window::next_event))
        {
            acted = true;
            match sdl::event_type(&event) {
                sdl::EVENT_QUIT => {
                    closing(&mut paint, &mut desk);
                    return Ok(());
                }
                sdl::EVENT_WINDOW_PIXEL_SIZE_CHANGED => paint.rescale(),
                sdl::EVENT_MOUSE_MOTION => {
                    mouse.at = points(&paint, sdl::event_mouse(&event));
                }
                sdl::EVENT_MOUSE_BUTTON_DOWN if sdl::event_is_left_button(&event) => {
                    mouse.at = points(&paint, sdl::event_mouse(&event));
                    mouse.down = true;
                    mouse.began = Some(mouse.at);
                }
                sdl::EVENT_MOUSE_BUTTON_UP if sdl::event_is_left_button(&event) => {
                    mouse.at = points(&paint, sdl::event_mouse(&event));
                    mouse.down = false;
                    mouse.click = Some(mouse.at);
                    desk.released();
                }
                sdl::EVENT_MOUSE_WHEEL => mouse.wheel = sdl::event_wheel(&event),
                sdl::EVENT_TEXT_INPUT => {
                    if desk.takes_typing()
                        && let Some(text) = sdl::event_text(&event)
                    {
                        desk.typing().push_str(&text);
                    }
                }
                sdl::EVENT_KEY_DOWN => match sdl::event_key(&event) {
                    sdl::KEY_ESCAPE => {
                        closing(&mut paint, &mut desk);
                        return Ok(());
                    }
                    key if key == u32::from(b'v')
                        && sdl::event_has_ctrl(&event)
                        && desk.takes_typing() =>
                    {
                        if let Some(text) = paint.window().and_then(sdl::Window::clipboard_text) {
                            desk.paste(&text);
                        }
                    }
                    key if key == u32::from(b'c')
                        && sdl::event_has_ctrl(&event)
                        && desk.page == Page::Prompt
                        && !desk.being_typed().is_empty() =>
                    {
                        if let Some(window) = paint.window() {
                            let _went = window.put_on_clipboard(desk.being_typed());
                        }
                    }
                    sdl::KEY_BACKSPACE if desk.takes_typing() => {
                        let _removed = desk.typing().pop();
                    }
                    sdl::KEY_RETURN if desk.takes_typing() => {
                        desk.returned(sdl::event_has_ctrl(&event));
                    }
                    key if key == u32::from(b'q') && !desk.takes_typing() => {
                        closing(&mut paint, &mut desk);
                        return Ok(());
                    }
                    key if key == u32::from(b'r') && !desk.takes_typing() => {
                        desk.refresh();
                        desk.sample();
                        desk.read_hosted();
                        desk.read_components();
                        desk.read_faults();
                        desk.read_build();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if desk.hear() {
            acted = true;
        }

        let a_run = desk.doing.busy();
        let due = (matches!(desk.page, Page::Monitor | Page::Hosting) || a_run)
            && last.elapsed() >= std::time::Duration::from_secs(1);
        if due {
            match desk.page {
                Page::Monitor => desk.sample(),
                _ => desk.read_hosted(),
            }
            if a_run {
                if desk.page != Page::Monitor {
                    desk.sample();
                }
                if desk.page == Page::Monitor {
                    desk.read_hosted();
                }
                desk.tally();
            }
            last = std::time::Instant::now();
            acted = true;
        }

        if acted && let Some(act) = view::draw(&mut paint, &desk, &mouse) {
            taken(&mut paint, &mut desk, act);
            dirty = true;
        }

        if !dirty {
            waiting = paint
                .window()
                .and_then(|window| window.wait_event(std::time::Duration::from_secs(1)));
        }
    }
}

fn closing(paint: &mut paint::Painter, desk: &mut Desk) {
    let Some(held) = desk.to_let_go() else {
        return;
    };
    view::saying(paint, &format!("letting go of {held} before closing…"));
    desk.stop_hosting();
}

fn fraction_of(done: i64, of: i64) -> Option<f32> {
    if of <= 0 {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "counts of a few steps")]
    Some((done as f32 / of as f32).clamp(0.0, 1.0))
}

fn keep_findings(held: &mut Vec<Finding>, found: Vec<Finding>) {
    for finding in found {
        match held.iter_mut().find(|had| had.name == finding.name) {
            Some(entry) => *entry = finding,
            None => held.push(finding),
        }
    }
}

fn findings_of(job: &job::Job) -> Vec<Finding> {
    let now = mcf_core::time::Timestamp::now().to_string();
    job.answers
        .iter()
        .filter_map(|answer| {
            let name = answer.get("step")?.get("name")?.as_text()?.to_owned();
            let lines: Vec<String> = answer
                .get("lines")?
                .as_list()?
                .iter()
                .filter_map(Value::as_text)
                .map(str::to_owned)
                .collect();
            (!lines.is_empty()).then_some(Finding {
                name,
                at: Some(now.clone()),
                engine: None,
                lines,
            })
        })
        .collect()
}

#[must_use]
pub fn when_said(at: &str) -> String {
    at.get(..16)
        .map_or_else(|| at.to_owned(), |head| head.replace('T', " "))
}

impl Desk {
    fn split(&mut self, splitter: Splitter, to: i32) {
        self.grabbed = Some(splitter);
        self.splits.set(splitter, as_points(to));
    }

    pub fn released(&mut self) {
        self.grabbed = None;
    }
}

fn taken(paint: &mut paint::Painter, desk: &mut Desk, act: Act) {
    if let Act::Copy(text) = &act
        && let Some(window) = paint.window()
    {
        let _went = window.put_on_clipboard(text);
    }
    desk.act(act);
}

fn points(paint: &paint::Painter, at: (f32, f32)) -> (f32, f32) {
    let scale = if paint.scale > 0.0 { paint.scale } else { 1.0 };
    (at.0 / scale, at.1 / scale)
}

#[cfg(test)]
mod tests;
