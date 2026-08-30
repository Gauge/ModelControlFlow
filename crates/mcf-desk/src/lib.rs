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
    /// Every model on this computer.
    Models,
    /// One of them, by its position in the list.
    Model(usize),
    /// Measuring.
    Speed,
    /// What this machine can do.
    Computer,
}

impl Page {
    /// The navigation column, in order.
    ///
    /// Three entries, because three are built. A column offering *Chat* and
    /// *Add a model* before either works would be advertising what MCF cannot
    /// do, and a person who clicks one and finds nothing has been told
    /// something false about the whole application (A19).
    pub const MENU: &'static [(Self, &'static str)] = &[
        (Self::Models, "Your models"),
        (Self::Speed, "Speed tests"),
        (Self::Computer, "Your computer"),
    ];

    /// Which entry in the column should be lit while this page shows.
    #[must_use]
    pub fn section(self) -> Self {
        match self {
            Self::Model(_) | Self::Models => Self::Models,
            Self::Speed => Self::Speed,
            Self::Computer => Self::Computer,
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
            sampler: mcf_tui::machine::Sampler::new(),
        }
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
                sdl::EVENT_KEY_DOWN => match sdl::event_key(&event) {
                    sdl::KEY_ESCAPE => return Ok(()),
                    key if key == u32::from(b'q') => return Ok(()),
                    key if key == u32::from(b'r') => {
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

        // A second between readings, and only where they are shown — the same
        // rule the console follows, for the same reason: an idle window should
        // not be why a fan is running (B-071).
        let due =
            desk.page == Page::Computer && last.elapsed() >= std::time::Duration::from_secs(1);
        if due {
            desk.sample();
            last = std::time::Instant::now();
            acted = true;
        }

        if let Some(going) = view::draw(&mut paint, &desk, &mouse)
            && going != desk.page
        {
            desk.scroll = 0.0;
            if going.section() == Page::Computer {
                desk.sample();
            }
            desk.page = going;
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
