//! The console's screens, drawn in pixels.
//!
//! **The layout is the terminal's; the drawing is not.** The console's three
//! screens — the machine live, a model with everything known about it, and a
//! measurement being set up — are what an operator asked for and worked
//! through, and a window that invented its own arrangement of the same facts
//! threw that away. So this draws *those* screens: the same sections in the
//! same order, the same columns, the same words.
//!
//! **What the window adds is room and type, not a different idea.** A cell
//! grid can right-align a column; it cannot set a heading in a lighter weight,
//! give a table a hairline, or put a button under a list at the size a pointer
//! wants. Those are the differences and they are the only ones.
//!
//! **Where the console says `Unknown`, so does this.** A7 is not a terminal
//! convention: a figure nobody measured is absent on every surface, and the
//! two say so in the same word.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "every number here is a screen coordinate, a pixel count or a \
              font size. They are bounded by the size of a window — a few \
              thousand — which is exactly representable in f32 and far inside \
              i64, and the conversions are between the integer a buffer is \
              indexed by and the float geometry is done in"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter, Rgb};
use crate::ui::{self, Kind, Mouse};
use crate::words;
use crate::{Act, Desk, Doing, Model, Page};
use mcf_record::json::Value;

/// The bar across the top, in points.
const MENU: f32 = 46.0;
/// The breathing room around a screen's content.
const PAD: f32 = 26.0;

/// The type scale.
mod size {
    /// A column heading: small, spaced, quiet.
    pub(super) const LABEL: f32 = 9.5;
    /// A caption or a unit.
    pub(super) const SMALL: f32 = 12.0;
    /// A table row.
    pub(super) const BODY: f32 = 13.5;
    /// A section heading.
    pub(super) const HEAD: f32 = 17.0;
}

/// What the console prints where it has no figure. The same word, because it
/// is the same absence (A7).
pub const UNKNOWN: &str = "Unknown";

/// Draws everything, and returns what the click meant.
pub fn draw(paint: &mut Painter, desk: &Desk, mouse: &Mouse) -> Option<Act> {
    let (width, height) = paint.size();
    paint.begin();
    let mut act = menu_bar(paint, desk, mouse, width);

    let main = Box::new(
        PAD,
        MENU + PAD,
        (width - PAD * 2.0).max(10.0),
        (height - MENU - PAD * 2.0).max(10.0),
    );
    let went = match desk.page {
        Page::Monitor => monitor(paint, desk, main),
        Page::Host | Page::Models => host(paint, desk, mouse, main),
        Page::Diagnostics => diagnostics(paint, desk, mouse, main),
        Page::Adding => adding(paint, desk, mouse, main),
        Page::Hosting => hosting(paint, desk, mouse, main),
        Page::Settings => settings(paint, main),
        Page::Exit => leaving(paint, mouse, main),
    };
    act = went.or(act);
    paint.end();
    act
}

/// The menu across the top: the console's entries, in the console's order.
fn menu_bar(paint: &mut Painter, desk: &Desk, mouse: &Mouse, width: f32) -> Option<Act> {
    let ink = paint.ink;
    paint.rect(Box::new(0.0, 0.0, width, MENU), ink.sunk);
    paint.rule((0.0, MENU), (width, MENU), ink.line, 255);

    let mut act = None;
    let mut x = 14.0;
    for (page, label) in Page::MENU {
        let wide = paint.measure(label, Weight::Bold, size::BODY) + 26.0;
        let area = Box::new(x, 8.0, wide, 30.0);
        let here = desk.page.section() == *page;
        if here {
            paint.panel(area, ui::RADIUS, ink.accent, 255);
        } else if mouse.over(area) {
            paint.panel(area, ui::RADIUS, ink.line, 130);
        }
        paint.say_centred(
            area,
            label,
            if here { Weight::Bold } else { Weight::Regular },
            size::BODY,
            if here { ink.accent_ink } else { ink.quiet },
        );
        if mouse.clicked(area) {
            act = Some(Act::Go(*page));
        }
        x += wide + 4.0;
    }

    // What MCF is, at the right, where the console puts it.
    let said = desk.state_word();
    paint.say_right(
        width - 16.0,
        15.0,
        &said,
        Weight::Bold,
        size::SMALL,
        ink.faint,
    );
    act
}

/// A column in a table: where it ends, and whether its figures are flush right.
struct Column {
    /// The heading.
    head: &'static str,
    /// Where the column ends, in points from the table's left edge.
    at: f32,
    /// Whether the value is set flush against `at`.
    right: bool,
}

/// Draws a table's heading row and returns the row below it.
fn heads(paint: &mut Painter, area: Box, first: &str, columns: &[Column]) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, first, ink.faint);
    for column in columns {
        if column.right {
            let wide = spaced_width(paint, column.head);
            spaced(
                paint,
                area.x + column.at - wide,
                area.y,
                column.head,
                ink.faint,
            );
        } else {
            spaced(paint, area.x + column.at, area.y, column.head, ink.faint);
        }
    }
    let under = area.y + 17.0;
    paint.rule((area.x, under), (area.right(), under), ink.line, 255);
    under + 10.0
}

/// One row of a table.
fn row(paint: &mut Painter, area: Box, y: f32, first: &str, cells: &[(&Column, String)]) {
    let ink = paint.ink;
    let room = cells.first().map_or(area.w, |(column, _)| column.at - 14.0);
    let shortened = paint.elide(first, Weight::Regular, size::BODY, room);
    paint.say_at(
        area.x,
        y,
        &shortened,
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    for (column, value) in cells {
        // An absent figure is drawn quietly and a present one is not, so that
        // a table of unknowns reads as an absence rather than as data (A7).
        let colour = if value == UNKNOWN || value == "—" {
            ink.faint
        } else {
            ink.ink
        };
        if column.right {
            paint.say_right(
                area.x + column.at,
                y,
                value,
                Weight::Bold,
                size::BODY,
                colour,
            );
        } else {
            paint.say_at(
                area.x + column.at,
                y,
                value,
                Weight::Bold,
                size::BODY,
                colour,
            );
        }
    }
}

/// A heading in small capitals with a little tracking, as the console sets its
/// column headings — by hand, because a run of glyphs has no such thing.
fn spaced(paint: &mut Painter, x: f32, y: f32, text: &str, colour: Rgb) {
    let shouted = text.to_uppercase();
    let mut pen = x;
    for letter in shouted.chars() {
        let one = letter.to_string();
        paint.say_at(pen, y, &one, Weight::Bold, size::LABEL, colour);
        pen += paint.measure(&one, Weight::Bold, size::LABEL) + 0.7;
    }
}

/// How wide [`spaced`] will draw something.
fn spaced_width(paint: &mut Painter, text: &str) -> f32 {
    text.to_uppercase()
        .chars()
        .map(|letter| paint.measure(&letter.to_string(), Weight::Bold, size::LABEL) + 0.7)
        .sum()
}

// ── Monitor ──────────────────────────────────────────────────────────────────

/// **The machine, live.** Three tables and a line saying what MCF is doing, in
/// the console's order.
fn monitor(paint: &mut Painter, desk: &Desk, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let wide = area.w.min(700.0);
    let mut y = processors_table(paint, Box::new(area.x, area.y, wide, 0.0), desk);
    y = memory_table(paint, Box::new(area.x, y + 16.0, wide, 0.0), desk);
    let _bottom = storage_table(
        paint,
        Box::new(area.x, y + 16.0, wide, area.bottom() - y - 90.0),
        desk,
    );

    // The divider and the state line sit against the bottom, as the console
    // has them, so the tables above can grow without moving them.
    let divider = area.bottom() - 64.0;
    paint.rule((area.x, divider), (area.x + wide, divider), ink.line, 255);
    let (word, said) = desk.state_line();
    paint.say_at(
        area.x,
        divider + 22.0,
        &word,
        Weight::Bold,
        size::BODY,
        ink.accent,
    );
    paint.say_at(
        area.x + 82.0,
        divider + 22.0,
        &said,
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    None
}

/// The processor and every card, with what each is doing.
fn processors_table(paint: &mut Painter, table: Box, desk: &Desk) -> f32 {
    let reading = &desk.reading;
    let dash = || "—".to_owned();
    let processors = [
        Column {
            head: "load",
            at: 270.0,
            right: true,
        },
        Column {
            head: "temp",
            at: 360.0,
            right: true,
        },
        Column {
            head: "power",
            at: 450.0,
            right: true,
        },
        Column {
            head: "clock",
            at: 560.0,
            right: true,
        },
        Column {
            head: "cores",
            at: 650.0,
            right: true,
        },
    ];
    let mut y = heads(paint, table, "processor", &processors);
    let cpu = &reading.processor;
    row(
        paint,
        table,
        y,
        "CPU",
        &[
            (
                &processors[0],
                cpu.load
                    .map_or_else(dash, |load| format!("{} %", load.whole())),
            ),
            (
                &processors[1],
                cpu.temperature
                    .map_or_else(dash, |held| format!("{held} °C")),
            ),
            (&processors[2], dash()),
            (
                &processors[3],
                cpu.clock
                    .map_or_else(dash, |mhz| format!("{:.2} GHz", f64::from(mhz) / 1000.0)),
            ),
            (
                &processors[4],
                cpu.cores.map_or_else(dash, |held| held.to_string()),
            ),
        ],
    );
    y += 23.0;
    for card in &reading.cards {
        row(
            paint,
            table,
            y,
            &format!("GPU  {}", card.name),
            &[
                (
                    &processors[0],
                    card.load
                        .map_or_else(dash, |load| format!("{} %", load.whole())),
                ),
                (
                    &processors[1],
                    card.temperature
                        .map_or_else(dash, |held| format!("{held} °C")),
                ),
                (
                    &processors[2],
                    card.power.map_or_else(dash, |watts| format!("{watts} W")),
                ),
                (&processors[3], dash()),
                (&processors[4], dash()),
            ],
        );
        y += 23.0;
    }
    y
}

/// System memory and each card's, used against total.
fn memory_table(paint: &mut Painter, table: Box, desk: &Desk) -> f32 {
    let reading = &desk.reading;
    let dash = || "—".to_owned();
    let memory = [
        Column {
            head: "used",
            at: 270.0,
            right: true,
        },
        Column {
            head: "total",
            at: 375.0,
            right: true,
        },
        Column {
            head: "free",
            at: 480.0,
            right: true,
        },
    ];
    let mut y = heads(paint, table, "memory", &memory);
    let gigabytes =
        |bytes: Option<u64>| bytes.map_or_else(dash, |held| format!("{:.2} GB", held as f64 / 1e9));
    let system = &reading.memory;
    row(
        paint,
        table,
        y,
        "System",
        &[
            (
                &memory[0],
                gigabytes(match (system.total, system.available) {
                    (Some(total), Some(free)) => Some(total.saturating_sub(free)),
                    _ => None,
                }),
            ),
            (&memory[1], gigabytes(system.total)),
            (&memory[2], gigabytes(system.available)),
        ],
    );
    y += 23.0;
    for card in &reading.cards {
        row(
            paint,
            table,
            y,
            "Graphics",
            &[
                (&memory[0], gigabytes(card.used)),
                (&memory[1], gigabytes(card.total)),
                (
                    &memory[2],
                    gigabytes(match (card.total, card.used) {
                        (Some(total), Some(used)) => Some(total.saturating_sub(used)),
                        _ => None,
                    }),
                ),
            ],
        );
        y += 23.0;
    }
    y
}

/// Every disk with traffic worth showing.
fn storage_table(paint: &mut Painter, table: Box, desk: &Desk) -> f32 {
    let reading = &desk.reading;
    let dash = || "—".to_owned();
    let storage = [
        Column {
            head: "read",
            at: 270.0,
            right: true,
        },
        Column {
            head: "write",
            at: 375.0,
            right: true,
        },
        Column {
            head: "temp",
            at: 480.0,
            right: true,
        },
    ];
    let mut y = heads(paint, table, "storage", &storage);
    let rate = |bytes: Option<u64>| {
        bytes.map_or_else(dash, |held| format!("{:.1} MB/s", held as f64 / 1e6))
    };
    for disk in &reading.disks {
        row(
            paint,
            table,
            y,
            &disk.name,
            &[
                (&storage[0], rate(disk.read)),
                (&storage[1], rate(disk.written)),
                (
                    &storage[2],
                    disk.temperature
                        .map_or_else(dash, |held| format!("{held} °C")),
                ),
            ],
        );
        y += 23.0;
        if y > table.bottom() {
            break;
        }
    }
    y
}

// ── Host, and Models, which is the same screen ───────────────────────────────

/// **A model and everything known about it.** A narrow list on the left with
/// the actions under it, and every statistic on the right — the console's
/// arrangement, and the reason it is this way round: the list is what you move
/// through, the detail is what you read.
fn host(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let list = 250.0_f32;
    let right = area.x + list + 40.0;
    let mut act = None;
    let actions_at = area.bottom() - 160.0;

    spaced(paint, area.x, area.y, "models", ink.faint);
    let mut y = area.y + 26.0;
    if desk.models.is_empty() {
        paint.say_at(
            area.x,
            y,
            "none held",
            Weight::Regular,
            size::BODY,
            ink.faint,
        );
    }
    for (at, held) in desk.models.iter().enumerate() {
        if y > actions_at - 46.0 {
            paint.say_at(
                area.x,
                y,
                &format!("… and {} more", desk.models.len().saturating_sub(at)),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            break;
        }
        let where_ = Box::new(area.x - 6.0, y - 4.0, list, 24.0);
        let chosen = desk.chosen == Some(at);
        if chosen {
            paint.panel(where_, 6.0, ink.accent_soft, 255);
        } else if mouse.over(where_) {
            paint.panel(where_, 6.0, ink.line, 110);
        }
        let name = paint.elide(&held.name, Weight::Regular, size::BODY, list - 80.0);
        paint.say_at(
            area.x,
            y,
            &name,
            if chosen {
                Weight::Bold
            } else {
                Weight::Regular
            },
            size::BODY,
            if chosen { ink.accent } else { ink.ink },
        );
        paint.say_right(
            area.x + list - 14.0,
            y,
            &held.bytes.map_or_else(
                || UNKNOWN.to_owned(),
                |bytes| format!("{:.2}G", bytes as f64 / 1e9),
            ),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        if mouse.clicked(where_) {
            act = Some(Act::Choose(at));
        }
        y += 24.0;
    }

    // The actions, under the list, where they were asked to be.
    spaced(paint, area.x, actions_at, "actions", ink.faint);
    let mut y = actions_at + 26.0;
    for (label, kind, what) in [
        ("Host this model", Kind::Primary, Act::Go(Page::Hosting)),
        (
            "Run diagnostics",
            Kind::Ordinary,
            Act::Go(Page::Diagnostics),
        ),
        ("Add a model", Kind::Ordinary, Act::Go(Page::Adding)),
    ] {
        let where_ = Box::new(area.x, y, list - 20.0, 30.0);
        let needs_one = what != Act::Go(Page::Adding);
        if ui::button(paint, mouse, where_, label, kind) && (!needs_one || desk.chosen.is_some()) {
            act = Some(what);
        }
        y += 36.0;
    }

    let Some(held) = desk.chosen.and_then(|at| desk.models.get(at)) else {
        paint.say_at(
            right,
            area.y,
            "Choose a model on the left.",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return act;
    };
    detail(
        paint,
        Box::new(right, area.y, area.right() - right, area.h),
        held,
    );
    act
}

/// Everything known about one model, as the console lists it.
fn detail(paint: &mut Painter, area: Box, held: &Model) {
    let ink = paint.ink;
    let name = paint.elide(&held.name, Weight::Bold, size::HEAD, area.w);
    paint.say_at(area.x, area.y, &name, Weight::Bold, size::HEAD, ink.ink);
    let mut y = area.y + 36.0;
    let unknown = || UNKNOWN.to_owned();

    let said = |paint: &mut Painter, y: f32, name: &str, value: &str, colour: Rgb| {
        paint.say_at(area.x, y, name, Weight::Regular, size::BODY, ink.quiet);
        let shown = paint.elide(value, Weight::Bold, size::BODY, area.w - 200.0);
        paint.say_at(area.x + 190.0, y, &shown, Weight::Bold, size::BODY, colour);
    };

    for (name, value) in [
        (
            "size",
            held.bytes
                .map_or_else(unknown, |bytes| format!("{:.2} GB", bytes as f64 / 1e9)),
        ),
        (
            "architecture",
            held.architecture.clone().unwrap_or_else(unknown),
        ),
        (
            "trained context",
            held.trained
                .map_or_else(unknown, |held| format!("{} tokens", words::grouped(held))),
        ),
        (
            "cache per token",
            held.cache_per_token.map_or_else(unknown, |bytes| {
                #[expect(clippy::integer_division, reason = "bytes into whole kibibytes")]
                let kibibytes = bytes / 1024;
                format!("{kibibytes} KiB")
            }),
        ),
    ] {
        let colour = if value == UNKNOWN { ink.faint } else { ink.ink };
        said(paint, y, name, &value, colour);
        y += 22.0;
    }
    y += 14.0;

    match &held.refused {
        Some(why) => {
            let lines = paint.wrap(why, Weight::Regular, size::BODY, area.w);
            for line in lines.iter().take(3) {
                paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
                y += 20.0;
            }
        }
        None => {
            for (name, value) in [
                ("engine", held.engine.clone().unwrap_or_else(unknown)),
                ("runs on", held.device.clone().unwrap_or_else(unknown)),
                (
                    "largest window",
                    held.context
                        .map_or_else(unknown, |held| format!("{} tokens", words::grouped(held))),
                ),
            ] {
                let colour = if value == UNKNOWN { ink.faint } else { ink.ink };
                said(paint, y, name, &value, colour);
                y += 22.0;
            }
        }
    }
    y += 20.0;

    // What has been measured, which for most models is nothing — and the
    // console says so in this many words, so this does too (A7, A9).
    let wide = area.w.min(430.0);
    let speed = Column {
        head: "speed",
        at: wide,
        right: true,
    };
    let table = Box::new(area.x, y, wide, 0.0);
    y = heads(paint, table, "measured", &[speed]);
    let speed = Column {
        head: "speed",
        at: wide,
        right: true,
    };
    for (what, value) in [
        ("at 512 tokens", held.speed_at_512()),
        ("at the largest window", held.speed_at_window()),
        ("cold start", held.cold_start()),
    ] {
        row(paint, table, y, what, &[(&speed, value)]);
        y += 22.0;
    }
    if !held.measured() {
        paint.say_at(
            area.x,
            y + 8.0,
            "run diagnostics to fill these in",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

// ── Diagnostics ──────────────────────────────────────────────────────────────

/// **Setting up a measurement.** Two buttons with what they cost, what will be
/// measured, and the tests — the console's arrangement.
fn diagnostics(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let running = matches!(&desk.doing, Doing::Measuring(job) if !job.finished);
    let wide = area.w.min(680.0);

    let quick = Box::new(area.x, area.y, 150.0, 34.0);
    let selected = Box::new(quick.right() + 18.0, area.y, 170.0, 34.0);
    if ui::button(paint, mouse, quick, "Quick Run", Kind::Primary) && !running {
        act = Some(Act::Measure {
            deepest: desk.quick_depth(),
        });
    }
    if ui::button(paint, mouse, selected, "Run Selected", Kind::Ordinary) && !running {
        act = Some(Act::Measure {
            deepest: desk.window,
        });
    }
    let (low, high) = desk.estimate(true);
    paint.say_centred(
        Box::new(quick.x, quick.bottom(), quick.w, 20.0),
        &span(low, high),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let (low, high) = desk.estimate(false);
    paint.say_centred(
        Box::new(selected.x, selected.bottom(), selected.w, 20.0),
        &span(low, high),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let mut y = area.y + 78.0;

    spaced(paint, area.x, y, "what to measure", ink.faint);
    y += 28.0;

    let chosen = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .map_or_else(|| "none chosen".to_owned(), |held| held.name.clone());
    for (label, value, act_of) in [
        ("model", chosen, Act::Go(Page::Host)),
        (
            "context window",
            format!("{} tokens", words::grouped(desk.window)),
            Act::NextWindow,
        ),
    ] {
        paint.say_at(area.x, y, label, Weight::Regular, size::BODY, ink.quiet);
        let picker = Box::new(area.x + 190.0, y - 6.0, 340.0, 28.0);
        let hot = mouse.over(picker);
        paint.edge(
            picker,
            ui::RADIUS,
            ink.line,
            if hot { ink.sunk } else { ink.card },
        );
        let shown = paint.elide(&value, Weight::Regular, size::BODY, picker.w - 44.0);
        paint.say_at(
            picker.x + 12.0,
            y,
            &shown,
            Weight::Regular,
            size::BODY,
            ink.ink,
        );
        ui::chevron(paint, (picker.right() - 22.0, y + 7.0), ink.faint);
        if mouse.clicked(picker) {
            act = Some(act_of);
        }
        y += 34.0;
    }

    // Choosing a window implies every power of two below it, so the depths are
    // stated under it rather than offered as a second set of choices somebody
    // could contradict the first with.
    paint.say_at(area.x, y, "samples", Weight::Regular, size::BODY, ink.quiet);
    paint.say_at(
        area.x + 190.0,
        y,
        &desk.ladder_line(),
        Weight::Regular,
        size::BODY,
        ink.ink,
    );
    paint.say_right(
        area.x + wide,
        y,
        "every step",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 42.0;

    let (chose, below) = tests_table(paint, desk, mouse, Box::new(area.x, y, wide, 0.0));
    act = chose.or(act);
    readings(
        paint,
        desk,
        Box::new(area.x, below, wide, area.bottom() - below),
    );
    act
}

/// The tests, one row each, with what each needs and what each costs.
fn tests_table(paint: &mut Painter, desk: &Desk, mouse: &Mouse, table: Box) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let wide = table.w;
    let mut act = None;
    let mut y = heads(
        paint,
        table,
        "tests",
        &[
            Column {
                head: "devices",
                at: wide - 130.0,
                right: true,
            },
            Column {
                head: "time",
                at: wide,
                right: true,
            },
        ],
    );
    for (at, test) in desk.tests.iter().enumerate() {
        let hit = Box::new(table.x - 6.0, y - 5.0, wide - 100.0, 24.0);
        if mouse.over(hit) {
            paint.panel(hit, 6.0, ink.line, 90);
        }
        let mark = Box::new(table.x, y + 2.0, 13.0, 13.0);
        if test.chosen {
            paint.panel(mark, 3.0, ink.accent, 255);
            ui::tick(paint, mark, ink.accent_ink);
        } else {
            paint.edge(mark, 3.0, ink.line, ink.card);
        }
        let name = paint.elide(test.name, Weight::Regular, size::BODY, wide - 290.0);
        paint.say_at(
            table.x + 26.0,
            y,
            &name,
            Weight::Regular,
            size::BODY,
            ink.ink,
        );
        paint.say_right(
            table.x + wide - 130.0,
            y,
            test.devices,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        paint.say_right(
            table.x + wide,
            y,
            &clock(test.seconds),
            Weight::Bold,
            size::BODY,
            ink.ink,
        );
        if mouse.clicked(hit) {
            act = Some(Act::Toggle(at));
        }
        y += 24.0;
    }
    y += 12.0;

    let picked = desk.tests.iter().filter(|test| test.chosen).count();
    let (low, high) = desk.estimate(false);
    paint.say_at(
        table.x,
        y,
        "selected",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    paint.say_at(
        table.x + 90.0,
        y,
        &format!("{picked} of {}", desk.tests.len()),
        Weight::Bold,
        size::SMALL,
        ink.ink,
    );
    paint.say_at(
        table.x + 200.0,
        y,
        "estimate",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    paint.say_at(
        table.x + 290.0,
        y,
        &span(low, high),
        Weight::Bold,
        size::SMALL,
        ink.ink,
    );
    (act, y + 30.0)
}

/// What a run has said so far, under the setup rather than instead of it.
fn readings(paint: &mut Painter, desk: &Desk, area: Box) {
    let ink = paint.ink;
    let Doing::Measuring(job) = &desk.doing else {
        return;
    };
    let mut y = area.y;
    if let Some(why) = &job.refused {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w)
            .iter()
            .take(2)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
            y += 20.0;
        }
        return;
    }
    paint.rule(
        (area.x, y - 12.0),
        (area.x + area.w, y - 12.0),
        ink.line,
        255,
    );
    for answer in &job.answers {
        let Some(reading) = answer.get("reading") else {
            continue;
        };
        let depth = reading
            .get("depth")
            .and_then(Value::as_integer)
            .unwrap_or(0);
        paint.say_at(
            area.x,
            y,
            &format!(
                "at {} tokens",
                words::grouped(u64::try_from(depth).unwrap_or(0))
            ),
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        let (said, colour) = if matches!(reading.get("measured"), Some(Value::Bool(true))) {
            (
                reading
                    .get("ms_per_token")
                    .and_then(Value::as_text)
                    .map_or_else(|| UNKNOWN.to_owned(), |ms| format!("{ms} ms a token")),
                ink.ink,
            )
        } else {
            (UNKNOWN.to_owned(), ink.faint)
        };
        paint.say_at(area.x + 190.0, y, &said, Weight::Bold, size::BODY, colour);
        y += 22.0;
        if y > area.bottom() - 24.0 {
            return;
        }
    }
    if let Some(conditions) = job.conclusion().and_then(|body| body.get("conditions")) {
        // B65 and D31: a timing from MCF's own stand-in measures the stand-in.
        // Which engine ran is a condition of every number above it.
        let ran = conditions
            .get("engine_ran")
            .and_then(Value::as_text)
            .unwrap_or("MCF did not say");
        let shown = paint.elide(
            &format!("measured on {ran}"),
            Weight::Regular,
            size::SMALL,
            area.w,
        );
        paint.say_at(
            area.x,
            y + 6.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

/// Seconds as the console writes them.
fn clock(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds} s");
    }
    #[expect(clippy::integer_division, reason = "whole minutes and the rest")]
    let (minutes, rest) = (seconds / 60, seconds % 60);
    format!("{minutes} min {rest:02} s")
}

/// A range of seconds, as the console writes one.
fn span(low: u64, high: u64) -> String {
    format!("{} – {}", clock(low), clock(high))
}

// ── The two screens the actions lead to ──────────────────────────────────────

/// Fetching a model that is not here yet.
fn adding(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "add a model", ink.faint);
    let mut y = area.y + 28.0;
    paint.say_at(
        area.x,
        y,
        "Name a model published on a hub, in the form owner/repository.",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    y += 30.0;

    let mut act = None;
    let field = Box::new(area.x, y, (area.w - 130.0).min(520.0), 32.0);
    let _clicked = ui::field(paint, mouse, field, &desk.typed, "owner/repository", true);
    let (looked, _) = ui::fitted(
        paint,
        mouse,
        (field.right() + 10.0, y),
        "Look up",
        Kind::Primary,
    );
    if looked && !desk.doing.busy() {
        act = Some(Act::LookUp);
    }
    y += 50.0;

    match &desk.doing {
        Doing::Listing(job) if !job.finished => {
            paint.say_at(area.x, y, &job.what, Weight::Regular, size::BODY, ink.quiet);
            return act;
        }
        Doing::Downloading(job) => {
            paint.say_at(area.x, y, &job.what, Weight::Bold, size::BODY, ink.ink);
            y += 24.0;
            ui::progress(
                paint,
                Box::new(area.x, y, area.w.min(520.0), 8.0),
                job.fraction(),
            );
            y += 22.0;
            paint.say_at(
                area.x,
                y,
                &downloading_line(job),
                Weight::Regular,
                size::SMALL,
                ink.quiet,
            );
            return act;
        }
        _ => {}
    }
    let Some(job) = desk.doing.job() else {
        return act;
    };
    if let Some(why) = &job.refused {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w.min(560.0))
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
            y += 20.0;
        }
        return act;
    }
    let found = job.conclusion().or_else(|| job.latest())?;
    published(
        paint,
        mouse,
        Box::new(area.x, y, area.w, area.bottom() - y),
        found,
    )
    .or(act)
}

/// The files a repository publishes, one row each.
fn published(paint: &mut Painter, mouse: &Mouse, area: Box, found: &Value) -> Option<Act> {
    let ink = paint.ink;
    let repository = found
        .get("repository")
        .and_then(Value::as_text)
        .unwrap_or("")
        .to_owned();
    let files = found
        .get("files")
        .and_then(Value::as_list)
        .map(<[Value]>::to_vec)?;
    let wide = area.w.min(640.0);
    let mut y = area.y;
    paint.say_at(area.x, y, &repository, Weight::Bold, size::HEAD, ink.ink);
    y += 30.0;
    if let Some(why) = found.get("no_plan").and_then(Value::as_text) {
        for line in paint.wrap(
            &format!("MCF cannot say which of these would run here: {why}"),
            Weight::Regular,
            size::SMALL,
            wide,
        ) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.warn);
            y += 16.0;
        }
        y += 10.0;
    }
    let mut act = None;
    for file in files.iter().take(12) {
        let name = file
            .get("file")
            .and_then(Value::as_text)
            .unwrap_or("?")
            .to_owned();
        let bytes = file
            .get("bytes")
            .and_then(Value::as_integer)
            .and_then(|bytes| u64::try_from(bytes).ok());
        let where_ = Box::new(area.x - 6.0, y - 5.0, wide, 30.0);
        if mouse.over(where_) {
            paint.panel(where_, 6.0, ink.line, 110);
        }
        let shown = paint.elide(&name, Weight::Regular, size::BODY, wide - 230.0);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::BODY, ink.ink);
        paint.say_right(
            area.x + wide - 110.0,
            y,
            &bytes.map_or_else(
                || UNKNOWN.to_owned(),
                |held| format!("{:.2} GB", held as f64 / 1e9),
            ),
            Weight::Bold,
            size::SMALL,
            ink.quiet,
        );
        let get = Box::new(area.x + wide - 90.0, y - 3.0, 76.0, 26.0);
        if ui::button(paint, mouse, get, "Get", Kind::Ordinary) {
            act = Some(Act::Download {
                reference: repository.clone(),
                file: name.clone(),
            });
        }
        y += 32.0;
        if y > area.bottom() - 20.0 {
            break;
        }
    }
    act
}

/// What a download is doing, in one line.
fn downloading_line(job: &crate::job::Job) -> String {
    let Some(latest) = job.latest() else {
        return "starting".to_owned();
    };
    if matches!(latest.get("done"), Some(Value::Bool(true))) {
        return "Done. It is on this computer and MCF has written down where it came from."
            .to_owned();
    }
    let held = |key: &str| {
        latest
            .get(key)
            .and_then(Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
    };
    match latest.get("doing").and_then(Value::as_text) {
        Some("checking") => "checking that what arrived is what was published".to_owned(),
        _ => match (held("arrived"), held("bytes")) {
            (Some(arrived), Some(total)) => format!(
                "{} of {}",
                words::size_in_words(Some(arrived)).unwrap_or_default(),
                words::size_in_words(Some(total)).unwrap_or_default()
            ),
            _ => "starting".to_owned(),
        },
    }
}

/// A model, held and answering.
fn hosting(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let Some(held) = desk.chosen.and_then(|at| desk.models.get(at)) else {
        paint.say_at(
            area.x,
            area.y,
            "Choose a model on the Host screen first.",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return None;
    };
    spaced(paint, area.x, area.y, "hosting", ink.faint);
    let name = paint.elide(&held.name, Weight::Bold, size::HEAD, area.w);
    paint.say_at(
        area.x,
        area.y + 24.0,
        &name,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    let mut y = area.y + 62.0;

    let mut act = None;
    let field = Box::new(area.x, y, (area.w - 120.0).min(640.0), 32.0);
    let _clicked = ui::field(paint, mouse, field, &desk.typed, "Ask it something", true);
    let (asked, _) = ui::fitted(
        paint,
        mouse,
        (field.right() + 10.0, y),
        "Ask",
        Kind::Primary,
    );
    if asked
        && !desk.doing.busy()
        && let Some(at) = desk.chosen
    {
        act = Some(Act::Ask { at });
    }
    y += 50.0;

    if let Doing::Answering(job) = &desk.doing
        && let Some(why) = &job.refused
    {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w.min(640.0))
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
            y += 20.0;
        }
        return act;
    }
    if desk.said.is_empty() {
        if desk.doing.busy() {
            paint.say_at(
                area.x,
                y,
                "thinking",
                Weight::Regular,
                size::BODY,
                ink.quiet,
            );
        }
        return act;
    }
    let panel = Box::new(area.x, y, area.w.min(640.0), (area.bottom() - y).max(60.0));
    ui::card(paint, panel, false);
    let lines = paint.wrap(&desk.said, Weight::Regular, size::BODY, panel.w - 32.0);
    let mut at = panel.y + 14.0;
    for line in lines {
        paint.say_at(
            panel.x + 16.0,
            at,
            &line,
            Weight::Regular,
            size::BODY,
            ink.ink,
        );
        at += 20.0;
        if at > panel.bottom() - 18.0 {
            break;
        }
    }
    act
}

/// How MCF is set up, which is nothing yet — and the console says so in these
/// words, so this does too.
fn settings(paint: &mut Painter, area: Box) -> Option<Act> {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "settings", ink.faint);
    paint.say_at(
        area.x,
        area.y + 28.0,
        "nothing to set yet",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    None
}

/// Leaving.
fn leaving(paint: &mut Painter, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    paint.say_at(
        area.x,
        area.y,
        "Close MCF?",
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    paint.say_at(
        area.x,
        area.y + 30.0,
        "Anything MCF is holding is dropped. Nothing measured is lost.",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    let (close, _) = ui::fitted(
        paint,
        mouse,
        (area.x, area.y + 62.0),
        "Close",
        Kind::Primary,
    );
    close.then_some(Act::Close)
}
