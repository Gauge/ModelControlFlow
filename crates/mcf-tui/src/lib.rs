pub mod job;
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

#[derive(Debug, Clone, Copy)]
pub struct Action {
    pub key: &'static str,
    pub does: &'static str,
    pub reaches: Option<&'static str>,
}

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
        key: "⇥",
        does: "to the buttons and back",
        reaches: None,
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

#[derive(Debug)]
struct Console {
    socket: std::path::PathBuf,
    cursor: usize,
    at: Where,
    row: usize,
    button: usize,
    on_buttons: bool,
    model: usize,
    status: Option<Result<Value, String>>,
    models: Vec<Held>,
    card_unused: Option<String>,
    components: Vec<(String, String, bool, bool, bool)>,
    said: Option<(String, Ink)>,
    sampler: machine::Sampler,
    reading: machine::Reading,
    confirming: bool,
}

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

fn why(body: &Value) -> String {
    body.get("what")
        .and_then(Value::as_text)
        .unwrap_or("MCF did not say why")
        .to_owned()
}

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
        let Some(depth) = reading
            .get("depth")
            .and_then(Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
        else {
            continue;
        };
        if ends.shallowest.is_none() {
            ends.shallowest = Some((depth, ms.to_owned()));
        }
        ends.deepest = Some((depth, ms.to_owned()));
    }
    ends
}

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
            model: 0,
            status: None,
            models: Vec::new(),
            card_unused: None,
            components: Vec::new(),
            said: None,
            sampler: machine::Sampler::new(),
            reading: machine::Reading::default(),
            confirming: false,
        }
    }

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
            Ok(answer) if answer.served => {
                self.card_unused = answer
                    .body
                    .get("card_unused")
                    .and_then(|held| held.get("component"))
                    .and_then(Value::as_text)
                    .map(str::to_owned);
                answer
                    .body
                    .get("models")
                    .and_then(Value::as_list)
                    .map(|models| models.iter().map(Self::describe).collect())
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        if self.row >= self.models.len() {
            self.row = self.models.len().saturating_sub(1);
        }
    }

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
            path: model
                .get("path")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned(),
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

    fn rows(&self) -> usize {
        match self.at {
            Where::Models => self.models.len(),
            _ => 0,
        }
    }

    fn buttons(&self) -> usize {
        match self.at {
            Where::Models => screens::host::BUTTONS.len(),
            _ => 0,
        }
    }

    fn press(&mut self) {
        match (self.at, self.button) {
            (Where::Models, 0) => {
                self.said = Some((
                    "holding from the console is not built yet — `mcf host` does it, and \
                     the window"
                        .to_owned(),
                    Ink::Refusal,
                ));
            }
            (Where::Models, _) => self.open(Where::Monitor),
            _ => {}
        }
    }

    fn open(&mut self, screen: Where) {
        self.at = screen;
        self.cursor = Where::ALL
            .iter()
            .position(|held| *held == screen)
            .unwrap_or(0);
        self.said = None;
        self.on_buttons = self.rows() == 0 && self.buttons() > 0;
        self.button = 0;
        self.row = if screen == Where::Models {
            self.model
        } else {
            0
        };
        if matches!(screen, Where::Models) {
            self.refresh();
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
        Where::Components => {
            into.put(2, from + 1, "COMPONENTS", Ink::Heading);
            if console.components.is_empty() {
                into.put(2, from + 3, "Unknown", Ink::Quiet);
            } else {
                let mut row = from + 3;
                for (name, commit, provisioned, present, usable) in &console.components {
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

pub fn run(socket: std::path::PathBuf) -> Result<(), String> {
    let mut restored = terminal::take()?;
    let mut console = Console::new(socket);
    console.refresh();
    console.reading = console.sampler.read();

    let mut input = std::io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut buffer = [0_u8; 64];

    loop {
        let waiting = console.at != Where::Monitor;
        terminal::wait_for_a_key(waiting);
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
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("the terminal stopped answering: {error}")),
        };
        if read == 0 {
            if !pending.is_empty() {
                let settled = keys::flush_incomplete(&pending);
                pending.clear();
                if matches!(settled, Key::Escape) {
                    console.said = None;
                }
            }
            if !waiting {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaving {
    Yes,
    No,
}

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
            Key::Enter if console.on_buttons => console.press(),
            Key::Enter => {
                let chosen = Where::ALL
                    .get(console.cursor)
                    .copied()
                    .unwrap_or(Where::Monitor);
                if chosen == Where::Exit {
                    return Leaving::Yes;
                }
                console.open(chosen);
            }
            Key::Up => {
                if console.on_buttons {
                    console.button = console.button.saturating_sub(1);
                } else {
                    console.row = console.row.saturating_sub(1);
                }
                if console.at == Where::Models {
                    console.model = console.row;
                }
            }
            Key::Down => {
                if console.on_buttons {
                    console.button = (console.button + 1).min(console.buttons().saturating_sub(1));
                } else {
                    console.row = (console.row + 1).min(console.rows().saturating_sub(1));
                }
                if console.at == Where::Models {
                    console.model = console.row;
                }
            }
            Key::Tab if console.buttons() > 0 && console.rows() > 0 => {
                console.on_buttons = !console.on_buttons;
            }
            Key::Character('r') => {
                console.said = None;
                console.refresh();
            }
            Key::Character('S') => console.confirming = true,
            _ => {}
        }
    }
    Leaving::No
}

#[cfg(test)]
mod tests;
