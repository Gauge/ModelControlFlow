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
use crate::{Act, Desk, Doing, Model, Page, Picker, windows};
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
    if let Some(said) = held_window_cost(desk) {
        paint.say_at(
            at.x + 14.0,
            at.y + 93.0,
            &said,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
    }
    at.bottom()
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
        116.0
    } else {
        96.0
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

/// How many answers several seeds gave, and what that can mean here.
fn how_many_answers(paint: &mut Painter, at: (f32, f32), found: &Value) {
    let ink = paint.ink;
    let area = Box::new(at.0, at.1, 0.0, 0.0);
    let distinct = found
        .get("distinct_answers")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let seeds = found
        .get("seeds_asked")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    paint.say_at(
        area.x,
        area.y,
        &{
            let asked = count_of(usize::try_from(seeds).unwrap_or(0), "seed");
            if distinct <= 1 {
                // **Not "this prompt settles it".** Every generation is asked
                // at temperature 0, which takes the likeliest token every
                // time, so the seed changes nothing and this line said the
                // same for every prompt on every model. It was reporting the
                // sampler (F147).
                format!("{asked} gave one answer — under temperature 0 they could not differ")
            } else {
                format!("{asked} gave {distinct} answers, which temperature 0 should not do")
            }
        },
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
}

/// Which words the model did not expect, in the space beside the bars.
///
/// **A second reading of the prompt that does not compare two answers.** The
/// bars measure what changed when a sentence was removed, and removing
/// anything shifts everything after it. This asks where each token ranked
/// against what the model would have written there itself: first means the
/// writer supplied nothing, outside the list means the prompt said something
/// the model did not expect. Drift does not touch it, because nothing is
/// compared to anything (§3.8).
fn not_expected(paint: &mut Painter, area: Box, found: &Value) {
    let ink = paint.ink;
    let ranked = found
        .get("expected")
        .and_then(Value::as_list)
        .unwrap_or(&[]);
    if ranked.is_empty() {
        // Why it is missing, where it is missing: an empty column and one MCF
        // could not fill look the same (A7).
        if let Some(why) = found.get("expected_refused").and_then(Value::as_text) {
            spaced(paint, area.x, area.y, "not expected", ink.faint);
            for line in paint
                .wrap(why, Weight::Regular, size::SMALL, area.w)
                .iter()
                .take(3)
            {
                paint.say_at(
                    area.x,
                    area.y + 20.0,
                    line,
                    Weight::Regular,
                    size::SMALL,
                    ink.faint,
                );
            }
        }
        return;
    }
    let depth = found
        .get("ranked_depth")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let mut surprising: Vec<(i64, String)> = Vec::new();
    let mut first_choice = 0_usize;
    for held in ranked {
        let said = held
            .get("text")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned();
        match held.get("rank").and_then(Value::as_integer) {
            None => surprising.push((i64::MAX, said)),
            Some(1) => first_choice = first_choice.saturating_add(1),
            Some(rank) => surprising.push((rank, said)),
        }
    }
    surprising.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    spaced(paint, area.x, area.y, "words it did not expect", ink.faint);
    let mut y = area.y + 22.0;
    for (rank, said) in surprising.iter().take(8) {
        if y > area.bottom() - 34.0 {
            break;
        }
        let where_it_sat = if *rank == i64::MAX {
            format!("past {depth}")
        } else {
            format!("#{rank}")
        };
        paint.say_at(
            area.x,
            y,
            &where_it_sat,
            Weight::Bold,
            size::SMALL,
            ink.warn,
        );
        let shown = paint.elide(said.trim(), Weight::Regular, size::SMALL, area.w - 56.0);
        paint.say_at(
            area.x + 52.0,
            y,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.ink,
        );
        y += 17.0;
    }
    for line in paint
        .wrap(
            &format!(
                "{first_choice} of {} were its own first choice — a word it would have written \
                 anyway carries nothing from the writer",
                ranked.len()
            ),
            Weight::Regular,
            size::SMALL,
            area.w,
        )
        .iter()
        .take(3)
    {
        paint.say_at(
            area.x,
            y + 8.0,
            line,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        y += 15.0;
    }
}

/// Where the floor swamps the column, that is the finding.
///
/// **A run that separated nothing and one that worked drew the same screen.**
/// A floor of 87.9% means removing a sentence carrying no instruction moved
/// almost the whole answer, so no bar below it means anything — and the bars
/// were drawn first, in full colour, with the number that invalidates them in
/// grey underneath. A reader reads the bars (§3.15, A7, F147).
fn a_run_that_separated_nothing(paint: &mut Painter, at: (f32, f32), found: &Value) -> f32 {
    let ink = paint.ink;
    let floor = found
        .get("floor_parts_per_million")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    if floor < 500_000 {
        return at.1;
    }
    paint.say_at(
        at.0,
        at.1,
        "This run cannot separate your sentences.",
        Weight::Bold,
        size::BODY,
        ink.bad,
    );
    paint.say_at(
        at.0,
        at.1 + 19.0,
        &format!(
            "removing a sentence carrying no instruction moved {} of the answer — a bar near \
             that has told you nothing",
            as_percent(floor)
        ),
        Weight::Regular,
        size::SMALL,
        ink.warn,
    );
    paint.say_at(
        at.0,
        at.1 + 35.0,
        "usually the answer is long and open-ended: try one whose answer is short, or ask for \
         one part at a time",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    at.1 + 58.0
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
    for (what, value) in [
        ("at 512 tokens", held.speed_at_512()),
        ("at the largest window", held.speed_at_window()),
        ("start-up to first token", held.start_up()),
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
    // A prompt is a diagnostic about a prompt rather than about the model, so
    // it is reached from here and not from the column (B-072).
    let taking = Box::new(selected.right() + 18.0, area.y, 170.0, 34.0);
    if ui::button(paint, mouse, taking, "Prompt analysis", Kind::Ordinary) && !running {
        act = Some(Act::Go(Page::Prompt));
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

    // **The two pickers are dropdowns now, and were not before.** The model
    // one navigated to the Host page and the window one cycled to the next
    // power of two, while both wore a chevron. The list each opens is drawn
    // last, over the table below, because in immediate mode the last thing
    // painted is the thing on top.
    let mut menu: Option<(Picker, Box)> = None;
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
    if let Some((picker, at)) = menu
        && let Some(picked) = open_menu(paint, desk, mouse, picker, at)
    {
        act = Some(picked);
    }
    act
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
    paint.say_right(
        x + wide - 160.0,
        y,
        &clock(test.seconds),
        Weight::Regular,
        size::BODY,
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
            "one generation for the prompt, one for each sentence left out, one for each seed",
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
        &format!("what a prompt does to {named}, sentence by sentence"),
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );

    let mut act = None;
    let y = area.y + 52.0;
    let field = Box::new(area.x, y, (area.w - 150.0).min(680.0), 32.0);
    let _clicked = ui::field(
        paint,
        mouse,
        field,
        &desk.typed,
        "a prompt to analyse",
        true,
    );
    let (asked, _) = ui::fitted(
        paint,
        mouse,
        (field.right() + 10.0, y),
        "Analyse",
        Kind::Primary,
    );
    if asked && !desk.doing.busy() && desk.chosen.is_some() {
        act = Some(Act::ReportPrompt);
    }

    what_it_will_cost(paint, desk, (area.x, y + 38.0));

    let mut at = y + 62.0;
    let Some(found) = a_report_or_why_not(paint, desk, area, at) else {
        return act;
    };
    at = a_run_that_separated_nothing(paint, (area.x, at), found);
    let (after, pressed) = steering(
        paint,
        desk,
        mouse,
        Box::new(area.x, at, area.w, area.bottom() - at),
        found,
    );
    // Beside the bars rather than under them: the screen is wider than the
    // column and this window does not scroll.
    let column = 820.0_f32.min(area.w - 260.0).max(0.0);
    if area.w - column > 200.0 {
        not_expected(
            paint,
            Box::new(
                area.x + column + 20.0,
                at,
                area.w - column - 30.0,
                area.bottom() - at,
            ),
            found,
        );
    }
    at = after;
    act = pressed.or(act);
    act = copy_out(paint, desk, mouse, Box::new(area.x, at + 6.0, area.w, 30.0)).or(act);
    at += 40.0;
    how_many_answers(paint, (area.x, at + 12.0), found);
    the_answer(
        paint,
        desk,
        Box::new(area.x, at + 38.0, area.w, area.bottom() - at - 38.0),
        found,
    );
    act
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
    // The answer without whichever sentence is being asked about, or the
    // answer as written when none is.
    let chosen = desk.without.and_then(|at| {
        let clause = found.get("clauses").and_then(Value::as_list)?.get(at)?;
        let without = clause.get("without").and_then(Value::as_text)?;
        let text = clause.get("text").and_then(Value::as_text)?;
        Some((without.to_owned(), text.to_owned()))
    });
    let (said, title) = match &chosen {
        Some((without, text)) => (without.as_str(), format!("the answer without “{text}”")),
        None => (
            found.get("baseline").and_then(Value::as_text).unwrap_or(""),
            "the answer as written".to_owned(),
        ),
    };
    if said.trim().is_empty() || area.h < 40.0 {
        return;
    }
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
    // As many as fit, and no more: this window does not scroll, and the Copy
    // button beside it takes the whole report out for a reader who wants it.
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
                "{} more — Copy takes the whole report",
                count_of(lines.len().saturating_sub(fits), "line")
            ),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

/// The whole analysis as text, with a way to take it out of the window.
///
/// **MCF draws its own text, so nothing here is a thing a window manager can
/// select.** A report somebody wants to paste into a message has to be handed
/// over deliberately or it cannot leave at all. The panel shows what would be
/// copied so that pressing the button is not a guess (§3.15).
fn copy_out(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let (pressed, button) = ui::fitted(
        paint,
        mouse,
        (area.x, area.y),
        if desk.copied {
            "Copied"
        } else {
            "Copy analysis"
        },
        if desk.copied {
            Kind::Ordinary
        } else {
            Kind::Primary
        },
    );
    if pressed {
        act = Some(Act::CopyAnalysis);
    }
    paint.say_at(
        button.right() + 12.0,
        area.y + 8.0,
        if desk.copied {
            "the whole analysis is on the clipboard, ready to paste"
        } else {
            "puts the whole analysis on the clipboard: the figures, the floor and the answer"
        },
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    act
}

/// How much each sentence steered the answer, drawn as bars.
/// What pressing Analyse will cost, before it is pressed.
///
/// One generation per sentence plus a seed each, which on a large model on the
/// processor is minutes. The only place this was said was a line that appeared
/// once the wait had already started, which is a cost disclosed after it is
/// incurred (§3.8, §3.15).
fn what_it_will_cost(paint: &mut Painter, desk: &Desk, at: (f32, f32)) {
    if desk.doing.busy() {
        return;
    }
    let Some(sentences) = sentences_in(&desk.typed) else {
        return;
    };
    paint.say_at(
        at.0,
        at.1,
        &format!(
            "{} — {}: one for the prompt, one for each sentence left out, one for the control \
             sentence, and one for each of 2 further seeds",
            count_of(sentences, "sentence"),
            // Baseline, one per sentence, the floor's control, and SEEDS - 1
            // further seeds. This said `sentences + 3` and forgot the control,
            // so a three-sentence prompt was forecast at six generations and
            // cost seven (F147).
            count_of(sentences + 4, "generation")
        ),
        Weight::Regular,
        size::SMALL,
        paint.ink.faint,
    );
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

/// How many sentences a prompt has, for saying what a run will cost.
///
/// The same rule the measurement uses — a sentence ends at `.`, `?` or `!` —
/// so the forecast and the report cannot disagree about how many there are.
/// `None` for a prompt with nothing in it to take apart.
fn sentences_in(prompt: &str) -> Option<usize> {
    let held = prompt
        .split_inclusive(['.', '?', '!'])
        .filter(|part| !part.trim().is_empty())
        .count();
    (held > 0).then_some(held)
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

/// The sentence under the bars: what the quiet ones mean, what the number is
/// not, and what was left unmeasured.
fn what_the_bars_mean(
    paint: &mut Painter,
    at: (f32, f32),
    floor: i64,
    shown: usize,
    found: &Value,
) -> f32 {
    let ink = paint.ink;
    let area = Box::new(at.0, at.1, 820.0, 0.0);
    let clauses_len = shown;
    let mut y = at.1;
    // **What the quiet rows mean, and what the number is not.** The bars
    // were coloured against the floor and the floor was never shown, so a
    // reader had a distinction drawn for them with nothing to read it by.
    paint.say_at(
        area.x,
        y + 6.0,
        &format!(
            "the floor is {} — a sentence carrying no instruction, put in and taken out \
                 again. Rows at or under it are quiet.",
            as_percent(floor)
        ),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    paint.say_at(
        area.x,
        y + 22.0,
        "An ordering, not relevance: removing anything shifts what follows it.",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 42.0;
    // Sentences past the cap are not measured, and a list that quietly
    // shortened itself is the one thing a list must not do (A1, A4).
    let over = found
        .get("clauses_over_the_cap")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    if over > 0 {
        paint.say_at(
            area.x,
            y,
            &format!(
                "{} not measured: each one costs a generation, and the first {} are what \
                     MCF ablates",
                count_of(usize::try_from(over).unwrap_or(0), "further sentence"),
                clauses_len
            ),
            Weight::Regular,
            size::SMALL,
            ink.warn,
        );
        y += 20.0;
    }
    paint.say_at(
        area.x,
        y,
        "Press a sentence to see what the model wrote without it.",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 20.0;
    y
}

fn steering(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    found: &Value,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let floor = found
        .get("floor_parts_per_million")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let clauses = found.get("clauses").and_then(Value::as_list).unwrap_or(&[]);
    let wide = area.w.min(820.0);
    let mut y = area.y;
    let mut act = None;
    for (at, clause) in clauses.iter().take(8).enumerate() {
        let moved = clause
            .get("moved_parts_per_million")
            .and_then(Value::as_integer)
            .unwrap_or(0);
        let said = clause
            .get("text")
            .and_then(Value::as_text)
            .unwrap_or_default();
        // **The whole row is the control.** What a sentence did is answerable
        // from what the model wrote without it, and MCF has that answer for
        // every row; pressing one shows it, and pressing it again puts the
        // answer as written back (A19).
        let hit = Box::new(area.x - 6.0, y - 3.0, wide + 12.0, 22.0);
        let chosen = desk.without == Some(at);
        if chosen || mouse.over(hit) {
            paint.panel(hit, 6.0, ink.line, if chosen { 140 } else { 80 });
        }
        if mouse.clicked(hit) {
            act = Some(Act::ShowWithout(at));
        }
        let bar = Box::new(area.x, y + 4.0, 180.0, 10.0);
        paint.panel(bar, 5.0, ink.sunk, 255);
        #[allow(
            clippy::cast_precision_loss,
            reason = "a bar's width in points; a part of a point is not drawn"
        )]
        let filled = (moved as f32 / 1_000_000.0).clamp(0.0, 1.0) * bar.w;
        paint.panel(
            Box::new(bar.x, bar.y, filled.max(1.0), bar.h),
            5.0,
            // Above the floor is the accent; at or under it is quiet, because
            // what is at the floor steered nothing that is visible.
            if moved > floor { ink.accent } else { ink.line },
            255,
        );
        // **The figure beside the bar.** A bar shows an ordering and cannot be
        // read off: two sentences a few per cent apart draw the same, and a
        // reader comparing this run against the last one has nothing to
        // compare. The console has always printed the number (§3.15).
        paint.say_at(
            bar.right() + 12.0,
            y,
            &as_percent(moved),
            Weight::Bold,
            size::SMALL,
            if moved > floor { ink.ink } else { ink.quiet },
        );
        let text_at = bar.right() + 68.0;
        let shown = paint.elide(
            said,
            Weight::Regular,
            size::BODY,
            wide - (text_at - area.x) - 10.0,
        );
        paint.say_at(
            text_at,
            y,
            &shown,
            Weight::Regular,
            size::BODY,
            if moved > floor { ink.ink } else { ink.quiet },
        );
        y += 24.0;
    }
    if !clauses.is_empty() {
        y = what_the_bars_mean(paint, (area.x, y), floor, clauses.len(), found);
    }
    (y, act)
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
