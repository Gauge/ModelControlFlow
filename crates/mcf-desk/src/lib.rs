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
        key: "q or Escape",
        does: "close the window",
        reaches: None,
    },
];

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
    /// How MCF is set up.
    Settings,
    /// Leave.
    Exit,
    /// Fetching a model that is not here yet. Reached from Host's actions
    /// rather than the menu, the way the console's screens lead onward.
    Adding,
    /// A model, held and answering.
    Hosting,
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
        (Self::Host, "Host"),
        (Self::Diagnostics, "Diagnostics"),
        (Self::Models, "Models"),
        (Self::Settings, "Settings"),
        (Self::Exit, "Exit"),
    ];

    /// Which entry in the column should be lit while this page shows.
    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Monitor => Self::Monitor,
            // The two screens Host's actions lead to belong to Host, so the
            // menu still shows where you came from.
            Self::Host | Self::Adding | Self::Hosting => Self::Host,
            Self::Diagnostics => Self::Diagnostics,
            Self::Models => Self::Models,
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
}

/// One measurement that can be asked for, as the console lists them.
#[derive(Debug, Clone)]
pub struct Test {
    /// What it measures, in words.
    pub name: &'static str,
    /// Which devices it needs.
    pub devices: &'static str,
    /// Roughly how long, in seconds, at this machine's speed.
    pub seconds: u64,
    /// Whether it is selected.
    pub chosen: bool,
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
        },
        Test {
            name: "Cold start cost",
            devices: "both",
            seconds: 25,
            chosen: true,
        },
        Test {
            name: "Memory ceiling — largest context",
            devices: "both",
            seconds: 120,
            chosen: false,
        },
        Test {
            name: "CPU and GPU agree on the output",
            devices: "needs both",
            seconds: 90,
            chosen: false,
        },
        Test {
            name: "Prompt reading speed",
            devices: "both",
            seconds: 45,
            chosen: false,
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
        // Nothing below has been measured yet by anything the daemon answers,
        // and an absent measurement is left absent (A7). When the diagnostic
        // writes them into the record, they are read here and every sentence
        // above changes on its own.
        speed: None,
        wakes: None,
        fastest: None,
        slowest: None,
    }
}

/// Asks the daemon one question.
fn ask(socket: &Path, request: &Request) -> Result<Answer, String> {
    let mut connection = UnixStream::connect(socket)
        .map_err(|_| "MCF is not answering on this computer".to_owned())?;
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("the request could not be sent: {error}"))?;
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|error| format!("MCF did not answer: {error}"))?;
    Answer::read(line.trim_end()).map_err(|failure| failure.to_string())
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
    /// Move the context window on to the next power of two, wrapping.
    NextWindow,
    /// Turn one test on or off.
    Toggle(usize),
    /// Close the window.
    Close,
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
    /// Asking a hub what it publishes.
    Listing(job::Job),
    /// Fetching a model.
    Downloading(job::Job),
    /// Timing one.
    Measuring(job::Job),
    /// Waiting for a model to answer.
    Answering(job::Job),
}

impl Doing {
    /// The job behind it, whatever it is.
    #[must_use]
    pub fn job(&self) -> Option<&job::Job> {
        match self {
            Self::Nothing => None,
            Self::Listing(job)
            | Self::Downloading(job)
            | Self::Measuring(job)
            | Self::Answering(job) => Some(job),
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
    /// How far down a long list has been scrolled, in points.
    pub scroll: f32,
    /// The last reading of the machine.
    pub reading: mcf_tui::machine::Reading,
    /// Why MCF could not be reached, when it could not.
    pub refusal: Option<String>,
    /// What is being typed, on the screen that has a field.
    pub typed: String,
    /// Which model a measurement or a question is about.
    pub chosen: Option<usize>,
    /// What is running.
    pub doing: Doing,
    /// What a model has said so far, this turn.
    pub said: String,
    /// The tests offered on the diagnostics screen.
    pub tests: Vec<Test>,
    /// The context window a measurement is set up for.
    ///
    /// Choosing it implies every power of two below it, which is why the
    /// depths are stated under it rather than offered as a second set of
    /// choices somebody could contradict the first with.
    pub window: u64,
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
            scroll: 0.0,
            reading: mcf_tui::machine::Reading::default(),
            refusal: None,
            typed: String::new(),
            chosen: None,
            doing: Doing::Nothing,
            said: String::new(),
            tests: tests(),
            window: 8192,
            sampler: mcf_tui::machine::Sampler::new(),
        }
    }

    /// Whether the screen showing has a field somebody could be typing into.
    #[must_use]
    pub fn takes_typing(&self) -> bool {
        matches!(self.page, Page::Adding | Page::Hosting)
    }

    /// What pressing Return means on the screen showing.
    pub fn entered(&mut self) {
        match self.page {
            Page::Adding => self.look_up(),
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
            | Doing::Answering(job) => job.drain(),
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
        true
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
            // Powers of two, wrapping — the console's dropdown, and the
            // reason it is powers of two is that a context window is asked
            // for in them.
            Act::NextWindow => {
                self.window = match self.window {
                    held if held >= 65_536 => 1_024,
                    held => held.saturating_mul(2),
                };
            }
            Act::Toggle(at) => {
                if let Some(test) = self.tests.get_mut(at) {
                    test.chosen = !test.chosen;
                }
            }
            Act::Close => {}
            Act::Ask { at } => self.ask(at),
            Act::Choose(at) => self.chosen = Some(at),
            Act::Clear => self.typed.clear(),
            Act::Dismiss => self.doing = Doing::Nothing,
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
    pub fn refresh(&mut self) {
        match ask(&self.socket, &Request::Holding) {
            Ok(answer) if answer.served => {
                self.refusal = None;
                self.models = answer
                    .body
                    .get("models")
                    .and_then(Value::as_list)
                    .map(|held| held.iter().map(model_from).collect())
                    .unwrap_or_default();
                self.models.sort_by(|one, two| one.name.cmp(&two.name));
            }
            Ok(answer) => {
                self.refusal = Some(
                    answer
                        .body
                        .get("what")
                        .and_then(Value::as_text)
                        .unwrap_or("MCF did not say why")
                        .to_owned(),
                );
            }
            Err(why) => self.refusal = Some(why),
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
        match &self.doing {
            Doing::Nothing => (
                "IDLE".to_owned(),
                "nothing is being served — Host holds a model here".to_owned(),
            ),
            Doing::Listing(job)
            | Doing::Downloading(job)
            | Doing::Measuring(job)
            | Doing::Answering(job) => (
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
