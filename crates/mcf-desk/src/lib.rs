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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosted {
    /// Which model, by the path the daemon holds it under.
    pub model: String,
    /// Where a caller reaches it. The one fact an API is for.
    pub address: String,
    /// Since when, as the daemon stamped it.
    pub since: String,
    /// The context window it is held at, where the daemon said.
    pub context: Option<u64>,
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
        (Self::Monitor, "Monitor"),
        (Self::Models, "Models"),
        (Self::Diagnostics, "Diagnostics"),
        (Self::Components, "Components"),
        (Self::Settings, "Settings"),
        (Self::Exit, "Exit"),
    ];

    /// Which entry in the column should be lit while this page shows.
    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Monitor => Self::Monitor,
            // Host was a second entry for the list Models already shows —
            // `view::host` draws both — so the column carried one screen
            // twice. The screens its actions lead to belong to Models now,
            // and the menu still shows where you came from.
            Self::Host | Self::Adding | Self::Hosting | Self::Models => Self::Models,
            Self::Diagnostics | Self::Prompt => Self::Diagnostics,
            Self::Components => Self::Components,
            Self::Settings => Self::Settings,
            Self::Exit => Self::Exit,
        }
    }
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
    /// Whether that device is a graphics card.
    pub on_a_card: bool,
    /// Why it will not run, where it will not.
    pub refused: Option<String>,
    /// Tokens a second, where it has been measured.
    pub speed: Option<f64>,
    /// Seconds to become ready.
    pub wakes: Option<f64>,
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

/// One measurement that can be asked for, as the console lists them.
#[derive(Debug, Clone)]
pub struct Test {
    /// What it measures, in words.
    pub name: &'static str,
    /// Which devices it needs.
    pub devices: &'static str,
    /// Roughly how long, in seconds, at this machine's speed. An estimate, and
    /// named one on the screen: MCF's own estimate lands between 0.58x and
    /// 1.42x of what a run takes, so a column headed `time` beside a column of
    /// measured times would have read as the same kind of number.
    pub seconds: u64,
    /// Whether it is selected.
    pub chosen: bool,
    /// How long the last run of this test actually took, in seconds.
    ///
    /// **`None` until it has run, and `None` is not zero** (A7). A test that
    /// has never run has no run time, and the screen draws a dash rather than
    /// a figure somebody could read as *instant*.
    pub ran: Option<u64>,
    /// What the last run found, in the words the daemon used.
    ///
    /// **Absent until there is a result to show**, which is what the results
    /// button on the screen is enabled by. Nothing here is written by the
    /// window: every line comes from the answer MCF sent.
    pub result: Option<Vec<String>>,
}

/// The tests MCF knows how to run.
///
/// The console's list, because it is the same set of measurements and a second
/// surface offering a different five would make *what MCF can measure* a fact
/// about which surface you opened.
#[must_use]
pub fn tests() -> Vec<Test> {
    vec![
        Test {
            name: "Generation speed against depth",
            devices: "both",
            seconds: 180,
            chosen: true,
            ran: None,
            result: None,
        },
        Test {
            name: "Cold start cost",
            devices: "both",
            seconds: 25,
            chosen: true,
            ran: None,
            result: None,
        },
        Test {
            name: "Memory ceiling — largest context",
            devices: "both",
            seconds: 120,
            chosen: false,
            ran: None,
            result: None,
        },
        Test {
            name: "CPU and GPU agree on the output",
            devices: "needs both",
            seconds: 90,
            chosen: false,
            ran: None,
            result: None,
        },
        Test {
            name: "Prompt reading speed",
            devices: "both",
            seconds: 45,
            chosen: false,
            ran: None,
            result: None,
        },
    ]
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

    /// What was measured at the largest window this machine allows.
    #[must_use]
    pub fn speed_at_window(&self) -> String {
        self.slowest.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |ms| format!("{ms:.2} ms/token"),
        )
    }

    /// How long it takes to become ready.
    #[must_use]
    pub fn cold_start(&self) -> String {
        self.wakes.map_or_else(
            || crate::view::UNKNOWN.to_owned(),
            |seconds| format!("{seconds:.1} s"),
        )
    }

    /// Whether anything has been measured about it.
    #[must_use]
    pub fn measured(&self) -> bool {
        self.fastest.is_some() || self.slowest.is_some() || self.wakes.is_some()
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
fn model_from(held: &Value) -> Model {
    let text = |key: &str| held.get(key).and_then(Value::as_text).map(str::to_owned);
    let path = text("path").unwrap_or_default();
    let name = path
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("a model")
        .trim_end_matches(".gguf")
        .to_owned();
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
        wakes: None,
        fastest: measured.as_ref().and_then(|held| held.fastest),
        slowest: measured.as_ref().and_then(|held| held.slowest),
        ladder: measured.map(|held| held.ladder).unwrap_or_default(),
    }
}

/// Milliseconds a token, as tokens a second.
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
}

/// Reads the ends out of what the record kept.
///
/// Only the depths that *separated*: a rung the arithmetic could not measure
/// is not a slow one, and letting it stand in for the deepest reading would
/// put a number where there is none (A7, A9).
fn measured_ends(held: &Value) -> Measured {
    let mut ends = Measured::default();
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
}

/// The context windows a measurement can be set up for.
///
/// **Powers of two, because a context window is asked for in them**, and every
/// one below the chosen depth is sampled — which is why this is a list to pick
/// from rather than a number to type.
#[must_use]
pub const fn windows() -> [u64; 7] {
    [1_024, 2_048, 4_096, 8_192, 16_384, 32_768, 65_536]
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
    /// Ask a hub what it publishes under what has been typed.
    LookUp,
    /// Fetch one published file.
    Download {
        /// The repository.
        reference: String,
        /// The file within it.
        file: String,
    },
    /// Time the chosen model.
    Measure {
        /// The deepest context to sample.
        deepest: u64,
    },
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
    /// Close whatever dropdown is open, choosing nothing.
    Shut,
    /// Set the context window to one of the offered powers of two.
    SetWindow(u64),
    /// Show, or hide, what one test's last run found.
    Result(usize),
    /// Turn one test on or off.
    Toggle(usize),
    /// Move one hosting setting on to its next value.
    Cycle(usize),
    /// Put every setting back to what MCF recommended.
    Recommended,
    /// Hold the chosen model under the settings as they stand.
    HostIt,
    /// Stop holding it.
    StopHosting,
    /// Close the window.
    Close,
    /// Analyse the typed prompt on the chosen model.
    ReportPrompt,
    /// Put the analysis on the system clipboard.
    CopyAnalysis,
    /// Ask a model what has been typed.
    Ask {
        /// Which, by position in the list.
        at: usize,
    },
    /// Choose a model without leaving the screen.
    Choose(usize),
    /// Empty the field.
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
    /// Asking a hub what it publishes.
    Listing(job::Job),
    /// Fetching a model.
    Downloading(job::Job),
    /// Timing one.
    Measuring(job::Job),
    /// Waiting for a model to answer.
    Answering(job::Job),
    /// Taking a prompt apart.
    Reporting(job::Job),
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
            | Self::Answering(job)
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
    /// Which sentence's absence is being shown on the prompt screen, where one
    /// is. `None` is the answer to the prompt as written.
    pub without: Option<usize>,
    /// How far down a long list has been scrolled, in points.
    pub scroll: f32,
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
    /// Whether the analysis was just put on the clipboard, so the screen can
    /// say so — a button that gives no sign is one somebody presses twice.
    pub copied: bool,
    /// Which model a measurement or a question is about.
    pub chosen: Option<usize>,
    /// What is running.
    pub doing: Doing,
    /// What a model has said so far, this turn.
    pub said: String,
    /// The tests offered on the diagnostics screen.
    pub tests: Vec<Test>,
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
    /// What is being hosted: where it is reachable, and since when.
    pub hosted: Option<Hosted>,
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
    /// Which test's last run is being read, if any.
    pub showing: Option<usize>,
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
            without: None,
            scroll: 0.0,
            reading: mcf_tui::machine::Reading::default(),
            refusal: None,
            busy: false,
            typed: String::new(),
            copied: false,
            chosen: None,
            doing: Doing::Nothing,
            said: String::new(),
            tests: tests(),
            components: Vec::new(),
            settings: None,
            recommended: None,
            no_settings: None,
            hosted: None,
            window: 8192,
            open: None,
            showing: None,
            sampler: mcf_tui::machine::Sampler::new(),
        }
    }

    /// Whether the screen showing has a field somebody could be typing into.
    #[must_use]
    pub fn takes_typing(&self) -> bool {
        matches!(self.page, Page::Adding | Page::Hosting | Page::Prompt)
    }

    /// The longest a pasted value may be.
    ///
    /// An owner/repository reference and a hub URL are both far shorter than
    /// this. The cap is here because a clipboard can hold a whole document and
    /// a field that accepted one would be a field that stopped drawing.
    const PASTE_LIMIT: usize = 512;

    /// Adds pasted text to the field, as much of it as is a value.
    ///
    /// **What is on the clipboard was put there by something else.** A
    /// reference copied out of a browser arrives with a trailing newline; one
    /// copied out of a terminal can arrive with a tab or a stray control
    /// character. None of those are part of a name, and a field that kept them
    /// would send them to a hub and report a refusal the person could not see
    /// the cause of. So this takes the text's first line and drops what is not
    /// printable, rather than refusing a paste that is almost right.
    pub fn paste(&mut self, text: &str) {
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
        let room = Self::PASTE_LIMIT.saturating_sub(self.typed.chars().count());
        if room == 0 {
            return;
        }
        self.typed.extend(kept.chars().take(room));
    }

    /// What pressing Return means on the screen showing.
    pub fn entered(&mut self) {
        match self.page {
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
            | Doing::Answering(job)
            | Doing::Reporting(job)
            | Doing::Hosting(job) => job.drain(),
        };
        if !heard {
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
        }
        // A model that has just started answering is one MCF is holding, and
        // the screen says where it is without anybody asking it to.
        if let Doing::Hosting(job) = &self.doing
            && job.finished
        {
            self.read_hosted();
        }
        if let Doing::Measuring(job) = &self.doing
            && job.finished
        {
            self.keep_the_run();
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
        let ran = job.ran();
        let mut lines: Vec<String> = Vec::new();
        if let Some(why) = &job.refused {
            lines.push(why.clone());
        } else {
            for answer in &job.answers {
                let Some(reading) = answer.get("reading") else {
                    continue;
                };
                let depth = reading
                    .get("depth")
                    .and_then(Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
                    .unwrap_or(0);
                let said = if matches!(reading.get("measured"), Some(Value::Bool(true))) {
                    reading
                        .get("ms_per_token")
                        .and_then(Value::as_text)
                        .map_or_else(
                            || crate::view::UNKNOWN.to_owned(),
                            |ms| format!("{ms} ms a token"),
                        )
                } else {
                    crate::view::UNKNOWN.to_owned()
                };
                lines.push(format!("at {} tokens   {said}", words::grouped(depth)));
            }
            if let Some(conditions) = job.conclusion().and_then(|body| body.get("conditions")) {
                // B65 and D31: which engine ran is a condition of every figure
                // above it, so it travels with them rather than being read off
                // a screen that has moved on.
                let engine = conditions
                    .get("engine_ran")
                    .and_then(Value::as_text)
                    .unwrap_or("MCF did not say");
                lines.push(format!("measured on {engine}"));
            }
        }
        if let Some(test) = self
            .tests
            .iter_mut()
            .find(|test| test.name == "Generation speed against depth")
        {
            test.ran = Some(ran);
            test.result = Some(lines);
        }
    }

    /// Does what a screen said a click meant.
    pub fn act(&mut self, act: Act) {
        match act {
            Act::Go(page) => {
                if page != self.page {
                    self.scroll = 0.0;
                    if page.section() == Page::Monitor {
                        self.sample();
                    }
                    self.page = page;
                }
            }
            Act::LookUp => self.look_up(),
            Act::Download { reference, file } => self.download(&reference, &file),
            Act::Measure { deepest } => {
                if let Some(at) = self.chosen {
                    self.page = Page::Diagnostics;
                    self.measure(at, deepest);
                }
            }
            // A second click on the open picker shuts it, which is what every
            // dropdown does and what a reader tries first.
            Act::Open(picker) => {
                self.open = if self.open == Some(picker) {
                    None
                } else {
                    Some(picker)
                };
            }
            Act::Shut => self.open = None,
            Act::SetWindow(window) => {
                self.window = window;
                self.open = None;
            }
            Act::Result(at) => {
                self.showing = if self.showing == Some(at) {
                    None
                } else {
                    Some(at)
                };
            }
            Act::Toggle(at) => {
                if let Some(test) = self.tests.get_mut(at) {
                    test.chosen = !test.chosen;
                }
            }
            Act::Cycle(at) => self.cycle(at),
            Act::Recommended => self.settings.clone_from(&self.recommended),
            Act::HostIt => self.host_it(),
            Act::StopHosting => self.stop_hosting(),
            // Both are the loop's: closing is the window's own, and copying
            // needs the clipboard, which `act` cannot reach from here.
            Act::Close | Act::CopyAnalysis => {}
            Act::ShowWithout(at) => {
                // Pressing the row already shown puts the answer as written
                // back, so the two are one control rather than a mode nothing
                // leaves.
                self.without = if self.without == Some(at) {
                    None
                } else {
                    Some(at)
                };
            }
            Act::Ask { at } => self.ask(at),
            Act::Choose(at) => {
                self.chosen = Some(at);
                self.open = None;
                // **What this model would be held under.** `host_it` needs it
                // and nothing fetched it: `read_settings` existed, was never
                // called, and so `settings` was `None` for the life of the
                // window — which made the Host button return early and do
                // nothing at all, silently. A control that does nothing is
                // worse than one that refuses (§3.15, A7).
                self.read_settings();
            }
            Act::ReportPrompt => self.report_prompt(),
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
                        &mcf_serve::hosting::Hosting::recommended("", "", false, 0, None, false),
                    )
                });
                self.settings.clone_from(&recommended);
                self.recommended = recommended;
            }
            Ok(answer) => {
                self.no_settings = Some(refused_because(&answer.body));
            }
            Err(why) => self.no_settings = Some(why),
        }
    }

    /// Asks MCF what it is holding.
    pub fn read_hosted(&mut self) {
        // Held rather than replaced: a poll that went unanswered says nothing
        // about what is hosted, and blanking the screen on it would report
        // MCF's own busyness as the model being gone.
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
        self.hosted = read;
    }

    /// Holds the chosen model under the settings as they stand.
    pub fn host_it(&mut self) {
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            self.no_settings = Some("choose a model first".to_owned());
            return;
        };
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
            self.socket.clone(),
            Request::Host {
                model: held.path.clone(),
                settings: settings.to_value(),
            },
            format!("holding {}", held.name),
        ));
    }

    /// Stops holding whatever is held.
    pub fn stop_hosting(&mut self) {
        let _answered = ask(&self.socket, &Request::Unhost);
        self.hosted = None;
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
            1 => settings.gpu_layers = if settings.gpu_layers == 0 { 999 } else { 0 },
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
        self.doing = Doing::Listing(job::Job::start(
            self.socket.clone(),
            Request::Offered {
                reference: asked.clone(),
                from: None,
            },
            format!("looking up {asked}"),
        ));
    }

    /// The prompt analysis as plain text, for taking out of the window.
    ///
    /// **Assembled from the reading rather than scraped off the screen.** What
    /// is drawn is glyphs; what somebody wants to paste into a message is the
    /// figures and the sentences they belong to, in an order that survives
    /// leaving here (A25's shape).
    #[must_use]
    pub fn analysis_as_text(&self) -> Option<String> {
        use std::fmt::Write as _;
        let Doing::Reporting(job) = &self.doing else {
            return None;
        };
        let found = job.conclusion().or_else(|| job.latest())?;
        let named = self
            .chosen
            .and_then(|at| self.models.get(at))
            .map_or("a model", |held| held.name.as_str());
        let count = |key: &str| found.get(key).and_then(Value::as_integer).unwrap_or(0);
        let share = |parts: i64| {
            format!(
                "{}.{}%",
                parts.saturating_div(10_000),
                parts.saturating_div(1_000).rem_euclid(10)
            )
        };
        let mut out = format!("prompt analysis on {named}\n\n");
        let _wrote = write!(out, "prompt:\n{}\n\n", self.typed.trim());

        // **What the screen leads with, led with here too.** This text is what
        // somebody pastes into a message to a colleague, and a report that
        // said *96.1%* without saying the run could not separate anything
        // would travel further than the screen that qualified it (§3.15).
        let floor = count("floor_parts_per_million");
        if floor >= 500_000 {
            let _wrote = writeln!(
                out,
                "THIS RUN CANNOT SEPARATE THESE SENTENCES: removing a sentence carrying no \
                 instruction moved {} of the answer, so a figure near that has told you \
                 nothing.\n",
                share(floor)
            );
        }

        out.push_str("how much each sentence steered the answer:\n");
        for clause in found.get("clauses").and_then(Value::as_list).unwrap_or(&[]) {
            let moved = clause
                .get("moved_parts_per_million")
                .and_then(Value::as_integer)
                .unwrap_or(0);
            let said = clause
                .get("text")
                .and_then(Value::as_text)
                .unwrap_or_default();
            let _wrote = writeln!(out, "  {:>7}  {said}", share(moved));
        }
        let _wrote = writeln!(
            out,
            "\nfloor {} — how much the answer moved for a sentence carrying no instruction. \
             An ordering, not relevance.",
            share(floor)
        );

        out.push_str(&self.words_not_expected(found));

        let _wrote = writeln!(
            out,
            "\n{} gave {} distinct answer(s) — asked at temperature 0, where the seed cannot \
             change the answer, so this measures the sampler rather than the prompt",
            count("seeds_asked"),
            count("distinct_answers")
        );
        if count("prompt_tokens") > 0 {
            let _wrote = writeln!(
                out,
                "\nthe prompt reached the model as {} token(s); each generation stopped at {}",
                count("prompt_tokens"),
                count("token_limit")
            );
        }
        if let Some(said) = found.get("baseline").and_then(Value::as_text) {
            let _wrote = writeln!(out, "\nthe answer to the prompt as written:\n{said}");
        }
        Some(out)
    }

    /// The tokens the model did not expect, for the text that leaves the
    /// window.
    ///
    /// Its own function because `analysis_as_text` is already the length the
    /// workspace allows, and because what goes on a clipboard and what goes on
    /// a screen have to be the same report.
    fn words_not_expected(&self, found: &Value) -> String {
        use std::fmt::Write as _;
        let _ = self;
        let ranked = found
            .get("expected")
            .and_then(Value::as_list)
            .unwrap_or(&[]);
        if ranked.is_empty() {
            return found
                .get("expected_refused")
                .and_then(Value::as_text)
                .map_or_else(String::new, |why| {
                    format!("\nwhich words the model did not expect — not taken: {why}\n")
                });
        }
        let depth = found
            .get("ranked_depth")
            .and_then(Value::as_integer)
            .unwrap_or(0);
        let mut surprising: Vec<(i64, String)> = Vec::new();
        let mut first = 0_usize;
        for held in ranked {
            let said = held
                .get("text")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned();
            match held.get("rank").and_then(Value::as_integer) {
                None => surprising.push((i64::MAX, said)),
                Some(1) => first = first.saturating_add(1),
                Some(rank) => surprising.push((rank, said)),
            }
        }
        surprising.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
        let mut out = String::from(
            "\nwhich words the model did not expect (where each sat in what it would have \
             written itself):\n",
        );
        for (rank, said) in surprising.iter().take(10) {
            let where_it_sat = if *rank == i64::MAX {
                format!("past {depth}")
            } else {
                format!("#{rank}")
            };
            let _wrote = writeln!(out, "  {where_it_sat:>8}  {said:?}");
        }
        let _wrote = writeln!(
            out,
            "  {first} of {} were its own first choice",
            ranked.len()
        );
        out
    }

    /// Asks what the typed prompt does to the chosen model.
    ///
    /// Many generations behind one request, so it is a job like a measurement
    /// rather than something the window waits on: a screen that froze for
    /// minutes is one a person cannot tell from a broken one (B-227).
    pub fn report_prompt(&mut self) {
        let asked = self.typed.trim().to_owned();
        if asked.is_empty() {
            return;
        }
        let Some(held) = self.chosen.and_then(|at| self.models.get(at)) else {
            return;
        };
        self.doing = Doing::Reporting(job::Job::start(
            self.socket.clone(),
            Request::PromptReport {
                model: held.path.clone(),
                prompt: asked,
                seed: 41,
            },
            format!("taking the prompt apart on {}", held.name),
        ));
    }

    /// Fetches one published file.
    pub fn download(&mut self, reference: &str, file: &str) {
        self.doing = Doing::Downloading(job::Job::start(
            self.socket.clone(),
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
            self.socket.clone(),
            Request::Measure {
                model: held.path.clone(),
                engine: None,
                deepest,
            },
            format!("measuring {}", held.name),
        ));
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
        self.doing = Doing::Answering(job::Job::start(
            self.socket.clone(),
            Request::Generate {
                model: held.path.clone(),
                prompt: question,
                limit: Some(256),
                seed: 0,
                tokens: None,
                engine: None,
                // A person's, which is what this window is for. B-146 and
                // §6.8: whose text it is travels with the request rather than
                // being inferred at the far end, and a window is never a
                // probe.
                whose: mcf_record::content::Whose::User,
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
        } else if self.doing.busy() {
            "working".to_owned()
        } else {
            "MCF".to_owned()
        }
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
            | Doing::Answering(job)
            | Doing::Reporting(job)
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
        1024
    }

    /// Roughly how long a run takes, as a range.
    ///
    /// A range because MCF's own estimates land between 0.58× and 1.42× of
    /// what runs actually take, and a single number would be a promise it
    /// cannot keep.
    #[must_use]
    pub fn estimate(&self, quick: bool) -> (u64, u64) {
        let seconds: u64 = if quick {
            // A quick run is the first test only, and only to 1 024.
            #[expect(
                clippy::integer_division,
                reason = "a sixth of a test, in whole seconds"
            )]
            let sixth = self.tests.first().map_or(30, |test| test.seconds / 6);
            sixth
        } else {
            self.tests
                .iter()
                .filter(|test| test.chosen)
                .map(|test| test.seconds)
                .sum()
        };
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
                }
                sdl::EVENT_MOUSE_WHEEL => mouse.wheel = sdl::event_wheel(&event),
                // Typing. It arrives already composed, so a layout, a
                // modifier or an input method is the platform's business and
                // not something this spells out of keycodes.
                sdl::EVENT_TEXT_INPUT => {
                    if desk.takes_typing()
                        && let Some(text) = sdl::event_text(&event)
                    {
                        desk.typed.push_str(&text);
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
                    sdl::KEY_BACKSPACE if desk.takes_typing() => {
                        let _removed = desk.typed.pop();
                    }
                    sdl::KEY_RETURN if desk.takes_typing() => desk.entered(),
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

        if mouse.wheel.abs() > 0.0 {
            desk.scroll = (desk.scroll - mouse.wheel * 48.0).max(0.0);
        }

        // Anything a running job has said since the last frame.
        if desk.hear() {
            acted = true;
        }

        // A second between readings, and only where they are shown — the same
        // rule the console follows, for the same reason: an idle window should
        // not be why a fan is running (B-071).
        let due = desk.page == Page::Monitor && last.elapsed() >= std::time::Duration::from_secs(1);
        if due {
            desk.sample();
            last = std::time::Instant::now();
            acted = true;
        }

        if let Some(act) = view::draw(&mut paint, &desk, &mouse) {
            if act == Act::CopyAnalysis {
                // The window owns the clipboard, so the copy happens here
                // rather than inside `act`.
                desk.copied = desk.analysis_as_text().is_some_and(|text| {
                    paint
                        .window()
                        .is_some_and(|window| window.put_on_clipboard(&text))
                });
            }
            desk.act(act);
            acted = true;
        }
        let _ = acted;

        // Nothing to do until something happens.
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

/// A pointer position in pixels, as the points everything is laid out in.
fn points(paint: &paint::Painter, at: (f32, f32)) -> (f32, f32) {
    let scale = if paint.scale > 0.0 { paint.scale } else { 1.0 };
    (at.0 / scale, at.1 / scale)
}

#[cfg(test)]
mod tests;
