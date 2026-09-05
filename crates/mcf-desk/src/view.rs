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
use crate::{Act, Caret, Desk, Doing, Model, Page, Picker, windows};
use mcf_record::json::Value;
use mcf_serve::anatomy::SaidVocabulary;

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
        Page::Prompt => prompt(paint, desk, mouse, main),
        Page::Components => components(paint, desk, mouse, main),
        Page::Anatomy => anatomy(paint, desk, mouse, main),
        Page::Vocabulary => vocabulary(paint, desk, mouse, main),
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

    // What MCF is, at the right, where the console puts it — and what it is
    // doing and for how long, where it is doing anything, on every page. Cut
    // to the room left of the menu rather than drawn over it.
    let said = paint.elide(
        &desk.state_word(),
        Weight::Bold,
        size::SMALL,
        (width - 16.0 - x - 12.0).max(40.0),
    );
    paint.say_right(
        width - 16.0,
        15.0,
        &said,
        Weight::Bold,
        size::SMALL,
        if desk.doing.busy() {
            ink.accent
        } else {
            ink.faint
        },
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
/// What is being served, and where a caller reaches it.
///
/// Returns the bottom of the card, so what follows does not need to know how
/// tall it was.
/// What a context window of this size costs, in bytes of cache and with the
/// weights beside it.
///
/// **The window is the part somebody chooses, and it was the part nobody
/// could see.** A model's weights are what they are; the context is a setting,
/// and on a large machine MCF's recommendation is the whole trained window —
/// which reserved three times the model's own size on one held here. A setting
/// whose cost only appears in `free -h` after the fact is a decision made
/// blind (§3.15).
#[must_use]
pub fn reserve_of(held: &Model, context: u64) -> Option<(u64, Option<u64>)> {
    let cache = held.cache_per_token?.saturating_mul(context);
    Some((cache, held.bytes.map(|held| held.saturating_add(cache))))
}

/// Bytes as a figure a person reads.
fn gigabytes(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a memory figure shown to one decimal place"
    )]
    let held = bytes as f64 / 1e9;
    format!("{held:.1} GB")
}

/// What one context window reserves, as a line.
#[must_use]
pub fn reserve_line(held: &Model, context: u64) -> Option<String> {
    let (cache, total) = reserve_of(held, context)?;
    Some(match total {
        Some(total) => format!(
            "{} of cache reserved — {} with the weights",
            gigabytes(cache),
            gigabytes(total)
        ),
        None => format!("{} of cache reserved", gigabytes(cache)),
    })
}

/// What the window now being held is costing, where that can be said.
///
/// Asked before the card is laid out as well as inside it, because a card
/// sized for four lines and drawn with five puts the fifth through its own
/// border.
fn held_window_cost(desk: &Desk) -> Option<String> {
    let context = desk.hosted.as_ref()?.context?;
    reserve_line(desk.hosted_model()?, context)
}

fn hosted_card(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    ui::card(paint, at, desk.hosted.is_some());
    let Some(hosting) = &desk.hosted else {
        paint.say_at(
            at.x + 14.0,
            at.y + 20.0,
            "nothing is being served",
            Weight::Bold,
            size::BODY,
            ink.quiet,
        );
        paint.say_at(
            at.x + 14.0,
            at.y + 42.0,
            "Models holds one here, and this screen then says where it answers.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return at.bottom();
    };

    // The name, not the path: a path is where a file is, and the question this
    // screen answers is what is answering.
    let name = hosting
        .model
        .rsplit('/')
        .next()
        .unwrap_or(&hosting.model)
        .to_owned();
    let shown = paint.elide(&name, Weight::Bold, size::HEAD, at.w - 190.0);
    paint.say_at(
        at.x + 14.0,
        at.y + 14.0,
        &shown,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    let _wide = ui::tag(
        paint,
        (at.right() - 120.0, at.y + 15.0),
        "Resident",
        ink.accent_soft,
        ink.good,
    );

    paint.say_at(
        at.x + 14.0,
        at.y + 40.0,
        "reachable at",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let after = paint.measure("reachable at", Weight::Regular, size::SMALL);
    paint.say_at(
        at.x + 14.0 + after + 8.0,
        at.y + 40.0,
        &hosting.address,
        Weight::Bold,
        size::BODY,
        ink.accent,
    );
    paint.say_at(
        at.x + 14.0,
        at.y + 58.0,
        "an OpenAI-compatible endpoint: give this to a tool as its base URL",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );

    // What it is held under. Conditions, beside the thing they condition.
    let context = hosting
        .context
        .map_or_else(|| UNKNOWN.to_owned(), |context| format!("{context} tokens"));
    paint.say_at(
        at.x + 14.0,
        at.y + 76.0,
        &format!("context {context}   ·   since {}", hosting.since),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    // **And what that window is costing right now.** The context is the one
    // condition on this card somebody chose, and until this line it was the
    // one whose price was invisible: a model of 17.5 GB held 38.6 GB, and the
    // difference was the window (§3.15).
    let mut y = at.y + 93.0;
    if let Some(said) = held_window_cost(desk) {
        paint.say_at(
            at.x + 14.0,
            y,
            &said,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        y += 17.0;
    }
    takes_row(paint, hosting, at, y, ink.quiet);
    at.bottom()
}

/// **What reaches it through the port, as the engine reported.** A model
/// hosted with its projector takes pictures; one hosted without takes text
/// and declines the rest without a word, so the card says which (A21,
/// §3.15). And what its template does with tools and thinking, because
/// those are the capabilities a caller on the port is choosing it for.
fn takes_row(paint: &mut Painter, hosting: &crate::Hosted, at: Box, y: f32, colour: Rgb) {
    let shown = paint.elide(
        &takes_line(hosting),
        Weight::Regular,
        size::SMALL,
        at.w - 28.0,
    );
    paint.say_at(at.x + 14.0, y, &shown, Weight::Regular, size::SMALL, colour);
}

/// What the engine said it takes, on one line.
fn takes_line(hosting: &crate::Hosted) -> String {
    let projector = hosting.projector.as_ref().map_or_else(
        || "no projector".to_owned(),
        |name| format!("projector {name}"),
    );
    hosting.takes.as_ref().map_or_else(
        || format!("takes: the engine did not say   ·   {projector}"),
        |takes| {
            format!(
                "takes {}   ·   template: {}   ·   thinking: {}",
                takes.media(),
                takes.template(),
                takes.thinking_said()
            )
        },
    )
}

/// What using it costs.
///
/// **Nothing has been timed, so nothing is claimed.** An empty figure carrying
/// the reason it is empty beats a zero that reads as a measurement (A7, §3.4).
fn cost_tiles(paint: &mut Painter, at: Box) {
    let ink = paint.ink;
    let across = (at.w - 3.0 * 10.0) / 4.0;
    for (index, (label, why)) in [
        ("Generation", "no engine is provisioned"),
        ("Prompt reading", "no engine is provisioned"),
        ("First token", "no engine is provisioned"),
        ("Energy", "this machine publishes no counter"),
    ]
    .into_iter()
    .enumerate()
    {
        #[allow(
            clippy::cast_precision_loss,
            reason = "four tiles: the index is never large enough to lose one"
        )]
        let tile = Box::new(at.x + (across + 10.0) * index as f32, at.y, across, 58.0);
        ui::card(paint, tile, false);
        spaced(paint, tile.x + 12.0, tile.y + 12.0, label, ink.faint);
        paint.say_at(
            tile.x + 12.0,
            tile.y + 28.0,
            words::UNMEASURED,
            Weight::Bold,
            size::BODY,
            ink.quiet,
        );
        let reason = paint.elide(why, Weight::Regular, size::SMALL, across - 24.0);
        paint.say_at(
            tile.x + 12.0,
            tile.y + 44.0,
            &reason,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

fn monitor(paint: &mut Painter, desk: &Desk, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let wide = area.w.min(940.0);

    // Section one: what is being served, and what it costs to use.
    spaced(paint, area.x, area.y, "hosted", ink.faint);
    // Tall enough for what it will actually say.
    let tall = if held_window_cost(desk).is_some() {
        133.0
    } else {
        113.0
    };
    let mut y = hosted_card(paint, desk, Box::new(area.x, area.y + 20.0, wide, tall));
    cost_tiles(paint, Box::new(area.x, y + 12.0, wide, 58.0));

    // Section two: the machine every one of those figures would be taken on.
    y += 82.0;
    spaced(paint, area.x, y, "this machine", ink.faint);
    y += 20.0;
    y = processors_table(paint, Box::new(area.x, y, wide, 0.0), desk);
    y = memory_table(paint, Box::new(area.x, y + 14.0, wide, 0.0), desk);
    let _bottom = storage_table(
        paint,
        Box::new(area.x, y + 14.0, wide, area.bottom() - y - 88.0),
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
    let actions_at = area.bottom() - 200.0;

    let chose = model_list(
        paint,
        desk,
        mouse,
        Box::new(area.x, area.y, list, actions_at - area.y),
    );
    act = chose.or(act);

    act = actions_panel(
        paint,
        desk,
        mouse,
        Box::new(area.x, actions_at, list, area.bottom() - actions_at),
    )
    .or(act);

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
    let after = detail(
        paint,
        Box::new(right, area.y, area.right() - right, area.h),
        held,
    );
    settings_table(
        paint,
        desk,
        mouse,
        Box::new(
            right,
            after + 18.0,
            area.right() - right,
            (area.bottom() - after - 18.0).max(10.0),
        ),
    )
    .or(act)
}

/// Why there are no settings, and what Host will do about it where it can.
fn no_settings(paint: &mut Painter, desk: &Desk, area: Box, why: &str) {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "settings", ink.faint);
    let lines = paint.wrap(why, Weight::Regular, size::SMALL, area.w.min(430.0));
    let mut y = area.y + 24.0;
    for line in lines.iter().take(3) {
        paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.warn);
        y += 16.0;
    }
    // What Host will do about it, said before it is pressed: a build is
    // minutes and a container image, and a button that started one without
    // saying so would be a hidden choice (§3.15, B-367).
    if let Some(engine) = &desk.needs_engine {
        let said = format!(
            "Host builds {engine} first — a pinned source compiled in a container, recorded — \
             then holds the model on it."
        );
        for line in paint
            .wrap(&said, Weight::Regular, size::SMALL, area.w.min(430.0))
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.quiet);
            y += 16.0;
        }
    }
}

/// Every setting the chosen model would be hosted under, with what MCF
/// recommended beside anything somebody has moved.
///
/// **These were not shown before, and one of them was wrong.** The engine was
/// started with the layer count written into the source as zero, so a model
/// resolved to a graphics card ran on the processor and nothing said so. A
/// default nobody can see is a decision nobody made (§3.15, F133).
fn settings_table(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    if let Some(why) = &desk.no_settings {
        no_settings(paint, desk, area, why);
        return None;
    }
    let (Some(settings), Some(recommended)) = (desk.settings.as_ref(), desk.recommended.as_ref())
    else {
        return None;
    };
    let wide = area.w.min(430.0);
    let mut act = None;
    let table = Box::new(area.x, area.y, wide, 0.0);
    let mut y = heads(
        paint,
        table,
        "settings",
        &[Column {
            head: "value",
            at: wide,
            right: true,
        }],
    );

    for (at, setting) in settings.listed(recommended).into_iter().enumerate() {
        if y > area.bottom() - 40.0 {
            break;
        }
        let moved = setting.value != setting.recommended;
        // Only the settings with a small set of sensible values can be
        // cycled; the engine and the device are what MCF resolved together,
        // and moving one without the other would be asking a build to use a
        // device it cannot.
        let can_cycle = matches!(at, 0 | 1 | 4 | 5 | 6 | 7 | 8);
        let hit = Box::new(area.x - 6.0, y - 4.0, wide + 12.0, 21.0);
        if can_cycle && mouse.over(hit) {
            paint.panel(hit, 6.0, ink.line, 90);
        }
        paint.say_at(
            area.x,
            y,
            setting.name,
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        let colour = if moved { ink.warn } else { ink.ink };
        let shown = paint.elide(&setting.value, Weight::Bold, size::BODY, wide - 170.0);
        paint.say_right(area.x + wide, y, &shown, Weight::Bold, size::BODY, colour);
        if can_cycle && mouse.clicked(hit) {
            act = Some(Act::Cycle(at));
        }
        y += 21.0;
        // **What this window will reserve, under the window itself.** The
        // context is chosen here and paid for in memory later, and the two
        // were on different screens — one of them `free -h`, after the fact.
        // It is recomputed from the value shown rather than fetched, so it
        // moves when the setting moves (§3.15, B-423).
        if at == 0 {
            y = what_the_window_costs(
                paint,
                desk,
                (area.x + wide, y),
                settings.context,
                recommended.context,
            );
        }
        // What MCF advised, under anything moved off it — a run under a
        // changed setting is not a run under the recommended one, and both
        // are facts (A6, §3.15).
        if moved {
            paint.say_right(
                area.x + wide,
                y,
                &format!("MCF recommends {}", setting.recommended),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            y += 17.0;
        }
    }

    if !settings.differs_from(recommended).is_empty() {
        let (reset, _) = ui::fitted(
            paint,
            mouse,
            (area.x - 8.0, y + 4.0),
            "Back to recommended",
            Kind::Quiet,
        );
        if reset {
            act = Some(Act::Recommended);
        }
    }
    act
}

/// The chosen window's price, written under the window itself.
///
/// Returns where the next row starts, which is unmoved when there is no figure
/// to give: a model whose header does not say what a token of cache costs is
/// one MCF cannot price, and a blank where a number belongs is better than a
/// zero that reads like one (A7).
fn what_the_window_costs(
    paint: &mut Painter,
    desk: &Desk,
    at: (f32, f32),
    context: u64,
    largest: u64,
) -> f32 {
    let ink = paint.ink;
    let (right, mut y) = at;
    let Some(said) = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .and_then(|held| reserve_line(held, context))
    else {
        return y;
    };
    let ceiling = context >= largest;
    let colour = if ceiling { ink.warn } else { ink.faint };
    paint.say_right(right, y, &said, Weight::Regular, size::SMALL, colour);
    y += 17.0;
    if ceiling {
        paint.say_right(
            right,
            y,
            "the largest window this machine can hold",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        y += 17.0;
    }
    y
}

/// The build, as it goes: what is being built and for what, how long so far,
/// and the last line the compiler printed — which is what tells a person that
/// minutes of silence are minutes of work (A2).
fn building(paint: &mut Painter, job: &crate::job::Job, x: f32, mut y: f32, wide: f32) {
    let ink = paint.ink;
    let printed = job
        .answers
        .iter()
        .rev()
        .find_map(|answer| answer.get("doing").and_then(Value::as_text))
        .unwrap_or("starting");
    let said = job.refused.clone().map_or_else(
        || {
            if job.finished {
                format!("built — {}", job.what)
            } else {
                format!("BUILDING — {}s so far. {}\n{printed}", job.ran(), job.what)
            }
        },
        |why| format!("the engine could not be built: {why}"),
    );
    let colour = if job.refused.is_some() {
        ink.bad
    } else {
        ink.quiet
    };
    let wrapped: Vec<String> = said
        .split('\n')
        .flat_map(|part| paint.wrap(part, Weight::Regular, size::SMALL, wide))
        .take(5)
        .collect();
    for line in &wrapped {
        paint.say_at(x, y, line, Weight::Regular, size::SMALL, colour);
        y += 16.0;
    }
}

/// What can be done with the model on the left, and where it is reachable
/// when it is being held.
fn actions_panel(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let list = area.w;
    let actions_at = area.y;
    let mut act = None;
    // The actions, under the list, where they were asked to be.
    spaced(paint, area.x, actions_at, "actions", ink.faint);
    let mut y = actions_at + 26.0;
    // Whether *this* model is the one being held, rather than whether
    // anything is: a person looking at one model and told *stop hosting* when
    // a different one is up has been told something false about what they are
    // looking at.
    let this_one = desk
        .hosted
        .as_ref()
        .zip(desk.chosen.and_then(|at| desk.models.get(at)))
        .is_some_and(|(hosting, held)| hosting.model == held.path);
    // *What is in it* on both: it reads the file and runs nothing, so a model
    // being held can be counted as well as one that is not.
    let actions: [(&str, Kind, Act); 4] = if this_one {
        [
            ("Ask it something", Kind::Primary, Act::Go(Page::Hosting)),
            ("Stop hosting", Kind::Ordinary, Act::StopHosting),
            ("What is in it", Kind::Ordinary, Act::Go(Page::Anatomy)),
            ("Add a model", Kind::Ordinary, Act::Go(Page::Adding)),
        ]
    } else {
        [
            ("Host this model", Kind::Primary, Act::HostIt),
            (
                "Run diagnostics",
                Kind::Ordinary,
                Act::Go(Page::Diagnostics),
            ),
            ("What is in it", Kind::Ordinary, Act::Go(Page::Anatomy)),
            ("Add a model", Kind::Ordinary, Act::Go(Page::Adding)),
        ]
    };
    for (label, kind, what) in actions {
        let where_ = Box::new(area.x, y, list - 20.0, 30.0);
        let needs_one = what != Act::Go(Page::Adding);
        if ui::button(paint, mouse, where_, label, kind) && (!needs_one || desk.chosen.is_some()) {
            act = Some(what);
        }
        y += 36.0;
    }
    // Where a caller reaches it. The one fact an API is for.
    if let Some(hosting) = &desk.hosted {
        paint.say_at(
            area.x,
            y + 4.0,
            "reachable at",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        let shown = paint.elide(&hosting.address, Weight::Bold, size::SMALL, list - 20.0);
        paint.say_at(
            area.x,
            y + 20.0,
            &shown,
            Weight::Bold,
            size::SMALL,
            ink.accent,
        );
    } else if let Doing::Provisioning(job) = &desk.doing {
        building(paint, job, area.x, y + 6.0, list - 16.0);
    } else if let Doing::Hosting(job) = &desk.doing {
        // **How long it has been going, rather than a promise about how long
        // it will take.** This said "a moment", and a seventy-gigabyte model
        // takes minutes: an operator who has been told *a moment* and waits
        // five is an operator who reasonably concludes it has failed. What MCF
        // knows is how long it has waited, so that is what it says (A7).
        let said = job.refused.clone().unwrap_or_else(|| {
            if job.finished {
                String::new()
            } else {
                format!(
                    "HOLDING — {}s so far. A large model is read from disk before it answers; \
                     this screen keeps up to date while it loads.",
                    job.ran()
                )
            }
        });
        let colour = if job.refused.is_some() {
            ink.bad
        } else {
            ink.quiet
        };
        let mut at = y + 6.0;
        for line in paint
            .wrap(&said, Weight::Regular, size::SMALL, list - 16.0)
            .iter()
            .take(4)
        {
            paint.say_at(area.x, at, line, Weight::Regular, size::SMALL, colour);
            at += 16.0;
        }
    }

    act
}

/// The models this machine holds, one row each.
fn model_list(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let list = area.w;
    let actions_at = area.bottom();
    let mut act = None;
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

    act
}

/// Everything known about one model, as the console lists it.
fn detail(paint: &mut Painter, area: Box, held: &Model) -> f32 {
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

    what_was_measured(paint, Box::new(area.x, y, area.w, area.h), held)
}

/// The MEASURED table, and under it the shape the table cannot carry.
fn what_was_measured(paint: &mut Painter, area: Box, held: &Model) -> f32 {
    let ink = paint.ink;
    let mut y = area.y;
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
    let [shallowest, deepest] = held.speed_rows();
    for (what, value) in [
        shallowest,
        deepest,
        ("start-up to first token".to_owned(), held.start_up()),
    ] {
        row(paint, table, y, &what, &[(&speed, value)]);
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
        return y + 20.0;
    }

    // The shape, which the table above cannot carry. Two readings are a line
    // and one is a point, so nothing is drawn until there are two (A11).
    if held.ladder.len() >= 2 {
        y += 12.0;
        paint.say_at(
            area.x,
            y,
            &format!(
                "does it slow down as the conversation grows?   {}",
                words::holds_up(held.fastest, held.slowest)
                    .unwrap_or_else(|| UNKNOWN.to_owned())
                    .to_lowercase()
            ),
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        y = crate::chart::falloff(
            paint,
            Box::new(area.x, y + 20.0, area.w.min(430.0), 72.0),
            &held.ladder,
        );
    }
    y
}

/// The three buttons at the top of the page and the estimate under each of
/// the two that run something.
fn run_buttons(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    running: bool,
) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let quick = Box::new(area.x, area.y, 150.0, 34.0);
    let selected = Box::new(quick.right() + 18.0, area.y, 170.0, 34.0);
    // Drawn quiet while a run is going: a button that will not do anything
    // should not look like the one thing to do (§3.15).
    let (quick_kind, selected_kind) = if running {
        (Kind::Quiet, Kind::Quiet)
    } else {
        (Kind::Primary, Kind::Ordinary)
    };
    if ui::button(paint, mouse, quick, "Quick Run", quick_kind) && !running {
        act = Some(Act::Measure {
            deepest: desk.quick_depth(),
        });
    }
    if ui::button(paint, mouse, selected, "Run Selected", selected_kind)
        && !running
        && desk.runs_something()
    {
        act = Some(Act::RunChosen);
    }
    // A prompt is a diagnostic about a prompt rather than about the model, so
    // it is reached from here and not from the column (B-072).
    let taking = Box::new(selected.right() + 18.0, area.y, 170.0, 34.0);
    if ui::button(paint, mouse, taking, "Prompt analysis", Kind::Ordinary) && !running {
        act = Some(Act::Go(Page::Prompt));
    }
    // Only while something runs: a run can be cut short, and the button
    // that does it is there for as long as there is one to cut.
    if running {
        let stop = Box::new(taking.right() + 18.0, area.y, 110.0, 34.0);
        if ui::button(paint, mouse, stop, "Stop", Kind::Primary) {
            act = Some(Act::Stop);
        }
    }
    let (low, high) = desk.estimate(true);
    paint.say_centred(
        Box::new(quick.x, quick.bottom(), quick.w, 20.0),
        &span(low, high),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    // Under Run Selected: the run's estimate, or that nothing chosen runs
    // from here — a button that would do nothing says so first (§3.15).
    let (low, high) = desk.estimate(false);
    paint.say_centred(
        Box::new(selected.x, selected.bottom(), selected.w, 20.0),
        &if desk.runs_something() {
            span(low, high)
        } else {
            "nothing chosen runs here".to_owned()
        },
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    act
}

/// **Setting up a measurement.** Two buttons with what they cost, what will be
/// measured, and the tests — the console's arrangement.
fn diagnostics(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let running = matches!(
        &desk.doing,
        Doing::Measuring(job) | Doing::CrossChecking(job) if !job.finished
    );
    let wide = area.w.min(680.0);

    if let Some(said) = run_buttons(paint, desk, mouse, area, running) {
        act = Some(said);
    }
    let mut y = area.y + 78.0;

    spaced(paint, area.x, y, "what to measure", ink.faint);
    y += 28.0;

    // **The two pickers are dropdowns now, and were not before.** The model
    // one navigated to the Host page and the window one cycled to the next
    // power of two, while both wore a chevron. The list each opens is drawn
    // last, over the table below, because in immediate mode the last thing
    // painted is the thing on top.
    let mut menu: Option<(Picker, Box)> = None;
    let mut act_from_card = None;
    let chosen_model = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .map_or_else(|| "none chosen".to_owned(), |held| held.name.clone());
    for (picker, label, value) in [
        (Picker::Model, "model", chosen_model),
        (
            Picker::Window,
            "context window",
            format!("{} tokens", words::grouped(desk.window)),
        ),
        (Picker::On, "put it on", on_label(desk.on)),
    ] {
        paint.say_at(area.x, y, label, Weight::Regular, size::BODY, ink.quiet);
        let box_of = Box::new(area.x + 190.0, y - 6.0, 340.0, 28.0);
        let open = desk.open == Some(picker);
        if ui::picker(paint, mouse, box_of, &value, open) {
            act = Some(Act::Open(picker));
        }
        if open {
            menu = Some((picker, box_of));
        }
        y += 34.0;
    }

    y = placement_rows(paint, desk, area.x, y, wide);
    if let Some((component, because)) = &desk.card_unused {
        if let Some(act) = card_unused(
            paint,
            desk,
            mouse,
            Box::new(area.x, y, wide, 0.0),
            component,
            because,
        ) {
            act_from_card = Some(act);
        }
        y += card_unused_height(paint, desk, wide, because);
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
    act = chose.or(act_from_card).or(act);
    readings(
        paint,
        desk,
        Box::new(area.x, below, wide, area.bottom() - below),
    );
    if let Some((picker, at)) = menu
        && let Some(picked) = open_menu(paint, desk, mouse, picker, at)
    {
        act = Some(picked);
    }
    act
}

/// Where the model lands — engine and device — as MCF resolved them, in the
/// words the Models page uses for the same two facts. Not pickers: a person
/// who wants them otherwise changes them where they are set. The device is
/// the line this page had nothing of: a run whose page does not say whether
/// it is timing a card or a processor is timing something the reader has to
/// guess at (A7, §3.4). Returns where the next row goes.
fn placement_rows(paint: &mut Painter, desk: &Desk, x: f32, mut y: f32, wide: f32) -> f32 {
    let ink = paint.ink;
    let placed = desk.chosen.and_then(|at| desk.models.get(at));
    for (label, value) in [
        ("engine", placed.and_then(|held| held.engine.clone())),
        (
            "runs on",
            placed.and_then(|held| {
                held.device.as_ref().map(|device| {
                    if held.on_a_card {
                        format!("{device} — the whole model on the card")
                    } else {
                        device.clone()
                    }
                })
            }),
        ),
    ] {
        paint.say_at(x, y, label, Weight::Regular, size::BODY, ink.quiet);
        let (said, colour) = value.map_or_else(
            || ("not resolved".to_owned(), ink.faint),
            |value| (value, ink.ink),
        );
        let shown = paint.elide(&said, Weight::Regular, size::BODY, wide - 190.0);
        paint.say_at(x + 190.0, y, &shown, Weight::Regular, size::BODY, colour);
        y += 26.0;
    }
    y + 8.0
}

/// A card nothing here drives: the daemon's sentence, a button to build the
/// engine that would, and — while it builds — how that is going.
fn card_unused(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
    component: &str,
    because: &str,
) -> Option<Act> {
    let ink = paint.ink;
    let mut y = at.y;
    for line in paint
        .wrap(because, Weight::Regular, size::SMALL, at.w)
        .iter()
        .take(3)
    {
        paint.say_at(at.x, y, line, Weight::Regular, size::SMALL, ink.warn);
        y += 16.0;
    }
    y += 6.0;
    if let Doing::Provisioning(job) = &desk.doing
        && desk.building.as_deref() == Some(component)
    {
        building(paint, job, at.x, y, at.w);
        return None;
    }
    let (pressed, _) = ui::fitted(
        paint,
        mouse,
        (at.x, y),
        &format!("Build {component}"),
        Kind::Ordinary,
    );
    (pressed && !desk.doing.busy()).then(|| Act::Build(component.to_owned()))
}

/// How much room the card line and its button take, so what follows is
/// drawn under them.
fn card_unused_height(paint: &mut Painter, desk: &Desk, wide: f32, because: &str) -> f32 {
    let lines = paint
        .wrap(because, Weight::Regular, size::SMALL, wide)
        .len()
        .min(3);
    // A build in progress prints up to four lines under its own heading.
    let under = if matches!(&desk.doing, Doing::Provisioning(_)) {
        90.0
    } else {
        ui::BUTTON + 12.0
    };
    #[expect(clippy::cast_precision_loss, reason = "at most three lines of text")]
    let text = lines as f32 * 16.0;
    text + 6.0 + under + 10.0
}

/// Where a run can put the model, in the order the menu offers them.
const ON_CHOICES: [Option<mcf_serve::control::On>; 3] = [
    None,
    Some(mcf_serve::control::On::Processor),
    Some(mcf_serve::control::On::Card),
];

/// The words for where the model goes.
fn on_label(on: Option<mcf_serve::control::On>) -> String {
    match on {
        None => "where MCF resolves it".to_owned(),
        Some(mcf_serve::control::On::Processor) => "the processor, nothing on a card".to_owned(),
        Some(mcf_serve::control::On::Card) => "the card, the whole model on it".to_owned(),
    }
}

/// Draws whichever dropdown is open, and says what was picked from it.
fn open_menu(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    picker: Picker,
    at: Box,
) -> Option<Act> {
    match picker {
        Picker::Model => {
            // **Every model this computer holds**, which is the same list Host
            // shows: a measurement is of a model, so the question is answered
            // where it is asked rather than by sending the reader elsewhere.
            let labels: Vec<String> = desk.models.iter().map(|held| held.name.clone()).collect();
            if labels.is_empty() {
                return None;
            }
            ui::options(paint, mouse, at, &labels, desk.chosen).map(Act::Choose)
        }
        Picker::On => {
            let labels: Vec<String> = ON_CHOICES.iter().map(|on| on_label(*on)).collect();
            let chosen = ON_CHOICES.iter().position(|on| *on == desk.on);
            ui::options(paint, mouse, at, &labels, chosen)
                .and_then(|index| ON_CHOICES.get(index).copied())
                .map(Act::SetOn)
        }
        Picker::Window => {
            let offered = windows();
            let labels: Vec<String> = offered
                .iter()
                .map(|held| format!("{} tokens", words::grouped(*held)))
                .collect();
            let on = offered.iter().position(|held| *held == desk.window);
            ui::options(paint, mouse, at, &labels, on)
                .and_then(|index| offered.get(index).copied())
                .map(Act::SetWindow)
        }
    }
}

/// The tests, one row each: what each needs, what it should cost, what it
/// actually cost, and a way into what it found.
///
/// **There used to be a second table under this one** — *selected 2 of 5* and
/// an estimate — and it said nothing this screen was not already saying: the
/// estimate for the selection is printed under the Run Selected button it
/// belongs to, and a count of ticks is a thing the ticks themselves show. It
/// was two tables where one would do, so the columns it was standing in for
/// are columns now.
///
/// **`time` became `estimate` because two different numbers cannot share a
/// heading.** MCF's estimate lands between 0.58x and 1.42x of a real run, so
/// the guess and the measurement sit in columns that say which they are.
fn tests_table(paint: &mut Painter, desk: &Desk, mouse: &Mouse, table: Box) -> (Option<Act>, f32) {
    let wide = table.w;
    let mut act = None;
    let mut y = heads(
        paint,
        table,
        "tests",
        &[
            Column {
                head: "devices",
                at: wide - 250.0,
                right: true,
            },
            Column {
                head: "estimate",
                at: wide - 160.0,
                right: true,
            },
            Column {
                head: "run time",
                at: wide - 76.0,
                right: true,
            },
            Column {
                head: "results",
                at: wide,
                right: true,
            },
        ],
    );
    for at in 0..desk.tests.len() {
        if let Some(said) = test_row(paint, desk, mouse, Box::new(table.x, y, wide, 0.0), at) {
            act = Some(said);
        }
        y += 24.0;
    }
    y += 12.0;
    y = found(paint, desk, Box::new(table.x, y, wide, 0.0));
    (act, y + 18.0)
}

/// One test's row. Returns what a click on it meant.
fn test_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at_row: Box,
    at: usize,
) -> Option<Act> {
    let ink = paint.ink;
    let (x, y, wide) = (at_row.x, at_row.y, at_row.w);
    let test = desk.tests.get(at)?;
    let mut act = None;
    // The row's hit area stops short of the results button, so a click meant
    // for the button never also toggles the test under it.
    let hit = Box::new(x - 6.0, y - 5.0, wide - 80.0, 24.0);
    if mouse.over(hit) {
        paint.panel(hit, 6.0, ink.line, 90);
    }
    let mark = Box::new(x, y + 2.0, 13.0, 13.0);
    if test.chosen {
        paint.panel(mark, 3.0, ink.accent, 255);
        ui::tick(paint, mark, ink.accent_ink);
    } else {
        paint.edge(mark, 3.0, ink.line, ink.card);
    }
    let name = paint.elide(test.name, Weight::Regular, size::BODY, wide - 420.0);
    paint.say_at(x + 26.0, y, &name, Weight::Regular, size::BODY, ink.ink);
    paint.say_right(
        x + wide - 250.0,
        y,
        test.devices,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    // The estimate is the run's, on the row that names it; a row the same
    // run answers says where its time is rather than inventing one (A7).
    let (estimate, estimate_size) = test.seconds.map_or_else(
        || {
            let held_by = match test.run {
                crate::Run::Ladder => "in the ladder",
                crate::Run::CrossCheck => "in the cross-check",
            };
            (held_by.to_owned(), size::SMALL)
        },
        |seconds| (clock(seconds), size::BODY),
    );
    paint.say_right(
        x + wide - 160.0,
        y,
        &estimate,
        Weight::Regular,
        estimate_size,
        ink.quiet,
    );
    // **A test that has never run has no run time, and says so** (A7). A zero
    // here would read as *instant*, which is the one thing it is not.
    let (ran, weight, colour) = test.ran.map_or_else(
        || (UNKNOWN.to_owned(), Weight::Regular, ink.faint),
        |seconds| (clock(seconds), Weight::Bold, ink.ink),
    );
    paint.say_right(x + wide - 76.0, y, &ran, weight, size::BODY, colour);
    // **The button only exists where there is something to read, and where
    // there is not, the column is empty.** It said `Unknown` at first, beside
    // the `Unknown` in the run-time column — two of them in a row, saying one
    // thing. The run time is the reading that is absent (A7); a results button
    // is furniture, and absent furniture is drawn by drawing nothing.
    if test.result.is_some() {
        let open = desk.showing == Some(at);
        let button = Box::new(x + wide - 66.0, y - 4.0, 66.0, 22.0);
        if ui::button(
            paint,
            mouse,
            button,
            if open { "Hide" } else { "View" },
            if open { Kind::Primary } else { Kind::Ordinary },
        ) {
            act = Some(Act::Result(at));
        }
    }
    if mouse.clicked(hit) {
        act = Some(Act::Toggle(at));
    }
    act
}

/// What the run of one test found, opened under the table rather than over it.
///
/// **Under, because the table is the thing being read.** A panel that covered
/// the rows would answer *what did this find* by hiding *which of them it was
/// about*.
fn found(paint: &mut Painter, desk: &Desk, area: Box) -> f32 {
    let ink = paint.ink;
    let Some(at) = desk.showing else {
        return area.y;
    };
    let Some(test) = desk.tests.get(at) else {
        return area.y;
    };
    let Some(lines) = test.result.as_ref() else {
        return area.y;
    };
    let mut y = area.y;
    paint.rule((area.x, y), (area.x + area.w, y), ink.line, 255);
    y += 12.0;
    paint.say_at(area.x, y, test.name, Weight::Bold, size::BODY, ink.ink);
    y += 22.0;
    for line in lines {
        // A sentence the daemon wrote — what a start-up figure includes, why
        // a cost could not be read — is longer than the panel is wide, and
        // an ellipsis in it would cut the caveat off the figure (A7).
        for shown in paint.wrap(line, Weight::Regular, size::BODY, area.w) {
            let shown = paint.elide(&shown, Weight::Regular, size::BODY, area.w);
            paint.say_at(area.x, y, &shown, Weight::Regular, size::BODY, ink.quiet);
            y += 20.0;
        }
    }
    y
}

/// What a cross-check has said so far: which half is running, then the
/// daemon's sentences.
fn cross_check_progress(paint: &mut Painter, job: &crate::job::Job, area: Box) {
    let ink = paint.ink;
    let mut y = area.y;
    paint.rule(
        (area.x, y - 12.0),
        (area.x + area.w, y - 12.0),
        ink.line,
        255,
    );
    let mut lines: Vec<(String, Weight, crate::paint::Rgb)> = Vec::new();
    // What it is and how long so far, first — the same line the window
    // carries at the top right, here where the run was started (A7).
    if !job.finished {
        lines.push((
            format!("{}, {} s so far", job.what, job.ran()),
            Weight::Bold,
            ink.ink,
        ));
    }
    for answer in &job.answers {
        if let (Some(low), Some(high)) = (
            answer
                .get("estimate_low_seconds")
                .and_then(Value::as_integer),
            answer
                .get("estimate_high_seconds")
                .and_then(Value::as_integer),
        ) {
            lines.push((
                format!(
                    "asking the provisioned engine for {} tokens, then reading them with MCF's \
                     own — {}",
                    answer
                        .get("positions")
                        .and_then(Value::as_integer)
                        .unwrap_or(0),
                    span(
                        u64::try_from(low).unwrap_or(0),
                        u64::try_from(high).unwrap_or(0)
                    )
                ),
                Weight::Regular,
                ink.quiet,
            ));
        } else if matches!(answer.get("reading"), Some(Value::Bool(true))) {
            lines.push((
                format!(
                    "{} produced {} tokens; MCF's own engine is reading them",
                    answer
                        .get("engine_ran")
                        .and_then(Value::as_text)
                        .unwrap_or("the provisioned engine"),
                    answer
                        .get("produced")
                        .and_then(Value::as_integer)
                        .unwrap_or(0)
                ),
                Weight::Regular,
                ink.quiet,
            ));
        } else if let Some(said) = answer.get("said").and_then(Value::as_list) {
            for sentence in said.iter().filter_map(Value::as_text) {
                lines.push((sentence.to_owned(), Weight::Bold, ink.ink));
            }
        }
    }
    if let Some(why) = &job.refused {
        lines.push((why.clone(), Weight::Regular, ink.bad));
    }
    for (line, weight, colour) in lines {
        for shown in paint.wrap(&line, weight, size::BODY, area.w).iter().take(3) {
            paint.say_at(area.x, y, shown, weight, size::BODY, colour);
            y += 20.0;
            if y > area.bottom() - 4.0 {
                return;
            }
        }
        y += 4.0;
    }
}

/// What a run has said so far, under the setup rather than instead of it.
fn readings(paint: &mut Painter, desk: &Desk, area: Box) {
    let ink = paint.ink;
    if let Doing::CrossChecking(job) = &desk.doing {
        return cross_check_progress(paint, job, area);
    }
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
    // The run under way, before its readings: what it is and how long so
    // far, then the daemon's estimate. A run still loading its first model
    // has no readings, and drawing nothing there drew a run that looked
    // stopped (A7). The console's arrangement (B-072).
    if let Some(said) = desk.under_way() {
        paint.say_at(area.x, y, &said, Weight::Bold, size::BODY, ink.ink);
        y += 22.0;
    }
    if let Some(estimate) = mcf_tui::screens::diagnostics::estimated_seconds(job) {
        paint.say_at(area.x, y, &estimate, Weight::Regular, size::BODY, ink.quiet);
        y += 22.0;
    }
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
    // Where it is now, after what it has found: the step the daemon last
    // announced, which changes every generation and is the sign the run is
    // alive between one reading and the next.
    if !job.finished
        && let Some(step) = mcf_tui::screens::diagnostics::step_of(job)
    {
        let shown = paint.elide(&step, Weight::Regular, size::BODY, area.w);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::BODY, ink.accent);
    }
    if let Some(conditions) = job.conclusion().and_then(|body| body.get("conditions")) {
        // B65 and D31: a timing from MCF's own stand-in measures the stand-in.
        // Which engine ran is a condition of every number above it.
        let ran = conditions
            .get("engine_ran")
            .and_then(Value::as_text)
            .unwrap_or("MCF did not say");
        let shown = paint.elide(
            &format!(
                "measured on {ran}{}",
                mcf_tui::screens::diagnostics::on_device(conditions)
            ),
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
                crate::job::fraction(job),
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
    // Before the files, because it is the thing somebody may need in order to
    // decide not to download at all — and after the download is too late
    // (B-023).
    if let Some(terms) = found.get("terms").and_then(Value::as_text) {
        // An unknown or unidentified licence is not a detail: it is the state
        // that needs a person's attention, and it is coloured accordingly.
        let colour = if terms.contains("unknown") || terms.contains("could not identify") {
            ink.warn
        } else {
            ink.quiet
        };
        for line in paint.wrap(terms, Weight::Regular, size::SMALL, wide) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, colour);
            y += 16.0;
        }
        y += 8.0;
    }
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
    // Where the shape came from, which is a condition of every verdict below.
    // A21: every verdict below rests on a number the repository supplied. The
    // arithmetic is MCF's; the shape it is over is not, and MCF has not
    // fetched the weights to check it.
    if let Some(from) = found.get("shape_from").and_then(Value::as_text) {
        for line in paint.wrap(
            &format!(
                "These rest on {from}. The arithmetic is MCF's; the shape is the \
                      repository's."
            ),
            Weight::Regular,
            size::SMALL,
            wide,
        ) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
            y += 16.0;
        }
        y += 8.0;
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

/// The turn a question is asked inside, and the picture shown with it: a
/// system turn, thinking, an effort, a file (B-462).
///
/// Every one of them is *what was said*, and empty is nothing said rather
/// than a default said out loud (D43, §3.15). Returns what was pressed and
/// where the row ends.
fn the_turn(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut act = None;
    let mut y = area.y;
    let label_at = area.x;
    let field_at = area.x + 74.0;
    let width = (area.w - 74.0).max(120.0);

    ui::label(paint, label_at, y + 9.0, "system");
    let system = Box::new(field_at, y, width, 28.0);
    if ui::field(
        paint,
        mouse,
        system,
        &desk.system,
        "none — the template's own",
        desk.caret == Caret::System,
    ) {
        act = Some(Act::Focus(Caret::System));
    }
    y += 36.0;

    ui::label(paint, label_at, y + 9.0, "picture");
    let picture = Box::new(field_at, y, width, 28.0);
    if ui::field(
        paint,
        mouse,
        picture,
        &desk.picture,
        "none — a path to a file the model can be shown",
        desk.caret == Caret::Picture,
    ) {
        act = Some(Act::Focus(Caret::Picture));
    }
    y += 36.0;

    ui::label(paint, label_at, y + 9.0, "thinking");
    let said = match desk.thinking {
        None => "unsaid",
        Some(true) => "on",
        Some(false) => "off",
    };
    // Outlined rather than text alone: a control a person cannot see is a
    // control they do not have (§3.15).
    let (pressed, switch) = ui::fitted(paint, mouse, (field_at, y - 2.0), said, Kind::Ordinary);
    if pressed {
        act = Some(Act::CycleThinking);
    }
    let effort_at = switch.right() + 16.0;
    ui::label(paint, effort_at, y + 9.0, "effort");
    let effort = Box::new(
        effort_at + 52.0,
        y,
        (area.right() - effort_at - 52.0).max(90.0),
        28.0,
    );
    if ui::field(
        paint,
        mouse,
        effort,
        &desk.effort,
        "none — its own word",
        desk.caret == Caret::Effort,
    ) {
        act = Some(Act::Focus(Caret::Effort));
    }
    y += 34.0;
    // A switch a template does not read is refused by the daemon in its own
    // words, and that refusal is what the panel below shows (A2, A4).
    paint.say_at(
        area.x,
        y,
        "unsaid is not off: what is left alone is the template's own",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    (act, y + 18.0)
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
            "Choose a model on the Models screen first.",
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
    if ui::field(
        paint,
        mouse,
        field,
        &desk.typed,
        "Ask it something",
        desk.caret == Caret::Document,
    ) {
        act = Some(Act::Focus(Caret::Document));
    }
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
    y += 42.0;
    // What the question is asked inside, and what goes with it: the same
    // switches `mcf run` takes, so what a person can ask at the prompt they
    // can ask here (A22, B-462).
    let (turn_act, after) = the_turn(paint, desk, mouse, Box::new(area.x, y, field.w, 0.0));
    act = act.or(turn_act);
    y = after + 10.0;

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
    what_it_said(paint, desk, Box::new(area.x, y, area.w, area.bottom() - y));
    act
}

/// What the model said, under the conditions it said it.
///
/// The conditions go above the words rather than under them, because a
/// reader meeting the words first has read them under conditions they were
/// not told (§3.15, A6, B-452).
fn what_it_said(paint: &mut Painter, desk: &Desk, area: Box) {
    let ink = paint.ink;
    let mut y = area.y;
    for said in what_it_ran_under(desk) {
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, area.w.min(640.0));
        paint.say_at(area.x, y, &shown, Weight::Regular, size::SMALL, ink.quiet);
        y += 16.0;
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
}

/// The conditions of the answer on the screen, from the account the daemon
/// ended the stream with: how the turn was addressed, and the picture where
/// one was shown (B-462, B-452).
fn what_it_ran_under(desk: &Desk) -> Vec<String> {
    let Doing::Answering(job) = &desk.doing else {
        return Vec::new();
    };
    // The line the daemon ends a generation with carries the account; the
    // ones before it carry the tokens.
    let Some(account) = job
        .answers
        .iter()
        .rev()
        .find_map(|body| match body.get("done") {
            Some(held @ Value::Map(_)) => Some(held),
            _ => None,
        })
    else {
        return Vec::new();
    };
    let under = |key: &str| {
        account
            .get("conditions")
            .and_then(|conditions| conditions.get(key))
    };
    let mut said = Vec::new();
    if let Some(addressed) = under("addressed_as").and_then(Value::as_text) {
        said.push(format!("addressed {addressed}"));
    }
    if let Some(shown) = under("shown").filter(|shown| !matches!(shown, Value::Null)) {
        let path = shown
            .get("picture")
            .and_then(|picture| picture.get("path"))
            .and_then(Value::as_text)
            .unwrap_or("a picture");
        let name = path.rsplit('/').next().unwrap_or(path);
        let bytes = shown
            .get("picture")
            .and_then(|picture| picture.get("bytes"))
            .and_then(Value::as_integer)
            .unwrap_or(0);
        let placed = shown
            .get("placed")
            .and_then(Value::as_text)
            .unwrap_or("in the turn");
        said.push(format!("shown {name} ({bytes} bytes), placed {placed}"));
    }
    said
}

/// How MCF is set up, which is nothing yet — and the console says so in these
/// words, so this does too.
/// What a prompt does to the chosen model.
///
/// **A client of one request, like the command line** (A22). The measuring is
/// the daemon's — one generation per sentence and one per seed — and what is
/// here is a field, a button and the reading. A screen that computed its own
/// answer would be a second answer to a question already served.
/// The finished report, or what stands in its place: the run in progress, or
/// the refusal.
fn a_report_or_why_not<'a>(
    paint: &mut Painter,
    desk: &'a Desk,
    area: Box,
    mut at: f32,
) -> Option<&'a Value> {
    let ink = paint.ink;
    if let Doing::Reporting(job) = &desk.doing
        && !job.finished
    {
        paint.say_at(
            area.x,
            at,
            &job.what,
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        paint.say_at(
            area.x,
            at + 20.0,
            "one generation for the prompt, one for each part left out, one for the control \
             sentence, one for each further seed",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return None;
    }
    let job = desk.doing.job()?;
    if let Some(why) = &job.refused {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w.min(600.0))
            .iter()
            .take(3)
        {
            paint.say_at(area.x, at, line, Weight::Regular, size::BODY, ink.bad);
            at += 20.0;
        }
        return None;
    }
    job.conclusion().or_else(|| job.latest())
}

fn prompt(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "prompt analysis", ink.faint);
    let named = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .map_or("no model chosen", |held| held.name.as_str());
    paint.say_at(
        area.x,
        area.y + 28.0,
        "model",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    paint.say_at(
        area.x + READING_CHOICE,
        area.y + 28.0,
        named,
        Weight::Bold,
        size::BODY,
        ink.ink,
    );

    let (mut act, under) = the_document(paint, desk, mouse, area);
    let (asked, button) = ui::fitted(paint, mouse, (area.x, under), "Analyse", Kind::Primary);
    if asked && !desk.doing.busy() && desk.chosen.is_some() {
        act = Some(Act::ReportPrompt);
    }
    let (cleared, cleared_button) = ui::fitted(
        paint,
        mouse,
        (button.right() + 8.0, under),
        "Clear",
        Kind::Ordinary,
    );
    if cleared && !desk.typed.is_empty() {
        act = Some(Act::Clear);
    }
    let (chosen, bottom) = the_readings(
        paint,
        desk,
        mouse,
        (area.x, cleared_button.bottom() + 12.0),
        // As wide as the document above it: four unit buttons and a
        // condition beside them need the room (B-443).
        area.w.min(960.0),
    );
    if chosen.is_some() {
        act = chosen;
    }

    // **The report scrolls under the controls.** It grew past one window
    // as the readings did, and a report that does not fit is read by the
    // wheel: the body is drawn from `scroll` points up and confined to the
    // region under the controls, and the mouse sees only that region.
    let top = bottom + 12.0;
    let body = Box::new(0.0, top, area.right() + PAD, (area.bottom() - top).max(0.0));
    let mouse = &mouse.within(body);
    paint.clip(body);
    let scrolled = prompt_report(paint, desk, mouse, area, top - desk.scroll);
    paint.unclip();
    scrolled.or(act)
}

/// A report as figures and short labels (§3.4, §3.15): the conditions of
/// the run, a table a reading, and the answer under them. Every row with an
/// answer behind it is the control that shows it (A19).
fn prompt_report(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    at: f32,
) -> Option<Act> {
    let found = a_report_or_why_not(paint, desk, area, at)?;
    let wide = area.w.min(820.0);
    let mut y = report_conditions(paint, (area.x, at + 10.0), wide, found);
    y = expected_table(paint, (area.x, y), wide, found);
    let (after, mut act) = removed_table(paint, desk, mouse, Box::new(area.x, y, wide, 0.0), found);
    y = floors_table(paint, (area.x, after), wide, found);
    let (after, pressed) = alone_table(paint, desk, mouse, Box::new(area.x, y, wide, 0.0), found);
    act = pressed.or(act);
    let (after, pressed) = prefixes_table(
        paint,
        desk,
        mouse,
        Box::new(area.x, after, wide, 0.0),
        found,
    );
    act = pressed.or(act);
    let (after, pressed) = swaps_table(
        paint,
        desk,
        mouse,
        Box::new(area.x, after, wide, 0.0),
        found,
    );
    act = pressed.or(act);
    let (after, pressed) = forms_table(
        paint,
        desk,
        mouse,
        Box::new(area.x, after, wide, 0.0),
        found,
    );
    act = pressed.or(act);
    y = seeds_line(paint, (area.x, after), wide, found);
    y = more_line(paint, (area.x, y), wide, found);
    // The page scrolls, so the answer has a page of its own below the rest.
    the_answer(
        paint,
        desk,
        Box::new(area.x, y + 10.0, area.w, 640.0),
        found,
    );
    act
}

/// A reading's label and the conditions it was read under, on one line
/// (§3.4). Returns the line under it.
fn section(
    paint: &mut Painter,
    at: (f32, f32),
    width: f32,
    label: &str,
    conditions: &[String],
) -> f32 {
    let ink = paint.ink;
    spaced(paint, at.0, at.1, label, ink.faint);
    let said = conditions.join(" · ");
    let shown = paint.elide(&said, Weight::Regular, size::SMALL, width - 96.0);
    paint.say_at(
        at.0 + 96.0,
        at.1 - 2.0,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    at.1 + 22.0
}

/// A line of figures under a table: a label and a value.
fn foot(paint: &mut Painter, at: (f32, f32), label: &str, value: &str, colour: Rgb) -> f32 {
    let ink = paint.ink;
    paint.say_at(at.0, at.1, label, Weight::Regular, size::SMALL, ink.quiet);
    paint.say_at(at.0 + 150.0, at.1, value, Weight::Bold, size::SMALL, colour);
    at.1 + 18.0
}

fn integer(held: &Value, key: &str) -> i64 {
    held.get(key).and_then(Value::as_integer).unwrap_or(0)
}

fn moved_of(read: &Value) -> i64 {
    integer(read, "moved_parts_per_million")
}

fn clauses_of(found: &Value) -> &[Value] {
    found.get("clauses").and_then(Value::as_list).unwrap_or(&[])
}

fn part_text(found: &Value, at: usize) -> String {
    clauses_of(found)
        .get(at)
        .and_then(|clause| clause.get("text"))
        .and_then(Value::as_text)
        .map(|said| said.trim().lines().next().unwrap_or_default().to_owned())
        .unwrap_or_default()
}

/// A difference of two shares with its sign, to a tenth: `+2.0`, `-1.3`.
fn signed_percent(difference: i64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a difference shown to one decimal place"
    )]
    let held = difference as f64 / 10_000.0;
    format!("{held:+.1}")
}

/// The conditions of the run, a figure a line (§3.4, §3.15): what the text
/// was cut into and who decided, how it reached the model, what every
/// generation was allowed to be, what the generations were spent on, the
/// floor and how it was drawn, whether the floor leaves the rows readable,
/// and where the figures survive this window.
fn report_conditions(paint: &mut Painter, at: (f32, f32), width: f32, found: &Value) -> f32 {
    let ink = paint.ink;
    let unit = unit_of(found);
    let text = |key: &str| found.get(key).and_then(Value::as_text);
    let mut rows: Vec<(&str, String, Rgb)> = vec![(
        "unit",
        match text("unit_chosen_by") {
            Some(by) => format!("{unit} · {by}"),
            None => unit.to_owned(),
        },
        ink.ink,
    )];
    if let Some(addressed) = text("addressed_as") {
        rows.push(("addressed", addressed.to_owned(), ink.ink));
    }
    // **Who read the prompt** (B-441): the engine that answered it, named,
    // or why there is no count at all (A7).
    let read_by = text("read_by").unwrap_or("not recorded");
    let tokens = integer(found, "prompt_tokens");
    if tokens > 0 {
        rows.push((
            "prompt",
            format!("{tokens} tokens · read by {read_by}"),
            ink.ink,
        ));
    } else if let Some(refused) = text("prompt_tokens_refused") {
        rows.push((
            "prompt",
            format!("not counted · {refused} · read by {read_by}"),
            ink.ink,
        ));
    }
    let limit = integer(found, "token_limit");
    if limit > 0 {
        rows.push(("cap", format!("{limit} tokens a generation"), ink.ink));
    }
    if integer(found, "generations") > 0 {
        rows.push(("generations", what_the_generations_were(found), ink.ink));
    }
    rows.push(("floor", the_floor_line(found, unit), ink.ink));
    let floor = integer(found, "floor_parts_per_million");
    // **Where the floor swamps the column, that is the finding** (§3.15,
    // A7, F147): said before the rows, in a line of its own.
    rows.push(if floor < 500_000 {
        ("separable", "yes".to_owned(), ink.ink)
    } else {
        (
            "separable",
            format!(
                "NO · floor {} >= 50.0% · rows below say this run did not work, not an ordering",
                as_percent(floor)
            ),
            ink.bad,
        )
    });
    // **Where this survives the window** (A1, A2, B-432): the figures under
    // their conditions, and none of the text.
    rows.push(match text("recorded") {
        Some(id) => (
            "record",
            format!("{id} · figures and conditions, no text"),
            ink.ink,
        ),
        None => (
            "record",
            "NOT RECORDED · the daemon could not write it · this window is the only copy"
                .to_owned(),
            ink.warn,
        ),
    });
    let mut y = at.1;
    for (label, value, colour) in rows {
        paint.say_at(at.0, y, label, Weight::Regular, size::SMALL, ink.quiet);
        let shown = paint.elide(&value, Weight::Regular, size::SMALL, width - 96.0);
        paint.say_at(at.0 + 96.0, y, &shown, Weight::Regular, size::SMALL, colour);
        y += 18.0;
    }
    y + 8.0
}

/// What the generations were spent on — each thing that cost some, and
/// only the things this run asked for (A19, §3.4).
fn what_the_generations_were(found: &Value) -> String {
    let list = |key: &str| found.get(key).and_then(Value::as_list).map(<[Value]>::len);
    let mut spent = vec![
        integer(found, "generations").to_string(),
        "as written 1".to_owned(),
        format!("removed {}", clauses_of(found).len()),
        match list("floors") {
            Some(positions) => format!("control {positions}"),
            None => "control 1".to_owned(),
        },
    ];
    if let Some(alone) = list("alone") {
        spent.push(format!("alone {}", alone.saturating_add(1)));
    }
    if let Some(prefixes) = list("prefixes") {
        spent.push(format!("prefixes {prefixes}"));
    }
    if let Some(swaps) = list("swaps") {
        spent.push(format!("swaps {swaps}"));
    }
    if let Some(forms) = found.get("forms").and_then(Value::as_list) {
        spent.push(format!("forms {}", rendered_forms(forms).len()));
    }
    if let Some(settled) = found
        .get("settled")
        .filter(|held| matches!(held, Value::Map(_)))
    {
        spent.push(format!("seeds {}", integer(settled, "seeds_asked")));
    }
    spent.join(" · ")
}

/// The floor's line: the figure, and how it was drawn — one draw before the
/// last part, or at every position with its spread (B-434, §3.4).
fn the_floor_line(found: &Value, unit: &str) -> String {
    let floor = as_percent(integer(found, "floor_parts_per_million"));
    let depth = integer(found, "forced_depth");
    let held = match found.get("floor_held") {
        Some(held) if !matches!(held, Value::Null) => format!(
            " · control in: 1st {} · open {}",
            crate::held_mark(Some(held), depth),
            crate::open_mark(Some(held))
        ),
        // Not taken, and why: a dash alone sent a reader to look for an
        // engine that was running the whole time (A2, F160).
        _ => format!(
            " · control in: not taken · {}",
            found
                .get("held_refused")
                .and_then(Value::as_text)
                .unwrap_or("needs the served engine")
        ),
    };
    match found
        .get("floor_spread")
        .filter(|spread| !matches!(spread, Value::Null))
    {
        Some(spread) => format!(
            "{floor} · drawn at {} · {}–{} · middle {}{held}",
            count_of(
                found
                    .get("floors")
                    .and_then(Value::as_list)
                    .map_or(0, <[Value]>::len),
                "position"
            ),
            as_percent(integer(spread, "least_parts_per_million")),
            as_percent(integer(spread, "most_parts_per_million")),
            as_percent(integer(spread, "middle_parts_per_million")),
        ),
        None => format!(
            "{floor} · 1 draw · control before the last {unit} · \"floor at every position\" \
             above draws it at each{held}"
        ),
    }
}

/// One row of a reading's table: its label, how much the answer moved and
/// whether that is above the floor, the cells under the columns after the
/// bar, the part's text, and what pressing it shows, if anything.
struct ReadingRow {
    first: String,
    moved: i64,
    loud: bool,
    cells: Vec<String>,
    text: String,
    act: Option<Act>,
    chosen: bool,
}

/// The columns every reading's table starts with: the figure, then the bar.
const MOVED_AT: f32 = 76.0;
const BAR_WIDTH: f32 = 160.0;

/// A table of readings: heads, a rule, and a row a reading with its bar
/// coloured against the floor. Rows with an answer behind them are pressed
/// to show it. Returns the line under the table and what was pressed.
fn reading_table(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    first: &str,
    columns: &[Column],
    rows: &[ReadingRow],
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut y = heads(paint, area, first, columns);
    let mut act = None;
    // The figure sits at the first column's edge, the bar after it.
    let moved_at = columns.first().map_or(MOVED_AT, |column| column.at);
    for row in rows {
        let hit = Box::new(area.x - 6.0, y - 3.0, area.w + 12.0, 22.0);
        if row.act.is_some() {
            if row.chosen || mouse.over(hit) {
                paint.panel(hit, 6.0, ink.line, if row.chosen { 140 } else { 80 });
            }
            if mouse.clicked(hit) {
                act.clone_from(&row.act);
            }
        }
        let colour = if row.loud { ink.ink } else { ink.quiet };
        paint.say_at(
            area.x,
            y,
            &row.first,
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        paint.say_right(
            area.x + moved_at,
            y,
            &as_percent(row.moved),
            Weight::Bold,
            size::BODY,
            colour,
        );
        let bar = Box::new(area.x + moved_at + 12.0, y + 4.0, BAR_WIDTH, 10.0);
        paint.panel(bar, 5.0, ink.sunk, 255);
        #[allow(
            clippy::cast_precision_loss,
            reason = "a bar's width in points; a part of a point is not drawn"
        )]
        let filled = (row.moved as f32 / 1_000_000.0).clamp(0.0, 1.0) * bar.w;
        paint.panel(
            Box::new(bar.x, bar.y, filled.max(1.0), bar.h),
            5.0,
            if row.loud { ink.accent } else { ink.line },
            255,
        );
        let mut text_at = bar.right() + 14.0;
        for (column, value) in columns.iter().skip(2).zip(&row.cells) {
            // An absent figure is drawn quietly (A7).
            let cell = if value == "—" { ink.faint } else { colour };
            paint.say_right(area.x + column.at, y, value, Weight::Bold, size::BODY, cell);
            text_at = area.x + column.at + 14.0;
        }
        let shown = paint.elide(
            &row.text,
            Weight::Regular,
            size::BODY,
            area.right() - text_at,
        );
        paint.say_at(text_at, y, &shown, Weight::Regular, size::BODY, colour);
        y += 22.0;
    }
    (y, act)
}

/// A row's cells after the bar: where the answer's first token ranked and
/// how much of its opening stayed.
fn held_cells(read: &Value, depth: i64) -> Vec<String> {
    vec![
        crate::held_mark(read.get("held"), depth),
        crate::open_mark(read.get("held")),
    ]
}

/// The parts removed in turn (§3.8, B-429): a row a part with what the
/// answer did without it, the control's row under them, and the parts at
/// or under the floor. Each row shows the answer without its part.
fn removed_table(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let unit = unit_of(found);
    let floor = integer(found, "floor_parts_per_million");
    let clauses = clauses_of(found);
    let grouped = found
        .get("expected_by_part")
        .filter(|held| !matches!(held, Value::Null));
    let mut y = section(
        paint,
        (area.x, area.y),
        area.w,
        "impact",
        &[
            format!("answer moved without each {unit}"),
            "moved = words changed, punctuation aside".to_owned(),
            "seed held".to_owned(),
            "ordering, not relevance".to_owned(),
            "own = pieces the model would have written".to_owned(),
            "row → answer".to_owned(),
        ],
    );
    if clauses.is_empty() {
        y = foot(
            paint,
            (area.x, y),
            "parts",
            &format!("one {unit} · nothing to remove"),
            ink.quiet,
        );
        return (y + 8.0, None);
    }
    let rows = removed_rows(desk, found, grouped.is_some());
    let mut columns = vec![
        Column {
            head: "moved",
            at: MOVED_AT,
            right: true,
        },
        Column {
            head: "",
            at: MOVED_AT + 12.0 + BAR_WIDTH,
            right: true,
        },
        Column {
            head: "vs floor",
            at: 318.0,
            right: true,
        },
        Column {
            head: "1st",
            at: 360.0,
            right: true,
        },
        Column {
            head: "open",
            at: 414.0,
            right: true,
        },
    ];
    if grouped.is_some() {
        columns.push(Column {
            head: "own",
            at: 468.0,
            right: true,
        });
    }
    let (after, act) = reading_table(
        paint,
        mouse,
        Box::new(area.x, y, area.w, 0.0),
        "#",
        &columns,
        &rows,
    );
    y = removed_foot(paint, (area.x, after + 4.0), found, floor);
    (y + 8.0, act)
}

/// The removed table's rows: a part a row, and the control's row last.
fn removed_rows(desk: &Desk, found: &Value, own: bool) -> Vec<ReadingRow> {
    let depth = integer(found, "forced_depth");
    let floor = integer(found, "floor_parts_per_million");
    let grouped = found
        .get("expected_by_part")
        .filter(|held| !matches!(held, Value::Null));
    let mut rows: Vec<ReadingRow> = clauses_of(found)
        .iter()
        .enumerate()
        .map(|(at, clause)| {
            let moved = moved_of(clause);
            let mut cells = vec![signed_percent(moved.saturating_sub(floor))];
            cells.extend(held_cells(clause, depth));
            if own {
                cells.push(crate::expected_mark(grouped, at).unwrap_or_else(|| "—".to_owned()));
            }
            ReadingRow {
                first: at.saturating_add(1).to_string(),
                moved,
                loud: moved > floor,
                cells,
                text: part_text(found, at),
                act: Some(Act::ShowWithout(at)),
                chosen: desk.shown == Some(crate::Shown::Without(at)),
            }
        })
        .collect();
    let mut control = vec!["floor".to_owned()];
    control.push(crate::held_mark(found.get("floor_held"), depth));
    control.push(crate::open_mark(found.get("floor_held")));
    if own {
        control.push("—".to_owned());
    }
    rows.push(ReadingRow {
        first: "ctl".to_owned(),
        moved: floor,
        loud: false,
        cells: control,
        text: "control sentence".to_owned(),
        act: None,
        chosen: false,
    });
    rows
}

/// Under the removed table: which parts sit at or under the floor, the
/// part the model least expected, and how many parts were not removed.
fn removed_foot(paint: &mut Painter, at: (f32, f32), found: &Value, floor: i64) -> f32 {
    let ink = paint.ink;
    let clauses = clauses_of(found);
    let quiet: Vec<String> = clauses
        .iter()
        .enumerate()
        .filter(|(_, clause)| moved_of(clause) <= floor)
        .map(|(at, _)| format!("#{}", at.saturating_add(1)))
        .collect();
    let mut y = at.1;
    // **Every removal giving the same answer is a finding, and it reads
    // like a broken tool** (A7).
    if floor == 0 && quiet.len() == clauses.len() {
        y = foot(
            paint,
            (at.0, y),
            "same answer",
            "every removal and the control · the prompt did not steer this model",
            ink.warn,
        );
    }
    y = foot(
        paint,
        (at.0, y),
        "at/under floor",
        &if quiet.is_empty() {
            "none".to_owned()
        } else {
            format!("{} · {}", quiet.len(), quiet.join(" "))
        },
        ink.ink,
    );
    if let Some(least) = least_expected(found) {
        y = foot(paint, (at.0, y), "least expected", &least, ink.ink);
    }
    // Sentences past the cap are not measured, and a list that quietly
    // shortened itself is the one thing a list must not do (A1, A4).
    let over = integer(found, "clauses_over_the_cap");
    if over > 0 {
        y = foot(
            paint,
            (at.0, y),
            "not removed",
            &format!("{over} · \"more\" or \"all\" above takes the rest"),
            ink.warn,
        );
    }
    y
}

/// The part with the smallest share of first choices (B-433), by the rank
/// reading grouped by part: shares compared crosswise so no division is
/// done, and a tie names nobody (A19). `None` where no reading was taken.
fn least_expected(found: &Value) -> Option<String> {
    let parts = found
        .get("expected_by_part")
        .filter(|held| !matches!(held, Value::Null))?
        .get("parts")
        .and_then(Value::as_list)?;
    let mut least: Option<(usize, i64, i64)> = None;
    let mut tied = false;
    for (at, part) in parts.iter().enumerate() {
        let (tokens, first) = (integer(part, "tokens"), integer(part, "first_choice"));
        if tokens == 0 {
            continue;
        }
        match least {
            Some((_, held_first, held_tokens)) => {
                let mine = first.saturating_mul(held_tokens);
                let theirs = held_first.saturating_mul(tokens);
                if mine < theirs {
                    least = Some((at, first, tokens));
                    tied = false;
                } else if mine == theirs {
                    tied = true;
                }
            }
            None => least = Some((at, first, tokens)),
        }
    }
    if parts.len() < 2 {
        return None;
    }
    Some(match least {
        Some((at, _, _)) if !tied => format!("#{}", at.saturating_add(1)),
        _ => "tied".to_owned(),
    })
}

/// The floor at every position, where it was drawn (B-434): a row a
/// position, and how many parts sit under the widest of them. Nothing
/// where one draw was taken; the floor's line above says so (A7).
fn floors_table(paint: &mut Painter, at: (f32, f32), width: f32, found: &Value) -> f32 {
    let ink = paint.ink;
    let Some(floors) = found.get("floors").and_then(Value::as_list) else {
        return at.1;
    };
    let depth = integer(found, "forced_depth");
    let unit = unit_of(found);
    let y = section(
        paint,
        at,
        width,
        "floors",
        &[
            "control at every position".to_owned(),
            format!("a {unit} matched anywhere → not steering"),
        ],
    );
    let rows: Vec<ReadingRow> = floors
        .iter()
        .map(|read| {
            let position = integer(read, "position");
            let first = if position.saturating_add(1) == i64::try_from(floors.len()).unwrap_or(0) {
                format!("after #{position}")
            } else {
                format!("before #{}", position.saturating_add(1))
            };
            ReadingRow {
                first,
                moved: moved_of(read),
                loud: true,
                cells: held_cells(read, depth),
                text: String::new(),
                act: None,
                chosen: false,
            }
        })
        .collect();
    let columns = [
        Column {
            head: "moved",
            at: MOVED_AT + 44.0,
            right: true,
        },
        Column {
            head: "",
            at: MOVED_AT + 56.0 + BAR_WIDTH,
            right: true,
        },
        Column {
            head: "1st",
            at: 320.0,
            right: true,
        },
        Column {
            head: "open",
            at: 374.0,
            right: true,
        },
    ];
    let (after, _) = reading_table(
        paint,
        &Mouse::default(),
        Box::new(at.0, y, width, 0.0),
        "control",
        &columns,
        &rows,
    );
    let most = found
        .get("floor_spread")
        .map_or(0, |spread| integer(spread, "most_parts_per_million"));
    let under = clauses_of(found)
        .iter()
        .filter(|clause| moved_of(clause) <= most)
        .count();
    foot(
        paint,
        (at.0, after + 4.0),
        "at/under widest",
        &under.to_string(),
        ink.ink,
    ) + 8.0
}

/// A reading that was not asked for, as a phrase for the `more` line: the
/// button above that asks it, and what it costs (§3.15).
fn not_asked(extra: mcf_serve::prompt::Extra, found: &Value) -> String {
    let removed = clauses_of(found).len();
    let parts = removed
        .saturating_add(usize::try_from(integer(found, "clauses_over_the_cap")).unwrap_or(0));
    format!(
        "{} {}{}",
        extra.name(),
        if extra.at_most() { "up to " } else { "" },
        count_of(extra.generations(parts, removed), "generation")
    )
}

/// The readings not asked for, on one line rather than four empty sections
/// (B-443): each with the button above that asks it and its cost. Nothing
/// where every one was asked.
fn more_line(paint: &mut Painter, at: (f32, f32), width: f32, found: &Value) -> f32 {
    let mut conditions = vec!["not asked".to_owned()];
    if found.get("alone").and_then(Value::as_list).is_none() {
        conditions.push(not_asked(mcf_serve::prompt::Extra::Alone, found));
    }
    if found.get("prefixes").and_then(Value::as_list).is_none() {
        conditions.push(not_asked(mcf_serve::prompt::Extra::Prefixes, found));
    }
    if found.get("swaps").and_then(Value::as_list).is_none() {
        conditions.push(not_asked(mcf_serve::prompt::Extra::Swaps, found));
    }
    if found.get("forms").and_then(Value::as_list).is_none() {
        conditions.push(not_asked(mcf_serve::prompt::Extra::Forms, found));
    }
    if !matches!(found.get("settled"), Some(Value::Map(_))) {
        conditions.push("temperature 3 generations · at 0 the seed changes nothing".to_owned());
    }
    if conditions.len() == 1 {
        return at.1;
    }
    conditions.push("buttons above".to_owned());
    section(paint, at, width, "more", &conditions) + 4.0
}

/// Each part asked as the whole prompt in turn (B-435), read against the
/// control alone; each row shows the answer to its part alone.
fn alone_table(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let unit = unit_of(found);
    let Some(alone) = found.get("alone").and_then(Value::as_list) else {
        return (area.y, None);
    };
    let y = section(
        paint,
        (area.x, area.y),
        area.w,
        "alone",
        &[
            format!("each {unit} as the whole prompt"),
            "vs as written".to_owned(),
            "low = carries it alone".to_owned(),
            "row → answer".to_owned(),
        ],
    );
    let control_moved = found
        .get("alone_floor")
        .filter(|held| matches!(held, Value::Map(_)))
        .map(moved_of);
    let rows = alone_rows(desk, found, alone);
    let columns = [
        Column {
            head: "moved",
            at: MOVED_AT,
            right: true,
        },
        Column {
            head: "",
            at: MOVED_AT + 12.0 + BAR_WIDTH,
            right: true,
        },
        Column {
            head: "1st",
            at: 300.0,
            right: true,
        },
        Column {
            head: "open",
            at: 354.0,
            right: true,
        },
    ];
    let (after, act) = reading_table(
        paint,
        mouse,
        Box::new(area.x, y, area.w, 0.0),
        "#",
        &columns,
        &rows,
    );
    let y = match control_moved {
        Some(control) => foot(
            paint,
            (area.x, after + 4.0),
            "as far as control or further",
            &alone
                .iter()
                .filter(|read| moved_of(read) >= control)
                .count()
                .to_string(),
            ink.ink,
        ),
        None => foot(
            paint,
            (area.x, after + 4.0),
            "control alone",
            "not read · nothing to read these against",
            ink.warn,
        ),
    };
    (y + 8.0, act)
}

/// The alone table's rows: a part a row, and the control alone last where
/// it was read.
fn alone_rows(desk: &Desk, found: &Value, alone: &[Value]) -> Vec<ReadingRow> {
    let depth = integer(found, "forced_depth");
    let control = found
        .get("alone_floor")
        .filter(|held| matches!(held, Value::Map(_)));
    let control_moved = control.map(moved_of);
    let mut rows: Vec<ReadingRow> = alone
        .iter()
        .enumerate()
        .map(|(at, read)| ReadingRow {
            first: at.saturating_add(1).to_string(),
            moved: moved_of(read),
            // Under the control alone is a part that carries some of the
            // answer by itself.
            loud: control_moved.is_some_and(|control| moved_of(read) < control),
            cells: held_cells(read, depth),
            text: part_text(found, at),
            act: Some(Act::ShowAlone(at)),
            chosen: desk.shown == Some(crate::Shown::Alone(at)),
        })
        .collect();
    if let Some(control) = control {
        rows.push(ReadingRow {
            first: "ctl".to_owned(),
            moved: moved_of(control),
            loud: false,
            cells: held_cells(control, depth),
            text: "control sentence alone".to_owned(),
            act: None,
            chosen: false,
        });
    }
    rows
}

/// The prompt grown from the front (B-436): a row a prefix, and the first
/// within the floor of the answer as written. Each row shows its answer.
fn prefixes_table(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let unit = unit_of(found);
    let depth = integer(found, "forced_depth");
    let floor = integer(found, "floor_parts_per_million");
    let Some(prefixes) = found.get("prefixes").and_then(Value::as_list) else {
        return (area.y, None);
    };
    let y = section(
        paint,
        (area.x, area.y),
        area.w,
        "prefixes",
        &[
            format!("grown a {unit} at a time"),
            "vs as written".to_owned(),
            "low = already had it".to_owned(),
            "row → answer".to_owned(),
        ],
    );
    let mut arrived = None;
    let rows: Vec<ReadingRow> = prefixes
        .iter()
        .enumerate()
        .map(|(at, read)| {
            let kept = at.saturating_add(1);
            if arrived.is_none() && moved_of(read) <= floor {
                arrived = Some(kept);
            }
            ReadingRow {
                first: format!("1–{kept}"),
                moved: moved_of(read),
                loud: moved_of(read) > floor,
                cells: held_cells(read, depth),
                text: part_text(found, at),
                act: Some(Act::ShowPrefix(at)),
                chosen: desk.shown == Some(crate::Shown::Prefix(at)),
            }
        })
        .collect();
    let columns = [
        Column {
            head: "moved",
            at: MOVED_AT,
            right: true,
        },
        Column {
            head: "",
            at: MOVED_AT + 12.0 + BAR_WIDTH,
            right: true,
        },
        Column {
            head: "1st",
            at: 300.0,
            right: true,
        },
        Column {
            head: "open",
            at: 354.0,
            right: true,
        },
    ];
    let (after, act) = reading_table(
        paint,
        mouse,
        Box::new(area.x, y, area.w, 0.0),
        "parts",
        &columns,
        &rows,
    );
    let y = foot(
        paint,
        (area.x, after + 4.0),
        &format!("within floor {}", as_percent(floor)),
        &match arrived {
            Some(kept) => format!("first at 1–{kept} · not a claim the rest is idle"),
            None => format!("none short of the whole · the last {unit} still moved the answer"),
        },
        ink.ink,
    );
    (y + 8.0, act)
}

/// Neighbouring parts swapped (B-437): a row a pair, and how many of them
/// moved the answer past the floor. Each row shows its answer.
fn swaps_table(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let unit = unit_of(found);
    let depth = integer(found, "forced_depth");
    let floor = integer(found, "floor_parts_per_million");
    let Some(swaps) = found.get("swaps").and_then(Value::as_list) else {
        return (area.y, None);
    };
    let y = section(
        paint,
        (area.x, area.y),
        area.w,
        "swaps",
        &[
            format!("each {unit} and the next in each other's places"),
            "vs as written".to_owned(),
            "high = the order carries it".to_owned(),
            "row → answer".to_owned(),
        ],
    );
    let rows: Vec<ReadingRow> = swaps
        .iter()
        .enumerate()
        .map(|(at, read)| ReadingRow {
            first: crate::pair_mark(at),
            moved: moved_of(read),
            loud: moved_of(read) > floor,
            cells: held_cells(read, depth),
            text: part_text(found, at),
            act: Some(Act::ShowSwap(at)),
            chosen: desk.shown == Some(crate::Shown::Swap(at)),
        })
        .collect();
    let columns = [
        Column {
            head: "moved",
            at: MOVED_AT,
            right: true,
        },
        Column {
            head: "",
            at: MOVED_AT + 12.0 + BAR_WIDTH,
            right: true,
        },
        Column {
            head: "1st",
            at: 300.0,
            right: true,
        },
        Column {
            head: "open",
            at: 354.0,
            right: true,
        },
    ];
    let (after, act) = reading_table(
        paint,
        mouse,
        Box::new(area.x, y, area.w, 0.0),
        "pair",
        &columns,
        &rows,
    );
    let y = foot(
        paint,
        (area.x, after + 4.0),
        &format!("past floor {}", as_percent(floor)),
        &format!(
            "{} of {} · order read, not words",
            swaps.iter().filter(|read| moved_of(read) > floor).count(),
            swaps.len()
        ),
        ink.ink,
    );
    (y + 8.0, act)
}

/// The forms that were read: the served rows without a *not rendered*.
fn rendered_forms(forms: &[Value]) -> Vec<&Value> {
    forms
        .iter()
        .filter(|formed| formed.get("not_rendered").is_none())
        .collect()
}

/// The same parts in each form (B-444): a row a form read, and how many
/// of them moved the answer past the floor. A form not rendered is a line
/// under the table saying why (A7). Each row shows its answer.
fn forms_table(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let depth = integer(found, "forced_depth");
    let floor = integer(found, "floor_parts_per_million");
    let Some(forms) = found.get("forms").and_then(Value::as_list) else {
        return (area.y, None);
    };
    let mut y = section(
        paint,
        (area.x, area.y),
        area.w,
        "forms",
        &[
            "the same parts, dressed another way".to_owned(),
            "vs as written".to_owned(),
            "high = the form carries it".to_owned(),
            "row → answer".to_owned(),
        ],
    );
    let read = rendered_forms(forms);
    let rows: Vec<ReadingRow> = forms
        .iter()
        .enumerate()
        .filter(|(_, formed)| formed.get("not_rendered").is_none())
        .map(|(at, formed)| ReadingRow {
            first: form_name(formed),
            moved: moved_of(formed),
            loud: moved_of(formed) > floor,
            cells: held_cells(formed, depth),
            text: String::new(),
            act: Some(Act::ShowForm(at)),
            chosen: desk.shown == Some(crate::Shown::Form(at)),
        })
        .collect();
    let mut act = None;
    if !rows.is_empty() {
        let columns = [
            Column {
                head: "moved",
                at: MOVED_AT + 24.0,
                right: true,
            },
            Column {
                head: "",
                at: MOVED_AT + 36.0 + BAR_WIDTH,
                right: true,
            },
            Column {
                head: "1st",
                at: 324.0,
                right: true,
            },
            Column {
                head: "open",
                at: 378.0,
                right: true,
            },
        ];
        let (after, pressed) = reading_table(
            paint,
            mouse,
            Box::new(area.x, y, area.w, 0.0),
            "form",
            &columns,
            &rows,
        );
        y = after;
        act = pressed;
    }
    for formed in forms
        .iter()
        .filter(|formed| formed.get("not_rendered").is_some())
    {
        y = foot(
            paint,
            (area.x, y + 4.0),
            &format!("{} not rendered", form_name(formed)),
            formed
                .get("not_rendered")
                .and_then(Value::as_text)
                .unwrap_or(""),
            ink.quiet,
        );
    }
    let y = foot(
        paint,
        (area.x, y + 4.0),
        &format!("past floor {}", as_percent(floor)),
        &format!(
            "{} of {} · form read, not words",
            read.iter()
                .filter(|formed| moved_of(formed) > floor)
                .count(),
            read.len()
        ),
        ink.ink,
    );
    (y + 8.0, act)
}

/// The form a served row names.
fn form_name(formed: &Value) -> String {
    formed
        .get("form")
        .and_then(Value::as_text)
        .unwrap_or("")
        .to_owned()
}

/// Whether several seeds gave several answers, under the temperature it
/// was asked at — or not asked, which is never *settled* (A7, B-431, B60).
fn seeds_line(paint: &mut Painter, at: (f32, f32), width: f32, found: &Value) -> f32 {
    let Some(settled) = found
        .get("settled")
        .filter(|held| matches!(held, Value::Map(_)))
    else {
        return at.1;
    };
    let temperature = settled
        .get("temperature")
        .and_then(Value::as_text)
        .unwrap_or("?");
    let y = section(
        paint,
        at,
        width,
        "seeds",
        &[
            format!(
                "{} at temperature {temperature}",
                count_of(
                    usize::try_from(integer(settled, "seeds_asked")).unwrap_or(0),
                    "seed"
                )
            ),
            format!("distinct answers {}", integer(settled, "distinct_answers")),
            format!(
                "farthest apart {}",
                as_percent(integer(settled, "spread_parts_per_million"))
            ),
            format!(
                "farthest from greedy {}",
                as_percent(integer(settled, "from_greedy_parts_per_million"))
            ),
            "about the pair, not the prompt".to_owned(),
        ],
    );
    section(paint, (at.0, y), width, "cut", &cut_lines(settled))
}

/// How each seeded draw was cut before it was taken, and whose cut it was
/// (B-440): `top_k 20 · top_p 0.950 · min_p off · declared by the file`. A
/// report from before the cut was stated says so rather than *off* (A7).
fn cut_lines(settled: &Value) -> Vec<String> {
    let named = |key: &str| settled.get(key).and_then(Value::as_text);
    match (
        named("top_k"),
        named("top_p"),
        named("min_p"),
        named("truncation"),
    ) {
        (Some(top_k), Some(top_p), Some(min_p), Some(whose)) => vec![
            format!("top_k {top_k}"),
            format!("top_p {top_p}"),
            format!("min_p {min_p}"),
            whose.to_owned(),
        ],
        _ => vec!["not recorded".to_owned()],
    }
}

/// How the model received each word (B-443): where its first piece ranked
/// in the model's own choice, how many of its pieces the model would have
/// written itself, and the part it begins in. Least expected first, a word
/// the model would have written whole left off the table; past the depth
/// read is a bound, not an absence (A7). Not taken is said with why.
fn expected_table(paint: &mut Painter, at: (f32, f32), width: f32, found: &Value) -> f32 {
    let ink = paint.ink;
    let Some(by_word) = found
        .get("expected_by_word")
        .filter(|held| matches!(held, Value::Map(_)))
    else {
        let why = found
            .get("expected_refused")
            .and_then(Value::as_text)
            .unwrap_or("not taken")
            .to_owned();
        return section(paint, at, width, "expected", &["not taken".to_owned(), why]) + 4.0;
    };
    let depth = integer(found, "ranked_depth");
    let words = by_word.get("words").and_then(Value::as_list).unwrap_or(&[]);
    let (surprising, tally) = tallied(words);
    let WordTally {
        whole,
        own,
        pieces,
        past,
        unread,
        no_context,
    } = tally;
    let mut y = section(paint, at, width, "expected", &expected_conditions(found));
    y = heads(paint, Box::new(at.0, y, width, 0.0), "rank", &WORD_COLUMNS);
    for (rank, word) in surprising.iter().take(MOST_WORDS) {
        word_row(paint, (at.0, y), width, (*rank, word), depth);
        y += 20.0;
    }
    let left = surprising.len().saturating_sub(MOST_WORDS);
    if left > 0 {
        y = foot(
            paint,
            (at.0, y + 4.0),
            "more",
            &format!("{} · mcf prompt --json", count_of(left, "word")),
            ink.quiet,
        );
    } else {
        y += 4.0;
    }
    y = foot(
        paint,
        (at.0, y),
        "first choice",
        &format!("{whole}/{} words · {own}/{pieces} pieces", words.len()),
        ink.ink,
    );
    if past > 0 {
        y = foot(
            paint,
            (at.0, y),
            "past depth",
            &format!(
                "{} · rank {depth} or further",
                count_of(usize::try_from(past).unwrap_or(0), "word")
            ),
            ink.warn,
        );
    }
    if no_context > 0 {
        y = foot(
            paint,
            (at.0, y),
            "no context",
            &format!(
                "{} · first in the prompt, nothing before it to rank against",
                count_of(usize::try_from(no_context).unwrap_or(0), "word")
            ),
            ink.quiet,
        );
    }
    if unread > 0 {
        y = foot(
            paint,
            (at.0, y),
            "in no piece",
            &count_of(usize::try_from(unread).unwrap_or(0), "word"),
            ink.warn,
        );
    }
    let unplaced = integer(by_word, "unplaced");
    if unplaced > 0 {
        y = foot(
            paint,
            (at.0, y),
            "in no word",
            &count_of(usize::try_from(unplaced).unwrap_or(0), "piece"),
            ink.quiet,
        );
    }
    y + 8.0
}

/// How many words the table shows before it says how many more there are.
const MOST_WORDS: usize = 15;

/// What the word table's foot counts.
#[derive(Default)]
struct WordTally {
    /// Words the model would have written whole.
    whole: i64,
    /// Pieces that were the model's own first choice.
    own: i64,
    /// Pieces read: the unread first piece of the prompt is not among them.
    pieces: i64,
    /// Words whose first ranked piece was past the depth read.
    past: i64,
    /// Words no piece fell in.
    unread: i64,
    /// Words nothing preceded, so no piece of them was ranked.
    no_context: i64,
}

/// The words the model did not write whole, least expected first, and the
/// foot's counts.
fn tallied(words: &[Value]) -> (Vec<(i64, &Value)>, WordTally) {
    let mut surprising: Vec<(i64, &Value)> = Vec::new();
    let mut tally = WordTally::default();
    for word in words {
        // A word no piece fell in was not read, and is not a first choice:
        // nought of nought is not every piece the model's own (A7, F160).
        if integer(word, "pieces") == 0 {
            tally.unread = tally.unread.saturating_add(1);
            continue;
        }
        // A word nothing preceded was not ranked either; only its read
        // pieces are counted (F160).
        let read = integer(word, "pieces").saturating_sub(integer(word, "unread"));
        if read == 0 {
            tally.no_context = tally.no_context.saturating_add(1);
            continue;
        }
        tally.pieces = tally.pieces.saturating_add(read);
        tally.own = tally.own.saturating_add(integer(word, "first_choice"));
        // Outside the list asked for: a bound, not an absence (A7).
        let rank = word
            .get("rank")
            .and_then(Value::as_integer)
            .unwrap_or(i64::MAX);
        if rank == i64::MAX {
            tally.past = tally.past.saturating_add(1);
        }
        if integer(word, "first_choice") == read {
            tally.whole = tally.whole.saturating_add(1);
        } else {
            surprising.push((rank, word));
        }
    }
    surprising.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    (surprising, tally)
}

/// The word table's columns after the rank.
const WORD_COLUMNS: [Column; 3] = [
    Column {
        head: "own",
        at: 110.0,
        right: true,
    },
    Column {
        head: "#",
        at: 150.0,
        right: true,
    },
    Column {
        head: "word",
        at: 170.0,
        right: false,
    },
];

/// What the word reading says and was read under (§3.4, B-429).
fn expected_conditions(found: &Value) -> Vec<String> {
    let mut conditions = vec![
        "rank of each word's first piece in the model's own choice".to_owned(),
        "1 = it would have written that".to_owned(),
        "own = pieces it would have written".to_owned(),
        format!("# = {}", unit_of(found)),
    ];
    if let Some(under) = found.get("ranked_under").and_then(Value::as_text) {
        conditions.push(format!("read under {under}"));
    }
    if let Some(by) = found.get("read_by").and_then(Value::as_text) {
        conditions.push(format!("read by {by}"));
    }
    conditions
}

/// One word's row: its first piece's rank, its pieces the model would have
/// written itself over its pieces, the part it begins in, and the word.
fn word_row(paint: &mut Painter, at: (f32, f32), width: f32, held: (i64, &Value), depth: i64) {
    let ink = paint.ink;
    let (rank, word) = held;
    let cell = if rank == i64::MAX {
        format!(">{depth}")
    } else {
        rank.to_string()
    };
    paint.say_right(
        at.0 + 52.0,
        at.1,
        &cell,
        Weight::Bold,
        size::SMALL,
        ink.warn,
    );
    let own = format!(
        "{}/{}",
        integer(word, "first_choice"),
        integer(word, "pieces").saturating_sub(integer(word, "unread"))
    );
    paint.say_right(at.0 + 110.0, at.1, &own, Weight::Bold, size::SMALL, ink.ink);
    // A word in no part is drawn quietly (A7).
    let part = word
        .get("part")
        .and_then(Value::as_integer)
        .map_or_else(|| "—".to_owned(), |part| part.to_string());
    let quiet = if part == "—" { ink.faint } else { ink.quiet };
    paint.say_right(
        at.0 + 150.0,
        at.1,
        &part,
        Weight::Regular,
        size::SMALL,
        quiet,
    );
    let shown = paint.elide(
        &format!(
            "{:?}",
            word.get("text")
                .and_then(Value::as_text)
                .unwrap_or_default()
        ),
        Weight::Regular,
        size::SMALL,
        width - 170.0,
    );
    paint.say_at(
        at.0 + 170.0,
        at.1,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.ink,
    );
}

/// What the model actually said, under the figures about it.
///
/// **The report was all measurement and no evidence.** Every number on this
/// screen is *how much of the answer moved*, and the answer itself was on the
/// console and not here — so a reader could see that a sentence moved 96% of
/// something they were never shown. The figures are checkable only beside the
/// thing they are about (A19, §3.15).
fn the_answer(paint: &mut Painter, desk: &Desk, area: Box, found: &Value) {
    let ink = paint.ink;
    // The answer without whichever part is being asked about, or to it
    // alone, or to that much of the prompt — or as written when none is.
    // The title names the row the way the table does.
    let chosen = desk.shown.and_then(|shown| match shown {
        crate::Shown::Without(at) => {
            let clause = found.get("clauses").and_then(Value::as_list)?.get(at)?;
            let without = clause.get("without").and_then(Value::as_text)?;
            Some((
                without.to_owned(),
                format!("answer · without #{}", at.saturating_add(1)),
            ))
        }
        crate::Shown::Alone(at) => {
            let read = found.get("alone").and_then(Value::as_list)?.get(at)?;
            let answer = read.get("answer").and_then(Value::as_text)?;
            Some((
                answer.to_owned(),
                format!("answer · #{} alone", at.saturating_add(1)),
            ))
        }
        crate::Shown::Prefix(at) => {
            let read = found.get("prefixes").and_then(Value::as_list)?.get(at)?;
            let answer = read.get("answer").and_then(Value::as_text)?;
            Some((
                answer.to_owned(),
                format!("answer · 1–{}", at.saturating_add(1)),
            ))
        }
        crate::Shown::Swap(at) => {
            let read = found.get("swaps").and_then(Value::as_list)?.get(at)?;
            let answer = read.get("answer").and_then(Value::as_text)?;
            Some((
                answer.to_owned(),
                format!("answer · {}", crate::pair_mark(at)),
            ))
        }
        crate::Shown::Form(at) => {
            let read = found.get("forms").and_then(Value::as_list)?.get(at)?;
            let answer = read.get("answer").and_then(Value::as_text)?;
            Some((answer.to_owned(), format!("answer · {}", form_name(read))))
        }
    });
    let (said, title) = if let Some((answer, title)) = &chosen {
        (answer.as_str(), title.clone())
    } else {
        let mut title = vec!["answer · as written".to_owned()];
        title.extend(crate::answer_marks(found));
        (
            found.get("baseline").and_then(Value::as_text).unwrap_or(""),
            title.join(" · "),
        )
    };
    if area.h < 40.0 {
        return;
    }
    // An empty answer is said, not skipped (A7, F160): the title stays, with
    // the token count and what ended it, over one line that says nothing
    // was written.
    let said = if said.trim().is_empty() {
        crate::NOTHING_WRITTEN
    } else {
        said
    };
    let title = paint.elide(&title, Weight::Regular, size::SMALL, area.w.min(820.0));
    spaced(paint, area.x, area.y, &title, ink.faint);
    let room = area.w.min(820.0);
    // **Wrapped line by line, so the answer keeps its shape.** `wrap` breaks on
    // width and treats a newline as a space, which turns a function into one
    // run-on line — and an answer is code as often as it is prose. Each of the
    // model's own lines is wrapped on its own and they are laid out in order.
    let mut lines = Vec::new();
    for written in said.trim().lines() {
        if written.trim().is_empty() {
            lines.push(String::new());
        } else if paint.measure(written, Weight::Regular, size::SMALL) <= room {
            // **Verbatim where it fits.** `wrap` breaks on words and rejoins
            // with single spaces, which loses the leading spaces — and an
            // answer is code as often as it is prose, where the indentation is
            // part of what the reader is checking. A line that fits needs no
            // wrapping and keeps exactly what the model wrote.
            lines.push(written.to_owned());
        } else {
            lines.extend(paint.wrap(written, Weight::Regular, size::SMALL, room));
        }
    }
    // As many as fit in the room given, and no more: the console's `mcf
    // prompt --json` carries the whole answer for a reader who wants it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "how many 15-point lines fit in the room left"
    )]
    let fits = (((area.h - 24.0) / 15.0).floor().max(0.0)) as usize;
    let mut y = area.y + 22.0;
    for line in lines.iter().take(fits) {
        paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.quiet);
        y += 15.0;
    }
    if lines.len() > fits {
        paint.say_at(
            area.x,
            y,
            &format!(
                "… {} more · mcf prompt --json",
                count_of(lines.len().saturating_sub(fits), "line")
            ),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

/// The one field: the prompt, whole.
///
/// Returns what was pressed and where the row of buttons goes.
fn the_document(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> (Option<Act>, f32) {
    let mut act = None;
    // **A document, not a line, and one of them.** What is analysed is a
    // persona or an instruction sheet, pasted in whole — every part of it,
    // with nothing held out of the ablation in a second field; the field is
    // sized for one and the page below it gives up a little height (B-430).
    // Once there is a report
    // the report is what the page is for, and the field keeps its tail and
    // three lines: this window does not scroll.
    let reported = desk
        .doing
        .job()
        .is_some_and(|job| job.finished && job.latest().is_some());
    let height = if reported {
        72.0
    } else {
        (area.h * 0.24).clamp(110.0, 220.0)
    };
    let field = Box::new(area.x, area.y + 52.0, area.w.min(960.0), height);
    if ui::area(
        paint,
        mouse,
        field,
        &desk.typed,
        "the prompt, whole · Return: new line · Ctrl+Return: analyse · Ctrl+C: copy",
        desk.caret == Caret::Document,
    ) {
        act = Some(Act::Focus(Caret::Document));
    }
    (act, field.bottom() + 8.0)
}

/// What pressing Analyse will do and cost, as a table: a row a reading,
/// with the choice in it, the condition it is read under, and the
/// generations it spends — and the total, before it is spent (§3.8,
/// §3.15). The unit the document is taken apart into, how many parts are
/// removed, the temperature the seeds are drawn at and every further
/// reading are choices on the page rather than constants behind it
/// (B-430, B-431, B-434, B-435, B-436, B60).
///
/// Returns what was pressed and the table's bottom.
fn the_readings(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: (f32, f32),
    width: f32,
) -> (Option<Act>, f32) {
    let taken = desk.taken();
    if desk.doing.busy() || taken.text.is_empty() {
        return (None, at.1);
    }
    let area = Box::new(at.0, at.1, width, 0.0);
    let columns = [
        Column {
            head: "choice",
            at: READING_CHOICE,
            right: false,
        },
        Column {
            head: "condition",
            at: READING_CONDITION,
            right: false,
        },
        Column {
            head: "generations",
            at: width,
            right: true,
        },
    ];
    // The answer as written is the one generation every run spends.
    let mut table = Readings {
        area,
        y: heads(paint, area, "reading", &columns),
        total: 1,
        act: None,
    };
    let ink = paint.ink;
    let parts = taken.parts().len();
    table.unit_row(paint, mouse, &taken);
    if parts <= 1 {
        table.row(
            paint,
            "removed",
            ("nothing to remove · one part", ink.warn),
            Some((0, true)),
        );
    } else {
        table.removed_row(paint, mouse, &taken);
        table.extra_rows(paint, desk, mouse, &taken);
        table.seeds_row(paint, desk, mouse);
    }
    let bottom = table.total_row(paint);
    (table.act, bottom)
}

/// The readings table as it is drawn, a row at a time.
struct Readings {
    area: Box,
    y: f32,
    total: usize,
    act: Option<Act>,
}

/// Where the readings table's choice and condition columns start.
const READING_CHOICE: f32 = 96.0;
const READING_CONDITION: f32 = 440.0;
/// A readings row: a button's height and a little air.
const READING_ROW: f32 = ui::BUTTON + 2.0;

impl Readings {
    /// One row's label, condition and cost. A reading not asked for still
    /// says what it would cost, quietly, and adds nothing to the total
    /// (§3.15).
    fn row(
        &mut self,
        paint: &mut Painter,
        label: &str,
        condition: (&str, Rgb),
        spent: Option<(usize, bool)>,
    ) {
        let ink = paint.ink;
        let (area, y) = (self.area, self.y);
        paint.say_at(
            area.x,
            y + 8.0,
            label,
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        let shown = paint.elide(
            condition.0,
            Weight::Regular,
            size::SMALL,
            area.w - READING_CONDITION - 110.0,
        );
        paint.say_at(
            area.x + READING_CONDITION,
            y + 10.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            condition.1,
        );
        if let Some((generations, asked)) = spent {
            paint.say_right(
                area.right(),
                y + 8.0,
                &generations.to_string(),
                Weight::Bold,
                size::BODY,
                if asked { ink.ink } else { ink.faint },
            );
            if asked {
                self.total = self.total.saturating_add(generations);
            }
        }
        self.y += READING_ROW;
    }

    /// The unit, decided by the text unless chosen here.
    fn unit_row(
        &mut self,
        paint: &mut Painter,
        mouse: &Mouse,
        taken: &mcf_serve::prompt::Taken<'_>,
    ) {
        use mcf_serve::prompt::Unit;
        let (unit, chosen) = taken.unit();
        let parts = taken.parts().len();
        let mut x = self.area.x + READING_CHOICE;
        for each in Unit::ALL {
            let (pressed, button) = ui::fitted(
                paint,
                mouse,
                (x, self.y),
                each.name(),
                if unit == each {
                    Kind::Primary
                } else {
                    Kind::Quiet
                },
            );
            if pressed {
                self.act = Some(Act::TakeApartBy(Some(each)));
            }
            x = button.right() + 4.0;
        }
        let decided = format!(
            "{} · {}",
            if chosen {
                "chosen here"
            } else if unit == Unit::Paragraph {
                "text: blank lines"
            } else {
                "text: no blank line"
            },
            count_of(parts, unit.name())
        );
        let quiet = paint.ink.quiet;
        self.row(paint, "unit", (&decided, quiet), None);
    }

    /// How many parts are removed: in steps, up to every one.
    fn removed_row(
        &mut self,
        paint: &mut Painter,
        mouse: &Mouse,
        taken: &mcf_serve::prompt::Taken<'_>,
    ) {
        let parts = taken.parts().len();
        let cap = taken.cap();
        let removed = parts.min(cap);
        let mut x = self.area.x + READING_CHOICE;
        for (name, to) in [
            ("fewer", cap.saturating_sub(4).max(1)),
            ("more", cap.saturating_add(4).min(parts)),
            ("all", parts),
        ] {
            let (pressed, button) = ui::fitted(paint, mouse, (x, self.y), name, Kind::Ordinary);
            if pressed {
                self.act = Some(Act::MostParts(to));
            }
            x = button.right() + 4.0;
        }
        let how_many = if removed == parts {
            format!("{removed} of {parts}")
        } else if taken.most.is_some() {
            format!("first {removed} of {parts} · chosen here")
        } else {
            format!(
                "first {removed} of {parts} · default {}",
                mcf_serve::prompt::MOST_CLAUSES
            )
        };
        let quiet = paint.ink.quiet;
        self.row(paint, "removed", (&how_many, quiet), Some((removed, true)));
    }

    /// The control — one draw, or at every position — and the further
    /// readings, each a switch with its cost beside it (B-434, B-435,
    /// B-436).
    fn extra_rows(
        &mut self,
        paint: &mut Painter,
        desk: &Desk,
        mouse: &Mouse,
        taken: &mcf_serve::prompt::Taken<'_>,
    ) {
        use mcf_serve::prompt::Extra;
        let parts = taken.parts().len();
        let removed = parts.min(taken.cap());
        for extra in Extra::ALL {
            let asked = desk.extras.has(extra);
            let (pressed, _) = ui::fitted(
                paint,
                mouse,
                (self.area.x + READING_CHOICE, self.y),
                if asked { "on" } else { "off" },
                if asked { Kind::Primary } else { Kind::Quiet },
            );
            if pressed {
                self.act = Some(Act::Extra(extra, !asked));
            }
            let (label, condition, generations) = match extra {
                Extra::Floors => (
                    "control",
                    if asked {
                        format!(
                            "every position · {}",
                            count_of(removed.saturating_add(1), "draw")
                        )
                    } else {
                        "1 draw · before the last part".to_owned()
                    },
                    if asked {
                        extra.generations(parts, removed).saturating_add(1)
                    } else {
                        1
                    },
                ),
                Extra::Alone => (
                    "alone",
                    "each part as the whole prompt · control alone".to_owned(),
                    extra.generations(parts, removed),
                ),
                Extra::Prefixes => (
                    "prefixes",
                    "grown a part at a time from the front · short of the whole".to_owned(),
                    extra.generations(parts, removed),
                ),
                Extra::Swaps => (
                    "swaps",
                    "each part and the next in each other's places · words kept".to_owned(),
                    extra.generations(parts, removed),
                ),
                // Counted from the text rather than forecast from the
                // count of parts: a form the prompt is already in is not
                // asked, and the page has the text (B-444).
                Extra::Forms => (
                    "forms",
                    "one line · bullets · numbered · headings · tags · capitals · words kept"
                        .to_owned(),
                    mcf_serve::prompt::forms_asked(&taken.parts()),
                ),
            };
            let quiet = paint.ink.quiet;
            self.row(
                paint,
                label,
                (&condition, quiet),
                Some((generations, asked || extra == Extra::Floors)),
            );
        }
    }

    /// The temperature the seeds are drawn at: empty asks nothing, and the
    /// page says so rather than settling on a value of its own (B60,
    /// B-431). What is not a temperature holds Analyse (§3.15).
    fn seeds_row(&mut self, paint: &mut Painter, desk: &Desk, mouse: &Mouse) {
        use mcf_serve::prompt::SEEDS;
        let ink = paint.ink;
        let field = Box::new(self.area.x + READING_CHOICE, self.y, 96.0, ui::BUTTON);
        if ui::field(
            paint,
            mouse,
            field,
            &desk.temperature,
            "e.g. 0.7",
            desk.caret == Caret::Temperature,
        ) {
            self.act = Some(Act::Focus(Caret::Temperature));
        }
        let (condition, colour, seeds) = match desk.settle() {
            Ok(None) => (
                "empty · at 0 the seed changes nothing · no house value".to_owned(),
                ink.quiet,
                false,
            ),
            Ok(Some(held)) => (
                format!(
                    "{SEEDS} at {held} · cut as the file declares, else off · every other greedy"
                ),
                ink.quiet,
                true,
            ),
            Err(typed) => (
                format!(
                    "\"{typed}\" is not a temperature · a decimal above 0, to 3 places, or \
                     empty · Analyse waits"
                ),
                ink.bad,
                false,
            ),
        };
        self.row(paint, "seeds", (&condition, colour), Some((SEEDS, seeds)));
    }

    /// The total under the rows. Returns the line under it.
    fn total_row(&self, paint: &mut Painter) -> f32 {
        let ink = paint.ink;
        let (area, y) = (self.area, self.y);
        paint.rule((area.x, y + 2.0), (area.right(), y + 2.0), ink.line, 255);
        paint.say_at(
            area.x,
            y + 10.0,
            "total",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        paint.say_at(
            area.x + READING_CONDITION,
            y + 12.0,
            "+ 1 as written · greedy · seed held",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        paint.say_right(
            area.right(),
            y + 10.0,
            &self.total.to_string(),
            Weight::Bold,
            size::BODY,
            ink.ink,
        );
        y + 36.0
    }
}

/// What the report took the document apart into, as it names one part.
fn unit_of(found: &Value) -> &str {
    found
        .get("unit")
        .and_then(Value::as_text)
        .unwrap_or("sentence")
}

/// A count and the thing counted, in English.
///
/// `1 sentence`, `2 sentences`. Every one of these read `1 sentence(s)`, which
/// is the sort of thing a reader forgives once and stops trusting the fourth
/// time.
fn count_of(how_many: usize, noun: &str) -> String {
    if how_many == 1 {
        format!("1 {noun}")
    } else {
        format!("{how_many} {noun}s")
    }
}

/// A share of the answer, as a person reads one.
///
/// The wire carries parts per million because that is a count and not a
/// rounding; a reader wants one decimal place.
fn as_percent(parts_per_million: i64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a percentage shown to one decimal place"
    )]
    let held = parts_per_million as f64 / 10_000.0;
    format!("{held:.1}%")
}

/// What MCF can build, and which of it is here.
///
/// **Read-only, and says so.** Building takes minutes and writes a record of
/// its own, so it is a command rather than something a window waits on a
/// socket for — and a screen that offered a button it could not honour would
/// be worse than one that names the command (A7).
fn components(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "components", ink.faint);
    paint.say_at(
        area.x,
        area.y + 28.0,
        "each built from pinned source in a pinned container, and removable without residue",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );

    if desk.components.is_empty() {
        paint.say_at(
            area.x,
            area.y + 58.0,
            words::UNMEASURED,
            Weight::Regular,
            size::BODY,
            ink.faint,
        );
        return None;
    }

    let mut act = None;
    let mut y = area.y + 58.0;
    let wide = area.w.min(920.0);
    for component in &desk.components {
        let tall = 86.0;
        let card = Box::new(area.x, y, wide, tall);
        if y + tall > area.bottom() {
            break;
        }
        act = component_card(paint, desk, mouse, component, card).or(act);
        y += tall + 10.0;
    }
    act
}

/// One component: what it is, what state it is in, and what can be done.
fn component_card(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    component: &crate::Component,
    card: Box,
) -> Option<Act> {
    let ink = paint.ink;
    ui::card(paint, card, component.provisioned);

    // The name, and exactly which source it was built from.
    paint.say_at(
        card.x + 14.0,
        card.y + 14.0,
        &component.name,
        Weight::Bold,
        size::BODY,
        ink.ink,
    );
    let after = paint.measure(&component.name, Weight::Bold, size::BODY);
    paint.say_at(
        card.x + 14.0 + after + 8.0,
        card.y + 14.0,
        &format!("@{}", component.commit),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );

    // What it is, wrapped rather than cut: the sentence is the reason to
    // have it, and half of one is not a reason.
    let mut at = card.y + 34.0;
    for line in paint
        .wrap(
            &component.role,
            Weight::Regular,
            size::SMALL,
            card.w - 190.0,
        )
        .iter()
        .take(2)
    {
        paint.say_at(
            card.x + 14.0,
            at,
            line,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        at += 16.0;
    }

    // A build of this one, running: the card it is for shows it, with how
    // long so far and the last line the build printed, so that a build that
    // takes minutes is watched rather than waited on (A2, B-367).
    let running = match &desk.doing {
        Doing::Provisioning(job)
            if !job.finished && desk.building.as_deref() == Some(component.name.as_str()) =>
        {
            Some(job)
        }
        _ => None,
    };

    // Four states, not two. The third is a run that stopped partway: a prefix
    // with no provenance beside it. Saying that plainly is what keeps a person
    // from reading a half-build as a build (A7).
    //
    // Whether MCF can reach it as an ENGINE is a separate fact, said below —
    // not every component is an engine, and a window library that reported
    // itself unreachable would be answering a question nobody asked.
    let (word, ground, colour) = if running.is_some() {
        ("Building", ink.warn_soft, ink.warn)
    } else if component.provisioned {
        ("Provisioned", ink.accent_soft, ink.good)
    } else if component.present {
        ("Incomplete", ink.warn_soft, ink.warn)
    } else {
        ("Not provisioned", ink.sunk, ink.quiet)
    };
    let _wide = ui::tag(
        paint,
        (card.right() - 130.0, card.y + 13.0),
        word,
        ground,
        colour,
    );

    let foot = card.bottom() - 24.0;
    if let Some(job) = running {
        let printed = job
            .answers
            .iter()
            .rev()
            .find_map(|answer| answer.get("doing").and_then(Value::as_text))
            .unwrap_or("starting");
        let said = format!("{}s so far — {printed}", job.ran());
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, card.w - 28.0);
        paint.say_right(
            card.right() - 14.0,
            foot,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        return None;
    }
    component_foot(paint, desk, mouse, component, card, foot)
}

/// The bottom line of a card that is not building: where it is, why the last
/// build stopped, and the button that builds it.
fn component_foot(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    component: &crate::Component,
    card: Box,
    foot: f32,
) -> Option<Act> {
    let ink = paint.ink;
    // The failure first, in red, where there was one; else where the prefix
    // is. A finished build puts the prefix on the right, alone; an
    // unfinished one leaves the right to the button that builds it again.
    let failed = desk
        .build_failed
        .as_ref()
        .filter(|(failed, _)| failed == &component.name)
        .map(|(_, why)| format!("not built: {why}"));
    if let Some(said) = failed {
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, card.w - 130.0);
        paint.say_at(
            card.x + 14.0,
            foot,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.bad,
        );
    } else if component.provisioned {
        paint.say_right(
            card.right() - 14.0,
            foot,
            &if component.usable_engine {
                format!("{} — MCF reaches this as an engine", component.prefix)
            } else {
                component.prefix.clone()
            },
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    } else if component.present {
        let shown = paint.elide(
            &component.prefix,
            Weight::Regular,
            size::SMALL,
            card.w - 130.0,
        );
        paint.say_at(
            card.x + 14.0,
            foot,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    // Build here, from the window, while nothing else runs: one job at a time
    // is the window's rule, and a button that would be refused is not drawn.
    let idle = desk.doing.job().is_none_or(|job| job.finished);
    if !component.provisioned && idle {
        let label = if component.present {
            "Build again"
        } else {
            "Build"
        };
        let width = paint.measure(label, Weight::Bold, size::SMALL) + 28.0;
        let button = Box::new(card.right() - 14.0 - width, foot - 6.0, width, 26.0);
        if ui::button(paint, mouse, button, label, Kind::Ordinary) {
            return Some(Act::Build(component.name.clone()));
        }
    }
    None
}

/// What a model is made of, as the daemon counted it.
///
/// **Counted there, drawn here.** Every figure on this screen came over the
/// socket from the same counting `mcf explain` prints, and nothing on it is
/// worked out by the window (B-072). Nothing on it is a measurement either:
/// it is the file's directory set against the file's header, and one token's
/// arithmetic from both (A20, A21). A speed is on the Diagnostics screen.
fn anatomy(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let (switched, top) = counted_head(
        paint,
        desk,
        mouse,
        area,
        Page::Anatomy,
        "counted from the file's tensor directory and set against its header — read, not \
         measured",
    );
    let Some(said) = &desk.anatomy else {
        not_counted(paint, desk, area, top);
        return switched;
    };
    // Two columns: the figures and the header check on the left, the tables
    // on the right. The window does not scroll, so a table that does not fit
    // is cut at a row that says so rather than drawn over the edge.
    let figures = 460.0_f32.min(area.w / 2.0);
    let left = Box::new(area.x, top, figures, area.bottom() - top);
    let after = counted(paint, left, said);
    // The header check before the arithmetic: a disagreement is the finding
    // this screen exists to show, and the arithmetic is drawn from a header
    // the check has just vouched for.
    let after = agreements(
        paint,
        Box::new(area.x, after + 24.0, figures, area.bottom() - after - 24.0),
        &said.agreements,
    );
    arithmetic(
        paint,
        Box::new(area.x, after + 24.0, figures, area.bottom() - after - 24.0),
        said,
    );
    let right = Box::new(
        area.x + figures + 40.0,
        top,
        (area.w - figures - 40.0).max(200.0),
        area.bottom() - top,
    );
    let after = share_table(paint, right, "by part", &said.parts, said.elements);
    let after = share_table(
        paint,
        Box::new(
            right.x,
            after + 24.0,
            right.w,
            right.bottom() - after - 24.0,
        ),
        "by encoding",
        &said.encodings,
        said.elements,
    );
    by_block(
        paint,
        Box::new(
            right.x,
            after + 24.0,
            right.w,
            right.bottom() - after - 24.0,
        ),
        said,
    );
    switched
}

/// The heading the two counted screens share: what is in it, and what it
/// says with — with the switch between them. Returns where the body starts.
fn counted_head(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    here: Page,
    subtitle: &str,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "what is in it", ink.faint);
    let named = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .map_or("no model chosen", |held| held.name.as_str());
    // The switch first, so the name can be cut short of it.
    let mut switched = None;
    let mut right = area.right();
    for (label, page) in [
        ("Vocabulary", Page::Vocabulary),
        ("What is in it", Page::Anatomy),
    ] {
        let width = paint.measure(label, Weight::Bold, size::SMALL) + 28.0;
        right -= width;
        let button = Box::new(right, area.y + 18.0, width, 28.0);
        // The screen this is is drawn as text and the other as a button, so
        // the pair reads as *where you are* and *where else you can be*.
        let kind = if page == here {
            Kind::Quiet
        } else {
            Kind::Ordinary
        };
        if ui::button(paint, mouse, button, label, kind) && page != here {
            switched = Some(Act::Go(page));
        }
        right -= 10.0;
    }
    let name = paint.elide(named, Weight::Bold, size::HEAD, right - area.x - 20.0);
    paint.say_at(
        area.x,
        area.y + 24.0,
        &name,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    paint.say_at(
        area.x,
        area.y + 54.0,
        subtitle,
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    (switched, area.y + 90.0)
}

/// Why there is nothing counted to draw, in the daemon's words or the
/// window's own (A2).
fn not_counted(paint: &mut Painter, desk: &Desk, area: Box, top: f32) {
    let ink = paint.ink;
    let why = desk
        .no_anatomy
        .clone()
        .unwrap_or_else(|| "Nothing has been asked yet.".to_owned());
    let mut y = top;
    for line in paint.wrap(&why, Weight::Regular, size::BODY, area.w.min(720.0)) {
        paint.say_at(area.x, y, &line, Weight::Regular, size::BODY, ink.bad);
        y += 20.0;
    }
}

/// What a model says with, as the daemon counted its token list.
///
/// Nothing is tokenised and nothing is rated: this is the list the header
/// carries, counted, and the tokens the header names looked up in it — a
/// number past the end of the list is shown as the fault it is (A2). Every
/// sentence on the screen came over the socket in the words `mcf explain`
/// prints (B-072).
fn vocabulary(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let (switched, top) = counted_head(
        paint,
        desk,
        mouse,
        area,
        Page::Vocabulary,
        "counted from the header's token list — nothing tokenised, nothing rated",
    );
    let Some(said) = &desk.anatomy else {
        not_counted(paint, desk, area, top);
        return switched;
    };
    let spoken = &said.vocabulary;
    let figures = 560.0_f32.min(area.w / 2.0);
    let left = Box::new(area.x, top, figures, area.bottom() - top);
    let after = spoken_figures(paint, left, spoken);
    template(
        paint,
        Box::new(area.x, after + 24.0, figures, area.bottom() - after - 24.0),
        spoken,
    );
    let right = Box::new(
        area.x + figures + 40.0,
        top,
        (area.w - figures - 40.0).max(200.0),
        area.bottom() - top,
    );
    named_tokens(paint, right, spoken);
    switched
}

/// The list, counted: one figure a line, the long ones wrapped.
fn spoken_figures(paint: &mut Painter, area: Box, spoken: &SaidVocabulary) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "the token list", ink.faint);
    let mut rows = vec![
        ("segmentation", spoken.segmentation.clone()),
        (
            "tokens",
            spoken.merges.map_or_else(
                || words::grouped(spoken.tokens),
                |merges| {
                    format!(
                        "{}, from {} merges",
                        words::grouped(spoken.tokens),
                        words::grouped(merges)
                    )
                },
            ),
        ),
        (
            "by kind",
            spoken.kinds.as_ref().map_or_else(
                || "the file does not type its tokens".to_owned(),
                |kinds| {
                    kinds
                        .iter()
                        .map(|(kind, count)| format!("{} {kind}", words::grouped(*count)))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            ),
        ),
        (
            "begin a word",
            format!(
                "{} ({})",
                words::grouped(spoken.word_starts),
                percent_of(spoken.word_starts, spoken.tokens)
            ),
        ),
        ("digit runs", spoken.digits.clone()),
    ];
    if let Some((token, bytes)) = &spoken.longest {
        rows.push(("longest token", format!("{bytes} bytes: {token:?}")));
    }
    rows.push(("beginning token added", spoken.beginning.clone()));
    let mut y = area.y + 26.0;
    for (name, value) in rows {
        paint.say_at(area.x, y, name, Weight::Regular, size::BODY, ink.quiet);
        for line in paint.wrap(&value, Weight::Bold, size::BODY, area.w - 160.0) {
            if y > area.bottom() - 18.0 {
                return y;
            }
            // Wrapped at the spaces, then cut: the longest token in a list
            // is one word hundreds of bytes long, and a line with no space
            // in it wraps nowhere.
            let line = paint.elide(&line, Weight::Bold, size::BODY, area.w - 160.0);
            paint.say_at(area.x + 150.0, y, &line, Weight::Bold, size::BODY, ink.ink);
            y += 20.0;
        }
        y += 2.0;
    }
    y
}

/// The chat template: its size, what it mentions, and the control tokens it
/// frames a turn with — or why none is shown.
fn template(paint: &mut Painter, area: Box, spoken: &SaidVocabulary) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "chat template", ink.faint);
    let mut y = area.y + 26.0;
    let mut say = |paint: &mut Painter, text: &str, weight: Weight, colour: Rgb| {
        for line in paint.wrap(text, weight, size::BODY, area.w) {
            if y > area.bottom() - 18.0 {
                return;
            }
            paint.say_at(area.x, y, &line, weight, size::BODY, colour);
            y += 20.0;
        }
    };
    match &spoken.template {
        Err(none) => say(paint, none, Weight::Regular, ink.quiet),
        Ok(held) => {
            let mentions = if held.mentions.is_empty() {
                String::new()
            } else {
                format!(", mentioning {}", held.mentions.join(", "))
            };
            say(
                paint,
                &format!("{} bytes{mentions}", words::grouped(held.bytes)),
                Weight::Bold,
                ink.ink,
            );
            match &held.markers {
                Ok(markers) => say(
                    paint,
                    &format!(
                        "frames a turn with {}",
                        markers
                            .iter()
                            .map(|marker| format!("{marker:?}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    Weight::Regular,
                    ink.ink,
                ),
                Err(why) => say(
                    paint,
                    &format!("markers it frames a turn with: {why}"),
                    Weight::Regular,
                    ink.quiet,
                ),
            }
        }
    }
    y
}

/// The tokens the header names by number, each spelled from the list — or
/// shown to be beyond it, which an engine reading the header would not
/// survive (A2).
fn named_tokens(paint: &mut Painter, area: Box, spoken: &SaidVocabulary) -> f32 {
    let ink = paint.ink;
    let columns = [
        Column {
            head: "number",
            at: 200.0,
            right: true,
        },
        Column {
            head: "spelled",
            at: 220.0,
            right: false,
        },
    ];
    let mut y = heads(paint, area, "named tokens", &columns);
    if spoken.named.is_empty() {
        paint.say_at(
            area.x,
            y,
            "the header names none",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return y + 22.0;
    }
    for (shown, named) in spoken.named.iter().enumerate() {
        if y > area.bottom() - 60.0 && shown + 1 < spoken.named.len() {
            let left = spoken.named.len() - shown;
            paint.say_at(
                area.x,
                y,
                &format!("and {left} more — the window is too short for them"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            return y + 20.0;
        }
        let spelled = named.spelled.as_deref().map_or_else(
            || "—".to_owned(),
            |token| {
                paint.elide(
                    &format!("{token:?}"),
                    Weight::Bold,
                    size::BODY,
                    area.w - 220.0,
                )
            },
        );
        let cells = [
            (&columns[0], named.identifier.to_string()),
            (&columns[1], spelled),
        ];
        row(paint, area, y, &named.what, &cells);
        y += 22.0;
        if let Some(beyond) = &named.beyond {
            for line in paint.wrap(beyond, Weight::Regular, size::SMALL, area.w) {
                paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.bad);
                y += 16.0;
            }
            y += 4.0;
        }
    }
    y
}

/// Name and figure, one line each, the way the model's own card is set.
fn figures(paint: &mut Painter, area: Box, rows: &[(&str, String)]) -> f32 {
    let ink = paint.ink;
    let mut y = area.y;
    for (name, value) in rows {
        if y > area.bottom() - 18.0 {
            break;
        }
        paint.say_at(area.x, y, name, Weight::Regular, size::BODY, ink.quiet);
        let colour = if value == UNKNOWN || value == "—" {
            ink.faint
        } else {
            ink.ink
        };
        let shown = paint.elide(value, Weight::Bold, size::BODY, area.w - 160.0);
        paint.say_at(area.x + 150.0, y, &shown, Weight::Bold, size::BODY, colour);
        y += 22.0;
    }
    y
}

/// Bits an element, to two places, where the bytes are known.
fn bits_an_element(bytes: Option<u64>, elements: u64) -> Option<String> {
    let bytes = bytes?;
    if elements == 0 {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a density shown to two decimal places"
    )]
    let bits = bytes as f64 * 8.0 / elements as f64;
    Some(format!("{bits:.2} bits an element"))
}

/// A share of the whole, in percent to one place.
fn percent_of(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "—".to_owned();
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a share shown to one decimal place"
    )]
    let share = part as f64 * 100.0 / whole as f64;
    format!("{share:.1}%")
}

/// What the file holds, in total.
fn counted(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "counted", ink.faint);
    let weight = said.bytes.map_or_else(
        || {
            format!(
                "not sized: {} tensor(s) in an encoding this reader does not size",
                said.unsized_tensors
            )
        },
        |bytes| {
            let density = bits_an_element(Some(bytes), said.elements).unwrap_or_default();
            format!("{} — {density}", gigabytes(bytes))
        },
    );
    let mixing = match (said.attending, said.recurrent) {
        (0, 0) => String::new(),
        (attending, 0) if attending == said.blocks => ", all attending".to_owned(),
        (attending, 0) => format!(" — {attending} attending"),
        (0, recurrent) => format!(" — {recurrent} keeping a recurrent state"),
        (attending, recurrent) => {
            format!(" — {attending} attending, {recurrent} keeping a recurrent state")
        }
    };
    let rows = [
        ("parameters", words::grouped(said.elements)),
        (
            "active per token",
            said.active.map_or_else(
                || "all of them".to_owned(),
                |(active, experts, used)| {
                    format!("{} — {used} of {experts} experts", words::grouped(active))
                },
            ),
        ),
        ("weights", weight),
        ("blocks", format!("{}{mixing}", said.blocks)),
        (
            "output head",
            if said.output_tied {
                "reuses the embedding table".to_owned()
            } else {
                "its own".to_owned()
            },
        ),
    ];
    figures(
        paint,
        Box::new(area.x, area.y + 26.0, area.w, area.h - 26.0),
        &rows,
    )
}

/// One token's arithmetic, and the cache — sized, or why not.
fn arithmetic(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) -> f32 {
    use mcf_serve::anatomy::SaidCache;
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "one token's arithmetic", ink.faint);
    let unknown = || UNKNOWN.to_owned();
    let mut rows = vec![
        ("multiply-adds", words::grouped(said.multiply_adds)),
        (
            "head width",
            said.head_width
                .map_or_else(unknown, |held| held.to_string()),
        ),
        (
            "queries per key",
            said.queries_per_key
                .map_or_else(unknown, |held| held.to_string()),
        ),
        (
            "attention at full context",
            said.attention_at_context
                .map_or_else(unknown, |held| format!("{} more", words::grouped(held))),
        ),
    ];
    let why = match &said.cache {
        SaidCache::Sized {
            per_token,
            key_heads,
            kept,
            at_context,
            sliding_window,
            attending,
            recurrent,
        } => {
            rows.push((
                "cache per token",
                format!("{} bytes", words::grouped(*per_token)),
            ));
            if let Some((tokens, bytes)) = at_context {
                rows.push((
                    "cache at full context",
                    format!(
                        "{} at {} tokens",
                        gigabytes(*bytes),
                        words::grouped(*tokens)
                    ),
                ));
            }
            if let Some(window) = sliding_window {
                rows.push((
                    "sliding window",
                    format!(
                        "{} tokens — the full-context figure is a ceiling",
                        words::grouped(*window)
                    ),
                ));
            }
            let blocks = if *recurrent > 0 {
                format!(
                    "; kept in the {} of {} blocks that attend, none in the {recurrent} that keep a state",
                    attending.0, attending.1
                )
            } else {
                format!("; kept in {} of {} blocks", attending.0, attending.1)
            };
            (
                format!("{key_heads} head(s) keeping {kept}{blocks}"),
                ink.quiet,
            )
        }
        SaidCache::Unsized(reason) => {
            rows.push(("cache per token", "not sized".to_owned()));
            (reason.clone(), ink.quiet)
        }
    };
    let mut y = figures(
        paint,
        Box::new(area.x, area.y + 26.0, area.w, area.h - 26.0),
        &rows,
    );
    let (said, colour) = why;
    y += 4.0;
    for line in paint
        .wrap(&said, Weight::Regular, size::SMALL, area.w)
        .iter()
        .take(5)
    {
        paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, colour);
        y += 16.0;
    }
    y
}

/// Bytes in a table cell: a figure a person reads, at the scale it has.
fn bytes_figure(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a size shown to one or two decimal places"
    )]
    let held = bytes as f64;
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", held / 1e9)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", held / 1e6)
    } else {
        format!("{} B", words::grouped(bytes))
    }
}

/// Tensors, elements, share and bytes for each part or each encoding.
fn share_table(
    paint: &mut Painter,
    area: Box,
    first: &str,
    shares: &[mcf_serve::anatomy::SaidShare],
    whole: u64,
) -> f32 {
    let ink = paint.ink;
    let columns = [
        Column {
            head: "tensors",
            at: area.w - 300.0,
            right: true,
        },
        Column {
            head: "elements",
            at: area.w - 170.0,
            right: true,
        },
        Column {
            head: "share",
            at: area.w - 90.0,
            right: true,
        },
        Column {
            head: "bytes",
            at: area.w,
            right: true,
        },
    ];
    let mut y = heads(paint, area, first, &columns);
    for (shown, share) in shares.iter().enumerate() {
        if y > area.bottom() - 40.0 && shown + 1 < shares.len() {
            let left = shares.len() - shown;
            paint.say_at(
                area.x,
                y,
                &format!("and {left} more — the window is too short for them"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            return y + 20.0;
        }
        let cells = [
            (&columns[0], share.tensors.to_string()),
            (&columns[1], words::grouped(share.elements)),
            (&columns[2], percent_of(share.elements, whole)),
            (
                &columns[3],
                share.bytes.map_or_else(|| "—".to_owned(), bytes_figure),
            ),
        ];
        row(paint, area, y, &share.name, &cells);
        y += 22.0;
    }
    y
}

/// Each shape of block: which blocks, what they hold, and what each is made
/// of in the words every surface uses.
fn by_block(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "by block", ink.faint);
    let under = area.y + 17.0;
    paint.rule((area.x, under), (area.right(), under), ink.line, 255);
    let mut y = under + 10.0;
    for (shown, family) in said.families.iter().enumerate() {
        if y > area.bottom() - 60.0 && shown + 1 < said.families.len() {
            let left = said.families.len() - shown;
            paint.say_at(
                area.x,
                y,
                &format!("and {left} more — the window is too short for them"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            return y + 20.0;
        }
        let bits = family.bits.map_or_else(String::new, |(least, most)| {
            #[expect(
                clippy::integer_division,
                reason = "hundredths of a bit into bits and hundredths"
            )]
            let said = |held: u64| format!("{}.{:02}", held / 100, held % 100);
            if least == most {
                format!(" · {} bits an element", said(least))
            } else {
                format!(" · {}–{} bits an element", said(least), said(most))
            }
        });
        let head = format!(
            "{} block(s), {} · {} elements{bits}",
            family.blocks.len(),
            family.ranged,
            words::grouped(family.share.elements)
        );
        let shown = paint.elide(&head, Weight::Bold, size::BODY, area.w);
        paint.say_at(area.x, y, &shown, Weight::Bold, size::BODY, ink.ink);
        y += 20.0;
        for line in paint
            .wrap(&family.said, Weight::Regular, size::SMALL, area.w)
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.quiet);
            y += 16.0;
        }
        y += 8.0;
    }
    y
}

/// What the header says beside what the directory shows, and whether they
/// agree — a disagreement in the alarm colour, a missing side in neither
/// (A21, A7).
fn agreements(paint: &mut Painter, area: Box, held: &[mcf_serve::anatomy::SaidAgreement]) -> f32 {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "header against directory", ink.faint);
    let table = Box::new(area.x, area.y + 22.0, area.w, area.h - 22.0);
    let columns = [
        Column {
            head: "header says",
            at: 120.0,
            right: false,
        },
        Column {
            head: "directory shows",
            at: 230.0,
            right: false,
        },
        Column {
            head: "",
            at: area.w,
            right: true,
        },
    ];
    let mut y = heads(paint, table, "what", &columns);
    for (shown, agreement) in held.iter().enumerate() {
        if y > table.bottom() - 40.0 && shown + 1 < held.len() {
            let left = held.len() - shown;
            paint.say_at(
                area.x,
                y,
                &format!("and {left} more — the window is too short for them"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            return y + 20.0;
        }
        let cell = |paint: &mut Painter, held: &Option<String>, room: f32| {
            held.as_deref().map_or_else(
                || "—".to_owned(),
                |text| paint.elide(text, Weight::Bold, size::BODY, room),
            )
        };
        let cells = [
            (&columns[0], cell(paint, &agreement.declared, 100.0)),
            (
                &columns[1],
                cell(paint, &agreement.observed, area.w - 310.0),
            ),
        ];
        row(paint, table, y, &agreement.what, &cells);
        let (verdict, colour) = match agreement.agrees {
            Some(true) => ("agree", ink.good),
            Some(false) => ("DISAGREE", ink.bad),
            None => ("—", ink.faint),
        };
        paint.say_right(table.right(), y, verdict, Weight::Bold, size::BODY, colour);
        y += 22.0;
    }
    y
}

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
