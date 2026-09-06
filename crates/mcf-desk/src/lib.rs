//! MCF in a window: its own control surface, for a mouse and a keyboard.
//!
//! **This is not the console at a larger scale, and the first one was.** That
//! design shared `mcf_tui`'s character grid so that one layout served both
//! surfaces; what it produced was a terminal with a mouse pointer over it. A
//! console is a table of cells read top to bottom, and an application is cards
//! and buttons that are scanned and clicked, and no amount of adjustment turns
//! one into the other. So the drawing is now this crate's own — a rasteriser
//! ([`font`]), a painter ([`paint`]) and six widgets ([`ui`]) — and what the
//! two surfaces share is the daemon they both ask and the record it keeps.
//!
//! **It is written for somebody who has never heard of a token.** [`words`]
//! turns measurements into sentences: tokens a second become words a second,
//! a context length becomes how much of a conversation a model remembers, and
//! a missing measurement becomes *Not measured yet* beside the button that
//! takes it — never a zero, which reads as a fact (A7).
//!
//! **It is a client, and adds nothing.** Every action turns into a request the
//! command line already sends, and [`ACTIONS`] names which (A22, B-072).

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

/// One thing the operator can do, and the control-plane request it reaches.
#[derive(Debug, Clone, Copy)]
pub struct Action {
    /// What it is called where it is shown.
    pub key: &'static str,
    /// What it does.
    pub does: &'static str,
    /// The `Request` variant it reaches, or `None` where it only moves about.
    pub reaches: Option<&'static str>,
}

/// Every action, and there are no others.
///
/// A22: the headless path can do everything a surface can. What keeps that
/// true is that this table exists and a check reads it — a window that grew an
/// action the command line could not perform would be a window that had become
/// the only way to do something.
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

/// What is being held, as the monitor needs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Hosted {
    /// Which model, by the path the daemon holds it under.
    pub model: String,
    /// Where a caller reaches it. The one fact an API is for.
    pub address: String,
    /// Since when, as the daemon stamped it.
    pub since: String,
    /// The context window it is held at, where the daemon said.
    pub context: Option<u64>,
    /// The projector loaded beside it, by file name, where one was.
    pub projector: Option<String>,
    /// What the engine said it takes for this model, where it answered.
    pub takes: Option<mcf_serve::takes::Takes>,
    /// Whether callers must present a key.
    pub api_key: bool,
    /// What it is doing now, where the daemon read the engine's counters.
    pub in_use: Option<Use>,
}

/// What a held model is doing right now, as the daemon read it off the
/// engine's own counters and the machine. Every figure is optional: the
/// engine publishes them only where it was started with counters on, and a
/// tile that has nothing says so rather than showing nought (A7).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Use {
    /// Tokens the engine has predicted since it came up.
    pub generated: Option<u64>,
    /// Tokens of prompt it has read since it came up.
    pub prompted: Option<u64>,
    /// Its predicting rate now, tokens a second.
    pub generated_per_second: Option<f32>,
    /// Its prompt-reading rate now, tokens a second.
    pub prompted_per_second: Option<f32>,
    /// How much of its cache is in use, nought to one.
    pub cache_used: Option<f32>,
    /// Requests it is answering now.
    pub processing: Option<u64>,
    /// Requests waiting behind them.
    pub queued: Option<u64>,
    /// What the engine holds in memory.
    pub resident: Option<u64>,
    /// What this hold has on the card.
    pub card: Option<u64>,
    /// How long it has been up.
    pub uptime_seconds: Option<u64>,
}

impl Use {
    /// Reads one from the daemon's answer.
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
            generated_per_second: rate("generated_tokens_per_second"),
            prompted_per_second: rate("prompt_tokens_per_second"),
            cache_used: rate("cache_used_ratio"),
            processing: count("requests_processing"),
            queued: count("requests_queued"),
            resident: count("resident_bytes"),
            card: count("card_bytes"),
            uptime_seconds: count("uptime_seconds"),
        }
    }
}

impl Hosted {
    /// The name, not the path: the question a screen answers is what is
    /// answering.
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

/// What was last held, as the record has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastHold {
    /// The model's path.
    pub model: String,
    /// The device it was on.
    pub device: String,
    /// The build it ran through.
    pub engine: String,
    /// Whether a stop was recorded after it.
    pub stopped: bool,
    /// How long ago it ended, or began where no end was recorded.
    pub ago_seconds: Option<u64>,
}

impl LastHold {
    /// Reads one from the daemon's answer.
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

    /// The name, not the path.
    #[must_use]
    pub fn name(&self) -> String {
        self.model
            .rsplit('/')
            .next()
            .unwrap_or(&self.model)
            .trim_end_matches(".gguf")
            .to_owned()
    }

    /// One line: what, on what, and when it ended.
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

/// Seconds as a span a person says: *40 s*, *12 min*, *2 h 5 min*.
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

/// One place a hold can put the model, as the daemon listed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// `resolved`, `processor` or `card`.
    pub on: String,
    /// The build that fits it.
    pub engine: String,
    /// The device.
    pub device: String,
    /// How much of the model goes on a card there.
    pub gpu_layers: u32,
    /// What the device has free, where the engine said.
    pub free: Option<u64>,
}

impl Placement {
    /// Reads one from the daemon's list.
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

/// What a hold will take against what the device it goes to has free, as
/// a sentence with a verdict: the weights and the cache for the window,
/// against the free figure the engine reported for the device the settings
/// name (§3.15). `None` where either side is unknown.
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

/// A load's progress as a sentence: what has been read of the weights, after
/// how long, and about how long is left once a twentieth is read and there
/// is a rate to read that off; past the weights, that the cache and the
/// engine's buffers follow. Said with *about* because it is arithmetic on
/// what has happened and not a promise (A6, A7).
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

/// One component MCF can build, as this window needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// What it is called.
    pub name: String,
    /// Which source, exactly — the first twelve of the commit, as the command
    /// line prints it.
    pub commit: String,
    /// What having it lets MCF claim.
    pub role: String,
    /// The base image it is built in.
    pub image: String,
    /// Whether it is here and finished — the provenance beside it, which the
    /// builder writes last.
    pub provisioned: bool,
    /// Whether anything is there at all. A prefix with no provenance is a run
    /// that stopped partway, which is a third state and not an absence.
    pub present: bool,
    /// Whether MCF can actually reach it as an engine. A directory that exists
    /// is not the same as a build that finished.
    pub usable_engine: bool,
    /// Where it went, or where it would go.
    pub prefix: String,
}

/// Reads one component out of what the daemon said.
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

/// Which field on the prompt screen typing goes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Caret {
    /// A setting on the model page's Configure tab.
    Setting,
    /// The prompt being taken apart.
    #[default]
    Document,
    /// The temperature the settledness seeds are drawn at (B-431).
    Temperature,
    /// The system turn a question is asked inside (B-462).
    System,
    /// How hard to reason, in the template's own word (B-462).
    Effort,
    /// The picture to show the model (B-462, B-452).
    Picture,
}

/// Which screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The machine, live.
    Monitor,
    /// Choosing a model, and everything known about it.
    Host,
    /// Setting up a measurement.
    Diagnostics,
    /// Everything held — the same screen as [`Self::Host`], as the console
    /// has it, because *what is held* and *what to host* are one list.
    Models,
    /// What MCF can build, and what it has.
    Components,
    /// How MCF is set up.
    Settings,
    /// Leave.
    Exit,
    /// Fetching a model that is not here yet. Reached from Host's actions
    /// rather than the menu, the way the console's screens lead onward.
    Adding,
    /// A model, held and answering.
    Hosting,
    /// What a prompt does to a model.
    ///
    /// Reached from Diagnostics rather than from the column: it is a
    /// diagnostic about a prompt, it needs the model Diagnostics already has
    /// chosen, and the console's menu row has four columns of slack where a
    /// seventh entry needs nine (B-072).
    Prompt,
    /// What a model is made of: what `mcf explain` counts, as the daemon
    /// says it (A22, B-072). Reached from Models' actions, for the model
    /// chosen there.
    Anatomy,
    /// What a model says with: its token list and chat template, counted by
    /// the daemon in the same answer as [`Self::Anatomy`], on a screen of its
    /// own because the two do not fit on one (B-072). Reached from there.
    Vocabulary,
}

impl Page {
    /// The navigation column, in order.
    ///
    /// The console's menu, in the console's order.
    ///
    /// Not a menu invented for the window: an operator worked this one out,
    /// and a second surface that rearranged the same six entries would make
    /// *where things are* a fact about which surface you happened to open
    /// (B-072).
    pub const MENU: &'static [(Self, &'static str)] = &[
        (Self::Monitor, "System"),
        (Self::Models, "Models"),
        (Self::Hosting, "Server"),
        (Self::Diagnostics, "Diagnostics"),
        (Self::Exit, "Exit"),
    ];

    /// Which entry in the column should be lit while this page shows.
    #[must_use]
    pub fn section(self) -> Self {
        match self {
            // Host was a second entry for the list Models already shows —
            // `view::host` draws both — so the column carried one screen
            // twice. The screens its actions lead to belong to Models now,
            // and the menu still shows where you came from.
            Self::Host | Self::Adding | Self::Anatomy | Self::Vocabulary | Self::Models => {
                Self::Models
            }
            // Running is what is held: its own place (D49).
            Self::Hosting => Self::Hosting,
            Self::Diagnostics | Self::Prompt => Self::Diagnostics,
            // The engines are part of the machine, and the empty settings
            // page went with the redesign; both land on Machine.
            Self::Monitor | Self::Components | Self::Settings => Self::Monitor,
            Self::Exit => Self::Exit,
        }
    }
}

/// One probe's or measurement's last finding on a model: the lines the
/// daemon wrote for it — or the record's one sentence, where the finding
/// is from before this window opened — with when it was taken and on
/// what engine (D53).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Finding {
    /// The probe's or measurement's name, as the daemon lists it.
    pub name: String,
    /// When it was taken, as the record wrote it; `None` where the record
    /// did not say.
    pub at: Option<String>,
    /// The engine it was taken through, where the record said.
    pub engine: Option<String>,
    /// What it found, a line each.
    pub lines: Vec<String>,
}

/// One model, as this window needs it.
///
/// Every field that can be absent is an `Option`, and nothing is defaulted to
/// zero on the way in. A zero here would be drawn as a measurement.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// What to call it: the file's name, without the path or the extension.
    pub name: String,
    /// Where it is, for the technical disclosure.
    pub path: String,
    /// How large the file is.
    pub bytes: Option<u64>,
    /// What kind of model it is, as its own header says.
    pub architecture: Option<String>,
    /// The longest conversation it was trained for.
    pub trained: Option<u64>,
    /// The longest conversation it can hold on this machine.
    pub context: Option<u64>,
    /// Bytes of cache one token of context costs — what the largest window is
    /// arithmetic over, and a figure the console shows because it is what
    /// makes the answer checkable rather than a claim (A6).
    pub cache_per_token: Option<u64>,
    /// The engine MCF would use.
    pub engine: Option<String>,
    /// The device it would run on.
    pub device: Option<String>,
    /// What that device had free when MCF resolved it.
    pub device_free: Option<u64>,
    /// The last measurement taken of it, whole, as the daemon keeps it: the
    /// readings and what was read off them, for the Statistics tab.
    pub measured_body: Option<Value>,
    /// When the last measurement was taken, as the record wrote it (D53).
    pub measured_at: Option<String>,
    /// What the last cross-check said, sentence by sentence.
    pub cross_checked: Vec<String>,
    /// When it was taken (D53).
    pub cross_checked_at: Option<String>,
    /// Whether a prompt report has been taken of it.
    pub prompt_reported: bool,
    /// When the last one was (D53).
    pub prompt_reported_at: Option<String>,
    /// The addressing a probe applied, by provenance, where one was.
    pub applied_addressing: Option<String>,
    /// The budget a probe applied, by provenance, where one was.
    pub applied_budget: Option<String>,
    /// What the probes and the measurements last found on it, one entry
    /// a method: the lines the daemon wrote for it, when and through what
    /// (B-478, D52, D53).
    pub probed: Vec<Finding>,
    /// The hub repository it came from, where its provenance names one: a
    /// model is a repository with its quantizations, and this is which
    /// (D51, B-486).
    pub repository: Option<String>,
    /// The file it is, by name: which quantization of its repository.
    pub file: String,
    /// Whether that device is a graphics card.
    pub on_a_card: bool,
    /// Why it will not run, where it will not.
    pub refused: Option<String>,
    /// Tokens a second, where it has been measured.
    pub speed: Option<f64>,
    /// Milliseconds to the first token at the shallowest depth the ladder
    /// measured, as the daemon said it — a warm figure, taken with the file
    /// already in the page cache (see `mcf_serve::ladder`).
    pub start_up: Option<String>,
    /// Milliseconds a token at the shortest depth measured.
    pub fastest: Option<f64>,
    /// Milliseconds a token at the longest.
    pub slowest: Option<f64>,
    /// Every depth that separated, in order — the shape rather than its ends.
    ///
    /// Kept whole because a table can say what a cost is at a depth and only a
    /// picture can say whether it is going anywhere, which is the question
    /// somebody actually has (B-410).
    pub ladder: Vec<crate::chart::Reading>,
}

/// One measurement that can be asked for — the console's row, with what its
/// last run found (B-072).
pub use mcf_tui::screens::diagnostics::Test;

/// What runs a test — the console's word for it.
pub use mcf_tui::screens::diagnostics::Run;

/// The tests MCF knows how to run.
///
/// The console's list, read from the console rather than copied from it: the
/// same set of measurements in the same order, because a second surface
/// offering a different five would make *what MCF can measure* a fact about
/// which surface you opened (B-072).
#[must_use]
pub fn tests() -> Vec<Test> {
    mcf_tui::screens::diagnostics::tests()
}

impl Model {
    /// Whether MCF found a way to run it here.
    #[must_use]
    pub fn will_run(&self) -> bool {
        self.refused.is_none() && self.engine.is_some()
    }

    /// Where it would run, in a phrase.
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

    /// The sentence on the card: what this model is, for this machine.
    ///
    /// Built from whatever is known, and it says less when less is known
    /// rather than filling the gap.
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

    /// What it takes out of the machine.
    #[must_use]
    pub fn memory_sentence(&self) -> String {
        words::size_in_words(self.bytes).map_or_else(
            || words::UNMEASURED.to_owned(),
            |size| format!("Uses {size}"),
        )
    }

    /// What was measured at the shallowest depth, or that nothing was.
    #[must_use]
    pub fn speed_at_512(&self) -> String {
        self.fastest.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |ms| format!("{ms:.2} ms/token"),
        )
    }

    /// What was measured at the deepest rung the latest ladder climbed.
    #[must_use]
    pub fn speed_at_window(&self) -> String {
        self.slowest.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |ms| format!("{ms:.2} ms/token"),
        )
    }

    /// What the two speed rows are rows of: the depth each end was measured
    /// at, since the deepest rung is as deep as the ladder was asked to climb
    /// and not the model's window — a Quick Run's 1,024 said *at the largest
    /// window* until the label came from the reading (A20). The console
    /// labels its card the same way (B-072).
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

    /// How long the first token takes, warm.
    #[must_use]
    pub fn start_up(&self) -> String {
        self.start_up
            .as_ref()
            .map_or_else(|| crate::view::UNKNOWN.to_owned(), |ms| format!("{ms} ms"))
    }

    /// Whether anything has been measured about it.
    #[must_use]
    pub fn measured(&self) -> bool {
        self.fastest.is_some() || self.slowest.is_some() || self.start_up.is_some()
    }

    /// Everything the screen decided not to lead with.
    ///
    /// Nothing is dropped on the way to a plain sentence — it is put here
    /// (A1), in the units it was measured in.
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

/// Reads a model out of what the daemon answered.
/// The hub repository a held model's provenance names, where it names one.
fn repository_of(held: &Value) -> Option<String> {
    held.get("provenance")
        .and_then(|provenance| provenance.get("origin"))
        .filter(|origin| origin.get("kind").and_then(Value::as_text) == Some("hub"))
        .and_then(|origin| origin.get("repository"))
        .and_then(Value::as_text)
        .map(str::to_owned)
}

/// What the record holds of the probes and the measurements, one
/// sentence each, with when and through what (B-483, D53).
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
        applied_addressing: held
            .get("configured")
            .and_then(|applied| applied.get("addressing"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        applied_budget: held
            .get("configured")
            .and_then(|applied| applied.get("budget"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        repository: repository_of(held),
        file,
        probed: probed_of(held),
        on_a_card: resolved_text("device_kind").as_deref() == Some("gpu"),
        cache_per_token: number_from_runs("cache_bytes_per_token"),
        refused: if known { None } else { resolved_text("why") },
        // What was measured, from the record, and absent where nothing was
        // (A7). The shallowest reading and the deepest are what the console's
        // MEASURED table shows; the whole ladder is in the entry for anything
        // that wants the shape rather than the ends.
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

/// When a run's body says it was recorded, where the daemon dated it (D53).
fn at_in(body: Option<&Value>) -> Option<String> {
    body.and_then(|body| body.get("at"))
        .and_then(Value::as_text)
        .map(str::to_owned)
}

/// What the daemon keeps of a model's runs, for the Statistics tab and the
/// diagnostics list: the last measurement whole, what the cross-check
/// said, whether a prompt report was taken, and when the last two were
/// (D53).
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

/// Milliseconds a token, as tokens a second.
/// Where the answer's first token went with a sentence gone, as a table
/// cell: `1` where the model would still have begun the same way, `17`
/// where it fell to its seventeenth choice, `>60` where it was outside the
/// depth read, and `—` where no reading was taken — which is a state and
/// not a rank (A7, B-429).
///
/// One implementation for the glass and the console (B-072).
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

/// How much of the answer's opening stayed the model's first choice with a
/// sentence gone, as `kept/of`, or `—` where no reading was taken (A7).
///
/// One implementation for the glass and the console (B-072).
#[must_use]
pub fn open_mark(held: Option<&Value>) -> String {
    let Some(held) = held.filter(|held| !matches!(held, Value::Null)) else {
        return "—".to_owned();
    };
    let count = |key: &str| held.get(key).and_then(Value::as_integer).unwrap_or(0);
    format!("{}/{}", count("kept"), count("of"))
}

/// What the answer as written amounted to: how many tokens, and what ended
/// it — its own stop token, the cap, or what the engine said. An answer
/// with neither under a 600-token cap left a reader to guess whether the
/// model said nothing or the report lost it (A7, F160). Empty where the
/// report served no account.
///
/// One implementation for the glass and the console (B-072).
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

/// The line that stands where an answer would, when the model wrote
/// nothing a reader can see: said, rather than left blank (A7).
pub const NOTHING_WRITTEN: &str = "nothing written";

/// The pair a swap changed the places of, as `1&2` for the first swap:
/// the parts counted from one, the way the removed rows count them
/// (B-437). An ampersand rather than an arrow because the window's face is
/// whichever the machine has, and the arrows are the glyphs it goes without.
///
/// One implementation for the glass and the console (B-072).
#[must_use]
pub fn pair_mark(at: usize) -> String {
    format!("{}&{}", at.saturating_add(1), at.saturating_add(2))
}

/// How much of one part the model would have written itself, as `1/4`: one
/// of its four tokens was the model's first choice (B-433). `None` where the
/// reading was not taken or has no such part, which the row then shows as
/// nothing rather than as a prompt wholly expected (A7).
///
/// One implementation for the glass and the text that leaves it (B-072).
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

/// The ends of a measured ladder: the shallowest reading and the deepest.
#[derive(Debug, Default)]
struct Measured {
    /// Milliseconds a token at the shallowest depth that separated.
    fastest: Option<f64>,
    /// The same at the deepest.
    slowest: Option<f64>,
    /// Every depth that separated, in order.
    ladder: Vec<crate::chart::Reading>,
    /// Milliseconds to the first token at the shallowest rung, where the
    /// daemon read one.
    start_up: Option<String>,
}

/// Reads the ends out of what the record kept.
///
/// Only the depths that *separated*: a rung the arithmetic could not measure
/// is not a slow one, and letting it stand in for the deepest reading would
/// put a number where there is none (A7, A9).
fn measured_ends(held: &Value) -> Measured {
    // The start-up figure is the daemon's, derived on its side of the wire
    // so that no surface derives it differently (B-072); a run that could not
    // read one says so there, and reads as nothing here.
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

/// Asks the daemon one question.
fn ask(socket: &Path, request: &Request) -> Result<Answer, String> {
    ask_within(socket, request, std::time::Duration::from_secs(30))
}

/// How long a reading waits before giving up on a busy daemon.
///
/// **Long enough to answer, short enough not to freeze the window.** The
/// daemon answers one thing at a time, so while it loads a large model every
/// reading waits — and with the thirty-second deadline a deliberate act
/// deserves, the window stopped repainting and stopped taking events entirely.
///
/// Four hundred milliseconds was the first attempt at that and was wrong:
/// *what this machine is holding* reads sixteen model headers off disk and
/// takes about a second and a half, so the list came back empty every time and
/// the window showed no models at all. A deadline shorter than the work is not
/// a deadline, it is a guarantee of failure (A7).
///
/// Five seconds: several times what the slowest reading takes, and a pause
/// rather than a freeze when the daemon is busy elsewhere.
const POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// Asks, waiting no longer than this for the answer.
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

/// Which dropdown a click was about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    /// Which model the measurement is of.
    Model,
    /// How deep a context it is set up for.
    Window,
    /// Where the model goes: as MCF resolves it, the processor, or the card.
    On,
    /// Where a hold puts the model, from the daemon's list of placements.
    Placement,
    /// The rope scaling a hold starts with.
    Rope,
    /// Which quantization of the repository the page is about (B-486).
    Quantization,
    /// The filters' architecture (B-489).
    Architecture,
    /// The filters' *fits here*.
    Fits,
    /// The filters' size.
    Size,
}

/// A region of the window that scrolls on its own (B-490).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    /// The library's rows.
    Library,
    /// The model page under its tabs.
    Page,
    /// A hub repository's page, or a pending file's.
    Hub,
    /// The diagnostic chosen on the Diagnostics page, shown whole (D53).
    Diagnostics,
    /// The Diagnostics page's list of every diagnostic (D53).
    Checks,
    /// The Server page.
    Server,
    /// The prompt page's report.
    Prompt,
    /// The System page.
    Monitor,
}

/// A boundary a person can drag (B-490).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Splitter {
    /// Between the column down the left and the page.
    Side,
    /// Between the library and the model page.
    List,
    /// Between the Diagnostics page's two columns.
    Diagnostics,
}

/// Where the splitters sit: the column's width, the library's width, and
/// the Diagnostics page's left column's width, in points (B-490).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Splits {
    /// The column down the left.
    pub side: f32,
    /// The library.
    pub list: f32,
    /// The Diagnostics page's left column.
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

/// Whole points as the screen measures them.
fn as_points(whole: i32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a point on the screen, exact in f32 at any window size"
    )]
    let at = whole as f32;
    at
}

/// The room around a page, which a splitter's position is measured past.
const PAGE_PAD: f32 = 26.0;

impl Splits {
    /// Moves one splitter to a position along the window, kept inside the
    /// room each area needs: a column narrower than its words or a page
    /// narrower than a control is no layout at all.
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

/// The filters beside the library's search field: applied with the words
/// to what is here, each *any* until somebody sets it, so an empty list is
/// a list nothing matched and not one a filter hid (D51, B-489).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filters {
    /// Whether the row of pickers is shown.
    pub open: bool,
    /// Only models of this architecture, where set.
    pub architecture: Option<String>,
    /// Only models MCF says will run here (`true`), or will not (`false`).
    pub fits: Option<bool>,
    /// Only models of at most this many bytes, where set.
    pub size: Option<u64>,
}

impl Filters {
    /// Whether any filter is set.
    #[must_use]
    pub const fn any_set(&self) -> bool {
        self.architecture.is_some() || self.fits.is_some() || self.size.is_some()
    }
}

/// The sizes the filter offers: any, then three ceilings in bytes.
pub const SIZE_CHOICES: [Option<u64>; 4] = [
    None,
    Some(8_000_000_000),
    Some(20_000_000_000),
    Some(50_000_000_000),
];

/// The *fits here* choices: any, will run here, will not.
pub const FITS_CHOICES: [Option<bool>; 3] = [None, Some(true), Some(false)];

/// What the hub answered a search with: the words, and the repositories
/// with GGUF files it lists for them, most downloaded first (D51).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubList {
    /// The words the hub was asked for.
    pub query: String,
    /// What it listed.
    pub repositories: Vec<HubRepo>,
}

/// One repository the hub listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubRepo {
    /// `owner/name`, as the hub names it.
    pub id: String,
    /// Downloads the hub counts, where it says.
    pub downloads: Option<u64>,
}

impl HubRepo {
    /// One repository as the daemon lists it.
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

/// One GGUF file a repository publishes, as the daemon lists it (B-486).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferedFile {
    /// The file's name in the repository.
    pub file: String,
    /// Its size, where the hub says.
    pub bytes: Option<u64>,
    /// Whether MCF says it would run here; `None` where it could not say.
    pub fits: Option<bool>,
}

impl OfferedFile {
    /// One file as the daemon lists it.
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

/// A quantization picked that is not here yet: the page's subject as *not
/// downloaded* (B-486, B-487).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The repository it is a file of.
    pub repository: String,
    /// The file.
    pub file: String,
    /// Its size, where the hub says.
    pub bytes: Option<u64>,
    /// Whether it would run here, where MCF could say.
    pub fits: Option<bool>,
}

/// One quantization of the page's repository, as the picker lists it:
/// here, by its place in the library, or on the hub (B-486).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quant {
    /// The file.
    pub file: String,
    /// Its size, where known.
    pub bytes: Option<u64>,
    /// Which held model it is, where it is here.
    pub here: Option<usize>,
    /// Whether it would run here, where MCF said.
    pub fits: Option<bool>,
}

/// One entry of the library: a repository and the held files that are its
/// quantizations, or a file no repository is known for (D51, B-486).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The repository, where the members' provenance names one.
    pub repository: Option<String>,
    /// The held models in it, by their place in the list.
    pub members: Vec<usize>,
}

impl Group {
    /// The name the list shows: the repository's own name, or the one
    /// member's.
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

/// One run the Diagnostics page offers, each on its own card with its own
/// controls, cost and Run (D50, B-477).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// The depth ladder: speed at each depth, and what a window costs.
    Throughput,
    /// MCF's own engine reading what the provisioned one produced.
    CrossCheck,
    /// The probes: what the template and the model do.
    Capabilities,
    /// The performance measurements: offload, prefill, prefix reuse,
    /// memory, concurrency, cold start (D52).
    Performance,
    /// The fidelity measurements: against a reference file, bits a byte,
    /// determinism, the tokenizer (D52).
    Fidelity,
    /// The behaviour measurements: retrieval, degeneration, grammar, an
    /// image's cost (D52).
    Behaviour,
    /// What each part of a prompt does to the answer.
    Prompt,
    /// Two models on one question under one engine.
    Comparison,
}

impl Card {
    /// Every card, in the order the page shows them.
    pub const ALL: [Self; 8] = [
        Self::Throughput,
        Self::CrossCheck,
        Self::Capabilities,
        Self::Performance,
        Self::Fidelity,
        Self::Behaviour,
        Self::Prompt,
        Self::Comparison,
    ];

    /// The measurements this card runs, where it is one of the three that
    /// run them (D52): the daemon's own family list, so the card and the
    /// run cannot disagree about what is in it.
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

    /// The card's name.
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
        }
    }

    /// What the run answers, in one line.
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
        }
    }

    /// The command that runs it, for the runs the daemon does not carry
    /// yet (B-478): the window says so and offers the command rather than
    /// a button that does nothing (§3.15).
    #[must_use]
    pub fn command(self, model: &str) -> Option<String> {
        match self {
            Self::Comparison => Some(format!("mcf bench {model} <other-model> --prompt \"…\"")),
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

/// One diagnostic MCF can take of a model: a row of the Diagnostics
/// page's list, and the whole of it when chosen (D53).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Diagnostic {
    /// The depth ladder.
    Throughput,
    /// MCF's own engine reading the provisioned one's tokens.
    CrossCheck,
    /// What each part of a prompt does to the answer.
    Prompt,
    /// Two models on one question, at the command line.
    Comparison,
    /// One probe, by its place in the daemon's list.
    Probe(usize),
    /// One measurement, by its place in the daemon's list.
    Measure(usize),
}

/// What each probe answers, in one line, in the daemon's order.
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

/// What each measurement answers, in one line, in the daemon's order.
const MEASURE_ANSWERS: [&str; 23] = [
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
];

impl Diagnostic {
    /// Every diagnostic, in the order the list shows them.
    #[must_use]
    pub fn all() -> Vec<Self> {
        Self::families()
            .into_iter()
            .flat_map(|(_, _, members)| members)
            .collect()
    }

    /// The list's families: each with its heading, the card whose Run
    /// runs every one of it where there is such a run, and its members.
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
        ]
    }

    /// The diagnostic's name, as the daemon and the record know it.
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
        }
    }

    /// What it answers, in one line.
    #[must_use]
    pub fn answers(self) -> &'static str {
        match self {
            Self::Throughput => Card::Throughput.answers(),
            Self::CrossCheck => Card::CrossCheck.answers(),
            Self::Prompt => Card::Prompt.answers(),
            Self::Comparison => Card::Comparison.answers(),
            Self::Probe(at) => PROBE_ANSWERS.get(at).copied().unwrap_or(""),
            Self::Measure(at) => MEASURE_ANSWERS.get(at).copied().unwrap_or(""),
        }
    }

    /// The run it belongs to.
    #[must_use]
    pub fn card(self) -> Card {
        match self {
            Self::Throughput => Card::Throughput,
            Self::CrossCheck => Card::CrossCheck,
            Self::Prompt => Card::Prompt,
            Self::Comparison => Card::Comparison,
            Self::Probe(_) => Card::Capabilities,
            Self::Measure(at) => {
                let name = mcf_serve::examine::MEASURES.get(at).copied().unwrap_or("");
                [Card::Performance, Card::Fidelity, Card::Behaviour]
                    .into_iter()
                    .find(|card| card.measures().contains(&name))
                    .unwrap_or(Card::Performance)
            }
        }
    }

    /// The method the record keeps its finding under, where it is a probe
    /// or a measurement.
    #[must_use]
    pub fn method(self) -> Option<&'static str> {
        match self {
            Self::Probe(_) | Self::Measure(_) => Some(self.name()),
            _ => None,
        }
    }

    /// The method the record keeps its readings under: the probes' and
    /// measurements' own names, and the ladder's and cross-check's (D54).
    #[must_use]
    pub fn readings_method(self) -> Option<&'static str> {
        match self {
            Self::Throughput => Some("throughput"),
            Self::CrossCheck => Some("cross-check"),
            Self::Prompt | Self::Comparison => None,
            // A probe's record name is not always its run name: the
            // usable-context probe runs as `context` and records as
            // `usable-context`, the tool probe as `tool-calls` and
            // `tool-calling`.
            Self::Probe(at) => mcf_serve::probes::run::RECORDED.get(at).copied(),
            Self::Measure(_) => Some(self.name()),
        }
    }
}

/// Which tab of the model page is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Every setting a hold takes, with a control on each.
    #[default]
    Configure,
    /// Everything measured or read about the model.
    Statistics,
    /// What the file holds: tensors and vocabulary.
    Contents,
}

impl Tab {
    /// The three, in order.
    pub const ALL: [Self; 3] = [Self::Configure, Self::Statistics, Self::Contents];

    /// The word on the tab.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Configure => "Configure",
            Self::Statistics => "Statistics",
            Self::Contents => "Contents",
        }
    }
}

/// A setting on the Configure tab that takes typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The context window, in tokens.
    Context,
    /// Processor threads.
    Threads,
    /// The prompt batch.
    Batch,
    /// The port.
    Port,
    /// The key callers present.
    ApiKey,
    /// The rope scaling's factor.
    RopeFactor,
}

/// A setting on the Configure tab that is a switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// The attention kernel that reads less memory.
    FlashAttention,
    /// Holding the pages in memory.
    KeepResident,
    /// Starting the draft head the file carries.
    DraftHead,
    /// Loading the projector beside the file.
    Projector,
}

/// The rope scalings a hold can start with, in the order the list offers
/// them: as the file has it, off, linear, yarn.
pub const ROPE_CHOICES: [Option<mcf_serve::declared::Scaling>; 4] = [
    None,
    Some(mcf_serve::declared::Scaling::Off),
    Some(mcf_serve::declared::Scaling::Linear),
    Some(mcf_serve::declared::Scaling::Yarn),
];

/// The context windows a measurement can be set up for.
///
/// **Powers of two, because a context window is asked for in them**, and every
/// one below the chosen depth is sampled — which is why this is a list to pick
/// from rather than a number to type.
#[must_use]
pub const fn windows() -> [u64; 7] {
    [1_024, 2_048, 4_096, 8_192, 16_384, 32_768, 65_536]
}

/// Which answer the prompt screen shows beside the figures: the one without
/// a part, or the one to a part alone (B-435).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// The answer without this part.
    Without(usize),
    /// The answer to this part alone.
    Alone(usize),
    /// The answer to the first `n + 1` parts, the prompt grown from the
    /// front (B-436).
    Prefix(usize),
    /// The answer with this part and the one after it in each other's
    /// places (B-437).
    Swap(usize),
    /// The answer to the parts in this form, counted among the forms as
    /// served (B-444).
    Form(usize),
}

/// One thing a screen asks the window to do.
///
/// Immediate mode has no callbacks: a screen draws, notices it was clicked,
/// and says what that meant. Everything that changes state happens in one
/// place, which is why a click can never leave the window half-changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    /// Show another screen.
    Go(Page),
    /// Thinking on, off, or unsaid: the next position of the switch a
    /// question is asked under (B-462).
    CycleThinking,
    /// Ask a hub what it publishes under what has been typed.
    LookUp,
    /// Fetch one published file.
    Download {
        /// The repository.
        reference: String,
        /// The file within it.
        file: String,
    },
    /// Time the chosen model: a quick climb of the ladder, whatever is
    /// ticked.
    Measure {
        /// The deepest context to sample.
        deepest: u64,
    },
    /// Start one run's card on the chosen model (D50).
    Run(Card),
    /// Search the hub for the words in the library's field (D51).
    SearchHub,
    /// Scroll one region to an offset, in whole points (B-490).
    Scroll(Region, i32),
    /// Move one splitter to a position along its axis, in whole points.
    Split(Splitter, i32),
    /// Show or hide the filters beside the search field (B-489).
    ToggleFilters,
    /// Set the architecture filter, by its place in the list offered.
    SetArchitecture(usize),
    /// Set the *fits here* filter, by its place in the list offered.
    SetFits(usize),
    /// Set the size filter, by its place in the list offered.
    SetSize(usize),
    /// Pick one quantization of the page's repository, by its place in
    /// the list the page offers: here first, then the hub's (B-486).
    Quantization(usize),
    /// Pick one file of the hub repository whose page is open, by its
    /// place in the hub's list: the page's subject as not downloaded.
    PickOffered(usize),
    /// Download the subject that is not here, then do the thing named:
    /// start the server, or a run (B-487).
    DownloadThen(std::boxed::Box<Act>),
    /// Open the page of one repository the hub listed, by its place.
    PickHub(usize),
    /// Flip whether the probes apply what they find.
    ApplyProbes,
    /// Choose one diagnostic of the list, to show it whole (D53).
    Show(Diagnostic),
    /// Run one diagnostic from its own pane: a probe or a measurement on
    /// its own, or the run it is (D53).
    RunOne(Diagnostic),
    /// Run every diagnostic in turn (B-508).
    RunAll,
    /// Read a run's figures where they are kept: the model's Statistics tab.
    SeeStatistics,
    /// Cut the run that is going short.
    Stop,
    /// Hold the model that was last held, again.
    HostAgain,
    /// Look up this repository, picked from a search.
    Pick(String),
    /// Open a tab of the model page.
    Tab(Tab),
    /// Show this half of Contents: the tensors or the vocabulary.
    Contents(Page),
    /// Start typing into a setting.
    Edit(Field),
    /// Flip a switch.
    Switch(Switch),
    /// Put the hold where the daemon's list says, by index.
    Place(usize),
    /// Start with this rope scaling, by index into `ROPE_CHOICES`.
    Rope(usize),
    /// Put this text on the clipboard — the loop's, because the clipboard is
    /// the window's and not the desk's.
    Copy(String),
    /// Where the next run puts the model; `None` is where MCF resolves it.
    SetOn(Option<mcf_serve::control::On>),
    /// Open a dropdown, or close it if it is the one already open.
    ///
    /// **The screen had two controls drawn as dropdowns that were not
    /// dropdowns**: the model picker navigated to another page and the window
    /// picker cycled to the next power of two. Both wore a chevron, which is
    /// the promise that a list will appear. A control that looks like a
    /// dropdown and does something else teaches the operator that the
    /// furniture is decoration.
    Open(Picker),
    /// Show the answer the model gave without one sentence, or the answer to
    /// the prompt as written when that sentence is already the one shown.
    ///
    /// **The figures were the whole report and the evidence for them was on
    /// the wire, unread.** A row saying a sentence moved 96% of the answer is
    /// checkable only beside the answer it moved, and MCF has held both since
    /// the measurement was written (A19).
    ShowWithout(usize),
    /// Show the answer to one part alone, where each was asked (B-435).
    ShowAlone(usize),
    /// Show the answer to the prompt grown through this many parts, or put
    /// the answer as written back.
    ShowPrefix(usize),
    /// Show the answer with this part and the next swapped, where the
    /// swaps were asked (B-437).
    ShowSwap(usize),
    /// Show the answer to the parts in one form, where the forms were
    /// asked (B-444).
    ShowForm(usize),
    /// Close whatever dropdown is open, choosing nothing.
    Shut,
    /// Set the context window to one of the offered powers of two.
    SetWindow(u64),
    /// Move one hosting setting on to its next value.
    Cycle(usize),
    /// Put every setting back to what MCF recommended.
    Recommended,
    /// Put every setting back to what the model was last held under.
    LastSettings,
    /// Hold the chosen model under the settings as they stand.
    HostIt,
    /// Build one component, by name, from the Components screen.
    ///
    /// The window is the surface; a card that said *mcf provision llama.cpp*
    /// was sending the operator to a terminal for what the daemon behind the
    /// window could do on request (A22, B-367).
    Build(String),
    /// Stop holding it.
    StopHosting,
    /// Close the window.
    Close,
    /// Analyse the typed prompt on the chosen model.
    ReportPrompt,
    /// Put the caret in one of the prompt screen's fields.
    Focus(Caret),
    /// Remove at most this many parts of the document (B-430).
    MostParts(usize),
    /// Take the document apart by this unit, or let the text decide.
    TakeApartBy(Option<mcf_serve::prompt::Unit>),
    /// Ask for a further reading, or stop asking (B-434, B-435).
    Extra(mcf_serve::prompt::Extra, bool),
    /// Ask a model what has been typed.
    Ask {
        /// Which, by position in the list.
        at: usize,
    },
    /// Choose a model without leaving the screen.
    Choose(usize),
    /// Empty the field — on the prompt screen, the document.
    Clear,
    /// Forget what just ran, so the screen goes back to its resting state.
    Dismiss,
}

/// What long-running thing the window is waiting on, if any.
///
/// One at a time, deliberately. Two measurements at once would be two
/// measurements of a machine that was running a measurement, and the second
/// would be a reading of the first (A6).
#[derive(Debug)]
pub enum Doing {
    /// Nothing.
    Nothing,
    /// Starting a model on a port.
    Hosting(job::Job),
    /// Building the engine a model needs, so that it can then be held.
    ///
    /// Started by Host, never on its own: the window builds only when the
    /// operator asked for a model to be held and MCF had nothing to hold it
    /// with — and it says what it is building while it does (§3.15, B-367).
    Provisioning(job::Job),
    /// Asking a hub what it publishes.
    Listing(job::Job),
    /// Fetching a model.
    Downloading(job::Job),
    /// Timing one.
    Measuring(job::Job),
    /// Reading what the provisioned engine produced with MCF's own.
    CrossChecking(job::Job),
    /// Waiting for a model to answer.
    Answering(job::Job),
    /// Taking a prompt apart.
    Reporting(job::Job),
    /// The probes are running on the chosen model (B-478).
    Probing(job::Job),
    /// The measurements are running on the chosen model (D52).
    Examining(job::Job),
}

impl Doing {
    /// The job behind it, whatever it is.
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
            | Self::Hosting(job) => Some(job),
        }
    }

    /// Whether something is still running.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.job().is_some_and(|job| !job.finished)
    }
}

/// The window's state.
#[derive(Debug)]
pub struct Desk {
    socket: std::path::PathBuf,
    /// Which screen is showing.
    pub page: Page,
    /// Every model this computer holds.
    pub models: Vec<Model>,
    /// Which answer is being shown on the prompt screen: one part's absence,
    /// or one part alone. `None` is the answer to the prompt as written.
    pub shown: Option<Shown>,
    /// How far each region that scrolls has been scrolled, in points
    /// (B-490).
    pub scrolls: std::collections::BTreeMap<Region, f32>,
    /// Where the splitters between the window's areas sit.
    pub splits: Splits,
    /// The splitter a press took hold of, until the button is let go: the
    /// line follows the pointer however far it goes, not only while the
    /// press is still inside the band (F195).
    pub grabbed: Option<Splitter>,
    /// The last reading of the machine.
    pub reading: mcf_tui::machine::Reading,
    /// Why MCF could not be reached, when it could not.
    pub refusal: Option<String>,
    /// Whether the last poll went unanswered in time.
    ///
    /// **Busy and absent are different facts.** A daemon loading a large model
    /// answers nothing for minutes, and a window that read that as *not up*
    /// would tell an operator their daemon had died at the exact moment it was
    /// doing what they asked (A7).
    pub busy: bool,
    /// What is being typed, on the screen that has a field.
    pub typed: String,
    /// What the library is being searched for: filters what is held as it
    /// is typed, and is the words the hub is asked for (D51, B-485).
    pub filter: String,
    /// What the hub answered for the words in the field, where it was
    /// asked: listed under what is here until the words change.
    pub hub: Option<HubList>,
    /// Which hub repository's page is open, where one is, by its place in
    /// the hub's list; `None` is a held model's page (B-485).
    pub hub_chosen: Option<usize>,
    /// What the hub publishes for each repository asked about, by
    /// repository: every GGUF file with its size and whether it would run
    /// here, kept for the window's life (B-486).
    pub offered: std::collections::BTreeMap<String, Vec<OfferedFile>>,
    /// A quantization picked that is not here: the page's subject as *not
    /// downloaded*, until it is got or another is picked (B-486).
    pub pending: Option<Pending>,
    /// What to do once the download going has finished: the thing the
    /// button named, on the file once it is here (B-487).
    pub after_download: Option<Act>,
    /// The filters beside the search field, each *any* until set (D51,
    /// B-489).
    pub filters: Filters,
    /// The temperature to draw the settledness seeds at, as typed; empty
    /// asks the question nothing, and the page says so (B-431).
    pub temperature: String,
    /// The system turn a question is asked inside, as typed; empty asks for
    /// none, which is not the same as an empty one (B-462, D43).
    pub system: String,
    /// How hard to reason, in the model's template's own vocabulary, as
    /// typed; empty asks for nothing (B-462).
    pub effort: String,
    /// Thinking on, off, or unsaid — three states, and unsaid is what the
    /// template does of its own accord (B-462, D43).
    pub thinking: Option<bool>,
    /// The picture to show the model, as a path typed; empty shows none
    /// (B-462, B-452).
    pub picture: String,
    /// Which field on the prompt screen typing goes into.
    pub caret: Caret,
    /// How many parts to remove at most, where the person chose; `None` is
    /// the report's default and the page says what that is.
    pub most: Option<usize>,
    /// What to take the document apart into, where the person chose; `None`
    /// lets the text decide.
    pub by: Option<mcf_serve::prompt::Unit>,
    /// The further readings asked for, each costing generations (B-434,
    /// B-435).
    pub extras: mcf_serve::prompt::Extras,
    /// Which model a measurement or a question is about.
    pub chosen: Option<usize>,
    /// What is running.
    pub doing: Doing,
    /// What a model has said so far, this turn.
    pub said: String,
    /// The tests offered on the diagnostics screen.
    pub tests: Vec<Test>,
    /// Whether the probes apply what they find, which is an act (D43).
    pub probes_apply: bool,
    /// The diagnostic chosen on the Diagnostics page, shown whole (D53).
    pub diagnostic: Diagnostic,
    /// The chosen model's readings runs, newest first, as the daemon
    /// answered them, with the path they are of (D54, B-516).
    pub readings: Option<(String, Vec<Value>)>,
    /// The runs still to start after the one going, where a Run all is
    /// under way; and how many the whole sequence had (B-508).
    pub queued: std::collections::VecDeque<Card>,
    /// How many runs the Run all under way began with, for the strip's
    /// *run 2 of 6*; nought where no Run all is going.
    pub queued_of: usize,
    /// What MCF can build, and which of it is here.
    pub components: Vec<Component>,
    /// What the chosen model would be hosted under, and what MCF advised.
    ///
    /// Both, because a run under a changed setting is not a run under the
    /// recommended one and a person needs to see which they have (§3.15).
    pub settings: Option<mcf_serve::hosting::Hosting>,
    /// What MCF recommended for the chosen model.
    pub recommended: Option<mcf_serve::hosting::Hosting>,
    /// Why there are no settings, where there are none.
    pub no_settings: Option<String>,
    /// The settings the chosen model was last held under, and since when,
    /// where it has been held: offered on Configure as *as you set it last
    /// time* beside the recommendation (B-475).
    pub last_settings: Option<(mcf_serve::hosting::Hosting, String)>,
    /// The engine MCF said it would build for the chosen model, where there
    /// are no settings because there is nothing to run it on. Named by the
    /// daemon, not worked out here, so that what Host builds is what MCF
    /// would have built from the command line (B-072).
    pub needs_engine: Option<String>,
    /// A card this computer has that no provisioned engine drives: the
    /// component that would, and the daemon's sentence saying so. Read
    /// beside the models, because it is a fact about the machine and not
    /// about any one of them (A21).
    pub card_unused: Option<(String, String)>,
    /// Where a hold can put the chosen model, as the daemon listed them:
    /// each with the build that fits and what the device has free.
    pub placements: Vec<Placement>,
    /// Where the next run puts the model, where the person chose: `None` is
    /// where MCF resolves it to.
    pub on: Option<mcf_serve::control::On>,
    /// Which tab of the model page is open.
    pub tab: Tab,
    /// Which half of Contents is shown: the tensors or the vocabulary.
    pub contents: Page,
    /// The setting being typed into, and what has been typed so far. Applied
    /// on Enter or when another control is pressed; a value that is not a
    /// number is said to be one and not sent (§3.15).
    pub editing: Option<(Field, String)>,
    /// Why the last typed value was not taken.
    pub edit_refused: Option<String>,
    /// What the chosen model's file declares that a hold may start.
    pub declared: Option<mcf_serve::declared::Declared>,
    /// The model to hold once the engine being built is there — the one Host
    /// was pressed for, so that a model chosen meanwhile is not held by a
    /// press that was for another.
    host_after: Option<String>,
    /// The component being built, by name, while a build runs — so the
    /// Components screen can show the build on the card it is for rather
    /// than somewhere else. Set on every build, whichever screen started it.
    pub building: Option<String>,
    /// Which build last stopped badly, and why, in the daemon's words: the
    /// component's name and the refusal. The card for it says so, and a new
    /// build clears it.
    pub build_failed: Option<(String, String)>,
    /// What is being hosted: where it is reachable, and since when.
    pub hosted: Option<Hosted>,
    /// What was last held, where nothing is: read from the daemon, which
    /// read it from the record, so it survives a restart of either (A1).
    pub last_hold: Option<LastHold>,
    /// The held engine's predicting rate, one reading a second while the
    /// Running page is looked at, newest last, for the line that shows it
    /// moving. Cleared when the hold changes.
    pub rates: std::collections::VecDeque<f32>,
    /// Why the last hold was refused, kept until the next is pressed for:
    /// a person who went to ask the model a question is owed the reason it
    /// is not there to ask, on that page (A2).
    pub host_refused: Option<String>,
    /// What the last stop freed, in the daemon's figure, until something
    /// else is held.
    pub freed: Option<String>,
    /// What the chosen model is made of, as the daemon counted it — read
    /// when the screen for it is opened, never counted here (B-072).
    pub anatomy: Option<mcf_serve::anatomy::Said>,
    /// Why there is no anatomy, where there is none: the daemon's refusal,
    /// or that it could not be asked (A2).
    pub no_anatomy: Option<String>,
    /// The context window a measurement is set up for.
    ///
    /// Choosing it implies every power of two below it, which is why the
    /// depths are stated under it rather than offered as a second set of
    /// choices somebody could contradict the first with.
    pub window: u64,
    /// Which dropdown is expanded, if any.
    ///
    /// **One at a time.** Two open lists would overlap each other and the
    /// screen under both, and a click landing in the overlap would belong to
    /// whichever happened to be drawn second.
    pub open: Option<Picker>,
    sampler: mcf_tui::machine::Sampler,
}

impl Desk {
    /// A window that has not asked anything yet.
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
            readings: None,
            queued: std::collections::VecDeque::new(),
            queued_of: 0,
            probes_apply: false,
            components: Vec::new(),
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

    /// Whether the screen showing has a field somebody could be typing into.
    #[must_use]
    pub fn takes_typing(&self) -> bool {
        matches!(
            self.page,
            Page::Adding | Page::Hosting | Page::Prompt | Page::Models
        ) || (self.page == Page::Host && self.editing.is_some())
    }

    /// The longest a pasted value may be, on a screen whose field takes a
    /// name.
    ///
    /// An owner/repository reference and a hub URL are both far shorter than
    /// this. The cap is here because a clipboard can hold a whole document and
    /// a field that accepted one would be a field that stopped drawing.
    const PASTE_LIMIT: usize = 512;

    /// The longest the prompt may be.
    ///
    /// **A prompt is a document.** What somebody analyses is a persona or an
    /// instruction sheet — pages, not a line — and the field it goes into
    /// takes a document (B-430). Sixty-four thousand characters is more than
    /// any context this window's models take, and the daemon says what a
    /// prompt past a model's window costs before it is spent (B-382).
    pub const PROMPT_LIMIT: usize = 65_536;

    /// Adds pasted text to the field, as much of it as is a value.
    ///
    /// **What is on the clipboard was put there by something else.** A
    /// reference copied out of a browser arrives with a trailing newline; one
    /// copied out of a terminal can arrive with a tab or a stray control
    /// character. None of those are part of a name, and a field that kept them
    /// would send them to a hub and report a refusal the person could not see
    /// the cause of. So on a screen whose field takes a name, this takes the
    /// text's first line and drops what is not printable, rather than refusing
    /// a paste that is almost right.
    ///
    /// **On the prompt screen the whole document is the value.** Its line
    /// breaks are where the sentences end and its paragraphs are what a report
    /// takes apart, so they are kept, and only what is neither text nor a
    /// break is dropped.
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

    /// The field typing goes into: whichever of the prompt screen's fields
    /// has the caret, the one field every other screen has otherwise.
    pub fn typing(&mut self) -> &mut String {
        if let (Page::Models | Page::Host, Some((_, typed))) = (self.page, self.editing.as_mut()) {
            return typed;
        }
        match (self.page, self.caret) {
            (Page::Prompt, Caret::Temperature) => &mut self.temperature,
            // The ask screen's own fields, and only there: a caret left
            // pointing at one of them by a screen that has them must not
            // swallow what is typed into a screen that does not.
            (Page::Hosting, Caret::System) => &mut self.system,
            (Page::Hosting, Caret::Effort) => &mut self.effort,
            (Page::Hosting, Caret::Picture) => &mut self.picture,
            // The library's search field, when no setting is being edited.
            (Page::Models, _) => &mut self.filter,
            _ => &mut self.typed,
        }
    }

    /// The same field, to read.
    #[must_use]
    pub fn being_typed(&self) -> &str {
        if let (Page::Models | Page::Host, Some((_, typed))) = (self.page, self.editing.as_ref()) {
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

    /// The temperature the settledness seeds would be drawn at: `None` where
    /// the field is empty and the question is not asked, `Err` with what was
    /// typed where it is not a temperature — which Analyse refuses rather
    /// than runs without, because a choice dropped on the way is a hidden
    /// one (§3.15, A2).
    ///
    /// # Errors
    ///
    /// The text as typed, where it is not a decimal above nought to three
    /// places.
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

    /// What is asked of the daemon from the prompt screen as it stands, or
    /// why nothing would be: the prompt, the unit and the cap, exactly as
    /// Analyse would send them (§3.15).
    #[must_use]
    pub fn taken(&self) -> mcf_serve::prompt::Taken<'_> {
        mcf_serve::prompt::Taken {
            text: self.typed.trim(),
            by: self.by,
            most: self.most,
            extras: self.extras,
        }
    }

    /// What pressing Return means on the screen showing.
    ///
    /// **In a document, Return is a line break.** The prompt field holds
    /// paragraphs, and a Return that ran the analysis would make a field
    /// nobody could write a second line into; the analysis runs from its
    /// button, or from Return with Control held. A field that takes one name
    /// runs on Return as it always has.
    pub fn returned(&mut self, with_control: bool) {
        if self.page == Page::Prompt && !with_control && self.caret == Caret::Document {
            if self.typed.chars().count() < Self::PROMPT_LIMIT {
                self.typed.push('\n');
            }
            return;
        }
        self.entered();
    }

    /// What Return runs on the screen showing.
    pub fn entered(&mut self) {
        match self.page {
            // A setting being typed is applied; otherwise Return searches
            // the hub for words nothing here matches (D51).
            Page::Models if self.editing.is_some() => self.apply_edit(),
            Page::Models => {
                if !self.filter.trim().is_empty() && self.library().is_empty() {
                    self.search_hub();
                }
            }
            Page::Host => self.apply_edit(),
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

    /// Takes whatever a running job has said. Returns whether anything had.
    pub fn hear(&mut self) -> bool {
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
            | Doing::Provisioning(job)
            | Doing::Hosting(job) => job.drain(),
        };
        // A run that finished without a last word — or one whose end was
        // heard a frame ago — still hands on to the next queued run (B-508).
        if !heard {
            self.start_the_next_queued();
            return false;
        }
        // A generation arrives a token at a time, so the text is built as it
        // comes rather than waiting for the end — which is the difference
        // between watching a model answer and watching a blank panel.
        if let Doing::Answering(job) = &self.doing {
            self.said = job
                .answers
                .iter()
                .filter_map(|answer| answer.get("token").and_then(Value::as_text))
                .collect::<Vec<_>>()
                .concat();
        }
        // A model that has just arrived is one this window is holding, and
        // the list says so without anybody asking it to.
        if let Doing::Downloading(job) = &self.doing
            && job.finished
            && job.refused.is_none()
        {
            self.refresh();
            self.settle_download();
        }
        // A model that has just started answering is one MCF is holding, and
        // the screen says where it is without anybody asking it to.
        if let Doing::Hosting(job) = &self.doing
            && job.finished
        {
            // A refusal stays on the page the model would have answered on,
            // until the next hold is pressed for (A2).
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
        self.start_the_next_queued();
        if let Doing::Listing(job) = &self.doing
            && job.finished
        {
            self.keep_the_hub();
            self.keep_the_files();
        }
        // The engine just built is what the model was waiting for: the
        // settings are asked again, now that there is something to run it
        // on, and the hold that was pressed for goes ahead — for the model it
        // was pressed for, if it is still the one chosen.
        if let Doing::Provisioning(job) = &self.doing
            && job.finished
        {
            let refused = job.refused.clone();
            let wanted = self.host_after.take();
            // The card the build was for flips on what the daemon now says
            // of it, which is read rather than assumed (A21).
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
                // Said where it was asked for: on Host when Host started the
                // build, on the component's card either way.
                if wanted.is_some() {
                    self.no_settings = Some(format!("the engine could not be built: {why}"));
                }
                self.build_failed = built.map(|name| (name, why));
            }
        }
        true
    }

    /// Writes a finished measurement onto the test it was a run of.
    ///
    /// **A run of the whole ladder is a run of one test**, "Generation speed
    /// against depth" — that is what `Request::Measure` asks for, so that is
    /// the row whose run time and result it fills. Nothing is written onto the
    /// other four, because nothing measured them: a screen that spread one
    /// run's timing across five rows would be reporting four measurements that
    /// never happened (A7).
    ///
    /// **The lines are the daemon's, not the window's.** Each is a depth and
    /// what was read at it, and a depth MCF could not measure says so rather
    /// than being dropped from the list — an absent row would read as a run
    /// that had nothing to say about that depth.
    fn keep_the_run(&mut self) {
        let Doing::Measuring(job) = &self.doing else {
            return;
        };
        mcf_tui::screens::diagnostics::keep_the_ladder(&mut self.tests, job);
    }

    /// Writes a finished cross-check onto the row that asked for it, in the
    /// console's words (B-072).
    fn keep_the_cross_check(&mut self) {
        let Doing::CrossChecking(job) = &self.doing else {
            return;
        };
        mcf_tui::screens::diagnostics::keep_the_cross_check(&mut self.tests, job);
    }

    /// Shows one answer beside the figures, or — pressing the one already
    /// shown — puts the answer as written back, so the rows are one control
    /// rather than a mode nothing leaves.
    fn show(&mut self, shown: Shown) {
        self.shown = if self.shown == Some(shown) {
            None
        } else {
            Some(shown)
        };
    }

    /// Does what a screen said a click meant.
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
            // A second click on the open picker shuts it, which is what every
            // dropdown does and what a reader tries first.
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
            // The loop's: closing is the window's own, and so is the clipboard.
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
                // A held model chosen is the page's subject: nothing pending
                // and no hub page over it (B-485, B-486).
                self.pending = None;
                self.hub_chosen = None;
                let _was = self.scrolls.remove(&Region::Page);
                // **What this model would be held under.** `host_it` needs it
                // and nothing fetched it: `read_settings` existed, was never
                // called, and so `settings` was `None` for the life of the
                // window — which made the Host button return early and do
                // nothing at all, silently. A control that does nothing is
                // worse than one that refuses (§3.15, A7).
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

    /// Asks MCF what the chosen model would run under.
    ///
    /// Nothing is started: this fills in a form. It is asked again whenever
    /// the chosen model changes, because a recommendation is about a model
    /// and a machine and neither is the one it was computed for any more.
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
                        // To the second: what a person reads *since* by.
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

    /// Asks MCF what the chosen model is made of.
    ///
    /// Synchronous, like [`Self::read_settings`]: the daemon reads a header
    /// and a tensor directory, which is milliseconds, and the answer is
    /// wanted before the screen it was asked for draws.
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

    /// Asks MCF what it is holding.
    pub fn read_hosted(&mut self) {
        // Held rather than replaced: a poll that went unanswered says nothing
        // about what is hosted, and blanking the screen on it would report
        // MCF's own busyness as the model being gone.
        let answered = ask_within(&self.socket, &Request::Hosted, POLL).ok();
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
                        in_use: answer.body.get("use").map(Use::from_value),
                    }),
                // Served, and nothing is held: that is an answer, and it clears.
                Ok(answer) if answer.served => None,
                // Unanswered. Keep what was there and say the daemon is busy.
                _ => {
                    self.busy = true;
                    return;
                }
            };
        self.busy = false;
        // The rate, kept: a hold that changed starts its line afresh.
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

    /// Holds the chosen model under the settings as they stand, and goes to
    /// the page that shows it loading, then answering: one page for the
    /// thing being held rather than a button here, a clock there and an ask
    /// box somewhere else.
    pub fn host_it(&mut self) {
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.no_settings = Some("choose a model first".to_owned());
            return;
        };
        self.host_refused = None;
        self.freed = None;
        self.page = Page::Hosting;
        // No engine is not a refusal but a step: MCF names what it would
        // build, and Host builds it — on screen, with the name, and recorded —
        // and holds the model once it is there. The operator pressed Host;
        // the build is what holding costs on this machine (B-367, §3.15).
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
        // Refused in words rather than by doing nothing. What settings a model
        // would run under is the daemon's to say, and where it will not say,
        // that is the answer and it belongs on the screen.
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
                settings: settings.to_value(),
            },
            format!("holding {}", held.name),
        ));
    }

    /// Builds one component the operator named on the Components screen.
    ///
    /// One job at a time: while anything else runs, the button is not drawn,
    /// and a press that reached here anyway does nothing rather than start a
    /// second build beside a first (A6).
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

    /// Stops holding whatever is held.
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

    /// How the load is going, where one is: what the engine holds of the
    /// model so far, after how long, and about how long is left once there
    /// is a rate to read that off. `None` where nothing is loading.
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
        // Onto a card, the card's memory is the figure that grows; the
        // engine's own does not show weights that went there.
        let read = match figure("card_bytes") {
            Some(on_card) => Some((on_card, true)),
            None => figure("resident_bytes").map(|resident| (resident, false)),
        };
        Some(match read {
            Some((read, on_card)) => loading_said(read, on_card, figure("of_bytes"), job.ran()),
            None => format!("Loading · {} s — no progress reported yet", job.ran()),
        })
    }

    /// The model behind whatever is being held, where this window is also
    /// listing it.
    ///
    /// `Hosted` carries the path the daemon holds it under; everything else
    /// worth saying about it — what a token of context costs, how large the
    /// weights are — is already on the list entry, and asking the daemon again
    /// for figures this window has would be a second answer to a settled
    /// question.
    #[must_use]
    pub fn hosted_model(&self) -> Option<&Model> {
        let hosting = self.hosted.as_ref()?;
        self.models.iter().find(|held| held.path == hosting.model)
    }

    /// Moves one setting on to its next value.
    ///
    /// Cycling rather than typing, because every one of these has a small set
    /// of values that make sense and a field would let somebody type a
    /// context of seven.
    pub fn cycle(&mut self, at: usize) {
        let (Some(settings), Some(recommended)) =
            (self.settings.as_mut(), self.recommended.as_ref())
        else {
            return;
        };
        match at {
            // Powers of two, never past what MCF worked out fits.
            0 => {
                settings.context = if settings.context >= recommended.context {
                    512
                } else {
                    settings.context.saturating_mul(2)
                };
            }
            // Where the model goes, in the daemon's own list: the next
            // placement after the one the settings are at, and with it the
            // build that fits — never the card's build with its layers moved
            // (F176). Without a list, the layers alone, as before.
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
            // The engine, the device and the key are not cycled: the first
            // two are what MCF resolved and changing one without the other
            // would be asking for a build to use a device it cannot, and a
            // key is typed rather than chosen.
            _ => {}
        }
    }

    /// Asks a hub what it publishes under what has been typed.
    pub fn look_up(&mut self) {
        let asked = self.typed.trim().to_owned();
        if asked.is_empty() {
            return;
        }
        // A reference — owner/name, or a hub URL — lists a repository's
        // files; anything else is a word, and a word searches the hub for
        // repositories to pick from (A2).
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

    /// Asks what the typed prompt does to the chosen model.
    ///
    /// Many generations behind one request, so it is a job like a measurement
    /// rather than something the window waits on: a screen that froze for
    /// minutes is one a person cannot tell from a broken one (B-227).
    pub fn report_prompt(&mut self) {
        let taken = self.taken();
        if taken.text.is_empty() {
            return;
        }
        // Not a temperature is not *no temperature*: the page says what was
        // typed is not one, and nothing runs until it is or is gone.
        let Ok(temperature) = self.settle() else {
            return;
        };
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        self.doing = Doing::Reporting(job::Job::start(
            &self.socket,
            Request::PromptReport {
                // The window has no way to ask for a turn yet; the socket
                // takes one (B-455), and the window's turn is B-462.
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

    /// Fetches one published file.
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

    /// Times the chosen model.
    pub fn measure(&mut self, at: usize, deepest: u64) {
        let Some(held) = self.models.get(at) else {
            return;
        };
        self.chosen = Some(at);
        self.doing = Doing::Measuring(job::Job::start(
            &self.socket,
            Request::Measure {
                // The plain load: the window times what the model does as
                // its file lays it out, and a switch is asked for at the
                // prompt (B-463).
                started: mcf_serve::declared::Started::default(),
                model: held.path.clone(),
                engine: None,
                on: self.on,
                deepest,
            },
            format!("measuring {}", held.name),
        ));
    }

    /// Runs what is ticked: the ladder, then the cross-check, each only if a
    /// row it answers is chosen.
    ///
    /// One after the other rather than at once, because both want the engine
    /// and the machine's memory to themselves, and a cross-check that ran
    /// beside a timing would have changed the timing (A6).
    pub fn run_card(&mut self, card: Card) {
        let Some(at) = self.chosen else {
            return;
        };
        self.run_card_on(at, card);
    }

    /// Runs one diagnostic on the chosen model: a probe or a measurement
    /// on its own, else the run it is (D53).
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
            other => self.run_card_on(at, other.card()),
        }
    }

    /// Runs every diagnostic in turn: the ladder, the cross-check, every
    /// probe, every measurement — the next starting as the last finishes,
    /// the whole stopping where one is refused or stopped (B-508).
    pub fn run_all(&mut self) {
        if self.chosen.is_none() || self.doing.busy() {
            return;
        }
        self.queued = Self::EVERY_RUN.iter().copied().collect();
        self.queued_of = self.queued.len();
        self.start_the_next_queued();
    }

    /// The runs a Run all takes, in order. The prompt analysis needs a
    /// prompt and the comparison runs at the command line, so neither is
    /// in it.
    pub const EVERY_RUN: [Card; 6] = [
        Card::Throughput,
        Card::CrossCheck,
        Card::Capabilities,
        Card::Performance,
        Card::Fidelity,
        Card::Behaviour,
    ];

    /// Starts the next queued run where the one going has finished well;
    /// a run refused or stopped ends the sequence, and says so by leaving
    /// the refusal where it is (A7).
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

    /// How far the run going has got, as a fraction, where its stream
    /// says: a step of how many for the probes and the measurements, a
    /// rung of how many for the ladder, the cross-check's two halves.
    /// `None` where nothing is running or the run does not say (B-509).
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
            fraction_of(done.max(0), of)
        };
        match &self.doing {
            Doing::Probing(_) | Doing::Examining(_) => of_step(),
            Doing::Measuring(_) => {
                let so_far = latest?.get("so_far").and_then(Value::as_integer);
                let of = latest?.get("of").and_then(Value::as_integer);
                match (so_far, of) {
                    (Some(so_far), Some(of)) => fraction_of(so_far, of),
                    _ => Some(0.0),
                }
            }
            Doing::CrossChecking(job) => Some(if job.answers.len() >= 2 { 0.5 } else { 0.05 }),
            // A prompt report's stream does not say how far it is.
            _ => None,
        }
    }

    /// The whole sequence's fraction while a Run all goes: the runs done
    /// and the one going's own fraction over how many there were (B-509).
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

    /// Which run of how many the Run all is on, where one is going.
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
            // Run at the command line until the daemon carries it; its
            // card says so and offers the command.
            Card::Comparison => {}
        }
    }

    /// How far a region has been scrolled.
    #[must_use]
    pub fn scrolled(&self, region: Region) -> f32 {
        self.scrolls.get(&region).copied().unwrap_or(0.0)
    }

    /// The library's entries, one a repository, each with the held models
    /// that are its quantizations and pass the search field: every one where
    /// the field is empty; else those whose name, architecture or path carry
    /// the words, case aside (D51, B-485, B-486).
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

    /// One act on the filters: the toggle, or one picker set by its place
    /// in the list it offers, the first of which is *any* (B-489).
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

    /// The architectures held, each once, in order: what the architecture
    /// filter offers after *any* (B-489).
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

    /// The quantizations of the page's repository: the files held, then the
    /// files the hub publishes that are not here, where the hub has been
    /// asked (B-486).
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

    /// The repository the page is about: the pending quantization's, or the
    /// chosen model's.
    #[must_use]
    pub fn subject_repository(&self) -> Option<String> {
        if let Some(pending) = &self.pending {
            return Some(pending.repository.clone());
        }
        self.chosen
            .and_then(|at| self.models.get(at))
            .and_then(|held| held.repository.clone())
    }

    /// Which quantization the page is on, by its place in the list.
    #[must_use]
    pub fn quantization_at(&self) -> Option<usize> {
        let listed = self.quantizations();
        match &self.pending {
            Some(pending) => listed.iter().position(|quant| quant.file == pending.file),
            None => listed.iter().position(|quant| quant.here == self.chosen),
        }
    }

    /// A second click on the open picker shuts it, which is what every
    /// dropdown does and what a reader tries first. Opening the quantization
    /// list asks the hub what else the repository publishes, once (B-486).
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

    /// Picks one quantization: one here becomes the page's subject; one on
    /// the hub becomes the subject as not downloaded (B-486).
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

    /// Makes one file of the open hub repository the page's subject as not
    /// downloaded, by its place in what the hub publishes (B-487).
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

    /// Downloads the subject that is not here, and keeps what to do once
    /// it is: the download goes first, and the thing follows on the file
    /// (B-487, D51).
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

    /// Once a download has finished and the library has been read again:
    /// the file that was pending is the subject now, and what the button
    /// named happens on it (B-487).
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

    /// Asks the hub what the page's repository publishes, where it has not
    /// been asked and nothing else is going.
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

    /// Keeps what the hub publishes for a repository, once the job has it.
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

    /// Whether the hub's answer on show is for the words in the field.
    #[must_use]
    pub fn hub_matches(&self) -> bool {
        self.hub
            .as_ref()
            .is_some_and(|hub| hub.query == self.filter.trim())
    }

    /// Asks the hub for the words in the field — the request `mcf pull
    /// <word>` sends — and lists what it answers under what is here.
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

    /// Opens one hub repository's page: its files are looked up, and the
    /// page shows them with a way to get each (B-485).
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

    /// What a frame does when the search job has answered, for a review
    /// that builds the state by hand rather than through a socket.
    pub fn hear_for_review(&mut self) {
        self.keep_the_hub();
        self.keep_the_files();
    }

    /// Keeps what the hub answered a search with, once the job has it.
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

    /// Runs every probe on the chosen model — the same request `mcf probe`
    /// sends (B-478, A22).
    pub fn probe(&mut self, at: usize) {
        self.probe_only(at, Vec::new());
    }

    /// Runs the probes named on the chosen model; none named is every one.
    pub fn probe_only(&mut self, at: usize, only: Vec<String>) {
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

    /// Runs a family's measurements on the chosen model — the same request
    /// `mcf examine` sends (D52, A22).
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

    /// Runs the measurements named on the chosen model.
    pub fn examine_only(&mut self, at: usize, only: Vec<String>) {
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

    /// The diagnostic running now, where one is: which row the list marks
    /// and which pane shows a Stop (D53).
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
            _ => None,
        }
    }

    /// The last finding of a probe or a measurement on the chosen model.
    #[must_use]
    pub fn finding_of(&self, diagnostic: Diagnostic) -> Option<&Finding> {
        let method = diagnostic.method()?;
        let recorded = diagnostic.readings_method();
        let held = self.chosen.and_then(|at| self.models.get(at))?;
        // A finding from a run this window made carries the run's name;
        // one from the record carries the record's, and both are it.
        held.probed.iter().find(|finding| {
            finding.name == method || recorded.is_some_and(|name| finding.name == name)
        })
    }

    /// When a diagnostic last ran on the chosen model, as the record wrote
    /// it, and through what where the record said (D53).
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
        }
    }

    /// Keeps what a finished examination found on the model it ran on,
    /// beside the probes' findings: a measurement taken again replaces
    /// its last reading, and one not taken keeps it (D52).
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

    /// Shows one diagnostic whole: its pane from the top, with the chosen
    /// model's readings fetched where they are not held yet (D53, D54).
    fn show_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostic = diagnostic;
        let _was = self.scrolls.insert(Region::Diagnostics, 0.0);
        self.read_readings();
    }

    /// Asks the daemon for the chosen model's readings, where they are not
    /// held already for this model (D54, B-516).
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

    /// Asks the daemon for the chosen model's readings again, after a run
    /// wrote some.
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

    /// The newest readings run of a diagnostic on the chosen model, where
    /// the daemon has answered with one.
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

    /// Keeps what a finished probe run found on the model it ran on, and
    /// reads the settings again, since the probes may have applied some.
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

    /// One act on the Configure tab. Whatever was being typed is applied
    /// first, so that a value left in a field is not lost to the next press.
    fn configure(&mut self, act: &Act) {
        match *act {
            Act::Tab(tab) => {
                self.apply_edit();
                self.tab = tab;
                self.open = None;
                // The file's contents are counted when first looked at,
                // and kept for the model chosen.
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

    /// Starts typing into a setting, with what it holds now as the text;
    /// whatever was being typed before is applied first.
    pub fn edit(&mut self, field: Field) {
        self.apply_edit();
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
        };
        self.editing = Some((field, now));
        self.edit_refused = None;
        self.caret = Caret::Setting;
    }

    /// Takes what was typed into the setting it was typed for, or says why
    /// not and leaves the setting as it was. A number is a number: a window
    /// of *lots* is refused with the word, not read as nought (A7, §3.15).
    pub fn apply_edit(&mut self) {
        let Some((field, typed)) = self.editing.take() else {
            return;
        };
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        let typed = typed.trim().replace([',', '_'], "");
        let not_a_number = |what: &str| Some(format!("{what} wants a whole number, not {typed:?}"));
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
        };
    }

    /// Flips a switch on the Configure tab.
    pub fn flip(&mut self, switch: Switch) {
        let Some(settings) = self.settings.as_mut() else {
            return;
        };
        match switch {
            Switch::FlashAttention => settings.flash_attention = !settings.flash_attention,
            Switch::KeepResident => settings.keep_resident = !settings.keep_resident,
            Switch::DraftHead => settings.started.draft_head = !settings.started.draft_head,
            // Text only, or the one beside the file: the recommendation
            // knows which projector that is, and *on* means that one.
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

    /// Puts the hold where the daemon's list says, engine, device and layers
    /// together (F176).
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

    /// Which placement the settings are at, where they are at one.
    #[must_use]
    pub fn placed_at(&self) -> Option<usize> {
        let settings = self.settings.as_ref()?;
        self.placements.iter().position(|held| {
            held.engine == settings.engine
                && held.device == settings.device
                && held.gpu_layers == settings.gpu_layers
        })
    }

    /// Holds again what was last held: the model is chosen by its path, its
    /// settings read as they would be for any hold, and Host pressed.
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

    /// Cuts the run that is going short: the ladder, the cross-check or the
    /// prompt analysis. The daemon stops the engine at its next glance and
    /// asks no further generation, and what was heard stays on the page
    /// (A7, B-479).
    pub fn stop_run(&mut self) {
        self.queued.clear();
        self.queued_of = 0;
        match &mut self.doing {
            Doing::Measuring(job)
            | Doing::CrossChecking(job)
            | Doing::Reporting(job)
            | Doing::Probing(job)
            | Doing::Examining(job) => {
                job.stop();
            }
            _ => {}
        }
    }

    /// Reads what the provisioned engine produces from the chosen model with
    /// MCF's own engine — the same request `mcf cross-check` sends (B-072).
    pub fn cross_check(&mut self, at: usize) {
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

    /// What the window asks of the model's own template, where anything was
    /// asked: `None` is nothing switched, which is not the same as every
    /// switch at its default position said out loud (D43, §3.15).
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

    /// Thinking on, off, or unsaid, in that round: three positions, because
    /// *unsaid* is a position and not the absence of one (D43).
    pub fn cycle_thinking(&mut self) {
        self.thinking = match self.thinking {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
    }

    /// Asks the chosen model what has been typed.
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
                // A person's, which is what this window is for. B-146 and
                // §6.8: whose text it is travels with the request rather than
                // being inferred at the far end, and a window is never a
                // probe.
                whose: mcf_record::content::Whose::User,
                pinned: false,
                // The template's own switches, as `mcf run` sends them: a
                // question asked inside a system turn is a question asked
                // where it will live, and nothing is switched unless
                // somebody switched it (B-462, D43, D47).
                turn: turn.clone(),
                // A picture, where a path was typed. The daemon reads the
                // file and refuses in its own words where it cannot, which
                // is what the panel shows (B-452, A2).
                image: picture,
                started: mcf_serve::declared::Started::default(),
            },
            format!("asking {}", held.name),
        ));
    }

    /// Asks MCF what it is holding.
    /// Asks what MCF can build and what is already here.
    ///
    /// A refusal leaves the list alone rather than emptying it: a daemon that
    /// stopped answering has not un-built anything, and a screen that went
    /// blank would say it had.
    pub fn read_components(&mut self) {
        if let Ok(answer) = ask_within(&self.socket, &Request::Components, POLL)
            && answer.served
            && let Some(listed) = answer.body.get("components").and_then(Value::as_list)
        {
            self.components = listed.iter().map(component_from).collect();
        }
    }

    /// Asks MCF what it is holding.
    ///
    /// **An unanswered reading leaves the list alone.** The daemon answers one
    /// client at a time by decision (DEC-012), so while it loads a model —
    /// minutes, for a large one — it answers nothing at all. Emptying the list
    /// on that told an operator *no models exist* at the moment MCF was busy
    /// with one of them, which is the most misleading thing this screen can
    /// say: the models are on the disk, `mcf list` finds them, and the only
    /// thing that changed is that MCF was mid-answer.
    ///
    /// So a reading replaces the list and a silence does not (A7).
    pub fn refresh(&mut self) {
        match ask_within(&self.socket, &Request::Holding, POLL) {
            Ok(answer) if answer.served => {
                self.refusal = None;
                self.busy = false;
                // **A companion is not a model and is not offered.** A vision
                // projector carries no transformer and answers no prompt:
                // pointed at one, an engine loads it and produces nothing. It
                // was in this list, so the window offered it to be hosted and
                // to be measured, and both could only ever fail.
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
            // **Unanswered in time is not *not up*.** A daemon loading a
            // large model answers nothing for minutes; reporting that as a
            // dead daemon tells an operator the opposite of what is happening.
            // Only a connection that could not be made at all is a refusal.
            Err(why) if why.contains("not answering on this computer") => {
                self.busy = false;
                self.refusal = Some(why);
            }
            Err(_) => self.busy = true,
        }
    }

    /// Takes a reading of the machine.
    pub fn sample(&mut self) {
        self.reading = self.sampler.read();
    }

    /// The word at the right of the menu bar.
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

    /// What is running and how long it has run, where anything is: the
    /// job's own sentence and the seconds since it was asked for. On every
    /// page rather than the one that started it, because a measurement is
    /// minutes and the operator may have gone to look at something else
    /// (A7). `None` where nothing is running.
    #[must_use]
    pub fn under_way(&self) -> Option<String> {
        let job = self.doing.job().filter(|job| !job.finished)?;
        Some(format!("{}, {} s so far", job.what, job.ran()))
    }

    /// The line under the monitor's divider: what MCF is doing, and what that
    /// means. The console's two states, in the console's words.
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

    /// The depths a run would sample, as the console states them.
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

    /// The window a Quick Run uses: the shallowest rung and one above it, so
    /// that it is a fall-off rather than a single number, and quick.
    #[must_use]
    pub fn quick_depth(&self) -> u64 {
        mcf_tui::screens::diagnostics::QUICK_DEPTH
    }

    /// Roughly how long the throughput run takes, as a range: the whole
    /// ladder to the chosen window, or the quick climb.
    ///
    /// A range because MCF's own estimates land between 0.58× and 1.42× of
    /// what runs actually take, and a single number would be a promise it
    /// cannot keep.
    #[must_use]
    pub fn estimate(&self, quick: bool) -> (u64, u64) {
        // A quick run is the ladder only, and only to `QUICK_DEPTH`; the
        // console prints the same two figures (B-072).
        let seconds: u64 = if quick {
            mcf_tui::screens::diagnostics::quick_seconds(&self.tests)
        } else {
            self.seconds_of(Run::Ladder)
        };
        Self::spread(seconds)
    }

    /// Roughly how long the cross-check takes, as a range.
    #[must_use]
    pub fn cross_check_estimate(&self) -> (u64, u64) {
        Self::spread(self.seconds_of(Run::CrossCheck))
    }

    /// The console's own estimate for one run, on the row that names it.
    fn seconds_of(&self, run: Run) -> u64 {
        self.tests
            .iter()
            .find(|test| test.run == run)
            .and_then(|test| test.seconds)
            .unwrap_or(30)
    }

    /// The range MCF's estimates land in against what runs take.
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

    /// The headline on *Your computer*: what this machine can run.
    ///
    /// Derived from the free memory on the largest card, or from system memory
    /// where there is no card. It says *about* because the figure moves as
    /// other programs come and go.
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

    /// The graphics card, in a phrase.
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

    /// System memory, in a phrase.
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

    /// The processor, in a phrase.
    #[must_use]
    pub fn processor_sentence(&self) -> String {
        self.reading.processor.cores.map_or_else(
            || words::UNMEASURED.to_owned(),
            |cores| format!("{cores} cores"),
        )
    }

    /// Whether MCF is doing anything.
    #[must_use]
    pub fn doing_sentence(&self) -> String {
        if self.refusal.is_some() {
            "MCF is not answering".to_owned()
        } else {
            "Nothing running".to_owned()
        }
    }
}

/// Opens the window and runs until it is closed.
///
/// # Errors
///
/// What SDL said, that the window library is not provisioned, or that no font
/// could be found on this computer.
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

    let mut mouse = ui::Mouse::default();
    let mut last = std::time::Instant::now();
    loop {
        mouse.settle();
        let mut acted = false;
        // The window is there: this loop only runs where one was opened.
        while let Some(event) = paint.window().and_then(sdl::Window::next_event) {
            acted = true;
            match sdl::event_type(&event) {
                sdl::EVENT_QUIT => return Ok(()),
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
                // Typing. It arrives already composed, so a layout, a
                // modifier or an input method is the platform's business and
                // not something this spells out of keycodes.
                sdl::EVENT_TEXT_INPUT => {
                    if desk.takes_typing()
                        && let Some(text) = sdl::event_text(&event)
                    {
                        desk.typing().push_str(&text);
                    }
                }
                sdl::EVENT_KEY_DOWN => match sdl::event_key(&event) {
                    sdl::KEY_ESCAPE => return Ok(()),
                    // Paste. Typing arrives already composed as text input,
                    // but a paste never does: Ctrl+V is a key event and the
                    // characters are on the clipboard, so a field that only
                    // read text input could be typed into and not pasted
                    // into — which is what a person hits first with an
                    // owner/repository name they copied from a browser.
                    key if key == u32::from(b'v')
                        && sdl::event_has_ctrl(&event)
                        && desk.takes_typing() =>
                    {
                        if let Some(text) = paint.window().and_then(sdl::Window::clipboard_text) {
                            desk.paste(&text);
                        }
                    }
                    // Copy. The window draws its own text, so nothing in it
                    // is a thing a window manager can select: the document
                    // in the prompt field leaves by Ctrl+C, whole, or it
                    // does not leave at all.
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
                    // `q` closes the window — except where somebody is
                    // typing, when it is a letter. A field that ate the
                    // application on the letter q would be a field nobody
                    // could type a name into.
                    key if key == u32::from(b'q') && !desk.takes_typing() => return Ok(()),
                    key if key == u32::from(b'r') && !desk.takes_typing() => {
                        desk.refresh();
                        desk.sample();
                        desk.read_hosted();
                        desk.read_components();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // Anything a running job has said since the last frame.
        if desk.hear() {
            acted = true;
        }

        // A second between readings, and only where they are shown — the same
        // rule the console follows, for the same reason: an idle window should
        // not be why a fan is running (B-071).
        let due = matches!(desk.page, Page::Monitor | Page::Hosting)
            && last.elapsed() >= std::time::Duration::from_secs(1);
        if due {
            match desk.page {
                Page::Monitor => desk.sample(),
                // Running: what the held engine is doing, read off its own
                // counters once a second while somebody is looking (B-071).
                _ => desk.read_hosted(),
            }
            last = std::time::Instant::now();
            acted = true;
        }

        if let Some(act) = view::draw(&mut paint, &desk, &mouse) {
            taken(&mut paint, &mut desk, act);
            acted = true;
        }
        let _ = acted;

        // Nothing to do until something happens.
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

/// A count over a count as a fraction between nought and one; `None` of
/// nothing.
fn fraction_of(done: i64, of: i64) -> Option<f32> {
    if of <= 0 {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "counts of a few steps")]
    Some((done as f32 / of as f32).clamp(0.0, 1.0))
}

/// Findings just taken join the ones held: a method taken again replaces
/// its last finding, and one not taken keeps it (D52, D53).
fn keep_findings(held: &mut Vec<Finding>, found: Vec<Finding>) {
    for finding in found {
        match held.iter_mut().find(|had| had.name == finding.name) {
            Some(entry) => *entry = finding,
            None => held.push(finding),
        }
    }
}

/// What a stepped run said, by step: each step's name with the lines the
/// daemon wrote for it, taken now, the steps with none left out.
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

/// A record's time as a person reads it: the day and the minute, without
/// the nanoseconds and the offset the record keeps (D53).
#[must_use]
pub fn when_said(at: &str) -> String {
    at.get(..16)
        .map_or_else(|| at.to_owned(), |head| head.replace('T', " "))
}

impl Desk {
    /// A splitter dragged to a position: it is held from here until the
    /// button is let go (F195).
    fn split(&mut self, splitter: Splitter, to: i32) {
        self.grabbed = Some(splitter);
        self.splits.set(splitter, as_points(to));
    }

    /// The button was let go: whatever a press took hold of is dropped.
    pub fn released(&mut self) {
        self.grabbed = None;
    }
}

/// One act from the screen, done: by the window where it is the window's —
/// the clipboard — and by the desk otherwise.
fn taken(paint: &mut paint::Painter, desk: &mut Desk, act: Act) {
    if let Act::Copy(text) = &act
        && let Some(window) = paint.window()
    {
        let _went = window.put_on_clipboard(text);
    }
    desk.act(act);
}

/// A pointer position in pixels, as the points everything is laid out in.
fn points(paint: &paint::Painter, at: (f32, f32)) -> (f32, f32) {
    let scale = if paint.scale > 0.0 { paint.scale } else { 1.0 };
    (at.0 / scale, at.1 / scale)
}

#[cfg(test)]
mod tests;
