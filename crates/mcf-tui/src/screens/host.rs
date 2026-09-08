use crate::screen::{Ink, Screen};
use crate::screens::{UNKNOWN, columns, gigabytes};

#[derive(Debug, Clone)]
pub struct Held {
    pub name: String,
    pub path: String,
    pub bytes: Option<u64>,
    pub architecture: Option<String>,
    pub trained: Option<u64>,
    pub cache_per_token: Option<u64>,
    pub engine: Result<Resolved, String>,
    pub measured: Measured,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Measured {
    pub shallowest: Option<(u64, String)>,
    pub deepest: Option<(u64, String)>,
    pub start_up: Option<String>,
}

impl Measured {
    #[must_use]
    pub fn rows(&self) -> [(String, Option<&str>, &'static str); 3] {
        let at = |end: &Option<(u64, String)>, or: &str| {
            end.as_ref().map_or_else(
                || or.to_owned(),
                |(depth, _)| format!("at {} tokens", super::grouped(*depth)),
            )
        };
        [
            (
                at(&self.shallowest, "at 512 tokens"),
                self.shallowest.as_ref().map(|(_, ms)| ms.as_str()),
                " ms/token",
            ),
            (
                at(&self.deepest, "at the deepest rung"),
                self.deepest.as_ref().map(|(_, ms)| ms.as_str()),
                " ms/token",
            ),
            (
                "start-up to first token".to_owned(),
                self.start_up.as_deref(),
                " ms",
            ),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub engine: String,
    pub device: String,
    pub context: u64,
}

pub const BUTTONS: [&str; 3] = [" Host this model ", " Run diagnostics ", " Back "];

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

fn pair(into: &mut Screen, right: usize, row: usize, label: &str, value: &str, ink: Ink) {
    columns(
        into,
        right,
        row,
        &[(label, 18, false, Ink::Quiet), (value, 24, false, ink)],
    );
}

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
        let mut missing = false;
        for (label, figure, unit) in model.measured.rows() {
            let (said, ink) = figure.map_or_else(
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
                    (label.as_str(), 26, false, Ink::Quiet),
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
