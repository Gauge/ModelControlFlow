//! MCF as a terminal application: a second surface, and a client of the same
//! control plane (A22, B-072).
//!
//! **It adds nothing.** Every key that does something turns into exactly one
//! [`mcf_serve::control::Request`], and [`ACTIONS`] says which. A22 is absolute
//! — *the interface may not be the only way to do anything* — and B-072 left an
//! enumeration waiting for the day a second surface arrived. This surface
//! brings its own.
//!
//! **It costs nothing while nobody is typing.** The terminal is put into a mode
//! where a read blocks until a key arrives, so the loop is not a loop that
//! spins; it is a thread asleep in `read`. B-071 asks that an idle window cost
//! what a closed one costs, and the way to get that is to have nothing to do
//! when nothing has happened. There is no timer here and no redraw that is not
//! caused by a key.
//!
//! **It draws what MCF said, and says when MCF said nothing.** The daemon's
//! refusals are rendered as refusals, in their own category, rather than as an
//! empty table — a screen that shows nothing where a failure happened is a
//! screen that reports success (A2).

pub mod keys;
pub mod machine;
pub mod screen;
pub mod screens;
pub mod terminal;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use keys::Key;
use screen::{Ink, Screen};
use screens::Where;
use screens::host::{Held, Resolved};

/// One thing the operator can do, and the control-plane request it reaches.
///
/// An action reaching nothing would be a capability this surface has and the
/// command line does not, which is the shape A22 forbids. The parity check
/// reads this table rather than a list somebody maintained beside it.
#[derive(Debug, Clone, Copy)]
pub struct Action {
    /// The key, as it is shown.
    pub key: &'static str,
    /// What it does, in the words on screen.
    pub does: &'static str,
    /// The `Request` variant it reaches, or `None` where it only moves the
    /// cursor and asks MCF nothing.
    pub reaches: Option<&'static str>,
}

/// Every action, and there are no others.
pub const ACTIONS: &[Action] = &[
    Action {
        key: "←→",
        does: "move along the menu",
        reaches: None,
    },
    Action {
        key: "⏎",
        does: "open",
        reaches: Some("Holding"),
    },
    Action {
        key: "↑↓",
        does: "choose",
        reaches: None,
    },
    Action {
        key: "r",
        does: "refresh",
        reaches: Some("Status"),
    },
    Action {
        key: "S",
        does: "stop the daemon",
        reaches: Some("Stop"),
    },
    Action {
        key: "q",
        does: "quit",
        reaches: None,
    },
];

/// What MCF last said, and where the operator is looking.
#[derive(Debug)]
struct Console {
    socket: std::path::PathBuf,
    /// Which menu item the cursor is on.
    cursor: usize,
    /// Which screen is open.
    at: Where,
    /// Row within whatever list the screen shows.
    row: usize,
    /// Which button, on a screen that has them.
    button: usize,
    /// Whether the cursor is in the buttons rather than the list.
    on_buttons: bool,
    status: Option<Result<Value, String>>,
    models: Vec<Held>,
    tests: Vec<screens::diagnostics::Test>,
    /// What MCF can build, and which of it is here.
    components: Vec<(String, String, bool, bool, bool)>,
    said: Option<(String, Ink)>,
    sampler: machine::Sampler,
    reading: machine::Reading,
    confirming: bool,
}

/// Asks the daemon one question.
fn ask(socket: &Path, request: &Request) -> Result<Answer, String> {
    let mut connection = UnixStream::connect(socket)
        .map_err(|_| format!("nothing is listening on {}", socket.display()))?;
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("the request could not be sent: {error}"))?;
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|error| format!("the daemon did not answer: {error}"))?;
    Answer::read(line.trim_end()).map_err(|failure| failure.to_string())
}

/// What a refusal says, in one line, from the record's own shape.
fn why(body: &Value) -> String {
    body.get("what")
        .and_then(Value::as_text)
        .unwrap_or("MCF did not say why")
        .to_owned()
}

/// The ends of what the ladder measured, as the daemon wrote them.
///
/// Only the depths that *separated*: a rung the arithmetic could not measure
/// is not a slow one, and letting it stand in for the deepest reading would
/// put a number where there is none (A7, A9). The start-up figure is the
/// daemon's, derived on its side of the wire (B-072); a run that could not
/// read one says so there, and reads as nothing here.
fn measured_ends(held: &Value) -> screens::host::Measured {
    let mut ends = screens::host::Measured {
        start_up: held
            .get("first_token")
            .filter(|figure| matches!(figure.get("measured"), Some(Value::Bool(true))))
            .and_then(|figure| figure.get("ms"))
            .and_then(Value::as_text)
            .map(str::to_owned),
        ..screens::host::Measured::default()
    };
    let Some(readings) = held.get("readings").and_then(Value::as_list) else {
        return ends;
    };
    for reading in readings {
        if !matches!(reading.get("measured"), Some(Value::Bool(true))) {
            continue;
        }
        let Some(ms) = reading.get("ms_per_token").and_then(Value::as_text) else {
            continue;
        };
        if ends.at_512.is_none() {
            ends.at_512 = Some(ms.to_owned());
        }
        ends.at_window = Some(ms.to_owned());
    }
    ends
}

/// The name a model is known by: the file, not where it sits.
fn name_of(model: &Value) -> String {
    model
        .get("path")
        .and_then(Value::as_text)
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or("unknown")
        .to_owned()
}

impl Console {
    fn new(socket: std::path::PathBuf) -> Self {
        Self {
            socket,
            cursor: 0,
            at: Where::Monitor,
            row: 0,
            button: 0,
            on_buttons: false,
            status: None,
            models: Vec::new(),
            tests: screens::diagnostics::tests(),
            components: Vec::new(),
            said: None,
            sampler: machine::Sampler::new(),
            reading: machine::Reading::default(),
            confirming: false,
        }
    }

    /// Everything the daemon knows, asked once and kept.
    fn refresh(&mut self) {
        self.status = Some(match ask(&self.socket, &Request::Status) {
            Ok(answer) if answer.served => Ok(answer.body),
            Ok(answer) => Err(why(&answer.body)),
            Err(error) => Err(error),
        });
        if let Ok(answer) = ask(&self.socket, &Request::Components)
            && answer.served
            && let Some(listed) = answer.body.get("components").and_then(Value::as_list)
        {
            self.components = listed
                .iter()
                .map(|held| {
                    let text = |key: &str| {
                        held.get(key)
                            .and_then(Value::as_text)
                            .unwrap_or_default()
                            .to_owned()
                    };
                    let flag = |key: &str| matches!(held.get(key), Some(Value::Bool(true)));
                    (
                        text("name"),
                        text("commit").chars().take(12).collect(),
                        flag("provisioned"),
                        flag("present"),
                        flag("usable_engine"),
                    )
                })
                .collect();
        }
        self.models = match ask(&self.socket, &Request::Holding) {
            Ok(answer) if answer.served => answer
                .body
                .get("models")
                .and_then(Value::as_list)
                .map(|models| models.iter().map(Self::describe).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if self.row >= self.models.len() {
            self.row = self.models.len().saturating_sub(1);
        }
    }

    /// One model, as the daemon described it.
    ///
    /// The daemon answers this: it holds the models and the engines, so it is
    /// where the question is answered — once, the same way, for every surface.
    /// A console that opened the store itself would be reaching past the wire.
    fn describe(model: &Value) -> Held {
        let runs = model.get("runs");
        let number = |key: &str| -> Option<u64> {
            runs?
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        let resolved = runs.and_then(|held| held.get("resolved"));
        let engine = match resolved {
            Some(held) if matches!(held.get("known"), Some(Value::Bool(true))) => Ok(Resolved {
                engine: held
                    .get("engine")
                    .and_then(Value::as_text)
                    .unwrap_or("unknown")
                    .to_owned(),
                device: held
                    .get("device")
                    .and_then(Value::as_text)
                    .unwrap_or("unknown")
                    .to_owned(),
                context: held
                    .get("context")
                    .and_then(Value::as_integer)
                    .and_then(|value| u64::try_from(value).ok())
                    .unwrap_or(0),
            }),
            Some(held) => Err(held
                .get("why")
                .and_then(Value::as_text)
                .unwrap_or("MCF did not say why")
                .to_owned()),
            None => Err("MCF has not looked at this model yet".to_owned()),
        };
        Held {
            name: name_of(model),
            bytes: model
                .get("bytes")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            architecture: runs
                .and_then(|held| held.get("architecture"))
                .and_then(Value::as_text)
                .map(str::to_owned),
            trained: number("trained_context"),
            cache_per_token: number("cache_bytes_per_token"),
            engine,
            measured: runs
                .and_then(|held| held.get("measured"))
                .map(measured_ends)
                .unwrap_or_default(),
        }
    }

    fn stop(&mut self) {
        let request = Request::Stop {
            reason: "asked to stop from the console".to_owned(),
        };
        match ask(&self.socket, &request) {
            Ok(answer) if answer.served => {
                self.said = Some(("the daemon was asked to stop".to_owned(), Ink::Held));
            }
            Ok(answer) => self.said = Some((why(&answer.body), Ink::Refusal)),
            Err(error) => self.said = Some((error, Ink::Refusal)),
        }
        self.refresh();
    }

    /// How many rows the open screen has to move through.
    fn rows(&self) -> usize {
        match self.at {
            Where::Models => self.models.len(),
            Where::Diagnostics => self.tests.len(),
            _ => 0,
        }
    }
}

fn draw(console: &Console, into: &mut Screen) {
    let serving = console
        .status
        .as_ref()
        .and_then(|held| held.as_ref().ok())
        .and_then(|status| status.get("resident"))
        .and_then(Value::as_text)
        .is_some();
    let state = match (&console.status, serving) {
        (Some(Err(_)), _) => Some(("not running", Ink::Refusal)),
        (_, true) => Some(("Serving", Ink::Held)),
        _ => Some(("Idle", Ink::Quiet)),
    };
    let from = screens::frame(into, console.at, console.cursor, state);

    if let Some(Err(error)) = &console.status {
        into.put(2, from + 1, "MCF is not running", Ink::Refusal);
        into.put(2, from + 3, error, Ink::Quiet);
        into.put(2, from + 5, "`mcf serve` starts it", Ink::Quiet);
        screens::close(into, from);
        return;
    }

    match console.at {
        Where::Monitor => {
            screens::monitor::draw(into, from, &console.reading, &screens::monitor::Doing::Idle);
        }
        Where::Models => screens::host::draw(
            into,
            from + 1,
            &console.models,
            console.row,
            console.button,
            console.on_buttons,
        ),
        Where::Diagnostics => {
            let model = console
                .models
                .get(console.row)
                .map_or("nothing selected", |held| held.name.as_str());
            let window = console
                .models
                .get(console.row)
                .and_then(|held| held.engine.as_ref().ok())
                .map(|resolved| resolved.context);
            screens::diagnostics::draw(into, from, model, window, &console.tests, console.row);
        }
        Where::Components => {
            into.put(2, from + 1, "COMPONENTS", Ink::Heading);
            if console.components.is_empty() {
                into.put(2, from + 3, "Unknown", Ink::Quiet);
            } else {
                let mut row = from + 3;
                for (name, commit, provisioned, present, usable) in &console.components {
                    // Three states, not two. The middle one is a run that
                    // stopped partway. Whether MCF reaches it as an engine is
                    // a separate fact: not every component is one.
                    let said = if *provisioned && *usable {
                        "provisioned; MCF reaches this as an engine"
                    } else if *provisioned {
                        "provisioned"
                    } else if *present {
                        "incomplete — a run stopped partway"
                    } else {
                        "not provisioned"
                    };
                    into.put(2, row, &format!("{name}@{commit}"), Ink::Heading);
                    into.put(
                        2,
                        row + 1,
                        said,
                        if *provisioned {
                            Ink::Quiet
                        } else {
                            Ink::Refusal
                        },
                    );
                    row += 3;
                }
                into.put(2, row, "`mcf provision <component>` builds one", Ink::Quiet);
            }
        }
        Where::Prompt => prompt_screen(into, from),
        Where::Settings => {
            into.put(2, from + 1, "SETTINGS", Ink::Heading);
            into.put(2, from + 3, "nothing to set yet", Ink::Quiet);
        }
        Where::Exit => {
            into.put(2, from + 1, "Press Enter to leave", Ink::Heading);
        }
    }

    let last = into.height().saturating_sub(2);
    if console.confirming {
        into.put(
            2,
            last,
            "Stop the daemon? It finishes what is in hand and exits.  [y/n]",
            Ink::Refusal,
        );
    } else if let Some((said, ink)) = &console.said {
        into.put(2, last, said, *ink);
    }
    screens::close(into, from);
}

/// What a prompt does, as the console shows it.
///
/// Its own function because `draw` is one arm per screen under a line cap: a
/// screen written inside it makes every other one harder to find.
fn prompt_screen(into: &mut Screen, from: usize) {
    into.put(2, from + 1, "WHAT A PROMPT DOES", Ink::Heading);
    into.put(
        2,
        from + 3,
        "`mcf prompt <model> --prompt \"...\"` takes one apart, sentence by sentence",
        Ink::Quiet,
    );
    into.put(
        2,
        from + 5,
        "one generation for the prompt, one for each sentence left out, one per seed",
        Ink::Quiet,
    );
    into.put(
        2,
        from + 7,
        "the reading is an ordering, not relevance: removing anything shifts what follows",
        Ink::Quiet,
    );
}

/// Runs until the operator leaves.
///
/// # Errors
///
/// The terminal could not be taken — most often because MCF's output is not
/// one, which is a fact and not a fault.
pub fn run(socket: std::path::PathBuf) -> Result<(), String> {
    let mut restored = terminal::take()?;
    let mut console = Console::new(socket);
    console.refresh();
    console.reading = console.sampler.read();

    let mut input = std::io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut buffer = [0_u8; 64];

    loop {
        // Live on the monitor, still everywhere else. This is the whole of it:
        // a read that gives up is a screen that redraws, and a read that waits
        // is a process asleep.
        terminal::wait_for_a_key(console.at != Where::Monitor);
        if console.at == Where::Monitor {
            console.reading = console.sampler.read();
        }

        let (width, height) = terminal::size();
        let mut screen = Screen::new(width, height);
        draw(&console, &mut screen);
        let mut out = std::io::stdout();
        let _drawn = out
            .write_all(screen.rendered().as_bytes())
            .and_then(|()| out.flush());

        let read = match input.read(&mut buffer) {
            Ok(read) => read,
            // A read that gave up with nothing is the monitor's clock, not an
            // end: come round, sample, draw again.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("the terminal stopped answering: {error}")),
        };
        if read == 0 {
            // Nothing more is coming, so a sequence left half-decoded was all
            // there ever was of it.
            if !pending.is_empty() {
                let settled = keys::flush_incomplete(&pending);
                pending.clear();
                if matches!(settled, Key::Escape) {
                    console.said = None;
                }
            }
            if console.at == Where::Monitor {
                continue;
            }
            break;
        }
        pending.extend_from_slice(buffer.get(..read).unwrap_or(&buffer));

        while let Some((key, used)) = keys::decode(&pending) {
            let used = used.min(pending.len());
            pending.rotate_left(used);
            pending.truncate(pending.len() - used);

            if console.confirming {
                console.confirming = false;
                if matches!(key, Key::Character('y' | 'Y')) {
                    console.stop();
                } else {
                    console.said = Some(("the daemon was left running".to_owned(), Ink::Quiet));
                }
                continue;
            }
            if act(&mut console, key) == Leaving::Yes {
                restored.now();
                return Ok(());
            }
        }
    }
    restored.now();
    Ok(())
}

/// Whether the operator is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaving {
    /// Yes.
    Yes,
    /// No.
    No,
}

/// One key, acted on.
fn act(console: &mut Console, key: Key) -> Leaving {
    {
        match key {
            Key::Character('q') | Key::Interrupt => {
                return Leaving::Yes;
            }
            Key::Left => console.cursor = console.cursor.saturating_sub(1),
            Key::Right => {
                console.cursor = (console.cursor + 1).min(Where::ALL.len() - 1);
            }
            Key::Enter => {
                let chosen = Where::ALL
                    .get(console.cursor)
                    .copied()
                    .unwrap_or(Where::Monitor);
                if chosen == Where::Exit {
                    return Leaving::Yes;
                }
                console.at = chosen;
                console.said = None;
                console.on_buttons = false;
                if matches!(chosen, Where::Models | Where::Diagnostics) {
                    console.refresh();
                }
            }
            Key::Up => {
                if console.on_buttons {
                    console.button = console.button.saturating_sub(1);
                } else {
                    console.row = console.row.saturating_sub(1);
                }
            }
            Key::Down => {
                if console.on_buttons {
                    console.button = (console.button + 1).min(screens::host::BUTTONS.len() - 1);
                } else {
                    console.row = (console.row + 1).min(console.rows().saturating_sub(1));
                }
            }
            Key::Character('\t') => console.on_buttons = !console.on_buttons,
            Key::Character('r') => {
                console.said = None;
                console.refresh();
            }
            Key::Character('S') => console.confirming = true,
            Key::Character(' ') if console.at == Where::Diagnostics => {
                screens::diagnostics::toggle(
                    console
                        .tests
                        .iter_mut()
                        .map(|test| (test.run, &mut test.chosen)),
                    console.row,
                );
            }
            _ => {}
        }
    }
    Leaving::No
}

#[cfg(test)]
mod tests;
