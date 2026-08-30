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
pub mod screen;
pub mod terminal;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use keys::Key;
use screen::{Ink, Screen};

/// One thing the operator can do, and the control-plane request it reaches.
///
/// The `reaches` field is why this table exists. An action reaching nothing
/// would be a capability this surface has and the command line does not, which
/// is the shape A22 forbids, and the parity check reads this rather than a list
/// somebody maintained beside it.
#[derive(Debug, Clone, Copy)]
pub struct Action {
    /// The key, as it is shown to the operator.
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
        key: "r",
        does: "refresh",
        reaches: Some("Status"),
    },
    Action {
        key: "m",
        does: "models",
        reaches: Some("Holding"),
    },
    Action {
        key: "S",
        does: "stop the daemon",
        reaches: Some("Stop"),
    },
    Action {
        key: "↑↓",
        does: "move",
        reaches: None,
    },
    Action {
        key: "q",
        does: "quit",
        reaches: None,
    },
];

/// What MCF last said, and what the operator is looking at.
#[derive(Debug)]
struct App {
    socket: std::path::PathBuf,
    status: Option<Result<Value, String>>,
    models: Option<Result<Vec<Value>, String>>,
    at: usize,
    said: Option<(String, Ink)>,
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
    let category = body
        .get("category")
        .and_then(Value::as_text)
        .unwrap_or("refused");
    let what = body
        .get("what")
        .and_then(Value::as_text)
        .unwrap_or("MCF did not say why");
    format!("{category}: {what}")
}

impl App {
    fn new(socket: std::path::PathBuf) -> Self {
        Self {
            socket,
            status: None,
            models: None,
            at: 0,
            said: None,
            confirming: false,
        }
    }

    fn refresh(&mut self) {
        self.status = Some(match ask(&self.socket, &Request::Status) {
            Ok(answer) if answer.served => Ok(answer.body),
            Ok(answer) => Err(why(&answer.body)),
            Err(error) => Err(error),
        });
        self.models = Some(match ask(&self.socket, &Request::Holding) {
            Ok(answer) if answer.served => Ok(answer
                .body
                .get("models")
                .and_then(Value::as_list)
                .map(<[Value]>::to_vec)
                .unwrap_or_default()),
            Ok(answer) => Err(why(&answer.body)),
            Err(error) => Err(error),
        });
        let held = self.held().len();
        if self.at >= held {
            self.at = held.saturating_sub(1);
        }
    }

    fn held(&self) -> &[Value] {
        match &self.models {
            Some(Ok(models)) => models,
            _ => &[],
        }
    }

    fn stop(&mut self) {
        let request = Request::Stop {
            reason: "asked to stop from the terminal application".to_owned(),
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
}

/// The name a model is known by: the file, not the path it happens to sit at.
fn name_of(model: &Value) -> String {
    model
        .get("path")
        .and_then(Value::as_text)
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or("unknown")
        .to_owned()
}

/// A size, in gigabytes, to two places.
///
/// Done in integers rather than floats, and the `integer_division` allow is the
/// point rather than an escape from it: a byte count can exceed what an `f64`
/// counts one by one, so dividing first and converting second would round a
/// size before printing it. Whole and hundredths are both exact here, and A6
/// asks that a number a person reads be the number MCF holds.
#[allow(
    clippy::integer_division,
    reason = "the division is exact and a float would not be"
)]
fn gigabytes(model: &Value) -> String {
    model.get("bytes").and_then(Value::as_integer).map_or_else(
        // A size MCF does not know is not a zero (A7).
        || "unknown".to_owned(),
        |bytes| {
            let whole = bytes / 1_000_000_000;
            let hundredths = (bytes % 1_000_000_000) / 10_000_000;
            format!("{whole}.{hundredths:02} GB")
        },
    )
}

fn draw(app: &App, into: &mut Screen) {
    let width = into.width();
    let inner = width.saturating_sub(2);

    // ── the daemon ────────────────────────────────────────────────────────
    into.put(0, 0, "MCF", Ink::Heading);
    match &app.status {
        Some(Ok(body)) => {
            let version = body
                .get("build")
                .and_then(|build| build.get("version"))
                .and_then(Value::as_text)
                .unwrap_or("unknown");
            let held = body
                .get("recovered")
                .and_then(|r| r.get("models_held"))
                .and_then(Value::as_integer)
                .unwrap_or(0);
            let entries = body
                .get("recovered")
                .and_then(|r| r.get("record_entries"))
                .and_then(Value::as_integer)
                .unwrap_or(0);
            into.put(4, 0, version, Ink::Figure);
            into.put_right(
                width,
                0,
                &format!("{held} models   {entries} entries"),
                Ink::Quiet,
            );
            if let Some(cannot) = body.get("cannot").and_then(Value::as_list)
                && let Some(first) = cannot.first().and_then(Value::as_text)
            {
                into.put(0, 1, &format!("cannot {first}"), Ink::Quiet);
            }
        }
        Some(Err(error)) => into.put(4, 0, error, Ink::Refusal),
        None => into.put(4, 0, "asking…", Ink::Quiet),
    }
    into.rule(0, 2, width, Ink::Quiet);

    // ── what is held ──────────────────────────────────────────────────────
    into.put(0, 3, "MODELS HELD", Ink::Heading);
    let first_row = 5;
    let last_row = into.height().saturating_sub(3);
    match &app.models {
        Some(Err(error)) => into.put(2, first_row, error, Ink::Refusal),
        Some(Ok(models)) if models.is_empty() => {
            into.put(
                2,
                first_row,
                "nothing — `mcf pull` brings a model here",
                Ink::Quiet,
            );
        }
        Some(Ok(models)) => {
            // Only what fits, and scrolled so the selected row is on screen.
            let room = last_row.saturating_sub(first_row);
            let from = app.at.saturating_sub(room.saturating_sub(1)).min(app.at);
            for (offset, model) in models.iter().skip(from).take(room).enumerate() {
                let row = first_row + offset;
                let index = from + offset;
                let here = index == app.at;
                if here {
                    into.select_row(row);
                    into.put(0, row, "›", Ink::Selected);
                }
                let ink = if here { Ink::Selected } else { Ink::Plain };
                into.put(2, row, &name_of(model), ink);
                into.put_right(inner, row, &gigabytes(model), ink);
            }
            if models.len() > room {
                into.put_right(
                    width,
                    3,
                    &format!("{} of {}", app.at + 1, models.len()),
                    Ink::Quiet,
                );
            }
        }
        None => into.put(2, first_row, "asking…", Ink::Quiet),
    }

    // ── what was said, and what can be pressed ────────────────────────────
    let foot = into.height().saturating_sub(1);
    if app.confirming {
        into.put(
            0,
            foot.saturating_sub(1),
            "Stop the daemon? It finishes what is in hand and exits.  [y/n]",
            Ink::Refusal,
        );
    } else if let Some((said, ink)) = &app.said {
        into.put(0, foot.saturating_sub(1), said, *ink);
    }
    let legend = ACTIONS
        .iter()
        .map(|action| format!("[{}] {}", action.key, action.does))
        .collect::<Vec<_>>()
        .join("   ");
    into.put(0, foot, &legend, Ink::Quiet);
}

/// Runs until the operator quits.
///
/// # Errors
///
/// The terminal could not be taken — most often because MCF's output is not
/// one, which is a fact and not a fault.
pub fn run(socket: std::path::PathBuf) -> Result<(), String> {
    let mut restored = terminal::take()?;
    let mut app = App::new(socket);
    app.refresh();

    let mut input = std::io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut buffer = [0_u8; 64];

    loop {
        let (width, height) = terminal::size();
        let mut screen = Screen::new(width, height);
        draw(&app, &mut screen);
        let mut out = std::io::stdout();
        let _drawn = out
            .write_all(screen.rendered().as_bytes())
            .and_then(|()| out.flush());

        // Blocks here, and this is the whole of B-071: nothing happens until a
        // key does.
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("the terminal stopped answering: {error}"))?;
        if read == 0 {
            break;
        }
        // `get` rather than a slice index: `read` comes from the platform and
        // a length MCF did not compute is a length MCF does not assume.
        pending.extend_from_slice(buffer.get(..read).unwrap_or(&buffer));

        while let Some((key, used)) = keys::decode(&pending) {
            // Consumed from the front without slicing: rotate what is left to
            // the start and drop the tail.
            let used = used.min(pending.len());
            pending.rotate_left(used);
            pending.truncate(pending.len() - used);
            if app.confirming {
                app.confirming = false;
                if matches!(key, Key::Character('y' | 'Y')) {
                    app.stop();
                } else {
                    app.said = Some(("the daemon was left running".to_owned(), Ink::Quiet));
                }
                continue;
            }
            match key {
                Key::Character('q') | Key::Interrupt => {
                    restored.now();
                    return Ok(());
                }
                Key::Character('r' | 'm') => {
                    app.said = None;
                    app.refresh();
                }
                Key::Character('S') => app.confirming = true,
                Key::Up => app.at = app.at.saturating_sub(1),
                Key::Down => {
                    let held = app.held().len();
                    app.at = (app.at + 1).min(held.saturating_sub(1));
                }
                _ => {}
            }
        }
    }
    restored.now();
    Ok(())
}

#[cfg(test)]
mod tests;
