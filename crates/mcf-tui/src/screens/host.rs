//! Choosing a model to host: a short list, and everything known about one.
//!
//! **The list stays out of the way.** It is the screen with the most to say,
//! so five rows and a count is all the left column gets, and the buttons go
//! directly beneath it — leaving the whole right side and every remaining row
//! for detail.
//!
//! **Unknown is a value.** A figure nothing has measured says so and says what
//! would find out. It is never a zero and never a plausible guess (A7).

use crate::screen::{Ink, Screen};
use crate::screens::{UNKNOWN, columns, gigabytes};

/// One model, as the screen needs it.
#[derive(Debug, Clone)]
pub struct Held {
    /// The file's name.
    pub name: String,
    /// Its size.
    pub bytes: Option<u64>,
    /// What the header says it is.
    pub architecture: Option<String>,
    /// The window it was trained for.
    pub trained: Option<u64>,
    /// Cache per token of context.
    pub cache_per_token: Option<u64>,
    /// The engine MCF worked out, or why there is none.
    pub engine: Result<Resolved, String>,
    /// What the ladder measured, where it has run.
    pub measured: Measured,
}

/// The ends of a measured ladder, in the daemon's own figures.
///
/// **Text, not numbers.** Each is the string the daemon put on the wire —
/// milliseconds to three places — shown as it came, so that no surface
/// rounds a measurement its own way (B-072). `None` is not measured, which
/// the card says in those words and never as a zero (A7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measured {
    /// Milliseconds a token at the shallowest depth that separated.
    pub at_512: Option<String>,
    /// The same at the deepest.
    pub at_window: Option<String>,
    /// Milliseconds to the first token at the shallowest rung, warm.
    pub start_up: Option<String>,
}

/// What MCF worked out about running it.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The engine's component name.
    pub engine: String,
    /// The device it would use.
    pub device: String,
    /// The largest window the pair can hold.
    pub context: u64,
}

/// The buttons, and which is under the cursor.
pub const BUTTONS: [&str; 3] = [" Host this model ", " Run diagnostics ", " Back "];

/// Draws the screen. `at` is the highlighted model, `button` the highlighted
/// button, and `on_buttons` says which column the cursor is in.
pub fn draw(
    into: &mut Screen,
    from: usize,
    models: &[Held],
    at: usize,
    button: usize,
    on_buttons: bool,
) {
    let left = 2;
    let list_width = 24;
    let right = left + list_width + 2;
    list_and_buttons(into, from, models, at, button, on_buttons, left, list_width);
    detail(into, from, models, at, right);
}

/// The left column: five rows of models and the buttons beneath them.
#[allow(
    clippy::too_many_arguments,
    reason = "one column's geometry, passed once"
)]
fn list_and_buttons(
    into: &mut Screen,
    from: usize,
    models: &[Held],
    at: usize,
    button: usize,
    on_buttons: bool,
    left: usize,
    list_width: usize,
) {
    into.put(left, from, "MODELS", Ink::Heading);
    if models.len() > 5 {
        into.put_right(
            left + list_width,
            from,
            &format!("{} of {}", at.saturating_add(1), models.len()),
            Ink::Quiet,
        );
    }
    let shown = 5.min(models.len());
    let start = at.saturating_sub(shown.saturating_sub(1)).min(at);
    for offset in 0..shown {
        let index = start + offset;
        let Some(model) = models.get(index) else {
            break;
        };
        let row = from + 1 + offset;
        let here = index == at && !on_buttons;
        let ink = if here { Ink::Selected } else { Ink::Plain };
        if here {
            for column in left..left + list_width {
                into.put(column, row, " ", Ink::Selected);
            }
        }
        let name: String = model.name.chars().take(15).collect();
        into.put(left, row, if here { "›" } else { " " }, ink);
        into.put(left + 1, row, &name, ink);
        into.put_right(
            left + list_width,
            row,
            &model
                .bytes
                .map_or_else(|| UNKNOWN.to_owned(), |b| gigabytes(b).replace(" GB", "G")),
            ink,
        );
    }

    // ── the buttons, directly beneath ───────────────────────────────────
    let mut row = from + 7;
    into.put(left, row, "ACTIONS", Ink::Heading);
    row += 1;
    for (index, label) in BUTTONS.iter().enumerate() {
        let ink = if on_buttons && index == button {
            Ink::Selected
        } else {
            Ink::Plain
        };
        into.put(left + 1, row + index, label, ink);
    }
}

/// The right column: everything known about the highlighted model.
fn detail(into: &mut Screen, from: usize, models: &[Held], at: usize, right: usize) {
    let last = into.height().saturating_sub(1);
    let Some(model) = models.get(at) else {
        into.put(right, from, "nothing is held", Ink::Quiet);
        into.put(
            right,
            from + 2,
            "`mcf pull` brings a model to this machine",
            Ink::Quiet,
        );
        return;
    };
    let name: String = model.name.chars().take(50).collect();
    into.put(right, from, &name, Ink::Heading);

    let mut row = from + 2;
    pair(
        into,
        right,
        row,
        "size",
        &model.bytes.map_or_else(|| UNKNOWN.to_owned(), gigabytes),
        Ink::Plain,
    );
    row += 1;
    pair(
        into,
        right,
        row,
        "architecture",
        model.architecture.as_deref().unwrap_or(UNKNOWN),
        Ink::Plain,
    );
    row += 1;
    pair(
        into,
        right,
        row,
        "trained context",
        &model
            .trained
            .map_or_else(|| UNKNOWN.to_owned(), |t| format!("{t} tokens")),
        Ink::Plain,
    );
    row += 1;
    pair(
        into,
        right,
        row,
        "cache per token",
        &model.cache_per_token.map_or_else(
            || UNKNOWN.to_owned(),
            |b| {
                #[allow(clippy::integer_division, reason = "bytes to kibibytes, exactly")]
                {
                    format!("{} KiB", b / 1024)
                }
            },
        ),
        Ink::Plain,
    );
    engine_and_measured(into, row + 2, model, right, last);
}

/// One label and its value, in the detail panel's two columns.
fn pair(into: &mut Screen, right: usize, row: usize, label: &str, value: &str, ink: Ink) {
    columns(
        into,
        right,
        row,
        &[(label, 18, false, Ink::Quiet), (value, 24, false, ink)],
    );
}

/// Where it runs, and what has been measured of it.
fn engine_and_measured(into: &mut Screen, from: usize, model: &Held, right: usize, last: usize) {
    let mut row = from;
    match &model.engine {
        Ok(resolved) => {
            pair(into, right, row, "engine", &resolved.engine, Ink::Held);
            row += 1;
            let device: String = resolved.device.chars().take(24).collect();
            pair(into, right, row, "runs on", &device, Ink::Held);
            row += 1;
            pair(
                into,
                right,
                row,
                "largest window",
                &format!("{} tokens", resolved.context),
                Ink::Plain,
            );
        }
        Err(why) => {
            into.put(right, row, "cannot run here", Ink::Refusal);
            row += 1;
            let wrapped: String = why.chars().take(50).collect();
            into.put(right, row, &wrapped, Ink::Refusal);
            if why.chars().count() > 50 {
                row += 1;
                let rest: String = why.chars().skip(50).take(50).collect();
                into.put(right, row, &rest, Ink::Refusal);
            }
        }
    }
    row += 2;

    // Measured figures, and what has not been.
    if row + 3 < last {
        columns(
            into,
            right,
            row,
            &[
                ("MEASURED", 26, false, Ink::Heading),
                ("SPEED", 16, true, Ink::Quiet),
            ],
        );
        row += 1;
        let figures = &model.measured;
        let mut missing = false;
        for (label, figure, unit) in [
            ("at 512 tokens", &figures.at_512, " ms/token"),
            ("at the largest window", &figures.at_window, " ms/token"),
            ("start-up to first token", &figures.start_up, " ms"),
        ] {
            let (said, ink) = figure.as_ref().map_or_else(
                || {
                    missing = true;
                    ("Unknown".to_owned(), Ink::Refusal)
                },
                |ms| (format!("{ms}{unit}"), Ink::Held),
            );
            columns(
                into,
                right,
                row,
                &[
                    (label, 26, false, Ink::Quiet),
                    (said.as_str(), 16, true, ink),
                ],
            );
            row += 1;
        }
        if row < last && missing {
            into.put(right, row, "run diagnostics to fill these in", Ink::Quiet);
        }
    }
}
