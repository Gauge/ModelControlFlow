pub mod chart;
pub mod font;
pub mod job;
pub mod paint;
pub mod paper;
pub mod sdl;
pub mod typing;
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
    pub cache: mcf_core::configuration::CacheType,
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
    pub card_energy_cost_millionths: Option<u64>,
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
            card_energy_cost_millionths: count("card_energy_cost_millionths"),
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
    let (_, total) = view::reserve_of(held, settings.context, settings.cache)?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Caret {
    Setting,
    #[default]
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Configure,
    Optimize,
    Statistics,
    Contents,
}

impl Tab {
    pub const ALL: [Self; 4] = [
        Self::Configure,
        Self::Optimize,
        Self::Statistics,
        Self::Contents,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Configure => "Configure",
            Self::Optimize => "Optimize",
            Self::Statistics => "Statistics",
            Self::Contents => "Contents",
        }
    }
}

#[derive(Debug, Default)]
pub struct Optimizing {
    pub sweep: mcf_optimize::dial::Sweep,
    pub report: mcf_optimize::reading::Report,
    pub running: bool,
    pub done: usize,
    pub refused: Option<String>,
    pub way: mcf_optimize::hunt::Way,
    pub measure: mcf_optimize::reading::Measure,
    pub custom: crate::typing::Typing,
    pub custom_focused: bool,
    pub custom_refused: Option<String>,
    pub known: usize,
    pub last_said: Option<String>,
    pub run: Option<mcf_optimize::running::Running>,
}

impl Optimizing {
    #[must_use]
    pub fn left(&self) -> usize {
        self.sweep.trials().saturating_sub(self.done)
    }

    #[must_use]
    pub fn fraction(&self) -> Option<f32> {
        let all = self.sweep.trials();
        if all == 0 {
            return None;
        }
        let done = u16::try_from(self.done).unwrap_or(u16::MAX);
        let all = u16::try_from(all).unwrap_or(u16::MAX);
        Some(f32::from(done) / f32::from(all.max(1)))
    }

    pub fn pick_dial(&mut self, at: usize) {
        if let Some(dial) = mcf_optimize::dial::Dial::ALL.get(at) {
            self.sweep = mcf_optimize::dial::Sweep::on(*dial);
            self.report = mcf_optimize::reading::Report::default();
            self.done = 0;
        }
    }

    pub fn toggle_value(&mut self, at: usize) {
        let offered = self.sweep.dial.suggested();
        let Some(step) = offered.get(at) else { return };
        if let Some(found) = self.sweep.steps.iter().position(|held| held == step) {
            let _dropped = self.sweep.steps.remove(found);
        } else {
            self.sweep.steps.push(*step);
            self.sweep.steps.sort_by_key(|held| match held {
                mcf_optimize::dial::Step::Whole(value)
                | mcf_optimize::dial::Step::Thousandths(value) => *value,
            });
        }
    }

    pub fn toggle_set(&mut self, number: usize) {
        if let Some(found) = self.sweep.sets.iter().position(|held| *held == number) {
            let _dropped = self.sweep.sets.remove(found);
        } else {
            self.sweep.sets.push(number);
            self.sweep.sets.sort_unstable();
        }
    }

    #[must_use]
    pub fn standing(&self) -> String {
        if let Some(said) = &self.last_said {
            return said.clone();
        }
        if self.known == 0 {
            return "nothing measured for this base yet".to_owned();
        }
        format!(
            "{} reading(s) already recorded for this configuration",
            self.known
        )
    }

    pub fn pick_way(&mut self, at: usize) {
        if let Some(way) = mcf_optimize::hunt::Way::ALL.get(at) {
            self.way = *way;
            self.refused = None;
        }
    }

    pub fn pick_measure(&mut self, at: usize) {
        if let Some(measure) = mcf_optimize::reading::Measure::ALL.get(at) {
            self.measure = *measure;
            self.refused = None;
        }
    }

    pub fn touch_the_custom(&mut self, touched: crate::ui::Touched) {
        if touched != crate::ui::Touched::No {
            self.custom_focused = true;
            self.custom_refused = None;
        }
        match touched {
            crate::ui::Touched::No => {}
            crate::ui::Touched::At(at) => self.custom.place(at, false),
            crate::ui::Touched::Word(at) => self.custom.word_at(at),
            crate::ui::Touched::DraggedTo(at) => self.custom.place(at, true),
        }
    }

    pub fn add_what_was_typed(&mut self) {
        self.custom_refused = None;
        let typed = self.custom.trim().to_owned();
        if typed.is_empty() {
            self.custom_refused = Some("type a value first".to_owned());
            return;
        }
        let dial = self.sweep.dial;
        let Some(step) = read_a_value(dial, &typed) else {
            self.custom_refused = Some(format!(
                "{typed:?} is not a value {} takes",
                dial.label().to_lowercase()
            ));
            return;
        };
        let span = dial.span();
        let held = match step {
            mcf_optimize::dial::Step::Whole(held) | mcf_optimize::dial::Step::Thousandths(held) => {
                held
            }
        };
        if !span.holds(held) {
            self.custom_refused = Some(format!(
                "{typed} is outside what this dial reaches — {} to {}",
                dial.step_of(span.floor).said(),
                dial.step_of(span.ceiling).said()
            ));
            return;
        }
        if self.sweep.steps.contains(&step) {
            self.custom_refused = Some(format!("{} is already in the list", step.said()));
            return;
        }
        self.sweep.steps.push(step);
        self.sweep.steps.sort_by_key(|held| match *held {
            mcf_optimize::dial::Step::Whole(value)
            | mcf_optimize::dial::Step::Thousandths(value) => value,
        });
        self.custom.clear();
        self.way = mcf_optimize::hunt::Way::ByHand;
    }

    pub fn cycle_repeats(&mut self) {
        self.sweep.repeats = match self.sweep.repeats {
            1 => 2,
            2 => 3,
            _ => 1,
        };
    }
}

const SWEEP_CEILING: u32 = 16_384;

fn hold_it_at(
    socket: &Path,
    model: &str,
    settings: &mcf_serve::hosting::Hosting,
    dial: mcf_optimize::dial::Dial,
    step: mcf_optimize::dial::Step,
) -> Result<u16, String> {
    let mut held = settings.clone();
    match dial {
        mcf_optimize::dial::Dial::MicroBatch => {
            let wanted = step.whole().unwrap_or(held.ubatch);
            held.ubatch = wanted;
            held.batch = held.batch.max(wanted);
        }
        mcf_optimize::dial::Dial::ThinkingBudget => held.started.thinking = step.whole(),
        mcf_optimize::dial::Dial::DraftDepth => {
            let wanted = step.whole().unwrap_or(0);
            held.started.draft_head = wanted > 0;
            held.started.drafted = (wanted > 0).then_some(wanted);
        }
        mcf_optimize::dial::Dial::Temperature
        | mcf_optimize::dial::Dial::TopP
        | mcf_optimize::dial::Dial::TopK => {}
    }
    let answer = ask(
        socket,
        &Request::Host {
            model: model.to_owned(),
            settings: held.to_request(),
        },
    )?;
    if !answer.served {
        return Err(refused_because(&answer.body));
    }
    answer
        .body
        .get("address")
        .and_then(Value::as_text)
        .and_then(port_of)
        .ok_or_else(|| "MCF held the model but named no port to reach it on".to_owned())
}

#[must_use]
pub fn port_of(address: &str) -> Option<u16> {
    address
        .rsplit(':')
        .next()?
        .trim_end_matches('/')
        .parse()
        .ok()
}

#[must_use]
pub fn read_a_value(
    dial: mcf_optimize::dial::Dial,
    typed: &str,
) -> Option<mcf_optimize::dial::Step> {
    let typed = typed.trim().replace([',', '_'], "");
    match dial.scale() {
        mcf_optimize::dial::Scale::Whole => typed
            .parse::<u32>()
            .ok()
            .map(mcf_optimize::dial::Step::Whole),
        mcf_optimize::dial::Scale::Thousandths => {
            let (whole, part) = match typed.split_once('.') {
                Some((before, after)) => (before, after),
                None => (typed.as_str(), ""),
            };
            let whole: u32 = if whole.is_empty() {
                0
            } else {
                whole.parse().ok()?
            };
            if !part.chars().all(|held| held.is_ascii_digit()) || part.len() > 3 {
                return None;
            }
            let mut thousandths = part.to_owned();
            while thousandths.len() < 3 {
                thousandths.push('0');
            }
            let part: u32 = if thousandths.is_empty() {
                0
            } else {
                thousandths.parse().ok()?
            };
            whole
                .checked_mul(1000)?
                .checked_add(part)
                .map(mcf_optimize::dial::Step::Thousandths)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Host,
    Models,
    Exit,
    Adding,
    Hosting,
    Anatomy,
    Vocabulary,
}

impl Page {
    pub const MENU: &'static [(Self, &'static str)] = &[
        (Self::Hosting, "Server"),
        (Self::Models, "Models"),
        (Self::Exit, "Exit"),
    ];

    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Host | Self::Adding | Self::Anatomy | Self::Vocabulary | Self::Models => {
                Self::Models
            }
            Self::Hosting => Self::Hosting,
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
    pub cache_elements_per_token: Option<u64>,
    pub engine: Option<String>,
    pub device: Option<String>,
    pub device_free: Option<u64>,
    pub repository: Option<String>,
    pub file: String,
    pub on_a_card: bool,
    pub refused: Option<String>,
    pub does_not_fit: Option<String>,
}

impl Model {
    #[must_use]
    pub fn will_run(&self) -> bool {
        self.refused.is_none() && self.does_not_fit.is_none() && self.engine.is_some()
    }

    #[must_use]
    pub fn where_it_runs(&self) -> String {
        if let Some(why) = &self.refused {
            return why.clone();
        }
        if let Some(why) = &self.does_not_fit {
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
        if let Some(why) = &self.does_not_fit {
            return why.clone();
        }
        let place = if self.on_a_card {
            "your graphics card"
        } else {
            "your processor"
        };
        format!("Will run on {place}.")
    }

    #[must_use]
    pub fn memory_sentence(&self) -> String {
        words::size_in_words(self.bytes).map_or_else(
            || words::UNMEASURED.to_owned(),
            |size| format!("Uses {size}"),
        )
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
    let _results = results_of(runs);
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
        repository: repository_of(held),
        file,
        on_a_card: resolved_text("device_kind").as_deref() == Some("gpu"),
        cache_per_token: number_from_runs("cache_bytes_per_token"),
        cache_elements_per_token: number_from_runs("cache_elements_per_token"),
        refused: if known { None } else { resolved_text("why") },
        does_not_fit: resolved_text("why_not"),
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
    Cache,
    Answers,
    Pooling,
    Loading,
    LargeTensors,
    SplitMode,
    Experts,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gone {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removing {
    pub model: String,
    pub name: String,
    pub files: Vec<Gone>,
    pub bytes: Option<u64>,
    pub reversible: bool,
    pub shelf: String,
    pub reason: crate::typing::Typing,
    pub purge: bool,
    pub refused: Option<String>,
    pub done: Option<String>,
}

impl Removing {
    #[must_use]
    pub fn finished(&self) -> bool {
        self.done.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Slots,
    Alias,
    Ubatch,
    ThreadsBatch,
    DenseLayersOnCpu,
    MainDevice,
    Devices,
    OverrideTensors,
    CacheReuse,
    PromptCacheMib,
    Checkpoints,
    CheckpointSpacing,
    Keep,
    Context,
    Threads,
    Batch,
    Port,
    ApiKey,
    RopeFactor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    CacheOnProcessor,
    PromptCache,
    IdleSlots,
    ContextShift,
    FlashAttention,
    KeepResident,
    DraftHead,
    Projector,
    Open,
}

pub const EXPERT_CHOICES: [mcf_serve::hosting::Experts; 3] = [
    mcf_serve::hosting::Experts::WithTheModel,
    mcf_serve::hosting::Experts::OnTheProcessor,
    mcf_serve::hosting::Experts::FirstLayers(8),
];

pub const CACHE_CHOICES: [mcf_core::configuration::CacheType; 9] =
    mcf_core::configuration::CacheType::ALL;

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
    LookUp,
    Download { reference: String, file: String },
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
    SeeStatistics,
    Stop,
    HostAgain,
    Pick(String),
    Tab(Tab),
    Dial(usize),
    SweepValue(usize),
    SweepWay(usize),
    SweepMeasure(usize),
    CustomValue(crate::ui::Touched),
    AddCustom,
    ForgetReadings,
    TestSet(usize),
    Repeats,
    Sweep,
    Contents(Page),
    Edit(Field, crate::ui::Touched),
    AskToRemove,
    RemoveReason(crate::ui::Touched),
    PurgeToggle,
    DoRemove,
    CancelRemove,
    Switch(Switch),
    Place(usize),
    Rope(usize),
    Cache(usize),
    SplitMode(usize),
    Loading(usize),
    Answers(usize),
    Pooling(usize),
    LargeTensors(usize),
    Experts(usize),
    Copy(String),
    SetOn(Option<mcf_serve::control::On>),
    Open(Picker),
    Shut,
    SetWindow(u64),
    Cycle(usize),
    Recommended,
    LastSettings,
    HostIt,
    Build(String),
    StopHosting,
    Close,
    Focus(Caret),
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
    Answering(job::Job),
}

impl Doing {
    #[must_use]
    pub fn job(&self) -> Option<&job::Job> {
        match self {
            Self::Nothing => None,
            Self::Listing(job)
            | Self::Downloading(job)
            | Self::Answering(job)
            | Self::Provisioning(job)
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
    pub typed: crate::typing::Typing,
    pub filter: crate::typing::Typing,
    pub hub: Option<HubList>,
    pub hub_chosen: Option<usize>,
    pub offered: std::collections::BTreeMap<String, Vec<OfferedFile>>,
    pub pending: Option<Pending>,
    pub after_download: Option<Act>,
    pub filters: Filters,
    pub caret: Caret,
    pub chosen: Option<usize>,
    pub doing: Doing,
    pub said: String,
    pub queued: std::collections::VecDeque<Card>,
    pub queued_of: usize,
    pub daemon_build: Option<String>,
    pub home: Option<std::path::PathBuf>,
    pub settings: Option<mcf_serve::hosting::Hosting>,
    pub recommended: Option<mcf_serve::hosting::Hosting>,
    pub no_settings: Option<String>,
    pub last_settings: Option<(mcf_serve::hosting::Hosting, String)>,
    pub needs_engine: Option<String>,
    pub card_unused: Option<(String, String)>,
    pub placements: Vec<Placement>,
    pub on: Option<mcf_serve::control::On>,
    pub tab: Tab,
    pub optimizing: Optimizing,
    pub contents: Page,
    pub editing: Option<(Field, crate::typing::Typing)>,
    pub edit_refused: Option<String>,
    pub removing: Option<Removing>,
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
            optimizing: Optimizing::default(),
            contents: Page::Anatomy,
            editing: None,
            removing: None,
            edit_refused: None,
            declared: None,
            shown: None,
            scrolls: std::collections::BTreeMap::new(),
            splits: Splits::default(),
            grabbed: None,
            reading: mcf_tui::machine::Reading::default(),
            refusal: None,
            busy: false,
            typed: crate::typing::Typing::default(),
            caret: Caret::Document,
            chosen: None,
            doing: Doing::Nothing,
            said: String::new(),
            queued: std::collections::VecDeque::new(),
            queued_of: 0,
            daemon_build: None,
            home: None,
            settings: None,
            recommended: None,
            no_settings: None,
            last_settings: None,
            filter: crate::typing::Typing::default(),
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
        self.typing_into_a_value()
            || self.removing.as_ref().is_some_and(|held| !held.finished())
            || matches!(self.page, Page::Adding | Page::Hosting | Page::Models)
            || (matches!(self.page, Page::Host) && self.editing.is_some())
    }

    const PASTE_LIMIT: usize = 512;

    pub const PROMPT_LIMIT: usize = 65_536;

    pub fn paste(&mut self, text: &str) {
        if self.page == Page::Hosting && self.caret == Caret::Document {
            let kept: String = text
                .replace("\r\n", "\n")
                .chars()
                .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                .collect();
            self.typed.put(&kept, Self::PROMPT_LIMIT);
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
        let limit = Self::PASTE_LIMIT;
        self.typing().put(kept, limit);
    }

    pub fn stopped_typing(&mut self) {
        if self.typing_into_a_value() {
            self.optimizing.custom_focused = false;
            self.optimizing.custom.clear();
            self.optimizing.custom_refused = None;
            return;
        }
        if self.removing.is_some() {
            self.removing = None;
            return;
        }
        if self.editing.is_some() {
            self.editing = None;
            self.edit_refused = None;
            return;
        }
        if self.page == Page::Models && !self.filter.is_empty() {
            self.filter.clear();
            return;
        }
        self.typing().none();
    }

    #[must_use]
    pub fn typing_into_a_value(&self) -> bool {
        self.tab == Tab::Optimize && self.optimizing.custom_focused
    }

    pub fn typing(&mut self) -> &mut crate::typing::Typing {
        if self.typing_into_a_value() {
            return &mut self.optimizing.custom;
        }
        if let Some(removing) = self.removing.as_mut().filter(|held| !held.finished()) {
            return &mut removing.reason;
        }
        if let (Page::Models | Page::Host, Some((_, typed))) = (self.page, self.editing.as_mut()) {
            return typed;
        }
        if self.page == Page::Models {
            return &mut self.filter;
        }
        &mut self.typed
    }

    #[must_use]
    pub fn being_typed(&self) -> &str {
        self.typing_now().said()
    }

    #[must_use]
    pub fn typing_now(&self) -> &crate::typing::Typing {
        if self.typing_into_a_value() {
            return &self.optimizing.custom;
        }
        if let Some(removing) = self.removing.as_ref().filter(|held| !held.finished()) {
            return &removing.reason;
        }
        if let (Page::Models | Page::Host, Some((_, typed))) = (self.page, self.editing.as_ref()) {
            return typed;
        }
        if self.page == Page::Models {
            return &self.filter;
        }
        &self.typed
    }

    pub fn returned(&mut self, with_control: bool) {
        if self.page == Page::Hosting && !with_control && self.caret == Caret::Document {
            let limit = Self::PROMPT_LIMIT;
            self.typed.put("\n", limit);
            return;
        }
        self.entered();
    }

    pub fn entered(&mut self) {
        if self.typing_into_a_value() {
            self.optimizing.add_what_was_typed();
            return;
        }
        match self.page {
            Page::Models if self.editing.is_some() => self.apply_edit(),
            Page::Models => {
                if !self.filter.trim().is_empty() && self.library().is_empty() {
                    self.search_hub();
                }
            }
            Page::Host => self.apply_edit(),
            Page::Adding => self.look_up(),
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
        let _before = self.doing.job().map_or(0, |job| job.answers.len());
        let heard = match &mut self.doing {
            Doing::Nothing => false,
            Doing::Listing(job)
            | Doing::Downloading(job)
            | Doing::Answering(job)
            | Doing::Provisioning(job)
            | Doing::Hosting(job) => job.drain(),
        };
        if !heard {
            return false;
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

    fn go(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        self.scrolls.clear();
        if page.section() == Page::Hosting {
            self.sample();
        }
        if matches!(page, Page::Anatomy | Page::Vocabulary) {
            self.read_anatomy();
        }
        self.page = page;
    }

    pub fn act(&mut self, act: Act) {
        match act {
            Act::Go(page) => self.go(page),
            Act::LookUp => self.look_up(),
            Act::Download { reference, file } => self.download(&reference, &file),
            Act::HostAgain => self.host_again(),
            Act::Pick(repository) => {
                self.typed.set(repository);
                self.look_up();
            }
            Act::Tab(_)
            | Act::Dial(_)
            | Act::SweepValue(_)
            | Act::SweepWay(_)
            | Act::SweepMeasure(_)
            | Act::CustomValue(_)
            | Act::AddCustom
            | Act::ForgetReadings
            | Act::TestSet(_)
            | Act::Repeats
            | Act::Sweep
            | Act::Edit(..)
            | Act::Switch(_)
            | Act::Place(_)
            | Act::Rope(_)
            | Act::Cache(_)
            | Act::SplitMode(_)
            | Act::Loading(_)
            | Act::Answers(_)
            | Act::Pooling(_)
            | Act::LargeTensors(_)
            | Act::Experts(_) => {
                self.configure(&act);
            }
            Act::Contents(page) => self.contents = page,
            Act::SetOn(on) => {
                self.on = on;
                self.open = None;
            }
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
            Act::AskToRemove
            | Act::RemoveReason(_)
            | Act::PurgeToggle
            | Act::DoRemove
            | Act::CancelRemove => self.removal(&act),
            Act::Close | Act::Copy(_) => {}
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
            Act::Focus(caret) => self.caret = caret,
            Act::Clear => self.typed.clear(),
            Act::Stop | Act::Dismiss => self.doing = Doing::Nothing,
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
                        cache: answer
                            .body
                            .get("settings")
                            .and_then(|settings| settings.get("cache"))
                            .and_then(Value::as_text)
                            .and_then(mcf_core::configuration::CacheType::parse)
                            .unwrap_or_default(),
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

    fn removal(&mut self, act: &Act) {
        match act {
            Act::AskToRemove => self.ask_to_remove(),
            Act::RemoveReason(touched) => self.touch_the_reason(*touched),
            Act::PurgeToggle => {
                if let Some(removing) = self.removing.as_mut() {
                    removing.purge = !removing.purge;
                    removing.refused = None;
                }
            }
            Act::DoRemove => self.do_remove(),
            Act::CancelRemove => self.removing = None,
            _ => {}
        }
    }

    pub fn ask_to_remove(&mut self) {
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        let model = held.path.clone();
        let name = held.name.clone();
        match ask(
            &self.socket,
            &Request::Removal {
                model: model.clone(),
            },
        ) {
            Ok(answer) if answer.served => {
                let whole = |key: &str| {
                    answer
                        .body
                        .get(key)
                        .and_then(Value::as_integer)
                        .and_then(|held| u64::try_from(held).ok())
                };
                let text = |key: &str| {
                    answer
                        .body
                        .get(key)
                        .and_then(Value::as_text)
                        .unwrap_or_default()
                        .to_owned()
                };
                let files = match answer.body.get("files") {
                    Some(Value::List(listed)) => listed
                        .iter()
                        .map(|one| Gone {
                            path: one
                                .get("path")
                                .and_then(Value::as_text)
                                .unwrap_or_default()
                                .to_owned(),
                            bytes: one
                                .get("bytes")
                                .and_then(Value::as_integer)
                                .and_then(|held| u64::try_from(held).ok())
                                .unwrap_or(0),
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                self.removing = Some(Removing {
                    model,
                    name,
                    files,
                    bytes: whole("bytes"),
                    reversible: matches!(answer.body.get("reversible"), Some(Value::Bool(true))),
                    shelf: text("shelf"),
                    reason: crate::typing::Typing::of(String::new()),
                    purge: false,
                    refused: None,
                    done: None,
                });
            }
            Ok(answer) => {
                self.removing = Some(Removing {
                    model,
                    name,
                    files: Vec::new(),
                    bytes: None,
                    reversible: false,
                    shelf: String::new(),
                    reason: crate::typing::Typing::of(String::new()),
                    purge: false,
                    refused: Some(refused_because(&answer.body)),
                    done: None,
                });
            }
            Err(why) => {
                self.removing = Some(Removing {
                    model,
                    name,
                    files: Vec::new(),
                    bytes: None,
                    reversible: false,
                    shelf: String::new(),
                    reason: crate::typing::Typing::of(String::new()),
                    purge: false,
                    refused: Some(why),
                    done: None,
                });
            }
        }
    }

    fn touch_the_reason(&mut self, touched: crate::ui::Touched) {
        let Some(removing) = self.removing.as_mut() else {
            return;
        };
        match touched {
            crate::ui::Touched::No => {}
            crate::ui::Touched::At(at) => removing.reason.place(at, false),
            crate::ui::Touched::Word(at) => removing.reason.word_at(at),
            crate::ui::Touched::DraggedTo(at) => removing.reason.place(at, true),
        }
    }

    pub fn do_remove(&mut self) {
        let Some(removing) = self.removing.as_ref() else {
            return;
        };
        if removing.finished() {
            return;
        }
        let reason = removing.reason.trim().to_owned();
        if reason.is_empty() {
            if let Some(removing) = self.removing.as_mut() {
                removing.refused = Some(
                    "Say why this is going. A removal nobody can account for is one nobody can \
                     answer for."
                        .to_owned(),
                );
            }
            return;
        }
        let asked = Request::Remove {
            model: removing.model.clone(),
            reason: reason.clone(),
            purge: removing.purge,
        };
        let answered = ask(&self.socket, &asked);
        let Some(removing) = self.removing.as_mut() else {
            return;
        };
        match answered {
            Ok(answer) if answer.served => {
                let bytes = answer
                    .body
                    .get("bytes")
                    .and_then(Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
                    .unwrap_or(0);
                let purged = answer
                    .body
                    .get("purged_bytes")
                    .and_then(Value::as_integer)
                    .is_some();
                let put_on = answer
                    .body
                    .get("shelf")
                    .and_then(Value::as_text)
                    .unwrap_or_default()
                    .to_owned();
                removing.done = Some(if purged {
                    format!(
                        "{} is gone — {} freed, and it cannot be brought back.",
                        removing.name,
                        view::gigabytes(bytes)
                    )
                } else {
                    format!(
                        "{} is off the list — {} shelved in {put_on}, and it can be moved back.",
                        removing.name,
                        view::gigabytes(bytes)
                    )
                });
                removing.refused = None;
            }
            Ok(answer) => removing.refused = Some(refused_because(&answer.body)),
            Err(why) => removing.refused = Some(why),
        }
        if self
            .removing
            .as_ref()
            .is_some_and(|held| held.done.is_some())
        {
            self.chosen = None;
            self.refresh();
        }
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

    pub const EVERY_RUN: [Card; 7] = [
        Card::Throughput,
        Card::CrossCheck,
        Card::Capabilities,
        Card::Performance,
        Card::Fidelity,
        Card::Behaviour,
        Card::Coding,
    ];

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

    #[must_use]
    pub fn ledger_path(&self) -> Option<std::path::PathBuf> {
        self.home
            .as_ref()
            .map(|home| mcf_optimize::ledger::Ledger::beside(home))
    }

    pub fn forget_readings(&mut self) {
        self.optimizing.refused = None;
        let Some(path) = self.ledger_path() else {
            return;
        };
        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.optimizing.known = 0;
                self.optimizing.report = mcf_optimize::reading::Report::default();
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.optimizing.known = 0;
            }
            Err(error) => {
                self.optimizing.refused = Some(format!(
                    "the record at {} could not be cleared: {error}",
                    path.display()
                ));
            }
        }
    }

    #[must_use]
    fn base_for_a_sweep(&self) -> Option<mcf_optimize::ledger::Under> {
        let held = self.chosen.and_then(|at| self.models.get(at))?;
        let settings = self.settings.as_ref()?;
        Some(mcf_optimize::ledger::Under {
            model: held.path.clone(),
            model_bytes: held.bytes.unwrap_or(0),
            engine: settings.engine.clone(),
            commit: self.daemon_build.clone().unwrap_or_default(),
            context: settings.context,
            batch: settings.batch,
            ubatch: settings.ubatch,
            cache: format!("{:?}", settings.cache),
            flash_attention: settings.flash_attention,
            draft_head: settings.started.draft_head,
            draft_depth: settings.started.drafted,
            thinking_budget: settings.started.thinking,
            temperature: None,
            top_p: None,
            top_k: None,
            corpus: mcf_optimize::ledger::CORPUS,
        })
    }

    pub fn read_the_ledger(&mut self) {
        let Some(path) = self.ledger_path() else {
            return;
        };
        let Ok(ledger) = mcf_optimize::ledger::Ledger::open(&path) else {
            return;
        };
        let Some(under) = self.base_for_a_sweep() else {
            self.optimizing.known = 0;
            return;
        };
        let against = ledger.against(&under, self.optimizing.sweep.dial);
        self.optimizing.known = against.len();
        let mut report = mcf_optimize::reading::Report::default();
        for row in against {
            report.record(row.reading.clone());
        }
        self.optimizing.report = report;
    }

    fn choosing_values(&mut self, act: &Act) {
        match *act {
            Act::SweepWay(at) => self.optimizing.pick_way(at),
            Act::SweepMeasure(at) => self.optimizing.pick_measure(at),
            Act::CustomValue(touched) => self.optimizing.touch_the_custom(touched),
            Act::AddCustom => self.optimizing.add_what_was_typed(),
            Act::ForgetReadings => self.forget_readings(),
            _ => {}
        }
    }

    fn start_or_stop_sweeping(&mut self) {
        if let Some(run) = self.optimizing.run.as_ref() {
            run.stop();
            self.optimizing.running = false;
            return;
        }
        self.optimizing.refused = None;
        let by_hand = self.optimizing.way == mcf_optimize::hunt::Way::ByHand;
        if by_hand && self.optimizing.sweep.steps.is_empty() {
            self.optimizing.refused =
                Some("choose at least one value, or let the automatic search pick them".to_owned());
            return;
        }
        if self.optimizing.sweep.sets.is_empty() {
            self.optimizing.refused =
                Some("choose at least one test set before running".to_owned());
            return;
        }
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.optimizing.refused = Some("choose a model on the left first".to_owned());
            return;
        };
        let model = held.path.clone();
        let Some(settings) = self.settings.clone() else {
            self.optimizing.refused = Some(
                "MCF has not said what this model would run under yet — open Configure and \
                 let it work that out"
                    .to_owned(),
            );
            return;
        };
        let Some(under) = self.base_for_a_sweep() else {
            self.optimizing.refused =
                Some("this model's settings have not been read yet".to_owned());
            return;
        };
        let Some(path) = self.ledger_path() else {
            self.optimizing.refused = Some(
                "MCF does not know where to write readings down, and a sweep nobody records is \
                 one that has to be run again"
                    .to_owned(),
            );
            return;
        };
        let ledger = match mcf_optimize::ledger::Ledger::open(&path) {
            Ok(ledger) => ledger,
            Err(failure) => {
                self.optimizing.refused = Some(failure.to_string());
                return;
            }
        };
        self.optimizing.known = ledger.against(&under, self.optimizing.sweep.dial).len();
        let course = mcf_optimize::course::Course::laid_out(
            under.clone(),
            self.optimizing.way,
            self.optimizing.sweep.dial,
            &self.optimizing.sweep.steps,
            &self.optimizing.sweep.sets,
            self.optimizing.sweep.repeats,
            self.optimizing.measure,
        );
        let orders = mcf_optimize::running::Orders {
            endpoint: mcf_optimize::trial::Endpoint {
                port: 0,
                key: None,
                patience: std::time::Duration::from_secs(7200),
            },
            under,
            dial: self.optimizing.sweep.dial,
            ceiling: SWEEP_CEILING,
            thinking: None,
            effort: None,
            mark: self.optimizing.measure.needs_the_answers_run(),
            room: path
                .parent()
                .map_or_else(std::env::temp_dir, |beside| beside.join("marking")),
        };
        let dial = self.optimizing.sweep.dial;
        let socket = self.socket.clone();
        let hosting =
            std::boxed::Box::new(move |step| hold_it_at(&socket, &model, &settings, dial, step));
        self.optimizing.report = mcf_optimize::reading::Report::default();
        self.optimizing.done = 0;
        self.optimizing.last_said = None;
        self.optimizing.run = Some(mcf_optimize::running::Running::begun(
            orders,
            course,
            ledger,
            hosting,
            || mcf_core::time::Timestamp::now().to_string(),
        ));
        self.optimizing.running = true;
    }

    pub fn hear_the_sweep(&mut self) -> bool {
        let Some(run) = self.optimizing.run.as_mut() else {
            return false;
        };
        let moved = run.hear();
        if !moved {
            return false;
        }
        self.optimizing.done = run.taken.saturating_add(run.skipped);
        self.optimizing.known = run.skipped;
        self.optimizing.report = run.report.clone();
        if let Some(why) = run.refused.clone() {
            self.optimizing.refused = Some(why);
        }
        if run.finished {
            self.optimizing.last_said = Some(run.said());
            self.optimizing.running = false;
            self.optimizing.run = None;
        }
        true
    }

    fn dialling(&mut self, act: &Act) -> bool {
        match *act {
            Act::Dial(at) => {
                self.open = None;
                self.optimizing.pick_dial(at);
                self.read_the_ledger();
            }
            Act::SweepValue(at) => self.optimizing.toggle_value(at),
            Act::SweepWay(_)
            | Act::SweepMeasure(_)
            | Act::CustomValue(_)
            | Act::AddCustom
            | Act::ForgetReadings => self.choosing_values(act),
            Act::TestSet(number) => self.optimizing.toggle_set(number),
            Act::Repeats => self.optimizing.cycle_repeats(),
            Act::Sweep => self.start_or_stop_sweeping(),
            _ => return false,
        }
        true
    }

    fn configure(&mut self, act: &Act) {
        if self.dialling(act) {
            return;
        }
        match *act {
            Act::Tab(tab) => {
                self.apply_edit();
                self.tab = tab;
                self.open = None;
                if tab == Tab::Contents && self.anatomy.is_none() {
                    self.read_anatomy();
                }
            }
            Act::Edit(field, touched) => self.edit(field, touched),
            Act::Switch(switch) => {
                self.apply_edit();
                self.flip(switch);
            }
            Act::Place(at) => {
                self.apply_edit();
                self.place(at);
                self.open = None;
            }
            Act::Answers(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(held) = mcf_serve::hosting::Answers::ALL.get(at).copied()
                {
                    settings.answers = held;
                }
                self.open = None;
            }
            Act::Pooling(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(held) = mcf_serve::hosting::Pooling::ALL.get(at).copied()
                {
                    settings.pooling = held;
                }
                self.open = None;
            }
            Act::Loading(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(held) = mcf_serve::hosting::Loading::ALL.get(at).copied()
                {
                    settings.loading = held;
                }
                self.open = None;
            }
            Act::LargeTensors(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(held) = mcf_serve::hosting::Lazily::ALL.get(at).copied()
                {
                    settings.lazily = held;
                }
                self.open = None;
            }
            Act::SplitMode(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(split) = mcf_serve::hosting::Split::ALL.get(at).copied()
                {
                    settings.spread.split = split;
                }
                self.open = None;
            }
            Act::Experts(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(held) = EXPERT_CHOICES.get(at).copied()
                {
                    settings.spread.experts = held;
                }
                self.open = None;
            }
            Act::Cache(at) => {
                self.apply_edit();
                if let Some(settings) = self.settings.as_mut()
                    && let Some(width) = mcf_core::configuration::CacheType::ALL.get(at).copied()
                {
                    settings.cache = width;
                    if width.is_quantized() {
                        settings.flash_attention = true;
                    }
                }
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

    fn touch(&mut self, touched: crate::ui::Touched) {
        let Some((_, held)) = self.editing.as_mut() else {
            return;
        };
        match touched {
            crate::ui::Touched::No => {}
            crate::ui::Touched::At(at) => held.place(at, false),
            crate::ui::Touched::Word(at) => held.word_at(at),
            crate::ui::Touched::DraggedTo(at) => held.place(at, true),
        }
    }

    pub fn edit(&mut self, field: Field, touched: crate::ui::Touched) {
        if self
            .editing
            .as_ref()
            .is_some_and(|(held, _)| *held == field)
        {
            self.touch(touched);
            return;
        }
        self.apply_edit();
        let Some(settings) = self.settings.as_ref() else {
            return;
        };
        let now = match field {
            Field::Context => settings.context.to_string(),
            Field::Threads => settings.threads.to_string(),
            Field::Batch => settings.batch.to_string(),
            Field::Slots => settings.slots.to_string(),
            Field::Alias => settings.alias.clone().unwrap_or_default(),
            Field::Ubatch => settings.ubatch.to_string(),
            Field::ThreadsBatch => settings.threads_batch.to_string(),
            Field::DenseLayersOnCpu => settings.spread.ffn_layers_on_processor.to_string(),
            Field::MainDevice => settings.spread.main_device.to_string(),
            Field::Devices => settings.spread.devices.clone().unwrap_or_default(),
            Field::OverrideTensors => settings.spread.override_tensors.clone().unwrap_or_default(),
            Field::CacheReuse => settings.reuse.cache_reuse.to_string(),
            Field::PromptCacheMib => settings.reuse.prompt_cache_mib.to_string(),
            Field::Checkpoints => settings.reuse.checkpoints.to_string(),
            Field::CheckpointSpacing => settings.reuse.checkpoint_min_step.to_string(),
            Field::Keep => settings.reuse.keep.to_string(),
            Field::Port => settings.port.to_string(),
            Field::ApiKey => settings.api_key.clone().unwrap_or_default(),
            Field::RopeFactor => settings
                .started
                .factor
                .map_or_else(String::new, |factor| factor.to_string()),
        };
        self.editing = Some((field, crate::typing::Typing::of(now)));
        self.touch(touched);
        self.edit_refused = None;
        self.caret = Caret::Setting;
    }

    #[allow(clippy::too_many_lines, reason = "one arm a field, each named")]
    pub fn apply_edit(&mut self) {
        let Some((field, typed)) = self.editing.take() else {
            return;
        };
        let _listed = typed.trim().to_owned();
        let typed = typed.trim().replace([',', '_'], "");
        let not_a_number = |what: &str| Some(format!("{what} wants a whole number, not {typed:?}"));
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
            Field::Slots => match typed.parse::<u32>() {
                Ok(slots) if slots >= 1 => {
                    settings.slots = slots;
                    None
                }
                _ => not_a_number("the slot count"),
            },
            Field::DenseLayersOnCpu => match typed.parse::<u32>() {
                Ok(held) => {
                    settings.spread.ffn_layers_on_processor = held;
                    None
                }
                Err(_) => not_a_number("the dense layer count"),
            },
            Field::MainDevice => match typed.parse::<u32>() {
                Ok(held) => {
                    settings.spread.main_device = held;
                    None
                }
                Err(_) => not_a_number("the main device"),
            },
            Field::Devices => {
                settings.spread.devices = (!typed.is_empty()).then(|| typed.clone());
                None
            }
            Field::OverrideTensors => {
                settings.spread.override_tensors = (!typed.is_empty()).then(|| typed.clone());
                None
            }
            Field::Alias => {
                settings.alias = (!typed.is_empty()).then(|| typed.clone());
                None
            }
            Field::Ubatch => match typed.parse::<u32>() {
                Ok(held) if held >= 1 => {
                    settings.ubatch = held;
                    None
                }
                _ => not_a_number("what is read at once"),
            },
            Field::ThreadsBatch => match typed.parse::<u32>() {
                Ok(held) if held >= 1 => {
                    settings.threads_batch = held;
                    None
                }
                _ => not_a_number("the prompt thread count"),
            },
            Field::CacheReuse => match typed.parse::<u32>() {
                Ok(held) => {
                    settings.reuse.cache_reuse = held;
                    None
                }
                Err(_) => not_a_number("the reuse chunk"),
            },
            Field::PromptCacheMib => match typed.parse::<i64>() {
                Ok(held) if held >= -1 => {
                    settings.reuse.prompt_cache_mib = held;
                    None
                }
                _ => not_a_number("the prompt cache memory"),
            },
            Field::Checkpoints => match typed.parse::<u32>() {
                Ok(held) => {
                    settings.reuse.checkpoints = held;
                    None
                }
                Err(_) => not_a_number("the checkpoint count"),
            },
            Field::CheckpointSpacing => match typed.parse::<u32>() {
                Ok(held) => {
                    settings.reuse.checkpoint_min_step = held;
                    None
                }
                Err(_) => not_a_number("the checkpoint spacing"),
            },
            Field::Keep => match typed.parse::<i64>() {
                Ok(held) if held >= -1 => {
                    settings.reuse.keep = held;
                    None
                }
                _ => not_a_number("what is kept from the front"),
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
        };
    }

    pub fn flip(&mut self, switch: Switch) {
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        match switch {
            Switch::CacheOnProcessor => {
                settings.spread.cache_on_processor = !settings.spread.cache_on_processor;
            }
            Switch::PromptCache => settings.reuse.prompt_cache = !settings.reuse.prompt_cache,
            Switch::IdleSlots => settings.reuse.idle_slots = !settings.reuse.idle_slots,
            Switch::ContextShift => settings.reuse.context_shift = !settings.reuse.context_shift,
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

    pub fn ask(&mut self, at: usize) {
        let Some(held) = self.models.get(at) else {
            return;
        };
        let question = self.typed.trim().to_owned();
        if question.is_empty() {
            return;
        }
        self.said.clear();
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
                turn: None,
                image: None,
                started: std::boxed::Box::new(mcf_serve::declared::Started::default()),
            },
            format!("asking {}", held.name),
        ));
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
        {
            if let Some(build) = answer.body.get("build") {
                let text = |key: &str| build.get(key).and_then(Value::as_text).unwrap_or_default();
                self.daemon_build = Some(said_of(text("version"), text("revision")));
            }
            self.home = answer
                .body
                .get("home")
                .and_then(Value::as_text)
                .map(std::path::PathBuf::from);
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
            | Doing::Answering(job)
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
    desk.read_build();

    let mut mouse = ui::Mouse::default();
    let mut last_click: Option<((f32, f32), std::time::Instant)> = None;
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
                    mouse.just_pressed = true;
                }
                sdl::EVENT_MOUSE_BUTTON_UP if sdl::event_is_left_button(&event) => {
                    mouse.at = points(&paint, sdl::event_mouse(&event));
                    mouse.down = false;
                    mouse.click = Some(mouse.at);
                    mouse.twice =
                        last_click.is_some_and(|(at, when): ((f32, f32), std::time::Instant)| {
                            when.elapsed() < std::time::Duration::from_millis(400)
                                && (at.0 - mouse.at.0).abs() < 4.0
                                && (at.1 - mouse.at.1).abs() < 4.0
                        });
                    last_click = Some((mouse.at, std::time::Instant::now()));
                    desk.released();
                }
                sdl::EVENT_MOUSE_WHEEL => mouse.wheel = sdl::event_wheel(&event),
                sdl::EVENT_TEXT_INPUT => {
                    if desk.takes_typing()
                        && let Some(text) = sdl::event_text(&event)
                    {
                        let limit = Desk::PASTE_LIMIT;
                        desk.typing().put(&text, limit);
                    }
                }
                sdl::EVENT_KEY_DOWN => match sdl::event_key(&event) {
                    sdl::KEY_ESCAPE if desk.takes_typing() => {
                        desk.stopped_typing();
                    }
                    sdl::KEY_ESCAPE => {
                        closing(&mut paint, &mut desk);
                        return Ok(());
                    }
                    key if matches!(key, sdl::KEY_LEFT | sdl::KEY_RIGHT) && desk.takes_typing() => {
                        let way = if key == sdl::KEY_LEFT {
                            crate::typing::Way::Back
                        } else {
                            crate::typing::Way::On
                        };
                        let by = if sdl::event_has_ctrl(&event) {
                            crate::typing::By::Word
                        } else {
                            crate::typing::By::Character
                        };
                        let keeping = sdl::event_has_shift(&event);
                        desk.typing().go(way, by, keeping);
                    }
                    key if matches!(key, sdl::KEY_HOME | sdl::KEY_END) && desk.takes_typing() => {
                        let way = if key == sdl::KEY_HOME {
                            crate::typing::Way::Back
                        } else {
                            crate::typing::Way::On
                        };
                        let keeping = sdl::event_has_shift(&event);
                        desk.typing().go(way, crate::typing::By::Line, keeping);
                    }
                    sdl::KEY_DELETE if desk.takes_typing() => {
                        desk.typing().rub(
                            crate::typing::Way::On,
                            if sdl::event_has_ctrl(&event) {
                                crate::typing::By::Word
                            } else {
                                crate::typing::By::Character
                            },
                        );
                    }
                    key if key == u32::from(b'a')
                        && sdl::event_has_ctrl(&event)
                        && desk.takes_typing() =>
                    {
                        desk.typing().all();
                    }
                    key if key == u32::from(b'x')
                        && sdl::event_has_ctrl(&event)
                        && desk.takes_typing() =>
                    {
                        if let Some(taken) = desk.typing().cut()
                            && let Some(window) = paint.window()
                        {
                            let _went = window.put_on_clipboard(&taken);
                        }
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
                        && !desk.being_typed().is_empty() =>
                    {
                        if let Some(copied) = desk.typing_now().copied()
                            && let Some(window) = paint.window()
                        {
                            let _went = window.put_on_clipboard(&copied);
                        }
                    }
                    sdl::KEY_BACKSPACE if desk.takes_typing() => {
                        desk.typing().rub(
                            crate::typing::Way::Back,
                            if sdl::event_has_ctrl(&event) {
                                crate::typing::By::Word
                            } else {
                                crate::typing::By::Character
                            },
                        );
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

        if desk.hear_the_sweep() {
            acted = true;
        }

        let a_run = desk.doing.busy();
        let due = (desk.page == Page::Hosting || a_run)
            && last.elapsed() >= std::time::Duration::from_secs(1);
        if due {
            desk.sample();
            desk.read_hosted();
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
