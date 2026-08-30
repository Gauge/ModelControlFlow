//! MCF in a window: the console's layout, drawn with pixels.
//!
//! **The same screens, not similar ones.** Every panel, table and column comes
//! from `mcf_tui::screens` — the code the terminal console draws with, tested
//! there. A window is a larger grid, not a different program, so a change to a
//! layout changes both surfaces at once and neither can drift from the other.
//!
//! **What the window buys is room and a picture.** More rows of models, more
//! measured depths, and the fall-off curve — which is the one thing a terminal
//! genuinely cannot draw, and the reason a second surface earns its keep.
//!
//! **It is a client, and adds nothing.** Every action turns into a request the
//! command line already sends, and [`ACTIONS`] names which (A22, B-072).

pub mod plot;
pub mod sdl;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};
use mcf_tui::screen::{Ink, Screen};
use mcf_tui::screens::{self, Where};

/// One thing the operator can do, and the control-plane request it reaches.
#[derive(Debug, Clone, Copy)]
pub struct Action {
    /// The key, as it is shown.
    pub key: &'static str,
    /// What it does.
    pub does: &'static str,
    /// The `Request` variant it reaches, or `None` where it only moves the
    /// cursor.
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
        key: "q",
        does: "quit",
        reaches: None,
    },
];

/// How many pixels one character occupies, before scaling.
const CELL: u16 = 8;
/// How much the 8×8 font is scaled up. Two is legible on an ordinary display
/// without turning the window into a poster.
const SCALE: u16 = 2;
/// One cell, in pixels.
const STEP: u16 = CELL * SCALE;

/// What a colour role looks like in pixels.
///
/// The same roles the console uses, so a refusal is the same colour on both
/// surfaces and neither has a palette the other does not.
const fn colour(ink: Ink) -> (u8, u8, u8) {
    match ink {
        Ink::Plain => (0xdf, 0xe3, 0xe8),
        Ink::Quiet => (0x8d, 0x94, 0x9e),
        Ink::Heading => (0xd7, 0xa2, 0x60),
        Ink::Figure => (0x6f, 0xb3, 0xcc),
        Ink::Refusal => (0xdd, 0x8b, 0x78),
        Ink::Held => (0x7c, 0xc3, 0x9b),
        Ink::Selected => (0x20, 0x23, 0x2a),
    }
}

/// The window's own ground.
const GROUND: (u8, u8, u8) = (0x20, 0x23, 0x2a);
/// What a selected row sits on.
const HIGHLIGHT: (u8, u8, u8) = (0xd7, 0xa2, 0x60);

/// A cell index as a pixel offset.
///
/// Screen coordinates are a few hundred at most and `f32` is the type SDL
/// takes, so the conversion is exact for every window anybody has.
fn at(cell: usize, step: f32) -> f32 {
    f32::from(u16::try_from(cell).unwrap_or(u16::MAX)) * step
}

/// Draws a whole screen of cells into the window.
///
/// Runs of one colour are drawn as one string rather than a character at a
/// time: a table row is one call, not eighty.
pub fn render(window: &sdl::Window, screen: &Screen) {
    window.clear(GROUND);
    let step = f32::from(STEP);
    for row in 0..screen.height() {
        let mut column = 0;
        while column < screen.width() {
            let ink = screen.ink(column, row);
            let mut run = String::new();
            let start = column;
            while column < screen.width() && screen.ink(column, row) == ink {
                run.push(screen.at(column, row));
                column += 1;
            }
            let x = at(start, step);
            let y = at(row, step);
            if ink == Ink::Selected {
                window.fill(
                    sdl::Rect {
                        x,
                        y,
                        w: at(run.chars().count(), step),
                        h: step,
                    },
                    HIGHLIGHT,
                );
            }
            // The font is drawn at its own size and stepped by the scale, so
            // the glyphs stay crisp rather than being stretched.
            for (offset, character) in run.chars().enumerate() {
                if character == ' ' {
                    continue;
                }
                let mut one = [0_u8; 4];
                window.text(
                    x + at(offset, step),
                    y,
                    character.encode_utf8(&mut one),
                    colour(ink),
                );
            }
        }
    }
}

/// How many cells fit in a window of this many pixels.
///
/// Never fewer than the smallest terminal: the layouts are drawn for eighty by
/// twenty-four and a window narrower than that would clip a screen that was
/// designed to fit.
#[must_use]
pub fn grid(width: i32, height: i32) -> (u16, u16) {
    #[allow(
        clippy::integer_division,
        reason = "whole cells; a partial one shows nothing"
    )]
    let (columns, rows) = (
        width.max(0) / i32::from(STEP),
        height.max(0) / i32::from(STEP),
    );
    (
        u16::try_from(columns).unwrap_or(80).max(80),
        u16::try_from(rows).unwrap_or(24).max(24),
    )
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

/// The window's state: where the cursor is and what MCF last said.
#[derive(Debug)]
struct Desk {
    socket: std::path::PathBuf,
    cursor: usize,
    at: Where,
    row: usize,
    status: Option<Result<Value, String>>,
    models: Vec<Value>,
    sampler: mcf_tui::machine::Sampler,
    reading: mcf_tui::machine::Reading,
}

impl Desk {
    fn new(socket: std::path::PathBuf) -> Self {
        Self {
            socket,
            cursor: 0,
            at: Where::Monitor,
            row: 0,
            status: None,
            models: Vec::new(),
            sampler: mcf_tui::machine::Sampler::new(),
            reading: mcf_tui::machine::Reading::default(),
        }
    }

    fn refresh(&mut self) {
        self.status = Some(match ask(&self.socket, &Request::Status) {
            Ok(answer) if answer.served => Ok(answer.body),
            Ok(answer) => Err(answer
                .body
                .get("what")
                .and_then(Value::as_text)
                .unwrap_or("MCF did not say why")
                .to_owned()),
            Err(error) => Err(error),
        });
        self.models = match ask(&self.socket, &Request::Holding) {
            Ok(answer) if answer.served => answer
                .body
                .get("models")
                .and_then(Value::as_list)
                .map(<[Value]>::to_vec)
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if self.row >= self.models.len() {
            self.row = self.models.len().saturating_sub(1);
        }
    }
}

fn draw(desk: &Desk, into: &mut Screen) {
    let from = screens::frame(into, desk.at, desk.cursor, Some(("MCF", Ink::Quiet)));
    if let Some(Err(error)) = &desk.status {
        into.put(2, from + 1, "MCF is not running", Ink::Refusal);
        into.put(2, from + 3, error, Ink::Quiet);
        into.put(2, from + 5, "`mcf serve` starts it", Ink::Quiet);
        screens::close(into, from);
        return;
    }
    if desk.at == Where::Monitor {
        screens::monitor::draw(into, from, &desk.reading, &screens::monitor::Doing::Idle);
    } else {
        into.put(2, from + 1, desk.at.title(), Ink::Heading);
        into.put(
            2,
            from + 3,
            "this screen is drawn by the console and comes here next",
            Ink::Quiet,
        );
    }
    screens::close(into, from);
}

/// Opens the window and runs until it is closed.
///
/// # Errors
///
/// What SDL said, or that the window library is not provisioned.
pub fn run(socket: std::path::PathBuf) -> Result<(), String> {
    let window = sdl::Window::open("MCF", 1280, 800)?;
    let mut desk = Desk::new(socket);
    desk.refresh();
    desk.reading = desk.sampler.read();

    let mut last = std::time::Instant::now();
    loop {
        while let Some(event) = window.next_event() {
            match sdl::event_type(&event) {
                sdl::EVENT_QUIT => return Ok(()),
                sdl::EVENT_KEY_DOWN => match sdl::event_key(&event) {
                    sdl::KEY_ESCAPE => return Ok(()),
                    key if key == u32::from(b'q') => return Ok(()),
                    sdl::KEY_LEFT => desk.cursor = desk.cursor.saturating_sub(1),
                    sdl::KEY_RIGHT => {
                        desk.cursor = (desk.cursor + 1).min(Where::ALL.len() - 1);
                    }
                    sdl::KEY_RETURN => {
                        let chosen = Where::ALL
                            .get(desk.cursor)
                            .copied()
                            .unwrap_or(Where::Monitor);
                        if chosen == Where::Exit {
                            return Ok(());
                        }
                        desk.at = chosen;
                        desk.refresh();
                    }
                    sdl::KEY_UP => desk.row = desk.row.saturating_sub(1),
                    sdl::KEY_DOWN => {
                        desk.row = (desk.row + 1).min(desk.models.len().saturating_sub(1));
                    }
                    key if key == u32::from(b'r') => desk.refresh(),
                    _ => {}
                },
                _ => {}
            }
        }

        // A second between samples, and only on the monitor — the same rule the
        // console follows, for the same reason: an idle window should not be
        // why a fan is running.
        if desk.at == Where::Monitor && last.elapsed() >= std::time::Duration::from_secs(1) {
            desk.reading = desk.sampler.read();
            last = std::time::Instant::now();
        }

        let (width, height) = window.size();
        let (columns, rows) = grid(width, height);
        let mut screen = Screen::new(columns, rows);
        draw(&desk, &mut screen);
        render(&window, &screen);
        window.present();

        // Nothing to do until something happens; sleeping is what makes an idle
        // window cost what a closed one costs.
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests;
