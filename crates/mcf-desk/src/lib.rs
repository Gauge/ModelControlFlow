pub mod chart;
pub mod font;
pub mod job;
pub mod notice;
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
    /// Whether the hold answers anything other than this computer.
    pub open: bool,
    pub network_address: Option<String>,
    pub in_use: Option<Use>,
}

/// How many readings of the token counts are kept.
///
/// At one a second this is the last hour of a hold. A graph of what was used when is only
/// worth having if it reaches back past the last few minutes — an hour of readings is
/// about a hundred and forty kilobytes, which is nothing against a model.
pub const TALLIES_KEPT: usize = 3_600;

/// One reading of what a hold has put through the model.
///
/// Three counts, not one rate: what came in, what of it the model actually had to read,
/// and what it wrote. The difference between the first two is what the prompt cache
/// saved, and on a conversation that keeps its prefix that is most of the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Prompt tokens asked for, those served from the cache included.
    pub asked: u64,
    /// Prompt tokens the model actually read, cache hits excluded.
    pub processed: u64,
    /// Tokens written.
    pub written: u64,
    /// When it was read.
    ///
    /// A rate is a difference over a span, and the span is only a second because that is
    /// how often MCF asks — a daemon that answers slowly makes it longer. Timing each
    /// reading means the rate is what happened rather than what the poll assumed.
    pub at: std::time::Instant,
}

impl Tally {
    /// What a reading of the hold says, where it says enough to be worth drawing.
    ///
    /// One count is enough. Engines differ in what they publish, and refusing a reading
    /// because one of three counts was missing drew an empty graph beside figures that
    /// were plainly there.
    #[must_use]
    pub fn of(in_use: &Use) -> Option<Self> {
        let processed = in_use.prompted;
        let written = in_use.generated_live.or(in_use.generated);
        let reused = in_use.prompt_reused;
        if processed.is_none() && written.is_none() && reused.is_none() {
            return None;
        }
        let processed = processed.unwrap_or(0);
        Some(Self {
            asked: processed.saturating_add(reused.unwrap_or(0)),
            processed,
            written: written.unwrap_or(0),
            at: std::time::Instant::now(),
        })
    }

    /// How fast a count was climbing between one reading and the next, per second.
    ///
    /// Nothing where the two readings are the same reading, or where a count went
    /// backwards — an engine restarted under the hold starts its counters again, and a
    /// negative rate is not a rate.
    #[must_use]
    pub fn per_second(then: Self, now: Self, of: fn(&Self) -> u64) -> Option<f32> {
        let over = now.at.saturating_duration_since(then.at).as_secs_f32();
        if over <= 0.0 {
            return None;
        }
        let gained = of(&now).checked_sub(of(&then))?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a token count over a span of seconds, shown to one decimal"
        )]
        let gained = gained as f32;
        Some(gained / over)
    }
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
    /// What the engine itself says its average throughput has been. MCF works a rate out
    /// between one reading and the next, which is the livelier figure but needs two
    /// readings; this one is there from the first, so a hold a second old says something
    /// true rather than nothing.
    pub engine_generated_per_second: Option<f32>,
    pub engine_prompted_per_second: Option<f32>,
    /// Prompt tokens the engine reused from its cache rather than reading again. On a
    /// conversation that keeps its prefix this is most of the prompt, and it is the
    /// difference between a fast second message and a slow one.
    pub prompt_reused: Option<u64>,
    /// The deepest sequence the engine has seen, prompt and generation together. What
    /// replaces the cache-occupancy gauge on an engine that no longer publishes one.
    pub deepest: Option<u64>,
    /// Tokens the draft head proposed, and how many the model took. MCF has a setting for
    /// the draft head and a sweep that searches it, and until now nothing that said
    /// whether the head was earning its keep while a model was actually being used.
    pub drafted: Option<u64>,
    pub drafted_taken: Option<u64>,
    /// How long the engine has spent reading prompts and how long writing answers. The
    /// counters MCF divides by to state an average: a total over the time actually spent
    /// on it is exact and needs no sampling, where a difference between two readings a
    /// second apart is nothing at all between requests.
    pub engine_prompt_seconds: Option<f32>,
    pub engine_generating_seconds: Option<f32>,
    /// How many requests the hold has answered. Counted by the daemon, because the engine
    /// publishes no such figure of its own.
    pub requests_served: Option<u64>,
}

impl Use {
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        // Read as a double, not a single. An engine writes its counters as floats, and a
        // single holds whole numbers exactly only up to about sixteen million — past
        // which a token count would quietly start rounding, on exactly the long session
        // where the count is worth reading.
        let count = |key: &str| match value.get(key) {
            Some(Value::Integer(held)) => u64::try_from(*held).ok(),
            Some(Value::Text(text)) => text.trim().parse::<f64>().ok().map(|held| {
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
            engine_generated_per_second: rate("engine_said_tokens_per_second"),
            engine_prompted_per_second: rate("engine_said_prompt_tokens_per_second"),
            prompt_reused: count("prompt_tokens_reused"),
            deepest: count("deepest_tokens"),
            drafted: count("drafted_tokens"),
            drafted_taken: count("drafted_tokens_taken"),
            engine_prompt_seconds: rate("engine_prompt_seconds"),
            engine_generating_seconds: rate("engine_generating_seconds"),
            requests_served: count("requests_served"),
        }
    }

    /// How much of what the draft head proposed the model actually took, as a share. Only
    /// where a draft head has proposed something: a share of nothing proposed is not a
    /// figure, and a draft head that is switched off has not failed at anything.
    #[must_use]
    pub fn draft_taken_share(&self) -> Option<f32> {
        let drafted = self.drafted.filter(|held| *held > 0)?;
        let taken = self.drafted_taken?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "token counts, shown as a whole percentage"
        )]
        let share = taken as f32 / drafted as f32;
        Some(share.clamp(0.0, 1.0))
    }

    /// What to show for generation throughput: MCF's own reading between two samples where
    /// there is one, and the engine's own average where there is not. A hold that has just
    /// started has no two samples yet, and showing nothing there reads as a model doing
    /// nothing.
    #[must_use]
    pub fn generating_per_second(&self) -> Option<f32> {
        self.generated_per_second
            .filter(|rate| *rate > 0.0)
            .or(self.engine_generated_per_second)
    }

    #[must_use]
    pub fn prompting_per_second(&self) -> Option<f32> {
        self.prompted_per_second
            .filter(|rate| *rate > 0.0)
            .or(self.engine_prompted_per_second)
    }

    /// What the hold has averaged while generating: everything written, over the time
    /// actually spent writing it.
    ///
    /// This is a division of two counters, not a difference between two readings. The
    /// token counters only move when a request finishes, so a difference taken a second
    /// apart is nought almost always and a whole answer's worth once in a while — which
    /// is what had the rate cards reading 0.0 while the model was plainly working. An
    /// average over time spent is exact, and says something from the first request on.
    #[must_use]
    pub fn generation_average(&self) -> Option<f32> {
        over(self.generated, self.engine_generating_seconds)
    }

    /// The same for reading prompts. Held apart from generation because the two run an
    /// order of magnitude apart, and an average across both describes neither.
    #[must_use]
    pub fn prefill_average(&self) -> Option<f32> {
        over(self.prompted, self.engine_prompt_seconds)
    }

    /// How much of everything asked for the engine answered out of its cache, as a share.
    /// Stated against the whole prompt, which is what was read plus what was reused.
    #[must_use]
    pub fn cache_share(&self) -> Option<f32> {
        let reused = self.prompt_reused?;
        let asked = reused.saturating_add(self.prompted.unwrap_or(0));
        if asked == 0 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "token counts, shown as a whole percentage"
        )]
        let share = reused as f32 / asked as f32;
        Some(share.clamp(0.0, 1.0))
    }

    /// The time the hold has spent working: reading prompts and writing answers together.
    #[must_use]
    pub fn seconds_working(&self) -> Option<f32> {
        match (self.engine_prompt_seconds, self.engine_generating_seconds) {
            (None, None) => None,
            (prompting, generating) => Some(prompting.unwrap_or(0.0) + generating.unwrap_or(0.0)),
        }
    }

    /// What one request came to on average, for a count the hold keeps. Nothing until a
    /// request has finished: a total divided by no requests is not an average.
    #[must_use]
    pub fn a_request(&self, of: Option<u64>) -> Option<f32> {
        let requests = self.requests_served.filter(|served| *served > 0)?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "counts of tokens and of requests, shown to a whole number"
        )]
        let each = of? as f32 / requests as f32;
        Some(each)
    }
}

/// A total over a span of seconds, where both are there and the span is real.
fn over(total: Option<u64>, seconds: Option<f32>) -> Option<f32> {
    let seconds = seconds.filter(|spent| *spent > 0.0)?;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a token count over seconds, shown to one decimal"
    )]
    let total = total? as f32;
    Some(total / seconds)
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
    pub named: Vec<String>,
    pub custom: crate::typing::Typing,
    pub custom_focused: bool,
    pub custom_refused: Option<String>,
    pub known: usize,
    pub rows: Vec<mcf_optimize::ledger::Row>,
    pub picked: Vec<mcf_optimize::ledger::At>,
    pub last_said: Option<String>,
    pub run: Option<mcf_optimize::running::Running>,
    /// What a finished sweep landed on, waiting to be told whether to keep it. A sweep
    /// measures; what the model is held under does not move until somebody says it should.
    pub settled: Option<mcf_optimize::dial::Step>,
    /// What was said about the last value taken up, so the answer to a decision does not
    /// vanish the moment it is made.
    pub adopted: Option<String>,
}

impl Optimizing {
    #[must_use]
    pub fn left(&self) -> usize {
        self.sweep.trials().saturating_sub(self.done)
    }

    /// What one trial of this sweep asks for, so that how far into it the sweep has got can
    /// be said against something. A reading trial reads a prompt and a writing one writes
    /// an answer of a set length, because that length is the work being timed. A graded
    /// one asks for nothing: a question is answered for as long as the model needs.
    #[must_use]
    pub fn ceiling_of_a_trial(&self) -> Option<u32> {
        if self.measure.needs_the_answers_run() {
            return None;
        }
        if self.sweep.dial.times_reading_the_prompt() {
            return Some(mcf_optimize::trial::TOKENS_PREFILLED);
        }
        Some(mcf_optimize::trial::TOKENS_TIMED)
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

    pub fn pick_dial_among(&mut self, at: usize, offered: &[mcf_optimize::dial::Dial]) {
        if let Some(dial) = offered.get(at) {
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
        if self.sweep.dial.values_are_a_list() {
            self.refused = Some(format!(
                "{} runs every value it has and nothing else, so there is no span to \
                 search over",
                self.sweep.dial.label().to_lowercase()
            ));
            return;
        }
        if let Some(way) = mcf_optimize::hunt::Way::ALL.get(at) {
            self.way = *way;
            self.refused = None;
        }
    }

    /// A setting whose values are named by the model is swept over exactly those names. It
    /// has no span to double through — the numbers are places in a list, and a search that
    /// climbed past the end of the list would ask for a level the model never named and
    /// write the number down as if it were one.
    fn only_the_levels_the_model_names(&mut self) {
        if !self.sweep.dial.values_are_a_list() {
            return;
        }
        self.way = mcf_optimize::hunt::Way::ByHand;
        // A setting whose words are MCF's own knows how many it has; one whose words are
        // the model's has as many as the model named.
        let held = self.sweep.dial.own_words().len().max(self.named.len());
        let every: Vec<mcf_optimize::dial::Step> = (0..held)
            .filter_map(|at| u32::try_from(at).ok())
            .map(mcf_optimize::dial::Step::Whole)
            .collect();
        // The values ticked are the values run. Only a choice that names a place the list
        // does not have — or names nothing — is put back to the whole list.
        let chosen_fits = !self.sweep.steps.is_empty()
            && self.sweep.steps.iter().all(|step| every.contains(step));
        if !chosen_fits {
            self.sweep.steps = every;
        }
    }

    /// Rank a sweep of this setting the way the setting asks to be ranked, and lay out the
    /// run that goes with it. Marking the answers runs the sets; timing runs one trial and
    /// takes the rate, so the sets would be eight times nothing.
    pub fn rank_as_the_setting_asks(&mut self) {
        self.measure = self.sweep.dial.ranked_by();
        self.lay_out_the_run();
    }

    fn lay_out_the_run(&mut self) {
        if self.measure.needs_the_answers_run() {
            // The short corpus: a thousand questions with one right answer apiece. A
            // score off eight hard programs moves in lumps, and a setting that makes
            // answers a little worse cannot be seen through a score like that.
            self.sweep.sets = mcf_optimize::corpus::Set::short()
                .iter()
                .map(|set| set.number)
                .collect();
            self.sweep.repeats = 1;
            return;
        }
        self.sweep.sets = vec![1];
        self.sweep.repeats = mcf_optimize::trial::TIMES_TIMED;
    }

    pub fn pick_measure(&mut self, at: usize) {
        let Some(measure) = mcf_optimize::reading::Measure::ALL.get(at).copied() else {
            return;
        };
        if measure.needs_the_answers_run() && self.sweep.dial.cannot_change_an_answer() {
            self.refused = Some(format!(
                "{} cannot change what a model answers, only how fast it answers it, so there \
                 is nothing for correctness to say about it",
                self.sweep.dial.label()
            ));
            return;
        }
        self.refused = None;
        if measure == self.measure {
            return;
        }
        self.measure = measure;
        self.lay_out_the_run();
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

    /// What was typed into the field beside the offered values, if it is a value this dial
    /// takes. There is nothing to press: a value that is set is a value that runs, so this
    /// is read when the field changes and again when the sweep starts.
    pub fn what_was_typed(&self) -> Result<Option<mcf_optimize::dial::Step>, String> {
        let typed = self.custom.trim().to_owned();
        if typed.is_empty() {
            return Ok(None);
        }
        let dial = self.sweep.dial;
        let Some(step) = dial.read_among(&typed, &self.named).or_else(|| {
            (!dial.is_named_by_the_model())
                .then(|| read_a_value(dial, &typed))
                .flatten()
        }) else {
            return Err(format!(
                "{typed:?} is not a value {} takes",
                dial.label().to_lowercase()
            ));
        };
        let span = dial.span();
        let held = match step {
            mcf_optimize::dial::Step::Whole(held) | mcf_optimize::dial::Step::Thousandths(held) => {
                held
            }
        };
        if !span.holds(held) {
            return Err(format!(
                "{typed} is outside what this dial reaches — {} to {}",
                dial.step_of(span.floor).said(),
                dial.step_of(span.ceiling).said()
            ));
        }
        Ok(Some(step))
    }

    /// Say straight away whether what has been typed will run, rather than at the moment the
    /// sweep is started and it is too late to have meant something else.
    pub fn look_at_what_was_typed(&mut self) {
        self.custom_refused = self.what_was_typed().err();
    }

    pub fn take_each(&mut self, times: usize) {
        self.sweep.repeats = u8::try_from(times).unwrap_or(1).clamp(1, 3);
    }
}

const HOLDING_PATIENCE: std::time::Duration = std::time::Duration::from_mins(30);

#[must_use]
pub fn loading_line(body: &Value) -> Option<String> {
    let loading = body.get("loading")?;
    let count = |key: &str| loading.get(key).and_then(Value::as_integer);
    let seconds = count("seconds").unwrap_or(0);
    let on = count("resident_bytes")
        .and_then(|held| u64::try_from(held).ok())
        .map(view::gigabytes);
    match on {
        Some(on) => Some(format!("{on} loaded, {seconds}s so far")),
        None => Some(format!("{seconds}s so far")),
    }
}

/// Where a dial lands in the settings a model is held under. One place says this, so a
/// value taken up after a sweep is written exactly where the sweep was moving it.
fn put_the_dial(
    dial: mcf_optimize::dial::Dial,
    step: mcf_optimize::dial::Step,
    named: &[String],
    settings: &mut mcf_serve::hosting::Hosting,
) {
    use mcf_optimize::dial::Dial;
    match dial {
        Dial::MicroBatch => {
            let wanted = step.whole().unwrap_or(settings.ubatch);
            settings.ubatch = wanted;
            settings.batch = settings.batch.max(wanted);
        }
        // The other way round, for the same reason: the engine will not run a pass wider
        // than the batch it was handed, so asking for a batch under the pass quietly
        // narrows the pass. Narrowing it here means the reading is taken under what was
        // actually run, and the ledger writes both down.
        Dial::Batch => {
            let wanted = step.whole().unwrap_or(settings.batch);
            settings.batch = wanted;
            settings.ubatch = settings.ubatch.min(wanted);
        }
        // Where the words for these live is the dial: a step is a place in its own list,
        // so the list and the setting cannot drift apart.
        Dial::CacheWidth => {
            if let Some(width) = step
                .whole()
                .and_then(|at| usize::try_from(at).ok())
                .and_then(|at| CACHE_CHOICES.get(at))
            {
                settings.cache = *width;
            }
        }
        Dial::Experts => {
            settings.spread.experts = match step.whole() {
                Some(0) => mcf_serve::hosting::Experts::WithTheModel,
                Some(_) => mcf_serve::hosting::Experts::OnTheProcessor,
                None => settings.spread.experts,
            };
        }
        Dial::FlashAttention => {
            settings.flash_attention = step.whole().is_some_and(|held| held > 0);
        }
        Dial::ThreadsForAPrompt => {
            if let Some(threads) = step.whole().filter(|held| *held > 0) {
                settings.threads_batch = threads;
            }
        }
        Dial::ThinkingBudget => settings.started.thinking = step.whole(),
        Dial::DraftDepth => {
            let wanted = step.whole().unwrap_or(0);
            settings.started.draft_head = wanted > 0;
            settings.started.drafted = (wanted > 0).then_some(wanted);
        }
        Dial::ThinkingLevel => {
            let wanted = step
                .whole()
                .and_then(|at| usize::try_from(at).ok())
                .and_then(|at| named.get(at))
                .cloned();
            if wanted.as_deref() == Some(Dial::OFF) {
                settings.started.effort = None;
                settings.started.thinking = Some(0);
                return;
            }
            settings.started.effort = wanted;
            if settings.started.thinking == Some(0) {
                settings.started.thinking = None;
            }
        }
        Dial::Temperature => {
            settings.started.temperature = step.thousandths();
        }
        Dial::TopP => {
            settings.started.top_p = step.thousandths();
        }
        Dial::TopK => settings.started.top_k = step.whole(),
    }
}

fn hold_it_at(
    socket: &Path,
    model: &str,
    settings: &mcf_serve::hosting::Hosting,
    dial: mcf_optimize::dial::Dial,
    step: mcf_optimize::dial::Step,
    along: &mut dyn FnMut(String),
) -> Result<u16, String> {
    let mut held = settings.clone();
    match dial {
        mcf_optimize::dial::Dial::MicroBatch => {
            let wanted = step.whole().unwrap_or(held.ubatch);
            held.ubatch = wanted;
            held.batch = held.batch.max(wanted);
        }
        mcf_optimize::dial::Dial::Batch => {
            let wanted = step.whole().unwrap_or(held.batch);
            held.batch = wanted;
            held.ubatch = held.ubatch.min(wanted);
        }
        mcf_optimize::dial::Dial::CacheWidth => {
            if let Some(width) = step
                .whole()
                .and_then(|at| usize::try_from(at).ok())
                .and_then(|at| CACHE_CHOICES.get(at))
            {
                held.cache = *width;
            }
        }
        mcf_optimize::dial::Dial::Experts => {
            held.spread.experts = match step.whole() {
                Some(0) => mcf_serve::hosting::Experts::WithTheModel,
                Some(_) => mcf_serve::hosting::Experts::OnTheProcessor,
                None => held.spread.experts,
            };
        }
        mcf_optimize::dial::Dial::FlashAttention => {
            held.flash_attention = step.whole().is_some_and(|value| value > 0);
        }
        mcf_optimize::dial::Dial::ThreadsForAPrompt => {
            if let Some(threads) = step.whole().filter(|value| *value > 0) {
                held.threads_batch = threads;
            }
        }
        mcf_optimize::dial::Dial::ThinkingBudget => held.started.thinking = step.whole(),
        mcf_optimize::dial::Dial::DraftDepth => {
            let wanted = step.whole().unwrap_or(0);
            held.started.draft_head = wanted > 0;
            held.started.drafted = (wanted > 0).then_some(wanted);
        }
        mcf_optimize::dial::Dial::ThinkingLevel
        | mcf_optimize::dial::Dial::Temperature
        | mcf_optimize::dial::Dial::TopP
        | mcf_optimize::dial::Dial::TopK => {}
    }
    let answer = asked_until_done(
        socket,
        &Request::Host {
            model: model.to_owned(),
            settings: held.to_request(),
        },
        HOLDING_PATIENCE,
        |body| {
            if let Some(said) = loading_line(body) {
                along(said);
            }
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
    /// Everything MCF is bringing here, and what can be done about each of it. Its own
    /// page because a transfer is not a step in choosing a model: it runs for an hour, it
    /// runs while other things run, and there can be several of them at once.
    Downloads,
}

impl Page {
    pub const MENU: &'static [(Self, &'static str)] = &[
        (Self::Hosting, "Server"),
        (Self::Models, "Models"),
        (Self::Downloads, "Downloads"),
        (Self::Exit, "Exit"),
    ];

    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Host | Self::Adding | Self::Anatomy | Self::Vocabulary | Self::Models => {
                Self::Models
            }
            Self::Hosting => Self::Hosting,
            Self::Downloads => Self::Downloads,
            Self::Exit => Self::Exit,
        }
    }
}

/// What the disk the models live on holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Storage {
    pub total: u64,
    pub free: u64,
}

impl Storage {
    /// What is used, which is what the filesystem holds less what it has free.
    #[must_use]
    pub const fn used(self) -> u64 {
        self.total.saturating_sub(self.free)
    }

    /// A share of the whole, in hundredths, or nothing where there is no whole to be a
    /// share of.
    #[must_use]
    pub fn share_of_the_disk(self, bytes: u64) -> Option<u64> {
        bytes.saturating_mul(100).checked_div(self.total)
    }

    /// A share of what is used rather than of the whole disk.
    #[must_use]
    pub fn share_of_what_is_used(self, bytes: u64) -> Option<u64> {
        bytes.saturating_mul(100).checked_div(self.used())
    }
}

/// What the weights on the shelf come to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Weights {
    pub bytes: u64,
    pub files: usize,
    /// Multimodal projectors with no model beside them.
    ///
    /// A projector is the half of a vision model that turns a picture into something the
    /// language half can read; on its own it runs nothing. They are left behind when the
    /// model they belonged to is removed, and they are not small.
    pub orphans: Vec<Orphan>,
}

impl Weights {
    /// What the orphaned projectors come to, which is what removing them would give back.
    #[must_use]
    pub fn reclaimable(&self) -> u64 {
        self.orphans
            .iter()
            .map(|held| held.bytes)
            .fold(0, u64::saturating_add)
    }
}

/// One file on the shelf that is a projector with no model beside it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Orphan {
    pub path: String,
    pub bytes: u64,
}

impl Orphan {
    #[must_use]
    pub fn name(&self) -> String {
        std::path::Path::new(&self.path).file_stem().map_or_else(
            || self.path.clone(),
            |stem| stem.to_string_lossy().into_owned(),
        )
    }

    /// The repository it was left behind by, as far as the path says.
    #[must_use]
    pub fn beside(&self) -> String {
        let held = std::path::Path::new(&self.path);
        held.parent()
            .and_then(|parent| parent.file_name())
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }
}

/// One transfer, as the window reads it off the daemon's queue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transfer {
    pub id: u64,
    pub reference: String,
    pub file: String,
    pub part: u64,
    pub parts: u64,
    pub arrived: u64,
    pub whole: u64,
    pub state: String,
    pub why: Option<String>,
    pub path: Option<String>,
}

impl Transfer {
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        let count = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(0)
        };
        Self {
            id: count("id"),
            reference: text("reference").unwrap_or_default(),
            file: text("file").unwrap_or_default(),
            part: count("part"),
            parts: count("parts"),
            arrived: count("arrived_bytes"),
            whole: count("whole_bytes"),
            state: text("state").unwrap_or_default(),
            // One line of it. A row in a list has room for what went wrong, not for every
            // scrap of context the refusal carries; the whole of it is in the record.
            why: value
                .get("why")
                .filter(|held| !matches!(held, Value::Null))
                .map(refused_because)
                .and_then(|why| why.lines().next().map(str::to_owned)),
            path: text("path"),
        }
    }

    /// The short name of the file, which is what a person recognises it by.
    #[must_use]
    pub fn name(&self) -> String {
        self.file
            .rsplit('/')
            .next()
            .unwrap_or(&self.file)
            .trim_end_matches(".gguf")
            .to_owned()
    }

    #[must_use]
    pub fn under_way(&self) -> bool {
        matches!(self.state.as_str(), "queued" | "fetching" | "checking")
    }

    #[must_use]
    pub fn settled(&self) -> bool {
        matches!(self.state.as_str(), "done" | "failed" | "cancelled")
    }

    /// How far along, as a share of the whole, or nothing while the whole is unknown. A
    /// bar drawn against a total nobody has stated is a bar that means nothing.
    #[must_use]
    pub fn fraction(&self) -> Option<f32> {
        if self.whole == 0 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "byte counts of a file, far inside f32 at these magnitudes"
        )]
        let share = self.arrived as f32 / self.whole as f32;
        Some(share.clamp(0.0, 1.0))
    }

    /// What this transfer is doing, in words rather than in a state name.
    #[must_use]
    pub fn said(&self) -> String {
        match self.state.as_str() {
            "queued" => "waiting its turn".to_owned(),
            "fetching" => match (self.parts > 1, words::size_in_words(Some(self.whole))) {
                (true, Some(whole)) => {
                    format!(
                        "part {} of {} — {whole} in all",
                        self.part.max(1),
                        self.parts
                    )
                }
                (true, None) => format!("part {} of {}", self.part.max(1), self.parts),
                (false, Some(whole)) => format!("arriving — {whole} in all"),
                (false, None) => "arriving".to_owned(),
            },
            "checking" => "reading it back against its digest".to_owned(),
            "paused" => "stopped where it stood — carry on to finish it".to_owned(),
            "done" => "here, and checked".to_owned(),
            "failed" => self
                .why
                .clone()
                .unwrap_or_else(|| "it did not arrive".to_owned()),
            "cancelled" => "given up, and what had arrived swept".to_owned(),
            other => other.to_owned(),
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
    /// How many files this one variant is published in. A model split across three files
    /// is one variant, and saying so keeps three parts from reading as three models.
    pub parts: Option<u32>,
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

/// What the shelf comes to, and what on it is a projector nothing uses.
fn weighed(listed: &[Value]) -> Weights {
    let bytes_of = |held: &Value| {
        held.get("bytes")
            .and_then(Value::as_integer)
            .and_then(|number| u64::try_from(number).ok())
            .unwrap_or(0)
    };
    let path_of = |held: &Value| {
        held.get("path")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned()
    };
    let is_companion = |held: &Value| matches!(held.get("companion"), Some(Value::Bool(true)));
    let beside = |path: &str| {
        std::path::Path::new(path)
            .parent()
            .map(|parent| parent.display().to_string())
            .unwrap_or_default()
    };
    // Which directories hold something that is not a projector. A projector in a
    // directory with a model is the model's; one on its own was left behind.
    let kept: std::collections::BTreeSet<String> = listed
        .iter()
        .filter(|held| !is_companion(held))
        .map(|held| beside(&path_of(held)))
        .collect();
    let mut weights = Weights {
        bytes: 0,
        files: listed.len(),
        orphans: Vec::new(),
    };
    for held in listed {
        let path = path_of(held);
        let bytes = bytes_of(held);
        weights.bytes = weights.bytes.saturating_add(bytes);
        if is_companion(held) && !kept.contains(&beside(&path)) {
            weights.orphans.push(Orphan { path, bytes });
        }
    }
    weights
        .orphans
        .sort_by(|one, two| two.bytes.cmp(&one.bytes).then(one.path.cmp(&two.path)));
    weights
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
    let parts = held
        .get("parts")
        .and_then(Value::as_integer)
        .and_then(|number| u32::try_from(number).ok());

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
        parts,
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

pub fn asked_until_done(
    socket: &Path,
    request: &Request,
    deadline: std::time::Duration,
    mut the_way: impl FnMut(&Value),
) -> Result<Answer, String> {
    let mut connection = UnixStream::connect(socket)
        .map_err(|_| "MCF is not answering on this computer".to_owned())?;
    let _deadline = connection.set_read_timeout(Some(deadline));
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("the request could not be sent: {error}"))?;
    let mut reading = BufReader::new(&connection);
    let mut last: Option<Answer> = None;
    loop {
        let mut line = String::new();
        let read = reading
            .read_line(&mut line)
            .map_err(|error| format!("MCF stopped answering: {error}"))?;
        if read == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        let answer = Answer::read(trimmed).map_err(|failure| failure.to_string())?;
        if !answer.served {
            return Ok(answer);
        }
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            return Ok(answer);
        }
        the_way(&answer.body);
        last = Some(answer);
    }
    last.ok_or_else(|| "MCF answered nothing at all".to_owned())
}

const POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// How long to wait for an answer about the hold.
///
/// Answering it costs the daemon two requests into the engine — its counters and its
/// slots — and the engine answers those on the same threads it decodes on, so an engine
/// that is busy answers slowly. A quarter of a second used to be the whole allowance,
/// which meant the figures stopped moving exactly when there was something to see. This
/// is asked for on [`Watch`]'s thread, so waiting here costs the window no frames.
const WATCH: std::time::Duration = std::time::Duration::from_secs(10);

/// How often to ask what the hold is doing.
const EVERY: std::time::Duration = std::time::Duration::from_secs(1);

/// Asks the daemon what the hold is doing, on its own thread, and leaves the latest
/// answer where the window can pick it up without waiting.
///
/// The figures a hold reports are live, and reading them costs the daemon a round trip
/// into the engine. Doing that on the thread that draws would either hold up the window
/// or have to give up so quickly that a working engine never answers in time.
#[derive(Debug)]
pub struct Watch {
    heard: std::sync::mpsc::Receiver<Result<Answer, String>>,
}

impl Watch {
    #[must_use]
    pub fn over(socket: &Path) -> Self {
        let (send, heard) = std::sync::mpsc::channel();
        let socket = socket.to_path_buf();
        let watching = std::thread::Builder::new()
            .name("mcf-desk-watch".to_owned())
            .spawn(move || {
                loop {
                    let began = std::time::Instant::now();
                    if send
                        .send(ask_within(&socket, &Request::Hosted, WATCH))
                        .is_err()
                    {
                        return;
                    }
                    if let Some(rest) = EVERY.checked_sub(began.elapsed()) {
                        std::thread::sleep(rest);
                    }
                }
            });
        let _started = watching;
        Self { heard }
    }

    /// The newest answer, if one has arrived since this was last asked. Older answers are
    /// dropped: a figure from three seconds ago is not worth drawing over one from now.
    #[must_use]
    pub fn latest(&self) -> Option<Result<Answer, String>> {
        let mut newest = None;
        while let Ok(answer) = self.heard.try_recv() {
            newest = Some(answer);
        }
        newest
    }
}

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
    /// A word this model's template reads, where the template says which words it takes.
    TemplateWord(u8),
    /// Which setting the sweep moves. A dozen dials named at once is the wall of chips
    /// this page was redrawn to get rid of, so they are named one at a time.
    Dial,
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
    ThinkingLevel,
    Architecture,
    Fits,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    Library,
    Downloads,
    Page,
    Template,
    Hub,
    Diagnostics,
    DiagnosticsPage,
    Checks,
    Server,
    Prompt,
    /// The optimize page's rail. Its own region because the rail scrolls apart from the
    /// column of readings beside it.
    Sweep,
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

/// What a notice about the hold is filed under. One subject, so the hold saying it was
/// refused replaces the hold saying it was saved rather than both standing.
pub const ABOUT_HOLD: &str = "hold";
/// What a notice about the queue of transfers is filed under.
pub const ABOUT_TRANSFERS: &str = "transfers";
/// What a notice about the daemon itself is filed under.
pub const ABOUT_DAEMON: &str = "daemon";
/// What a notice about the window's own job is filed under.
pub const ABOUT_WORK: &str = "work";
/// What a notice about what the queue is fetching is filed under.
pub const ABOUT_TRANSFERS_WORK: &str = "transfers:work";

pub const LANGUAGE_NAMES: [&str; 4] = ["python", "javascript", "rust", "go"];

/// One file a removal would take, as the daemon's preview names it.
fn gone_from(held: &Value) -> Gone {
    Gone {
        path: held
            .get("path")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned(),
        bytes: held
            .get("bytes")
            .and_then(Value::as_integer)
            .and_then(|number| u64::try_from(number).ok())
            .unwrap_or(0),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gone {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removing {
    /// Every model this removal is about. One, chosen from a model's own page; or the
    /// several ticked on the downloads page, which is where somebody clearing a disk
    /// works.
    pub models: Vec<String>,
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
    /// The one model this is about, where it is about one.
    #[must_use]
    pub fn only(&self) -> Option<&String> {
        match self.models.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        self.done.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Something this model's template reads that is neither a switch nor a word out of a
    /// set the template names: a line of identity, a count.
    TemplateWord(u8),
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
    ThinkingBudget,
    DraftDepth,
    Temperature,
    TopP,
    TopK,
    ChatTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// One of the switches this model's own chat template reads, by where it stands in
    /// the list the template asks for them in. Held by place rather than by name because
    /// a switch is copied about the window and a name is not; the name is what the choice
    /// is stored under, so it survives the template changing underneath.
    TemplateTakes(u8),
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
    Download {
        reference: String,
        file: String,
    },
    /// Tick or untick a model on the downloads page.
    PickOnDisk(String),
    /// Untick everything.
    ClearPicked,
    /// Ask about removing everything ticked.
    RemovePicked,
    /// Tick every projector with no model beside it.
    PickTheOrphans,
    PauseTransfer(u64),
    ResumeTransfer(u64),
    GiveUpTransfer(u64),
    ForgetTransfers,
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
    ForgetReadings,
    PickRow(mcf_optimize::ledger::At),
    RerunRow(mcf_optimize::ledger::At),
    ForgetRow(mcf_optimize::ledger::At),
    PickNone,
    RerunPicked,
    /// Take up the value a finished sweep landed on: write it into the settings above.
    AdoptBest,
    /// Leave the settings where they are, and stop asking.
    KeepAsIs,
    Takes(usize),
    Sweep,
    /// Stop a sweep where it stands, or tell a stopped one to carry on. Not the same as
    /// stopping it: a paused sweep keeps its place, and the readings it has already taken
    /// stay where they were written.
    PauseSweep,
    Contents(Page),
    Edit(Field, crate::ui::Touched),
    RemoveReason(crate::ui::Touched),
    PurgeToggle,
    DoRemove,
    CancelRemove,
    Switch(Switch),
    Place(usize),
    Rope(usize),
    ThinkingLevel(usize),
    /// A word chosen for one of the template's own parameters: which parameter, and which
    /// of the words it takes — nought being the template's own.
    TemplateWord(u8, usize),
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
    RememberSettings,
    /// Put the chat template back to the one the model's own file carries.
    TemplateAsPublished,
    LastSettings,
    HostIt,
    Build(String),
    StopHosting,
    Close,
    Focus(Caret),
    Ask {
        at: usize,
    },
    Choose(usize),
    /// Open the strip out into what MCF has said, or fold it away again.
    OpenNotices,
    /// Put away everything that is not still happening.
    DismissNotices,
    /// Show a repository's quantizations one by one, or fold them away again.
    OpenOut(String),
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
    /// What was last asked. Kept so the page can show the exchange rather than an answer
    /// with nothing above it; MCF holds one exchange, not a history.
    pub asked: String,
    pub said: String,
    pub queued: std::collections::VecDeque<Card>,
    pub queued_of: usize,
    pub daemon_build: Option<String>,
    pub home: Option<std::path::PathBuf>,
    /// Where the models are kept, which is the disk the downloads page reports on.
    pub models_root: Option<std::path::PathBuf>,
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

    pub hosted: Option<Hosted>,
    pub under_test: Option<UnderTest>,
    pub spent: Spent,
    pub last_hold: Option<LastHold>,
    /// The running token counts, one reading a second, for as long as this hold has been
    /// held. What the graph on the server page is drawn from.
    pub tallies: std::collections::VecDeque<Tally>,
    /// Everything the window has to say in passing, in one place — see [`notice`].
    pub notices: notice::Notices,
    /// Whether the strip is opened out into the list of what has been said.
    pub notices_open: bool,
    pub anatomy: Option<mcf_serve::anatomy::Said>,
    pub no_anatomy: Option<String>,
    pub window: u64,
    pub open: Option<Picker>,
    /// Which repositories are shown opened out in the library, quant by quant. A
    /// repository holding four quantizations is four models on this disk, and each of them
    /// is chosen, held and removed on its own — so each of them has to be reachable.
    pub opened_out: std::collections::BTreeSet<String>,
    pub transfers: Vec<Transfer>,
    /// What the disk the models live on holds and has free, read from the filesystem.
    pub disk: Option<Storage>,
    /// What the weights come to, every file on the shelf counted — companions included,
    /// because they take the same disk.
    pub weights: Weights,
    /// Model paths ticked on the downloads page, for removal.
    pub picked: std::collections::BTreeSet<String>,

    sampler: mcf_tui::machine::Sampler,
    watch: Watch,
}

impl Desk {
    #[must_use]
    pub fn new(socket: std::path::PathBuf) -> Self {
        Self {
            watch: Watch::over(&socket),
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
            asked: String::new(),
            said: String::new(),
            queued: std::collections::VecDeque::new(),
            queued_of: 0,
            daemon_build: None,
            home: None,
            models_root: None,
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
            hosted: None,
            under_test: None,
            spent: Spent::default(),
            last_hold: None,
            tallies: std::collections::VecDeque::new(),
            notices: notice::Notices::new(),
            notices_open: false,
            anatomy: None,
            no_anatomy: None,
            window: 8192,
            open: None,
            opened_out: std::collections::BTreeSet::new(),
            transfers: Vec::new(),
            disk: None,
            weights: Weights::default(),
            picked: std::collections::BTreeSet::new(),
            sampler: mcf_tui::machine::Sampler::new(),
        }
    }

    /// Put what is happening now into the one place that reports it.
    ///
    /// The window used to say this five ways: a word in the side bar, a sentence on the
    /// page, another in the hold's own block, a bar on the downloads page, and the bare
    /// word "thinking". One notice, carried by whatever is doing the work, means the strip
    /// is the answer wherever you happen to be looking.
    pub fn tell_what_is_happening(&mut self) {
        match self.notices.about(ABOUT_DAEMON) {
            _ if self.refusal.is_some() => {
                // A daemon that is not answering is a condition, not an event: it stays
                // said for as long as it is true, and goes the moment it stops being.
                if self.notices.about(ABOUT_DAEMON).is_none() {
                    self.notices.say(
                        notice::Notice::new(
                            notice::Tone::Refused,
                            ABOUT_DAEMON,
                            "MCF is not answering on this computer",
                        )
                        .saying("`mcf serve` starts it"),
                    );
                }
            }
            Some(_) => self.notices.forget(ABOUT_DAEMON),
            None => {}
        }

        let Some(job) = self.doing.job() else {
            self.notices.forget(ABOUT_WORK);
            return;
        };
        if job.finished {
            // What it came to is said by whoever asked for it; the work itself is over.
            self.notices.forget(ABOUT_WORK);
            return;
        }
        let share = job.progress().and_then(|(arrived, whole)| {
            (whole > 0).then(|| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "byte counts of a file, far inside f32 at these magnitudes"
                )]
                let share = arrived as f32 / whole as f32;
                share
            })
        });
        let what = job.what.clone();
        let detail = self
            .loading_line()
            .or_else(|| Some(format!("{} s so far", job.ran())));
        let mut notice = notice::Notice::new(notice::Tone::Working, ABOUT_WORK, what).so_far(share);
        if let Some(detail) = detail {
            notice = notice.saying(detail);
        }
        self.notices.say(notice);
    }

    /// Take whatever the watching thread has heard about the hold. Nothing to hear is not
    /// a failure: it means a second has not gone by yet.
    pub fn hear_the_hold(&mut self) -> bool {
        let Some(answered) = self.watch.latest() else {
            return false;
        };
        self.took_the_hosted(answered);
        true
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
        if !with_control && self.typing_across_lines() {
            let limit = Self::PASTE_LIMIT;
            self.typing().put("\n", limit);
            return;
        }
        self.entered();
    }

    /// Whether what is being typed into is a box with more than one line in it, where the
    /// arrow keys move between lines and return puts a new one in rather than finishing.
    #[must_use]
    pub fn typing_across_lines(&self) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|(field, _)| *field == Field::ChatTemplate)
    }

    pub fn entered(&mut self) {
        if self.typing_into_a_value() {
            self.optimizing.look_at_what_was_typed();
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
                self.notices.refused(ABOUT_HOLD, why.clone());
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
                if let Some(name) = built {
                    self.notices.say(
                        notice::Notice::new(
                            notice::Tone::Refused,
                            format!("engine:{name}"),
                            format!("Could not build {name}"),
                        )
                        .saying(why),
                    );
                }
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

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per thing a person can press, each a call; a table of them in \
                  one place reads better than the same table split by an arbitrary line \
                  count"
    )]
    pub fn act(&mut self, act: Act) {
        match act {
            Act::Go(page) => self.go(page),
            Act::LookUp => self.look_up(),
            Act::Download { reference, file } => self.download(&reference, &file),
            Act::PickOnDisk(ref path) => {
                let held = path.clone();
                if !self.picked.remove(&held) {
                    let _ticked = self.picked.insert(held);
                }
            }
            Act::ClearPicked => self.picked.clear(),
            Act::RemovePicked => self.ask_to_remove_the_picked(),
            Act::PickTheOrphans => {
                for orphan in &self.weights.orphans.clone() {
                    let _ticked = self.picked.insert(orphan.path.clone());
                }
            }
            Act::PauseTransfer(_)
            | Act::ResumeTransfer(_)
            | Act::GiveUpTransfer(_)
            | Act::ForgetTransfers => self.told_about_a_transfer(&act),
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
            | Act::ForgetReadings
            | Act::PickRow(_)
            | Act::RerunRow(_)
            | Act::ForgetRow(_)
            | Act::PickNone
            | Act::RerunPicked
            | Act::Takes(_)
            | Act::Sweep
            | Act::PauseSweep
            | Act::Edit(..)
            | Act::Switch(_)
            | Act::Place(_)
            | Act::Rope(_)
            | Act::ThinkingLevel(_)
            | Act::TemplateWord(..)
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
            Act::OpenNotices => self.notices_open = !self.notices_open,
            Act::DismissNotices => {
                self.notices.dismiss_the_settled();
                if self.notices.is_empty() {
                    self.notices_open = false;
                }
            }
            Act::OpenOut(ref repository) => self.open_out(repository),
            Act::Cycle(at) => self.cycle(at),
            Act::Recommended => self.settings.clone_from(&self.recommended),
            Act::RememberSettings => self.remember_settings(),
            Act::TemplateAsPublished => self.template_as_published_again(),
            Act::AdoptBest | Act::KeepAsIs => self.decide_about_the_best(&act),
            Act::LastSettings => {
                if let Some((last, _)) = &self.last_settings {
                    self.settings = Some(last.clone());
                }
            }
            Act::HostIt => self.host_it(),
            Act::Build(name) => self.build(&name),
            Act::StopHosting => self.stop_hosting(),
            Act::RemoveReason(_) | Act::PurgeToggle | Act::DoRemove | Act::CancelRemove => {
                self.removal(&act);
            }
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
                if let Some((saved, _)) = self.last_settings.as_ref() {
                    self.settings = Some(saved.clone());
                }
                self.declared = answer
                    .body
                    .get("declares")
                    .map(mcf_serve::declared::Declared::from_value);
                self.optimizing.named = self.levels_of_the_model();
                self.placements = answer
                    .body
                    .get("placements")
                    .and_then(Value::as_list)
                    .map(|listed| listed.iter().filter_map(Placement::from_value).collect())
                    .unwrap_or_default();
                self.read_the_ledger();
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
        let answered = ask_within(&self.socket, &Request::Hosted, WATCH);
        self.took_the_hosted(answered);
    }

    /// Read one answer about the hold. Everything the hold is doing arrives in a single
    /// answer — what is held, what is under test, and what the last hold was — so it is
    /// asked for once and read through once.
    ///
    /// It used to be asked for twice, once for each half that was wanted. The daemon works
    /// out a rate as the difference between the counters at one answer and the counters at
    /// the next, so the second ask measured the milliseconds since the first one: no whole
    /// token arrives in that time, and every rate read zero while the model was working.
    #[allow(
        clippy::too_many_lines,
        reason = "one field a figure the answer carries, each named"
    )]
    pub fn took_the_hosted(&mut self, answered: Result<Answer, String>) {
        let answer = match answered {
            Ok(answer) if answer.served => answer,
            _ => {
                self.busy = true;
                return;
            }
        };
        self.under_test = answer
            .body
            .get("under_test")
            .filter(|held| !matches!(held, Value::Null))
            .map(UnderTest::from_value);
        let read = {
            answer
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
                    // `api_key_set`, not `api_key`. The key itself is deliberately kept
                    // off the wire, so reading the key field always found nothing — and
                    // the page said "API key: none (localhost only)" about a hold that was
                    // answering the network with a key set, directly above the network
                    // address it was answering on.
                    api_key: answer
                        .body
                        .get("settings")
                        .and_then(|settings| settings.get("api_key_set"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    open: answer
                        .body
                        .get("settings")
                        .and_then(|settings| settings.get("open"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    network_address: answer
                        .body
                        .get("network_address")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    in_use: answer.body.get("use").map(Use::from_value),
                })
        };
        self.busy = false;
        match (&self.hosted, &read) {
            (Some(was), Some(now)) if was.model == now.model => {}
            // An engine's counters start at nought when it starts, so a count from the
            // hold before this one would draw a cliff that never happened.
            _ => self.tallies.clear(),
        }
        if let Some(tally) = read
            .as_ref()
            .and_then(|hosting| hosting.in_use.as_ref())
            .and_then(Tally::of)
        {
            self.tallies.push_back(tally);
            while self.tallies.len() > TALLIES_KEPT {
                let _oldest = self.tallies.pop_front();
            }
        }
        self.hosted = read;
        self.last_hold = answer.body.get("last").and_then(LastHold::from_value);
    }

    /// What to do with the value a finished sweep landed on. It is a decision either way:
    /// leaving the settings alone is as much an answer as taking the value up, and both put
    /// the question away.
    fn decide_about_the_best(&mut self, act: &Act) {
        if matches!(*act, Act::AdoptBest) {
            self.adopt_the_best();
            return;
        }
        self.optimizing.settled = None;
        self.optimizing.adopted = None;
    }

    /// Write the value a sweep landed on into the settings above it, so that the next hold
    /// runs under what was measured. Nothing is saved to disk and nothing is held: the
    /// Configure tab's own buttons still do that, and this only moves the dial.
    fn adopt_the_best(&mut self) {
        let Some(step) = self.optimizing.settled else {
            return;
        };
        let dial = self.optimizing.sweep.dial;
        let named = self.optimizing.named.clone();
        let Some(settings) = self.settings.as_mut() else {
            self.optimizing.refused = Some("there are no settings to put this into yet".to_owned());
            return;
        };
        let said = dial.said_among(step, &named);
        put_the_dial(dial, step, &named, settings);
        self.optimizing.settled = None;
        self.optimizing.adopted = Some(format!(
            "{} is now {said} — hold the model to run under it, or save it on the Configure \
             tab to have it come back",
            dial.label()
        ));
        self.read_the_ledger();
    }

    pub fn remember_settings(&mut self) {
        self.apply_edit();
        if let Some(why) = &self.edit_refused {
            self.notices.refused(ABOUT_HOLD, why.clone());
            return;
        }
        self.notices.forget(ABOUT_HOLD);
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        let Some(settings) = self.settings.clone() else {
            return;
        };
        let asked = Request::Remember {
            model: held.path.clone(),
            settings: settings.to_request(),
        };
        match ask(&self.socket, &asked) {
            Ok(answer) if answer.served => {
                self.notices.done(
                    ABOUT_HOLD,
                    format!("Saved for {} — these settings come back", held.name),
                );
                self.read_settings();
            }
            Ok(answer) => self
                .notices
                .refused(ABOUT_HOLD, refused_because(&answer.body)),
            Err(why) => self.notices.refused(ABOUT_HOLD, why),
        }
    }

    pub fn host_it(&mut self) {
        self.apply_edit();
        if let Some(why) = &self.edit_refused {
            self.notices.refused(ABOUT_HOLD, why.clone());
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
            self.notices.refused(
                ABOUT_HOLD,
                "Reachable from the network is on, so the hold needs an API key: type one in \
                 the API key field above, then Start server; or turn the switch off to keep \
                 the hold on this computer"
                    .to_owned(),
            );
            return;
        }
        self.notices.forget(ABOUT_HOLD);
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
        self.notices.forget(&format!("engine:{name}"));
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

    /// What the ticked models come to.
    #[must_use]
    pub fn picked_bytes(&self) -> u64 {
        self.picked
            .iter()
            .filter_map(|path| {
                self.models
                    .iter()
                    .find(|held| &held.path == path)
                    .and_then(|held| held.bytes)
                    .or_else(|| {
                        self.weights
                            .orphans
                            .iter()
                            .find(|held| &held.path == path)
                            .map(|held| held.bytes)
                    })
            })
            .fold(0, u64::saturating_add)
    }

    /// Whether the hold would be taken out from under itself.
    #[must_use]
    pub fn picked_the_served(&self) -> Option<String> {
        let hosting = self.hosted.as_ref()?;
        self.picked
            .iter()
            .find(|path| **path == hosting.model)
            .map(|_| hosting.name())
    }

    /// Ask about removing everything ticked on the downloads page.
    ///
    /// The preview is asked of the daemon for each, and what comes back is added up, so
    /// the page says what would actually go rather than what was ticked.
    pub fn ask_to_remove_the_picked(&mut self) {
        let picked: Vec<String> = self.picked.iter().cloned().collect();
        let Some(first) = picked.first().cloned() else {
            return;
        };
        let name = match picked.len() {
            1 => self
                .models
                .iter()
                .find(|held| held.path == first)
                .map_or_else(
                    || {
                        std::path::Path::new(&first).file_stem().map_or_else(
                            || first.clone(),
                            |stem| stem.to_string_lossy().into_owned(),
                        )
                    },
                    |held| held.name.clone(),
                ),
            many => format!("{many} models"),
        };
        let mut files: Vec<Gone> = Vec::new();
        let mut bytes = 0_u64;
        let mut reversible = true;
        let mut shelved_in = String::new();
        let mut refused = None;
        for model in &picked {
            match ask(
                &self.socket,
                &Request::Removal {
                    model: model.clone(),
                },
            ) {
                Ok(answer) if answer.served => {
                    if let Some(Value::List(listed)) = answer.body.get("files") {
                        files.extend(listed.iter().map(gone_from));
                    }
                    bytes = bytes.saturating_add(
                        answer
                            .body
                            .get("bytes")
                            .and_then(Value::as_integer)
                            .and_then(|held| u64::try_from(held).ok())
                            .unwrap_or(0),
                    );
                    reversible &= matches!(answer.body.get("reversible"), Some(Value::Bool(true)));
                    if shelved_in.is_empty()
                        && let Some(said) = answer.body.get("shelf").and_then(Value::as_text)
                    {
                        shelved_in.push_str(said);
                    }
                }
                Ok(answer) => refused = Some(refused_because(&answer.body)),
                Err(why) => refused = Some(why),
            }
        }
        self.removing = Some(Removing {
            models: picked,
            name,
            files,
            bytes: (bytes > 0).then_some(bytes),
            reversible,
            shelf: shelved_in,
            reason: crate::typing::Typing::of(String::new()),
            purge: false,
            refused,
            done: None,
        });
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

    /// Take each model, one at a time, and say what the lot of them came to.
    ///
    /// One at a time because a removal is authorized, recorded and answered for one model
    /// at a time — the record has an entry per artifact, not one per gesture. What went is
    /// reported whether or not the rest did: a removal that stopped part way through took
    /// what it took, and saying otherwise would leave somebody hunting for files that are
    /// already gone.
    fn remove_each(&self, models: &[String], reason: &str, purge: bool) -> Result<Answer, String> {
        let mut taken = 0_u64;
        let mut purged = false;
        let mut went = 0_usize;
        let mut refused: Option<String> = None;
        for model in models {
            match ask(
                &self.socket,
                &Request::Remove {
                    model: model.clone(),
                    reason: reason.to_owned(),
                    purge,
                },
            ) {
                Ok(answer) if answer.served => {
                    taken = taken.saturating_add(
                        answer
                            .body
                            .get("bytes")
                            .and_then(Value::as_integer)
                            .and_then(|held| u64::try_from(held).ok())
                            .unwrap_or(0),
                    );
                    purged |= answer.body.get("purged_bytes").is_some();
                    went += 1;
                }
                Ok(answer) => {
                    refused = Some(refused_because(&answer.body));
                    break;
                }
                Err(why) => {
                    refused = Some(why);
                    break;
                }
            }
        }
        match refused {
            Some(why) if went == 0 => Err(why),
            Some(why) => Err(format!("{went} of {} removed, then: {why}", models.len())),
            None => {
                let figure = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
                Ok(Answer::served(Value::map([
                    ("bytes", figure(taken)),
                    (
                        "purged_bytes",
                        if purged { figure(taken) } else { Value::Null },
                    ),
                ])))
            }
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
        let answered = self.remove_each(&removing.models.clone(), &reason, removing.purge);
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
        let said = answered.ok().map(|answer| {
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
        if let Some(said) = said {
            self.notices.done(ABOUT_HOLD, said);
        }
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

    /// Open a repository out into its quantizations, or fold it away again.
    ///
    /// A repository row stands for as many models as it has quantizations, and every one of
    /// them is a file on this disk with its own size, its own settings and its own place in
    /// the record. Folded up, only one of them could ever be chosen — and so only one of
    /// them could ever be removed.
    fn open_out(&mut self, repository: &str) {
        if !self.opened_out.remove(repository) {
            let _opened = self.opened_out.insert(repository.to_owned());
        }
    }

    /// Whether this repository is opened out. One with a single quantization is always
    /// opened out in effect: there is nothing to open.
    #[must_use]
    pub fn is_opened_out(&self, repository: Option<&str>) -> bool {
        repository.is_some_and(|held| self.opened_out.contains(held))
    }

    /// The other quantizations of the same repository that are on this disk, and the
    /// repository they belong to.
    ///
    /// Removing one file removes that file. Saying how many of its siblings stay put is
    /// what tells somebody they are removing a quantization and not a model.
    #[must_use]
    pub fn others_of_the_repository(&self, path: &str) -> Option<(usize, String)> {
        let held = self.models.iter().find(|held| held.path == path)?;
        let repository = held.repository.clone()?;
        let others = self
            .models
            .iter()
            .filter(|beside| beside.path != path)
            .filter(|beside| beside.repository.as_deref() == Some(repository.as_str()))
            .count();
        (others > 0).then_some((others, repository))
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

    /// Ask for a file, and go straight back to whatever else was happening.
    ///
    /// The queue belongs to the daemon, so asking for one takes nothing away from the
    /// window: several files can be on their way at once, another can be asked for while
    /// they are, and closing the window does not stop any of them. It used to be carried
    /// on the window's one job slot, which meant one at a time and only for as long as the
    /// window stayed open.
    pub fn download(&mut self, reference: &str, file: &str) {
        let asked = Request::Queue {
            reference: reference.to_owned(),
            file: file.to_owned(),
            from: None,
        };
        match ask(&self.socket, &asked) {
            Ok(answer) if answer.served => {
                self.notices.forget(ABOUT_TRANSFERS);
                self.take_the_transfers(&answer.body);
                self.settle_download();
            }
            Ok(answer) => self
                .notices
                .refused(ABOUT_TRANSFERS, refused_because(&answer.body)),
            Err(why) => self.notices.refused(ABOUT_TRANSFERS, why),
        }
    }

    /// One of the four things that can be asked of the queue, turned into the request that
    /// asks it. Kept together so that the queue's four buttons read as one thing.
    fn told_about_a_transfer(&mut self, act: &Act) {
        let asked = match *act {
            Act::PauseTransfer(id) => Request::PauseTransfer { id },
            Act::ResumeTransfer(id) => Request::ResumeTransfer { id },
            Act::GiveUpTransfer(id) => Request::GiveUpTransfer { id },
            _ => Request::ForgetTransfers,
        };
        self.about_a_transfer(&asked);
    }

    fn about_a_transfer(&mut self, asked: &Request) {
        match ask(&self.socket, asked) {
            Ok(answer) if answer.served => {
                self.notices.forget(ABOUT_TRANSFERS);
                self.take_the_transfers(&answer.body);
            }
            Ok(answer) => self
                .notices
                .refused(ABOUT_TRANSFERS, refused_because(&answer.body)),
            Err(why) => self.notices.refused(ABOUT_TRANSFERS, why),
        }
    }

    fn take_the_transfers(&mut self, body: &Value) {
        if let Some(rows) = body.get("transfers").and_then(Value::as_list) {
            self.transfers = rows.iter().map(Transfer::from_value).collect();
        }
    }

    /// Read the queue off the daemon. Cheap — the daemon answers it out of its own state
    /// without reaching for anything — so it is read on the same tick as everything else.
    pub fn read_transfers(&mut self) {
        match ask_within(&self.socket, &Request::Transfers, POLL) {
            Ok(answer) if answer.served => {
                self.notices.forget(ABOUT_TRANSFERS);
                self.take_the_transfers(&answer.body);
            }
            Ok(answer) => self
                .notices
                .refused(ABOUT_TRANSFERS, refused_because(&answer.body)),
            Err(_) => {}
        }
    }

    /// The transfer of the file this page is waiting on, if it is in the queue. What the
    /// models page shows about a file being fetched comes from the same queue the downloads
    /// page shows, so the two never disagree.
    #[must_use]
    pub fn transfer_of_the_pending(&self) -> Option<&Transfer> {
        let pending = self.pending.as_ref()?;
        self.transfers
            .iter()
            .find(|held| held.reference == pending.repository && held.file == pending.file)
    }

    /// What the queue is doing, said in the one place that says what is happening.
    ///
    /// One notice for the whole queue rather than one each: three files arriving is one
    /// thing happening, and a strip that said it three times would be a strip nobody read.
    pub fn tell_what_is_arriving(&mut self) {
        let under_way: Vec<&Transfer> = self
            .transfers
            .iter()
            .filter(|held| held.under_way())
            .collect();
        let Some(first) = under_way.first() else {
            self.notices.forget(ABOUT_TRANSFERS_WORK);
            return;
        };
        let (arrived, whole) = under_way
            .iter()
            .fold((0_u64, 0_u64), |(arrived, whole), held| {
                (
                    arrived.saturating_add(held.arrived),
                    whole.saturating_add(held.whole),
                )
            });
        let what = match under_way.len() {
            1 => format!("Getting {}", first.name()),
            many => format!("{many} files arriving"),
        };
        let share = (whole > 0).then(|| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "byte counts of files, far inside f32 at these magnitudes"
            )]
            let share = arrived as f32 / whole as f32;
            share
        });
        let waiting = self
            .transfers
            .iter()
            .filter(|held| held.state == "queued")
            .count();
        let detail = match (words::size_in_words(Some(whole)), waiting) {
            (Some(whole), 0) => format!("of {whole}"),
            (Some(whole), 1) => format!("of {whole} · 1 more queued"),
            (Some(whole), many) => format!("of {whole} · {many} more queued"),
            (None, _) => first.said(),
        };
        self.notices.say(
            notice::Notice::new(notice::Tone::Working, ABOUT_TRANSFERS_WORK, what)
                .so_far(share)
                .saying(detail),
        );
    }

    /// Whether anything is still on its way, which is what the side bar shows a mark for.
    #[must_use]
    pub fn transfers_under_way(&self) -> usize {
        self.transfers
            .iter()
            .filter(|transfer| transfer.under_way())
            .count()
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
        // Nothing is asked about what else is busy. A transfer is queued in the daemon and
        // competes with nothing, so refusing one because the window was doing something
        // else refused it for no reason.
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

    /// The thinking levels a sweep of this model can run. Off comes first where thinking can
    /// be turned off at all, which is wherever the template opens a thinking section: the
    /// engine cuts the section short itself and does not need the template's help. The
    /// template's own word for no thinking, where it has one, is dropped in favour of it —
    /// that word only takes the level away and leaves the template's default behind.
    #[must_use]
    pub fn levels_of_the_model(&self) -> Vec<String> {
        let Some(thinking) = self.declared.as_ref().map(|held| &held.thinking) else {
            return Vec::new();
        };
        let named = mcf_optimize::dial::Dial::OFF;
        let mut levels: Vec<String> = thinking
            .levels
            .iter()
            .filter(|held| held.as_str() != "none" && held.as_str() != named)
            .cloned()
            .collect();
        // Off wherever thinking can actually be stopped, which is wherever the template
        // marks a section the engine can cut short or reads a switch the engine can set.
        // It used to be offered only alongside a named level, so a model whose template
        // says nothing about effort — most of them — was reported as having no thinking
        // to control at all, though it could be switched off perfectly well.
        if thinking.section || thinking.switch {
            levels.insert(0, named.to_owned());
        }
        levels
    }

    #[must_use]
    pub fn dials_offered(&self) -> Vec<mcf_optimize::dial::Dial> {
        mcf_optimize::dial::Dial::ALL.into_iter().collect()
    }

    /// Why this dial would change nothing for the model in front of us, if it would not.
    #[must_use]
    pub fn why_the_dial_does_nothing(&self, dial: mcf_optimize::dial::Dial) -> Option<String> {
        let thinking = self.declared.as_ref().map(|held| &held.thinking)?;
        match dial {
            mcf_optimize::dial::Dial::ThinkingLevel
                if thinking.levels.is_empty() && !thinking.section && !thinking.switch =>
            {
                Some(
                    "this model's chat template neither names a thinking level nor marks a \
                     thinking section, so there is nothing here to ask for or to cut short"
                        .to_owned(),
                )
            }
            mcf_optimize::dial::Dial::ThinkingBudget if !thinking.section => Some(
                "this model's chat template marks no thinking section, so the engine has \
                 nothing to count and nothing to cut off — a budget would be written down and \
                 never applied"
                    .to_owned(),
            ),
            mcf_optimize::dial::Dial::DraftDepth
                if self
                    .declared
                    .as_ref()
                    .is_none_or(|held| held.draft_head.is_none()) =>
            {
                Some(
                    "this model file carries no draft head, so there is nothing to draft ahead \
                     with"
                        .to_owned(),
                )
            }
            _ => None,
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
            experts: settings.spread.experts.said(),
            threads_for_a_prompt: settings.threads_batch,
            draft_head: settings.started.draft_head,
            draft_depth: settings.started.drafted,
            thinking_budget: settings.started.thinking,
            thinking_level: None,
            temperature: None,
            top_p: None,
            top_k: None,
            corpus: mcf_optimize::ledger::CORPUS,
            timed: if self.optimizing.measure.needs_the_answers_run() {
                mcf_optimize::ledger::MARKED
            } else {
                mcf_optimize::ledger::TIMED
            },
        })
    }

    pub fn read_the_ledger(&mut self) {
        self.optimizing.named = self.levels_of_the_model();
        self.optimizing.rows.clear();
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
        for row in &against {
            report.record(row.reading.clone());
        }
        self.optimizing.rows = against.into_iter().cloned().collect();
        self.optimizing.rows.sort_by_key(|row| {
            (
                match row.at.step {
                    mcf_optimize::dial::Step::Whole(held)
                    | mcf_optimize::dial::Step::Thousandths(held) => held,
                },
                row.at.set,
                row.at.repeat,
            )
        });
        self.optimizing.report = report;
    }

    fn choosing_values(&mut self, act: &Act) {
        match *act {
            Act::SweepWay(at) => self.optimizing.pick_way(at),
            Act::SweepMeasure(at) => self.optimizing.pick_measure(at),
            Act::CustomValue(touched) => {
                self.optimizing.touch_the_custom(touched);
                self.optimizing.look_at_what_was_typed();
            }
            Act::ForgetReadings => self.forget_readings(),
            Act::PickRow(at) => self.pick_row(at),
            Act::RerunRow(at) => self.rerun_one(at),
            Act::ForgetRow(at) => self.forget_one(at),
            Act::PickNone => self.optimizing.picked.clear(),
            Act::RerunPicked => self.rerun_picked(),
            _ => {}
        }
    }

    fn pick_row(&mut self, at: mcf_optimize::ledger::At) {
        self.optimizing.refused = None;
        if let Some(found) = self.optimizing.picked.iter().position(|held| *held == at) {
            let _dropped = self.optimizing.picked.remove(found);
            return;
        }
        self.optimizing.picked.push(at);
    }

    fn rerun_one(&mut self, at: mcf_optimize::ledger::At) {
        self.optimizing.refused = None;
        if self.a_sweep_is_going() {
            return;
        }
        if let Some(why) = self.forget_these(&[at]) {
            self.optimizing.refused = Some(why);
            return;
        }
        self.read_the_ledger();
        self.sweeping_over(Some(&[at]));
    }

    #[must_use]
    fn a_sweep_is_going(&self) -> bool {
        if self.optimizing.run.is_none() && !self.optimizing.running {
            return false;
        }
        true
    }

    fn forget_one(&mut self, at: mcf_optimize::ledger::At) {
        self.optimizing.refused = None;
        if self.a_sweep_is_going() {
            self.optimizing.refused = Some(
                "a sweep is running — stop it before changing what is already written down"
                    .to_owned(),
            );
            return;
        }
        if let Some(why) = self.forget_these(&[at]) {
            self.optimizing.refused = Some(why);
            return;
        }
        if let Some(found) = self.optimizing.picked.iter().position(|held| *held == at) {
            let _dropped = self.optimizing.picked.remove(found);
        }
        self.read_the_ledger();
    }

    fn rerun_picked(&mut self) {
        self.optimizing.refused = None;
        if self.optimizing.picked.is_empty() || self.a_sweep_is_going() {
            return;
        }
        let picked = self.optimizing.picked.clone();
        self.optimizing.picked.clear();
        if let Some(why) = self.forget_these(&picked) {
            self.optimizing.refused = Some(why);
            return;
        }
        self.read_the_ledger();
        self.sweeping_over(Some(&picked));
    }

    fn forget_these(&mut self, picked: &[mcf_optimize::ledger::At]) -> Option<String> {
        let under = self.base_for_a_sweep()?;
        let path = self.ledger_path()?;
        let mut ledger = match mcf_optimize::ledger::Ledger::open(&path) {
            Ok(ledger) => ledger,
            Err(failure) => return Some(failure.to_string()),
        };
        match ledger.forget(&under, picked) {
            Ok(_gone) => None,
            Err(failure) => Some(failure.to_string()),
        }
    }

    fn start_or_stop_sweeping(&mut self) {
        self.sweeping_over(None);
    }

    /// Pause a running sweep, or tell a paused one to carry on. A pause takes effect after
    /// the trial under way finishes, because a reading cut in half is not a reading.
    fn pause_or_carry_on_sweeping(&mut self) {
        let Some(run) = self.optimizing.run.as_ref() else {
            return;
        };
        if run.asked_to_wait() {
            run.resume();
        } else {
            run.pause();
        }
    }

    fn sweeping_over(&mut self, exactly: Option<&[mcf_optimize::ledger::At]>) {
        if let Some(run) = self.optimizing.run.as_ref() {
            // A second press while it is already stopping is not a request to start again:
            // the worker is still winding down and `running` must stay true until it has,
            // or the button offers to carry on a sweep that is in the act of being thrown
            // away.
            if !run.stopping() {
                run.stop();
            }
            return;
        }
        self.optimizing.refused = None;
        if let Some(why) = self.why_the_dial_does_nothing(self.optimizing.sweep.dial) {
            self.optimizing.refused = Some(why);
            return;
        }
        self.optimizing.named = self.levels_of_the_model();
        self.optimizing.only_the_levels_the_model_names();
        match self.optimizing.what_was_typed() {
            Ok(Some(step)) if !self.optimizing.sweep.steps.contains(&step) => {
                self.optimizing.sweep.steps.push(step);
                self.optimizing.sweep.steps.sort_by_key(|held| match *held {
                    mcf_optimize::dial::Step::Whole(value)
                    | mcf_optimize::dial::Step::Thousandths(value) => value,
                });
            }
            Ok(_) => {}
            Err(why) => {
                self.optimizing.custom_refused = Some(why.clone());
                self.optimizing.refused = Some(why);
                return;
            }
        }
        let by_hand = self.optimizing.way == mcf_optimize::hunt::Way::ByHand;
        if exactly.is_none() && by_hand && self.optimizing.sweep.steps.is_empty() {
            self.optimizing.refused =
                Some("choose at least one value, or let the automatic search pick them".to_owned());
            return;
        }
        if exactly.is_none() && self.optimizing.sweep.sets.is_empty() {
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
        let course = match exactly {
            Some(spots) => {
                mcf_optimize::course::Course::over(under.clone(), spots, self.optimizing.measure)
            }
            None => mcf_optimize::course::Course::laid_out(
                under.clone(),
                self.optimizing.way,
                self.optimizing.sweep.dial,
                &self.optimizing.sweep.steps,
                &self.optimizing.sweep.sets,
                self.optimizing.sweep.repeats,
                self.optimizing.measure,
            ),
        };
        self.begin(under, course, ledger, model, settings, &path);
    }

    fn begin(
        &mut self,
        under: mcf_optimize::ledger::Under,
        course: mcf_optimize::course::Course,
        ledger: mcf_optimize::ledger::Ledger,
        model: String,
        settings: mcf_serve::hosting::Hosting,
        path: &std::path::Path,
    ) {
        let orders = mcf_optimize::running::Orders {
            endpoint: mcf_optimize::trial::Endpoint {
                port: 0,
                key: None,
                patience: std::time::Duration::from_secs(7200),
            },
            under,
            dial: self.optimizing.sweep.dial,
            ceiling: None,
            named: self.levels_of_the_model(),
            switch: self
                .declared
                .as_ref()
                .is_some_and(|held| held.thinking.switch),
            mark: self.optimizing.measure.needs_the_answers_run(),
            ready_within: HOLDING_PATIENCE,
            room: path
                .parent()
                .map_or_else(std::env::temp_dir, |beside| beside.join("marking")),
        };
        let dial = self.optimizing.sweep.dial;
        let socket = self.socket.clone();
        let hosting = std::boxed::Box::new(move |step, along: &mut dyn FnMut(String)| {
            hold_it_at(&socket, &model, &settings, dial, step, along)
        });
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
        if let Some(why) = run.refused.clone() {
            self.optimizing.refused = Some(why);
        }
        let ended = run.finished;
        if ended {
            self.optimizing.last_said = Some(run.said());
            self.optimizing.running = false;
            self.optimizing.run = None;
        }
        self.read_the_ledger();
        if ended {
            self.optimizing.settled = self
                .optimizing
                .report
                .best_by(self.optimizing.measure)
                .map(|best| best.step);
            self.optimizing.adopted = None;
        }
        true
    }

    fn dialling(&mut self, act: &Act) -> bool {
        match *act {
            Act::Dial(at) => {
                self.open = None;
                let offered = self.dials_offered();
                let levels = self.levels_of_the_model();
                self.optimizing.pick_dial_among(at, &offered);
                self.optimizing.rank_as_the_setting_asks();
                self.optimizing.named.clone_from(&levels);
                self.optimizing.only_the_levels_the_model_names();
                if self.optimizing.sweep.dial.is_named_by_the_model() {
                    self.optimizing.way = mcf_optimize::hunt::Way::ByHand;
                    self.optimizing.sweep.steps = (0..levels.len())
                        .filter_map(|at| u32::try_from(at).ok())
                        .map(mcf_optimize::dial::Step::Whole)
                        .collect();
                }
                self.read_the_ledger();
            }
            Act::SweepValue(at) => self.optimizing.toggle_value(at),
            Act::SweepWay(_)
            | Act::SweepMeasure(_)
            | Act::CustomValue(_)
            | Act::ForgetReadings
            | Act::PickRow(_)
            | Act::RerunRow(_)
            | Act::ForgetRow(_)
            | Act::PickNone
            | Act::RerunPicked => self.choosing_values(act),
            Act::Takes(times) => self.optimizing.take_each(times),
            Act::Sweep => self.start_or_stop_sweeping(),
            Act::PauseSweep => self.pause_or_carry_on_sweeping(),
            _ => return false,
        }
        true
    }

    fn open_the_tab(&mut self, tab: Tab) {
        self.apply_edit();
        self.tab = tab;
        self.open = None;
        if tab == Tab::Contents && self.anatomy.is_none() {
            self.read_anatomy();
        }
        if tab == Tab::Optimize {
            self.read_the_ledger();
        }
    }

    /// Choosing a thinking level by hand, which goes through exactly the mapping a sweep
    /// uses. Off is MCF's own word and reaches no template: one that checks its vocabulary
    /// refuses the whole request over it, which is how every trial of every setting on a
    /// hold made this way came back in seven milliseconds with nothing in it.
    /// Choose one of the words a template says it takes. Nought is the template's own,
    /// which is sending nothing at all.
    fn pick_a_word(&mut self, which: u8, at: usize) {
        self.open = None;
        let takes = self.template_takes();
        let Some(held) = takes.get(usize::from(which)).cloned() else {
            return;
        };
        let words = match &held.takes {
            mcf_serve::parameters::Takes::Word { allowed, .. } => allowed.clone(),
            _ => return,
        };
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        settings
            .started
            .template_taken
            .retain(|(named, _)| *named != held.name);
        let Some(wanted) = at.checked_sub(1).and_then(|at| words.get(at)) else {
            return;
        };
        settings
            .started
            .template_taken
            .push((held.name, mcf_record::json::Value::text(wanted.clone())));
    }

    fn pick_a_level(&mut self, at: usize) {
        self.open = None;
        let named = self.levels_of_the_model();
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        let Some(wanted) = at.checked_sub(1) else {
            settings.started.effort = None;
            return;
        };
        let Some(step) = u32::try_from(wanted)
            .ok()
            .map(mcf_optimize::dial::Step::Whole)
        else {
            return;
        };
        put_the_dial(
            mcf_optimize::dial::Dial::ThinkingLevel,
            step,
            &named,
            settings,
        );
    }

    fn configure(&mut self, act: &Act) {
        if self.dialling(act) {
            return;
        }
        match *act {
            Act::Tab(tab) => self.open_the_tab(tab),
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
            Act::ThinkingLevel(at) => self.pick_a_level(at),
            Act::TemplateWord(which, at) => self.pick_a_word(which, at),
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
            Field::TemplateWord(at) => self
                .template_takes()
                .get(usize::from(at))
                .and_then(|held| self.what_is_taken(&held.name))
                .map_or_else(String::new, said_plainly),
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
            Field::ThinkingBudget => settings
                .started
                .thinking
                .map_or_else(String::new, |held| held.to_string()),
            Field::DraftDepth => settings
                .started
                .drafted
                .map_or_else(String::new, |held| held.to_string()),
            Field::Temperature => settings
                .started
                .temperature
                .map_or_else(String::new, |held| held.to_string()),
            Field::TopP => settings
                .started
                .top_p
                .map_or_else(String::new, |held| held.to_string()),
            Field::TopK => settings
                .started
                .top_k
                .map_or_else(String::new, |held| held.to_string()),
            Field::ChatTemplate => self.template_now(),
        };
        self.editing = Some((field, crate::typing::Typing::of(now)));
        self.touch(touched);
        self.edit_refused = None;
        self.caret = Caret::Setting;
    }

    /// The template this model is held under: the one somebody has edited, or the one packed
    /// into the file, which is what it was published to be addressed with.
    #[must_use]
    pub fn template_now(&self) -> String {
        self.settings
            .as_ref()
            .and_then(|held| held.template.clone())
            .or_else(|| {
                self.declared
                    .as_ref()
                    .and_then(|held| held.template.clone())
            })
            .unwrap_or_default()
    }

    /// Put the template back to the one the file carries, and stop editing: what is in the
    /// box is about to change under the caret, and a caret left where it was would be
    /// somewhere else in a different template.
    fn template_as_published_again(&mut self) {
        self.editing = None;
        if let Some(settings) = self.settings.as_mut() {
            settings.template = None;
        }
    }

    /// Whether the model is held under the template its own file carries.
    #[must_use]
    pub fn template_is_the_model_s_own(&self) -> bool {
        self.settings
            .as_ref()
            .is_none_or(|held| held.template.is_none())
    }

    /// The template the file came with, before anybody touched it.
    #[must_use]
    pub fn template_as_published(&self) -> String {
        self.declared
            .as_ref()
            .and_then(|held| held.template.clone())
            .unwrap_or_default()
    }

    /// Keep an edited template, or — where it has been put back to what the file says — keep
    /// nothing, so that the model goes on being held under its own and a template MCF never
    /// has to write out is never written out.
    fn take_the_template(&mut self, said: &str) {
        let published = self.template_as_published();
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        settings.template = if said.trim().is_empty() || said == published {
            None
        } else {
            Some(said.to_owned())
        };
        self.edit_refused = None;
    }

    #[allow(clippy::too_many_lines, reason = "one arm a field, each named")]
    fn sampled(
        settings: &mut mcf_serve::hosting::Hosting,
        field: Field,
        typed: &str,
    ) -> Option<String> {
        if typed.is_empty() {
            match field {
                Field::Temperature => settings.started.temperature = None,
                Field::TopP => settings.started.top_p = None,
                Field::TopK => settings.started.top_k = None,
                _ => {}
            }
            return None;
        }
        if field == Field::TopK {
            return match typed.parse::<u32>() {
                Ok(held) => {
                    settings.started.top_k = Some(held);
                    None
                }
                Err(_) => Some(format!("top-k wants a whole number, not {typed:?}")),
            };
        }
        let Ok(held) = typed.parse::<mcf_core::configuration::Thousandths>() else {
            return Some(format!(
                "{} wants a number like 0.2, not {typed:?}",
                if field == Field::TopP {
                    "top-p"
                } else {
                    "the temperature"
                }
            ));
        };
        match field {
            Field::TopP if held.0 > 1000 => {
                Some("top-p is a share of the whole, so it never goes above 1".to_owned())
            }
            Field::TopP => {
                settings.started.top_p = Some(held);
                None
            }
            _ => {
                settings.started.temperature = Some(held);
                None
            }
        }
    }

    pub fn apply_edit(&mut self) {
        let Some((field, typed)) = self.editing.take() else {
            return;
        };
        if field == Field::ChatTemplate {
            self.take_the_template(typed.said());
            return;
        }
        let _listed = typed.trim().to_owned();
        let typed = typed.trim().replace([',', '_'], "");
        let not_a_number = |what: &str| Some(format!("{what} wants a whole number, not {typed:?}"));
        let takes = self.template_takes();
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        self.edit_refused = Self::a_number_for(settings, &takes, field, &typed, &not_a_number);
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one arm per field a person can type into, each refusing in its own words"
    )]
    fn a_number_for(
        settings: &mut mcf_serve::hosting::Hosting,
        takes: &[mcf_serve::parameters::Parameter],
        field: Field,
        typed: &str,
        not_a_number: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        match field {
            // Emptied is the template left alone, which is what the line underneath the
            // control says it does on its own.
            Field::TemplateWord(at) => {
                let held = takes.get(usize::from(at)).cloned()?;
                settings
                    .started
                    .template_taken
                    .retain(|(named, _)| *named != held.name);
                let typed = typed.trim();
                if typed.is_empty() {
                    return None;
                }
                let sent = match held.takes {
                    mcf_serve::parameters::Takes::Count { .. } => match typed.parse::<i64>() {
                        Ok(whole) => mcf_record::json::Value::Integer(whole),
                        Err(_) => return not_a_number(&held.name),
                    },
                    _ => mcf_record::json::Value::text(typed.to_owned()),
                };
                settings.started.template_taken.push((held.name, sent));
                None
            }
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
                settings.spread.devices = (!typed.is_empty()).then(|| typed.to_owned());
                None
            }
            Field::OverrideTensors => {
                settings.spread.override_tensors = (!typed.is_empty()).then(|| typed.to_owned());
                None
            }
            Field::Alias => {
                settings.alias = (!typed.is_empty()).then(|| typed.to_owned());
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
                settings.api_key = (!typed.is_empty()).then(|| typed.to_owned());
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
            Field::ThinkingBudget => {
                if typed.is_empty() {
                    settings.started.thinking = None;
                    None
                } else {
                    match typed.parse::<u32>() {
                        Ok(budget) => {
                            settings.started.thinking = Some(budget);
                            None
                        }
                        Err(_) => not_a_number("the thinking budget"),
                    }
                }
            }
            Field::Temperature | Field::TopP | Field::TopK => Self::sampled(settings, field, typed),
            Field::ChatTemplate => None,
            Field::DraftDepth => {
                if typed.is_empty() {
                    settings.started.drafted = None;
                    None
                } else {
                    match typed.parse::<u32>() {
                        Ok(depth) if depth >= 1 => {
                            settings.started.drafted = Some(depth);
                            None
                        }
                        Ok(_) => Some(
                            "a draft depth of nothing is the draft head turned off, which is \
                             the switch above"
                                .to_owned(),
                        ),
                        Err(_) => not_a_number("the draft depth"),
                    }
                }
            }
        }
    }

    /// What this model's own chat template will read, in the order it asks for them.
    ///
    /// Read off the template the model carries rather than from anything MCF knows about
    /// the family: the names differ between families, and the whole point of reading them
    /// is to reach the ones nobody has written down.
    #[must_use]
    pub fn template_takes(&self) -> Vec<mcf_serve::parameters::Parameter> {
        self.declared
            .as_ref()
            .and_then(|held| held.template.as_deref())
            .map(mcf_serve::parameters::in_template)
            .unwrap_or_default()
    }

    /// What is being sent for one of them, where anything is.
    #[must_use]
    pub fn what_is_taken(&self, name: &str) -> Option<&mcf_record::json::Value> {
        self.settings
            .as_ref()?
            .started
            .template_taken
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, held)| held)
    }

    /// Turn one of the template's own switches the other way.
    ///
    /// The first turn sends the opposite of whatever the template does on its own, because
    /// that is the only reason to touch it. Turning it back to what the template already
    /// does stops sending it at all, so leaving a switch alone and setting it to its own
    /// default are the same thing, which is what the template's own word underneath says.
    fn flip_what_the_template_takes(&mut self, at: u8) {
        let takes = self.template_takes();
        let Some(held) = usize::from(at).checked_sub(0).and_then(|at| takes.get(at)) else {
            return;
        };
        let name = held.name.clone();
        let on_its_own = match held.takes {
            mcf_serve::parameters::Takes::Switch { on_unless_asked } => {
                on_unless_asked.unwrap_or(false)
            }
            _ => return,
        };
        let now = self
            .what_is_taken(&name)
            .and_then(|held| match held {
                mcf_record::json::Value::Bool(on) => Some(*on),
                _ => None,
            })
            .unwrap_or(on_its_own);
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        settings
            .started
            .template_taken
            .retain(|(held, _)| *held != name);
        if now == on_its_own {
            settings
                .started
                .template_taken
                .push((name, mcf_record::json::Value::Bool(!now)));
        }
    }

    pub fn flip(&mut self, switch: Switch) {
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        match switch {
            Switch::TemplateTakes(at) => self.flip_what_the_template_takes(at),
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
        // Kept, and the box emptied. The question used to be left sitting in the box, so
        // it had to be cleared by hand before the next one — and with the question now
        // shown above the answer it would have been on the page twice.
        self.asked.clone_from(&question);
        self.typed = crate::typing::Typing::default();
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
                let listed = answer
                    .body
                    .get("models")
                    .and_then(Value::as_list)
                    .map(<[Value]>::to_vec)
                    .unwrap_or_default();
                // Weighed before the companions are filtered out. A projector takes the
                // same disk as anything else, and a figure for what the shelf comes to
                // that left them out would be short by twelve gigabytes here.
                self.weights = weighed(&listed);
                let mut read: Vec<Model> = listed
                    .iter()
                    .filter(|entry| !matches!(entry.get("companion"), Some(Value::Bool(true))))
                    .map(model_from)
                    .collect();
                read.sort_by(|one, two| one.name.cmp(&two.name));
                self.models = read;
                // Nothing stays ticked that is no longer there to remove.
                let here: std::collections::BTreeSet<String> =
                    self.models.iter().map(|held| held.path.clone()).collect();
                self.picked.retain(|path| here.contains(path));
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

    #[must_use]
    pub fn busy_elsewhere(&self) -> bool {
        if matches!(self.doing, Doing::Hosting(_) | Doing::Provisioning(_)) {
            return true;
        }
        self.optimizing.run.is_some()
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

    /// Read what the disk the models live on holds.
    ///
    /// Asked of the filesystem rather than of the daemon: the window and the daemon share
    /// a machine — the control socket is a Unix socket — so the figure is the same either
    /// way, and this one needs no round trip.
    pub fn read_the_disk(&mut self) {
        let Some(root) = self.models_root.as_ref() else {
            return;
        };
        self.disk = match mcf_core::hardware::space_on(root) {
            mcf_core::attested::Attested::Known(space) => Some(Storage {
                total: space.total.0,
                free: space.available.0,
            }),
            mcf_core::attested::Attested::Unknown => None,
        };
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
            self.models_root = answer
                .body
                .get("models")
                .and_then(Value::as_text)
                .map(std::path::PathBuf::from);
            self.read_the_disk();
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
                    key if matches!(key, sdl::KEY_UP | sdl::KEY_DOWN)
                        && desk.takes_typing()
                        && desk.typing_across_lines() =>
                    {
                        let way = if key == sdl::KEY_UP {
                            crate::typing::Way::Back
                        } else {
                            crate::typing::Way::On
                        };
                        let keeping = sdl::event_has_shift(&event);
                        desk.typing().go(way, crate::typing::By::Row, keeping);
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

        // Said once a frame, so a notice's clock is the window's clock and a thing that
        // has waited long enough goes without anybody having to touch it.
        desk.notices.expire(std::time::Instant::now());
        desk.tell_what_is_happening();

        if desk.hear_the_sweep() {
            acted = true;
        }

        // What the hold is doing arrives on its own thread, so it is picked up whatever
        // page is showing. It used to be asked for only while the server page was open,
        // which left every other page — the statistics beside a model among them —
        // showing figures from whenever that page was last left.
        if desk.hear_the_hold() {
            acted = true;
        }

        if last.elapsed() >= EVERY {
            desk.sample();
            // The queue is read whatever page is showing: a transfer finishing is worth
            // knowing about from the models page, where the model it brought now appears.
            let was = desk.transfers.clone();
            desk.read_transfers();
            desk.tell_what_is_arriving();
            if was != desk.transfers && desk.transfers.iter().any(Transfer::settled) {
                desk.refresh();
                // Whatever was asked to happen once the file arrived — hold it, most
                // often — happens now. A queued transfer answers straight away, so the
                // thing that was waiting on it has to be picked up when it lands rather
                // than when it was asked for.
                desk.settle_download();
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

/// One of a template's own values, as the person would have typed it.
fn said_plainly(held: &mcf_record::json::Value) -> String {
    match held {
        mcf_record::json::Value::Bool(on) => on.to_string(),
        mcf_record::json::Value::Integer(whole) => whole.to_string(),
        mcf_record::json::Value::Text(said) => said.clone(),
        _ => String::new(),
    }
}
