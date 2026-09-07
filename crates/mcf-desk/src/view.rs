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
use crate::{Act, Card, Caret, Desk, Doing, Model, Page, Picker, Region, Splitter, windows};
use mcf_record::json::Value;
use mcf_serve::anatomy::SaidVocabulary;

/// The column down the left, in points.
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
    let side = desk.splits.side;
    let mut act = side_bar(paint, desk, mouse, height, side);

    let main = Box::new(
        side + PAD,
        PAD,
        (width - side - PAD * 2.0).max(10.0),
        (height - PAD * 2.0).max(10.0),
    );
    let went = match desk.page {
        Page::Monitor => monitor(paint, desk, mouse, main),
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
    // A scroll act is the page keeping its offset within its content, not
    // something the person did; it never beats a press on the menu (B-575).
    act = match (went, act) {
        (Some(Act::Scroll(..)), Some(pressed)) => Some(pressed),
        (went, act) => went.or(act),
    };
    // The boundary between the column and the page is dragged (B-490).
    if let Some(to) = ui::splitter(
        paint,
        mouse,
        Box::new(side - 4.0, 0.0, 8.0, height),
        true,
        desk.grabbed == Some(Splitter::Side),
    ) {
        act = Some(Act::Split(Splitter::Side, whole(to)));
    }
    paint.end();
    act
}

/// A position in whole points, for an act to carry.
fn whole(at: f32) -> i32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a point on the screen, far inside i32"
    )]
    let rounded = at.round() as i32;
    rounded
}

/// A region drawn scrolled: its content from `offset` points up, clipped
/// to the region, with a bar at its right edge where the content is taller
/// than the region, moved by the wheel over it or the thumb (B-490).
fn scrolled(
    paint: &mut Painter,
    mouse: &Mouse,
    desk: &Desk,
    region: Region,
    area: Box,
    draw: impl FnOnce(&mut Painter, &Mouse, Box) -> Option<Act>,
) -> Option<Act> {
    let offset = desk.scrolled(region);
    let inner = Box::new(
        area.x,
        area.y - offset,
        (area.w - ui::BAR).max(10.0),
        area.h + offset,
    );
    let seen = mouse.within(area);
    paint.clip(area);
    paint.mark_at(inner.y);
    let act = draw(paint, &seen, inner);
    let content = (paint.lowest() - inner.y).max(0.0);
    paint.unclip();
    let moved = ui::scroll_region(paint, mouse, area, offset, content);
    act.or(moved.map(|to| Act::Scroll(region, whole(to))))
}

/// The column down the left: the four places and Exit, and under them
/// what MCF is doing right now, on every page (D49).
fn side_bar(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    height: f32,
    side: f32,
) -> Option<Act> {
    let ink = paint.ink;
    paint.rect(Box::new(0.0, 0.0, side, height), ink.sunk);
    paint.rule((side, 0.0), (side, height), ink.line, 255);
    paint.say_at(18.0, 16.0, "MCF", Weight::Bold, size::HEAD, ink.ink);

    let mut act = None;
    for (page, label) in Page::MENU {
        if *page == Page::Exit {
            continue;
        }
        let area = menu_box(*page, height, side);
        if ui::nav(paint, mouse, area, label, desk.page.section() == *page) {
            act = Some(Act::Go(*page));
        }
    }
    // What MCF is doing, where a person's eye rests between pages: the job
    // and its clock, or nothing.
    let said = desk.state_word();
    let lines = paint.wrap(&said, Weight::Regular, size::SMALL, side - 28.0);
    #[allow(clippy::cast_precision_loss, reason = "at most three lines")]
    let mut at = height - 92.0 - 16.0 * lines.len().min(3) as f32;
    for line in lines.iter().take(3) {
        paint.say_at(
            14.0,
            at,
            line,
            Weight::Regular,
            size::SMALL,
            if desk.doing.busy() {
                ink.accent
            } else {
                ink.faint
            },
        );
        at += 16.0;
    }
    let exit = menu_box(Page::Exit, height, side);
    if ui::nav(paint, mouse, exit, "Exit", desk.page == Page::Exit) {
        act = Some(Act::Go(Page::Exit));
    }
    act
}

/// Where one entry of the menu column sits, in a window of this height
/// with a side bar this wide: the entries from the top in `Page::MENU`'s
/// order, Exit at the bottom. One place for the layout, so that a test
/// pressing an entry presses where the window draws it rather than
/// sweeping the screen for it.
#[must_use]
pub fn menu_box(page: Page, height: f32, side: f32) -> Box {
    if page == Page::Exit {
        return Box::new(10.0, height - 46.0, side - 20.0, 32.0);
    }
    #[allow(clippy::cast_precision_loss, reason = "a handful of entries")]
    let at = Page::MENU
        .iter()
        .filter(|(held, _)| *held != Page::Exit)
        .position(|(held, _)| *held == page)
        .unwrap_or(0) as f32;
    Box::new(10.0, 56.0 + 36.0 * at, side - 20.0, 32.0)
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

/// The width a table's columns were laid out for; a wider table spreads
/// them in proportion, so a wide window is used and not left empty at
/// the right (B-510). A narrower one keeps them where they are, since
/// the figures need their room, and the page scrolls.
const TABLE_DESIGNED_FOR: f32 = 700.0;

/// Where a column's edge falls in a table of this width.
fn column_edge(area: Box, at: f32) -> f32 {
    area.x + at * (area.w / TABLE_DESIGNED_FOR).max(1.0)
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
                column_edge(area, column.at) - wide,
                area.y,
                column.head,
                ink.faint,
            );
        } else {
            spaced(
                paint,
                column_edge(area, column.at),
                area.y,
                column.head,
                ink.faint,
            );
        }
    }
    let under = area.y + 17.0;
    paint.rule((area.x, under), (area.right(), under), ink.line, 255);
    under + 10.0
}

/// One row of a table.
fn row(paint: &mut Painter, area: Box, y: f32, first: &str, cells: &[(&Column, String)]) {
    let ink = paint.ink;
    let room = cells.first().map_or(area.w, |(column, _)| {
        column_edge(area, column.at) - area.x - 14.0
    });
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
        let at = column_edge(area, column.at);
        if column.right {
            paint.say_right(at, y, value, Weight::Bold, size::BODY, colour);
        } else {
            paint.say_at(at, y, value, Weight::Bold, size::BODY, colour);
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
#[must_use]
pub fn gigabytes(bytes: u64) -> String {
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
            "KV cache {} · total {} with weights",
            gigabytes(cache),
            gigabytes(total)
        ),
        None => format!("KV cache {}", gigabytes(cache)),
    })
}

/// What the engine said it takes, on one line.
fn takes_line(hosting: &crate::Hosted) -> String {
    let projector = hosting.projector.as_ref().map_or_else(
        || "no projector".to_owned(),
        |name| format!("projector {name}"),
    );
    hosting.takes.as_ref().map_or_else(
        || format!("Input: not reported   ·   {projector}"),
        |takes| {
            format!(
                "Input: {}   ·   Template: {}   ·   Thinking: {}",
                takes.media(),
                takes.template(),
                takes.thinking_said()
            )
        },
    )
}

fn monitor(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    scrolled(
        paint,
        mouse,
        desk,
        Region::Monitor,
        area,
        |paint, mouse, inner| monitor_body(paint, desk, mouse, inner),
    )
}

/// The System page's content, from the top of its region.
fn monitor_body(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let wide = area.w;

    // The machine's own figures, then its engines; what is held is on
    // Running (D49).
    spaced(paint, area.x, area.y, "system", ink.faint);
    let mut y = area.y + 20.0;
    y = processors_table(paint, Box::new(area.x, y, wide, 0.0), desk);
    y = memory_table(paint, Box::new(area.x, y + 14.0, wide, 0.0), desk);
    y = storage_table(
        paint,
        Box::new(
            area.x,
            y + 14.0,
            wide,
            130.0_f32.min(area.bottom() - y - 260.0),
        ),
        desk,
    );
    // The engines: what MCF has built here and what it can build, each a
    // card with its build button, on the page about the machine.
    y += 18.0;
    spaced(paint, area.x, y, "engines", ink.faint);
    y += 20.0;
    let mut act = None;
    for component in &desk.components {
        let tall = 86.0;
        if y + tall > area.bottom() - 70.0 {
            break;
        }
        act = component_card(
            paint,
            desk,
            mouse,
            component,
            Box::new(area.x, y, wide, tall),
        )
        .or(act);
        y += tall + 10.0;
    }

    // The failures: what went wrong, classified, newest first, each with
    // the fields the taxonomy gives it, on the page about the machine
    // (B-074).
    y += 18.0;
    let heading = match desk.faults_in_record {
        0 => "failures — none in the record".to_owned(),
        n if n <= desk.faults.len() => format!("failures — {n} in the record, newest first"),
        n => format!(
            "failures — the newest {} of {n} in the record",
            desk.faults.len()
        ),
    };
    spaced(paint, area.x, y, &heading, ink.faint);
    y += 20.0;
    for fault in &desk.faults {
        let lines = crate::fault_lines(fault);
        let tall = 22.0 + 16.0 * lines.len() as f32;
        if y + tall > area.bottom() - 70.0 {
            break;
        }
        fault_card(paint, fault, &lines, Box::new(area.x, y, wide, tall));
        y += tall + 8.0;
    }

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
    act
}

/// One classified failure: when and what, in bold, then every line the
/// taxonomy gives it, wrapped rather than cut (B-074).
fn fault_card(paint: &mut Painter, fault: &crate::Fault, lines: &[String], card: Box) {
    let ink = paint.ink;
    ui::card(paint, card, false);
    let mut at = card.y + 12.0;
    let mut first = true;
    for line in lines {
        let (weight, colour, size) = if first {
            (Weight::Bold, ink.ink, size::BODY)
        } else {
            (Weight::Regular, ink.quiet, size::SMALL)
        };
        let said = if first && !fault.at.is_empty() {
            format!("{}  {line}", fault.at)
        } else {
            line.clone()
        };
        for wrapped in paint
            .wrap(&said, weight, size, card.w - 28.0)
            .iter()
            .take(2)
        {
            paint.say_at(card.x + 14.0, at, wrapped, weight, size, colour);
            at += 16.0;
        }
        first = false;
    }
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
    // The list gives way before the page does in a narrow window (B-510).
    let list = desk.splits.list.min((area.w - 300.0).max(150.0));
    let right = area.x + list + 40.0;
    let mut act = None;
    let actions_at = area.bottom() - 200.0;
    // The boundary between the library and the page is dragged (B-490).
    if let Some(to) = ui::splitter(
        paint,
        mouse,
        Box::new(area.x + list + 12.0, area.y, 16.0, area.h),
        true,
        desk.grabbed == Some(Splitter::List),
    ) {
        act = Some(Act::Split(Splitter::List, whole(to)));
    }

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

    let pane = Box::new(right, area.y, area.right() - right, area.h);
    // A filter's list drops over the library, last, so it is on top.
    let boxes = filter_boxes(Box::new(area.x, area.y, list, actions_at - area.y));
    let filter_menu = match desk.open {
        Some(Picker::Architecture) if desk.filters.open => Some((Picker::Architecture, boxes[0])),
        Some(Picker::Fits) if desk.filters.open => Some((Picker::Fits, boxes[1])),
        Some(Picker::Size) if desk.filters.open => Some((Picker::Size, boxes[2])),
        _ => None,
    };
    let over = if desk.hub_chosen.is_some() {
        scrolled(
            paint,
            mouse,
            desk,
            Region::Hub,
            pane,
            |paint, mouse, inner| hub_page(paint, desk, mouse, inner),
        )
    } else if desk.pending.is_some() {
        scrolled(
            paint,
            mouse,
            desk,
            Region::Hub,
            pane,
            |paint, mouse, inner| pending_page(paint, desk, mouse, inner),
        )
    } else {
        None
    };
    if let Some((picker, at)) = filter_menu
        && let Some(picked) = open_menu(paint, desk, mouse, picker, at)
    {
        return Some(picked);
    }
    if desk.hub_chosen.is_some() || desk.pending.is_some() {
        return over.or(act);
    }
    let Some(held) = desk.chosen.and_then(|at| desk.models.get(at)) else {
        paint.say_at(
            right,
            area.y,
            "Choose a model on the left, or type to search.",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return act;
    };
    model_page(paint, desk, mouse, pane, held).or(act)
}

/// The page of a quantization picked that is not here: what the hub says
/// of it, the quantization row to pick another, and a way to get it (D51,
/// B-486; the button that downloads and runs is B-487).
fn pending_page(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let pending = desk.pending.as_ref()?;
    let mut act = None;
    let name = paint.elide(
        pending.file.trim_end_matches(".gguf"),
        Weight::Bold,
        size::HEAD,
        area.w,
    );
    paint.say_at(area.x, area.y, &name, Weight::Bold, size::HEAD, ink.ink);
    paint.say_at(
        area.x,
        area.y + 30.0,
        &format!("On the hub — not downloaded · {}", pending.repository),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let mut y = area.y + 70.0;
    let column = area.x + label_column_of(area.w, 190.0);
    paint.say_at(
        area.x,
        y,
        "Quantization",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    let now = desk
        .quantizations()
        .get(desk.quantization_at().unwrap_or(usize::MAX))
        .map_or_else(|| pending.file.clone(), quant_label);
    let box_of = Box::new(column, y - 6.0, (area.w - 190.0).max(120.0), 28.0);
    let open = desk.open == Some(Picker::Quantization);
    if ui::picker(paint, mouse, box_of, &now, open) {
        act = Some(Act::Open(Picker::Quantization));
    }
    y += 34.0;
    for (name, value) in [
        (
            "Size",
            pending.bytes.map_or_else(|| UNKNOWN.to_owned(), gigabytes),
        ),
        (
            "Runs here",
            match pending.fits {
                Some(true) => "yes, by the shape the repository states".to_owned(),
                Some(false) => "no, by the shape the repository states".to_owned(),
                None => "MCF could not say".to_owned(),
            },
        ),
    ] {
        paint.say_at(area.x, y, name, Weight::Regular, size::BODY, ink.quiet);
        paint.say_at(column, y, &value, Weight::Bold, size::BODY, ink.ink);
        y += 30.0;
    }
    y += 10.0;
    if let Some(pressed) = pending_actions(
        paint,
        desk,
        mouse,
        Box::new(area.x, y, area.w, 0.0),
        pending,
    ) {
        act = Some(pressed);
    }
    if open && let Some(picked) = configure_menu(paint, desk, mouse, Picker::Quantization, box_of) {
        act = Some(picked);
    }
    act
}

/// Under a subject not here: the download going, or the buttons — the one
/// that downloads and starts the server, and Download alone (B-487).
fn pending_actions(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    pending: &crate::Pending,
) -> Option<Act> {
    let ink = paint.ink;
    let y = area.y;
    let mut act = None;
    if let Doing::Downloading(job) = &desk.doing {
        paint.say_at(area.x, y, &job.what, Weight::Bold, size::BODY, ink.ink);
        ui::progress(
            paint,
            Box::new(area.x, y + 24.0, area.w, 8.0),
            crate::job::fraction(job),
        );
        paint.say_at(
            area.x,
            y + 46.0,
            &downloading_line(job),
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
    } else {
        // The download goes first and the server follows on the file
        // once it is here (B-487); Download alone is the second button.
        let (start, drawn) = ui::fitted(
            paint,
            mouse,
            (area.x, y),
            "Download and start server",
            Kind::Primary,
        );
        if start && !desk.doing.busy() {
            act = Some(Act::DownloadThen(std::boxed::Box::new(Act::HostIt)));
        }
        let (get, _) = ui::fitted(
            paint,
            mouse,
            (drawn.right() + 12.0, y),
            "Download",
            Kind::Ordinary,
        );
        if get && !desk.doing.busy() {
            act = Some(Act::Download {
                reference: pending.repository.clone(),
                file: pending.file.clone(),
            });
        }
    }
    act
}

/// A hub repository's page: its files with their sizes and whether each
/// would run here, each with a way to get it; the look-up's progress or
/// refusal before that, and a download's progress while one goes (D51,
/// B-485).
fn hub_page(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let id = desk
        .hub
        .as_ref()
        .zip(desk.hub_chosen)
        .and_then(|(hub, at)| hub.repositories.get(at))
        .map_or("a repository", |found| found.id.as_str());
    paint.say_at(
        area.x,
        area.y,
        &format!("On the hub — not downloaded · {id}"),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let y = area.y + 26.0;
    match &desk.doing {
        Doing::Listing(job) if !job.finished => {
            paint.say_at(area.x, y, &job.what, Weight::Regular, size::BODY, ink.quiet);
            return None;
        }
        Doing::Downloading(job) => {
            paint.say_at(area.x, y, &job.what, Weight::Bold, size::BODY, ink.ink);
            ui::progress(
                paint,
                Box::new(area.x, y + 24.0, area.w, 8.0),
                crate::job::fraction(job),
            );
            paint.say_at(
                area.x,
                y + 46.0,
                &downloading_line(job),
                Weight::Regular,
                size::SMALL,
                ink.quiet,
            );
            return None;
        }
        _ => {}
    }
    let job = desk.doing.job()?;
    // The files come before any complaint about the stream: a listing that
    // arrived whole and then closed is the answer, not a death (F194).
    let files = job
        .conclusion()
        .or_else(|| job.latest())
        .filter(|found| found.get("files").is_some());
    match (files, &job.refused) {
        (Some(found), _) => published(
            paint,
            mouse,
            Box::new(area.x, y, area.w, area.bottom() - y),
            found,
        ),
        (None, Some(why)) => {
            let mut at = y;
            for line in paint
                .wrap(why, Weight::Regular, size::BODY, area.w)
                .iter()
                .take(3)
            {
                paint.say_at(area.x, at, line, Weight::Regular, size::BODY, ink.bad);
                at += 20.0;
            }
            None
        }
        (None, None) => None,
    }
}

/// Why there are no settings, and what Host will do about it where it can.
fn no_settings(paint: &mut Painter, desk: &Desk, area: Box, why: &str) {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "settings", ink.faint);
    let lines = paint.wrap(why, Weight::Regular, size::SMALL, area.w);
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
            .wrap(&said, Weight::Regular, size::SMALL, area.w)
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
/// The chosen model's own page: its name, three tabs, and the tab that is
/// open. Configure is every setting a hold takes with a control on each,
/// Statistics everything measured or read about it, Contents what the file
/// holds (D49).
fn model_page(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    held: &Model,
) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let name = paint.elide(&held.name, Weight::Bold, size::HEAD, area.w);
    paint.say_at(area.x, area.y, &name, Weight::Bold, size::HEAD, ink.ink);
    let mut x = area.x;
    let y = area.y + 34.0;
    for tab in crate::Tab::ALL {
        let wide = paint.measure(tab.label(), Weight::Bold, size::BODY) + 26.0;
        if ui::nav(
            paint,
            mouse,
            Box::new(x, y, wide, 30.0),
            tab.label(),
            desk.tab == tab,
        ) {
            act = Some(Act::Tab(tab));
        }
        x += wide + 6.0;
    }
    paint.rule((area.x, y + 38.0), (area.right(), y + 38.0), ink.line, 255);
    let below = Box::new(
        area.x,
        y + 52.0,
        area.w,
        (area.bottom() - y - 52.0).max(10.0),
    );
    let drawn = scrolled(
        paint,
        mouse,
        desk,
        Region::Page,
        below,
        |paint, mouse, inner| match desk.tab {
            crate::Tab::Configure => configure_tab(paint, desk, mouse, inner, held),
            crate::Tab::Statistics => {
                statistics_tab(paint, inner, held);
                None
            }
            crate::Tab::Contents => contents_tab(paint, desk, mouse, inner),
        },
    );
    drawn.or(act)
}

/// Everything measured or read about the model, in one place: the file's
/// figures, the ladder and what was read off it, the cross-check, the
/// prompt report, and what the probes applied and found (D49). Nothing here
/// is pressed: the actions are in the left column.
fn statistics_tab(paint: &mut Painter, area: Box, held: &Model) {
    let ink = paint.ink;
    // The detail block draws the throughput table and the chart itself.
    // A narrow pane stacks the column under the figures; the page scrolls
    // (B-490).
    let stacked = area.w < 700.0;
    let left = Box::new(
        area.x,
        area.y,
        if stacked {
            area.w
        } else {
            (area.w * 0.45).max(300.0)
        },
        area.h,
    );
    let after = detail(paint, left, held);
    // The right column: what was read off the ladder, then the rest, each
    // a short heading and the daemon's own sentences under it.
    let right = if stacked {
        Box::new(area.x, after + 24.0, area.w, 4000.0)
    } else {
        Box::new(
            area.x + left.w + 40.0,
            area.y,
            (area.w - left.w - 40.0).max(240.0),
            4000.0,
        )
    };
    let mut y = right.y;
    let section = |paint: &mut Painter, y: &mut f32, head: &str, lines: &[String], colour: Rgb| {
        spaced(paint, right.x, *y, head, ink.faint);
        *y += 18.0;
        for line in lines.iter().take(3) {
            for shown in paint
                .wrap(line, Weight::Regular, size::SMALL, right.w)
                .iter()
                .take(3)
            {
                paint.say_at(right.x, *y, shown, Weight::Regular, size::SMALL, colour);
                *y += 15.0;
            }
        }
        *y += 10.0;
    };
    for (head, lines, known) in statistic_sections(held) {
        section(
            paint,
            &mut y,
            head,
            &lines,
            if known { ink.quiet } else { ink.faint },
        );
    }
    // No button and no *Last served* here: the left column's actions say
    // both for the same model, and the column had grown past the window's
    // foot repeating them (F187).
}

/// Every section of the Statistics tab's right column, in order: its
/// heading, its lines, and whether there is a figure there or only where
/// one would come from (A7).
fn statistic_sections(held: &Model) -> Vec<(&'static str, Vec<String>, bool)> {
    let not_yet = |_what: &str| vec!["Not run — see Diagnostics".to_owned()];
    let mut sections = Vec::new();
    match held.measured_body.as_ref() {
        Some(body) => {
            for (head, said) in timing_said(body) {
                sections.push((head, said, true));
            }
        }
        None => sections.push(("Measured on", not_yet("timed"), false)),
    }
    if held.cross_checked.is_empty() {
        sections.push(("Cross-check", not_yet("cross-checked"), false));
    } else {
        sections.push(("Cross-check", held.cross_checked.clone(), true));
    }
    sections.push((
        "Prompt analysis",
        vec![if held.prompt_reported {
            "Report taken — Diagnostics, Prompt analysis".to_owned()
        } else {
            "Not run — see Diagnostics, Prompt analysis".to_owned()
        }],
        held.prompt_reported,
    ));
    if held.probed.is_empty() {
        sections.push(("Capabilities", not_yet("probed"), false));
    } else {
        sections.push(("Capabilities", probed_said(&held.probed), true));
    }
    sections.push((
        "Probed settings",
        vec![
            held.applied_addressing.clone().map_or_else(
                || "Chat template: not set".to_owned(),
                |said| format!("Chat template: {said}"),
            ),
            held.applied_budget.clone().map_or_else(
                || "Token budget: not set".to_owned(),
                |said| format!("Token budget: {said}"),
            ),
        ],
        held.applied_addressing.is_some() || held.applied_budget.is_some(),
    ));
    sections
}

/// What the probes found, one line a probe: its name and the first line
/// that says what it observed, leaving out what it asks and decides and
/// the conditions it ran under, which are on the record.
fn probed_said(probed: &[crate::Finding]) -> Vec<String> {
    probed
        .iter()
        .map(|found| {
            let (name, lines) = (&found.name, &found.lines);
            let finding = lines.iter().map(|line| line.trim()).find(|line| {
                !line.is_empty()
                    && line != name
                    && !line.starts_with("asks ")
                    && !line.starts_with("decides ")
                    && !line.starts_with("under:")
            });
            match finding {
                Some(finding) => format!("{name}: {finding}"),
                None => format!("{name}: ran"),
            }
        })
        .collect()
}

/// What the last measurement said, section by section: where it was taken,
/// then what was read off the ladder, in the daemon's own sentences.
fn timing_said(body: &Value) -> Vec<(&'static str, Vec<String>)> {
    let conditions = body.get("conditions");
    let on = conditions
        .and_then(|held| held.get("engine_ran"))
        .and_then(Value::as_text)
        .map_or_else(String::new, |engine| {
            format!(
                "measured on {engine}{}",
                conditions.map_or_else(String::new, mcf_tui::screens::diagnostics::on_device)
            )
        });
    vec![
        ("Measured on", vec![on]),
        (
            "Context scaling",
            mcf_serve::ladder::fall_off_said(body.get("fall_off")),
        ),
        (
            "Prefill",
            mcf_serve::ladder::prompt_reading_said(body.get("prompt_reading")),
        ),
        (
            "Time to first token",
            mcf_serve::ladder::first_token_said(body.get("first_token")),
        ),
        (
            "KV cache memory",
            mcf_serve::ladder::memory_said(body.get("memory")),
        ),
    ]
}

/// What the file holds, read and not measured: the tensor directory or the
/// vocabulary, one at a time, with a switch between them (D49).
fn contents_tab(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let mut x = area.x;
    for (label, page) in [("Tensors", Page::Anatomy), ("Vocabulary", Page::Vocabulary)] {
        let wide = paint.measure(label, Weight::Bold, size::BODY) + 26.0;
        if ui::nav(
            paint,
            mouse,
            Box::new(x, area.y, wide, 28.0),
            label,
            desk.contents == page,
        ) {
            act = Some(Act::Contents(page));
        }
        x += wide + 6.0;
    }
    let subtitle = if desk.contents == Page::Anatomy {
        "counted from the file's tensor directory and set against its header — read, not measured"
    } else {
        "counted from the header's token list — nothing tokenised, nothing rated"
    };
    let shown = paint.elide(
        subtitle,
        Weight::Regular,
        size::SMALL,
        area.w - x + area.x - 12.0,
    );
    paint.say_at(
        x + 8.0,
        area.y + 7.0,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let top = area.y + 44.0;
    let Some(said) = &desk.anatomy else {
        not_counted(paint, desk, area, top);
        return act;
    };
    let body = Box::new(area.x, top, area.w, (area.bottom() - top).max(10.0));
    if desk.contents == Page::Anatomy {
        anatomy_body(paint, body, said);
    } else {
        vocabulary_body(paint, body, said);
    }
    act
}

/// One row of the Configure tab: the setting's name at the left, its
/// control at the column, and — where the mouse is over it — what it does,
/// kept for the foot of the tab.
struct Row {
    /// The setting's name, as `Hosting::listed` names it.
    name: &'static str,
    /// What the row is for, shown at the foot while the mouse is over it.
    because: &'static str,
}

/// Every setting a hold takes, each with the control its value wants, then
/// what the hold will take of what is free, and Host last (D49, §3.15).
#[expect(
    clippy::too_many_lines,
    reason = "one row per setting a hold takes, in order"
)]
fn configure_tab(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    held: &Model,
) -> Option<Act> {
    let ink = paint.ink;
    let mut y = area.y;
    // A model that will not run says why first, in the refusal colour, and
    // nothing below pretends it can be configured into running (A2).
    if let Some(why) = &held.refused {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w)
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
            y += 20.0;
        }
        y += 8.0;
    }
    let (Some(settings), Some(recommended)) = (desk.settings.as_ref(), desk.recommended.as_ref())
    else {
        // Nothing to configure yet: MCF has not said what this model would
        // run under, or said why it cannot. The reason is here, and Host is
        // still here — pressing it says why in words rather than doing
        // nothing (§3.15).
        if let Some(why) = &desk.no_settings {
            no_settings(paint, desk, Box::new(area.x, y, area.w, 0.0), why);
            y += 90.0;
        } else if held.refused.is_none() {
            paint.say_at(
                area.x,
                y,
                "MCF has not said what this model would run under",
                Weight::Regular,
                size::BODY,
                ink.quiet,
            );
            y += 30.0;
        }
        let (host, _) = ui::fitted(paint, mouse, (area.x, y + 6.0), "Start server", Kind::Quiet);
        return (host && !desk.doing.busy()).then_some(Act::HostIt);
    };
    let listed = settings.listed(recommended);
    let because_of = |name: &str| {
        listed
            .iter()
            .find(|setting| setting.name == name)
            .map_or("", |setting| setting.because)
    };
    let recommended_for = |name: &str| {
        listed
            .iter()
            .find(|setting| setting.name == name)
            .filter(|setting| setting.value != setting.recommended)
            .map(|setting| setting.recommended.clone())
    };
    let column = area.x + label_column_of(area.w, 190.0);
    let control = (area.w - 190.0).max(120.0);
    let mut act = None;
    let mut hovered: Option<&'static str> = None;
    let mut menu: Option<(Picker, Box)> = None;

    let label = |paint: &mut Painter, y: f32, row: Row, hovered: &mut Option<&'static str>| {
        let hit = Box::new(area.x, y - 4.0, area.w, 26.0);
        if mouse.over(hit) {
            *hovered = Some(row.because);
        }
        paint.say_at(area.x, y, row.name, Weight::Regular, size::BODY, ink.quiet);
    };
    let recommends = |paint: &mut Painter, y: &mut f32, name: &str| {
        if let Some(was) = recommended_for(name) {
            paint.say_at(
                column,
                *y,
                &format!("Recommended: {was}"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            *y += 20.0;
        }
    };
    let typed_in = |paint: &mut Painter,
                    mouse: &Mouse,
                    y: f32,
                    field: crate::Field,
                    now: String,
                    placeholder: &str|
     -> (bool, String) {
        let focused = desk
            .editing
            .as_ref()
            .is_some_and(|(editing, _)| *editing == field);
        let text = if focused {
            desk.being_typed().to_owned()
        } else {
            now
        };
        let pressed = ui::field(
            paint,
            mouse,
            Box::new(column, y - 6.0, control, 28.0),
            &text,
            placeholder,
            focused,
        );
        (pressed, text)
    };
    let switch = |paint: &mut Painter, mouse: &Mouse, y: f32, on: bool| -> bool {
        let square = Box::new(column, y - 3.0, 20.0, 20.0);
        paint.edge(
            square,
            5.0,
            if on { ink.accent } else { ink.line },
            ink.card,
        );
        if on {
            ui::tick(
                paint,
                Box::new(square.x + 4.0, square.y + 4.0, 12.0, 12.0),
                ink.accent,
            );
        }
        paint.say_at(
            column + 30.0,
            y,
            if on { "On" } else { "Off" },
            Weight::Bold,
            size::BODY,
            ink.ink,
        );
        mouse.clicked(Box::new(column, y - 6.0, 90.0, 28.0))
    };

    // Where it goes.
    // Which quantization of the repository: the files here, then what the
    // hub publishes, once asked (D51, B-486).
    if desk.subject_repository().is_some() {
        label(
            paint,
            y,
            Row {
                name: "Quantization",
                because: "which file of the repository the page is about: the ones here, \
                          and the ones the hub publishes that are not",
            },
            &mut hovered,
        );
        let now = desk
            .quantizations()
            .get(desk.quantization_at().unwrap_or(usize::MAX))
            .map_or_else(|| held.file.clone(), quant_label);
        let box_of = Box::new(column, y - 6.0, control, 28.0);
        let open = desk.open == Some(Picker::Quantization);
        if ui::picker(paint, mouse, box_of, &now, open) {
            act = Some(Act::Open(Picker::Quantization));
        }
        if open {
            menu = Some((Picker::Quantization, box_of));
        }
        y += 34.0;
    }
    label(
        paint,
        y,
        Row {
            name: "Device",
            because: because_of("put it on"),
        },
        &mut hovered,
    );
    let placed = desk
        .placed_at()
        .and_then(|at| desk.placements.get(at))
        .map_or_else(
            || format!("{} on {}", settings.engine, settings.device),
            placement_label,
        );
    let box_of = Box::new(column, y - 6.0, control, 28.0);
    let open = desk.open == Some(Picker::Placement);
    if ui::picker(paint, mouse, box_of, &placed, open) {
        act = Some(Act::Open(Picker::Placement));
    }
    if open {
        menu = Some((Picker::Placement, box_of));
    }
    y += 34.0;
    recommends(paint, &mut y, "put it on");

    // The window, with what it reserves as it is typed.
    label(
        paint,
        y,
        Row {
            name: "Context length",
            because: because_of("context window"),
        },
        &mut hovered,
    );
    let (pressed, text) = typed_in(
        paint,
        mouse,
        y,
        crate::Field::Context,
        settings.context.to_string(),
        "tokens",
    );
    if pressed {
        act = Some(Act::Edit(crate::Field::Context));
    }
    y += 34.0;
    let typed_window = text.trim().replace([',', '_'], "").parse::<u64>().ok();
    if let Some(said) = reserve_line(held, typed_window.unwrap_or(settings.context)) {
        paint.say_at(column, y, &said, Weight::Regular, size::SMALL, ink.faint);
        y += 18.0;
    }
    recommends(paint, &mut y, "context window");

    // Numbers.
    for (name, field, now) in [
        (
            "Threads",
            crate::Field::Threads,
            settings.threads.to_string(),
        ),
        (
            "Batch size",
            crate::Field::Batch,
            settings.batch.to_string(),
        ),
        ("Port", crate::Field::Port, settings.port.to_string()),
    ] {
        label(
            paint,
            y,
            Row {
                name,
                because: because_of(name),
            },
            &mut hovered,
        );
        let (pressed, _) = typed_in(paint, mouse, y, field, now, "");
        if pressed {
            act = Some(Act::Edit(field));
        }
        y += 34.0;
        recommends(paint, &mut y, name);
    }

    // Switches.
    for (name, which, on) in [
        (
            "Flash attention",
            crate::Switch::FlashAttention,
            settings.flash_attention,
        ),
        (
            "Memory lock",
            crate::Switch::KeepResident,
            settings.keep_resident,
        ),
        (
            "Reachable from the network",
            crate::Switch::Open,
            settings.open,
        ),
    ] {
        label(
            paint,
            y,
            Row {
                name,
                because: because_of(name),
            },
            &mut hovered,
        );
        if switch(paint, mouse, y, on) {
            act = Some(Act::Switch(which));
        }
        y += 30.0;
        recommends(paint, &mut y, name);
    }

    // The key.
    label(
        paint,
        y,
        Row {
            name: "API key",
            because: because_of("API key"),
        },
        &mut hovered,
    );
    let (pressed, _) = typed_in(
        paint,
        mouse,
        y,
        crate::Field::ApiKey,
        settings.api_key.clone().unwrap_or_default(),
        "none (open on localhost)",
    );
    if pressed {
        act = Some(Act::Edit(crate::Field::ApiKey));
    }
    y += 34.0;

    // What the file declares and whether to start it.
    label(
        paint,
        y,
        Row {
            name: "Draft head",
            because: "the speculative draft head the file carries, started or left in it",
        },
        &mut hovered,
    );
    match desk
        .declared
        .as_ref()
        .and_then(|declared| declared.draft_head)
    {
        Some(layers) => {
            if switch(paint, mouse, y, settings.started.draft_head) {
                act = Some(Act::Switch(crate::Switch::DraftHead));
            }
            paint.say_at(
                column + 90.0,
                y,
                &format!("{layers}-layer draft head in file"),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        }
        None => paint.say_at(
            column,
            y,
            "None in file",
            Weight::Regular,
            size::BODY,
            ink.faint,
        ),
    }
    y += 30.0;
    label(
        paint,
        y,
        Row {
            name: "RoPE scaling",
            because: "how the context is stretched past what the model was trained for: default, off, linear or YaRN, by a factor",
        },
        &mut hovered,
    );
    let rope_at = crate::ROPE_CHOICES
        .iter()
        .position(|choice| *choice == settings.started.rope)
        .unwrap_or(0);
    let rope_box = Box::new(column, y - 6.0, 150.0, 28.0);
    let open = desk.open == Some(Picker::Rope);
    if ui::picker(paint, mouse, rope_box, rope_label(rope_at), open) {
        act = Some(Act::Open(Picker::Rope));
    }
    if open {
        menu = Some((Picker::Rope, rope_box));
    }
    if settings
        .started
        .rope
        .is_some_and(|rope| rope != mcf_serve::declared::Scaling::Off)
    {
        let focused = desk
            .editing
            .as_ref()
            .is_some_and(|(editing, _)| *editing == crate::Field::RopeFactor);
        let text = if focused {
            desk.being_typed().to_owned()
        } else {
            settings
                .started
                .factor
                .map_or_else(String::new, |factor| factor.to_string())
        };
        if ui::field(
            paint,
            mouse,
            Box::new(column + 160.0, y - 6.0, 100.0, 28.0),
            &text,
            "factor",
            focused,
        ) {
            act = Some(Act::Edit(crate::Field::RopeFactor));
        }
    }
    y += 34.0;

    // The projector.
    label(
        paint,
        y,
        Row {
            name: "Vision projector",
            because: because_of("projector"),
        },
        &mut hovered,
    );
    match recommended.projector.as_deref() {
        Some(path) => {
            if switch(paint, mouse, y, settings.projector.is_some()) {
                act = Some(Act::Switch(crate::Switch::Projector));
            }
            let named = path.rsplit('/').next().unwrap_or(path);
            let shown = paint.elide(
                &format!("{named} (off = text only)"),
                Weight::Regular,
                size::SMALL,
                area.right() - column - 96.0,
            );
            paint.say_at(
                column + 90.0,
                y,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        }
        None => paint.say_at(
            column,
            y,
            "None — text only",
            Weight::Regular,
            size::BODY,
            ink.faint,
        ),
    }
    y += 34.0;

    // A typed value that was not taken, said where it was typed.
    if let Some(why) = &desk.edit_refused {
        paint.say_at(area.x, y, why, Weight::Regular, size::SMALL, ink.warn);
        y += 18.0;
    }
    // What the hold will take of what is free, then the buttons.
    let (pressed, _after) = configure_foot(
        paint,
        desk,
        mouse,
        Box::new(area.x, y, area.w, area.bottom() - y),
        held,
        settings,
        hovered,
    );
    if let Some(pressed) = pressed {
        act = Some(pressed);
    }
    if let Some((picker, at)) = menu
        && let Some(picked) = configure_menu(paint, desk, mouse, picker, at)
    {
        act = Some(picked);
    }
    act
}

/// The foot of the Configure tab: what the hovered row is for, what the
/// hold will take, what the probes applied, and the buttons — Back to
/// recommended, and Host last.
fn configure_foot(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    held: &Model,
    settings: &mcf_serve::hosting::Hosting,
    hovered: Option<&'static str>,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut y = area.y + 6.0;
    let mut act = None;
    if let Some(because) = hovered {
        for line in paint
            .wrap(because, Weight::Regular, size::SMALL, area.w)
            .iter()
            .take(2)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.quiet);
            y += 16.0;
        }
    } else {
        y += 16.0;
    }
    y += 6.0;
    if let Some((said, fits)) = crate::will_take(held, settings, &desk.placements) {
        let shown = paint.elide(&said, Weight::Bold, size::BODY, area.w);
        paint.say_at(
            area.x,
            y,
            &shown,
            Weight::Bold,
            size::BODY,
            if fits { ink.ink } else { ink.warn },
        );
        y += 24.0;
    }
    // What the probes applied: a hold runs under these too (§3.15).
    for (what, applied) in [
        ("Chat template (probed)", held.applied_addressing.as_deref()),
        ("Token budget (probed)", held.applied_budget.as_deref()),
    ] {
        let said = applied.map_or_else(
            || format!("{what}: not set — run `mcf probe --apply`"),
            |applied| format!("{what}: {applied}"),
        );
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, area.w);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::SMALL, ink.faint);
        y += 16.0;
    }
    y += 10.0;
    let mut x = area.x;
    if desk
        .recommended
        .as_ref()
        .is_some_and(|recommended| !settings.differs_from(recommended).is_empty())
    {
        let (reset, drawn) = ui::fitted(paint, mouse, (x, y), "Reset to recommended", Kind::Quiet);
        if reset {
            act = Some(Act::Recommended);
        }
        x += drawn.w + 12.0;
    }
    // What it was last held under, where that is not what is set now: one
    // press puts the settings back the way they were (B-475).
    if let Some((last, _)) = &desk.last_settings
        && !settings.differs_from(last).is_empty()
    {
        let (again, drawn) =
            ui::fitted(paint, mouse, (x, y), "As you set it last time", Kind::Quiet);
        if again {
            act = Some(Act::LastSettings);
        }
        x += drawn.w + 12.0;
    }
    let waits = desk.needs_engine.is_some();
    let (host, _) = ui::fitted(
        paint,
        mouse,
        (x, y),
        if waits {
            "Start server (build the engine first)"
        } else {
            "Start server"
        },
        if waits { Kind::Quiet } else { Kind::Primary },
    );
    if host && !waits && !desk.doing.busy() {
        act = Some(Act::HostIt);
    }
    (act, y + 40.0)
}

/// The words for one placement in the list.
fn placement_label(placement: &crate::Placement) -> String {
    let where_ = match placement.on.as_str() {
        "resolved" => "Auto",
        "processor" => "CPU",
        "card" => "GPU",
        other => other,
    };
    format!("{where_} — {} on {}", placement.engine, placement.device)
}

/// The words for one rope choice in the list.
fn rope_label(at: usize) -> &'static str {
    match at {
        1 => "Off",
        2 => "Linear",
        3 => "YaRN",
        _ => "Default",
    }
}

/// Whichever of the Configure tab's lists is open, and what was picked.
fn configure_menu(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    picker: Picker,
    at: Box,
) -> Option<Act> {
    match picker {
        Picker::Placement => {
            let labels: Vec<String> = desk.placements.iter().map(placement_label).collect();
            if labels.is_empty() {
                return None;
            }
            ui::options(paint, mouse, at, &labels, desk.placed_at()).map(Act::Place)
        }
        Picker::Rope => {
            let labels: Vec<String> = (0..crate::ROPE_CHOICES.len())
                .map(|at| rope_label(at).to_owned())
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                crate::ROPE_CHOICES
                    .iter()
                    .position(|choice| *choice == settings.started.rope)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Rope)
        }
        Picker::Quantization => {
            let labels: Vec<String> = desk.quantizations().iter().map(quant_label).collect();
            if labels.is_empty() {
                return None;
            }
            ui::options(paint, mouse, at, &labels, desk.quantization_at()).map(Act::Quantization)
        }
        Picker::Model
        | Picker::Window
        | Picker::On
        | Picker::Architecture
        | Picker::Fits
        | Picker::Size => None,
    }
}

/// One quantization as the picker lists it: the file, its size, and
/// whether it is here or on the hub and would run here (A7).
fn quant_label(quant: &crate::Quant) -> String {
    let size = quant
        .bytes
        .map_or_else(String::new, |bytes| format!(" · {}", gigabytes(bytes)));
    let where_ = match (quant.here, quant.fits) {
        (Some(_), _) => "here",
        (None, Some(true)) => "on the hub · would run here",
        (None, Some(false)) => "on the hub · would not run here",
        (None, None) => "on the hub",
    };
    format!("{}{size} · {where_}", quant.file)
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
/// The buttons for the chosen model: ask and stop where it is held; else
/// Host — and a build as its own button, said as one, because minutes of
/// container build is not what a person pressing Host expects Host to do
/// (§3.15, B-367). Where no engine here runs the model at all, the build
/// comes first and Host waits on it; where a card's build is missing, it
/// is offered beside Host.
fn action_buttons(desk: &Desk, this_one: bool, stop_label: String) -> Vec<(String, Kind, Act)> {
    if this_one {
        return vec![
            ("Chat".to_owned(), Kind::Primary, Act::Go(Page::Hosting)),
            (stop_label, Kind::Ordinary, Act::StopHosting),
        ];
    }
    // Host is the last button on the Configure tab, after the settings are
    // read whole (D49); the builds stay here, said as builds.
    let mut listed = Vec::new();
    if let Some(engine) = &desk.needs_engine {
        listed.push((
            format!("Build {engine} first"),
            Kind::Primary,
            Act::Build(engine.clone()),
        ));
    } else if let Some((component, _)) = &desk.card_unused {
        listed.push((
            format!("Build {component} for the card"),
            Kind::Ordinary,
            Act::Build(component.clone()),
        ));
    }
    listed.push((
        "Diagnostics".to_owned(),
        Kind::Ordinary,
        Act::Go(Page::Diagnostics),
    ));
    listed
}

/// What was last held, in a line, and a button that holds it again.
fn last_hold_offer(
    paint: &mut Painter,
    mouse: &Mouse,
    last: &crate::LastHold,
    at: Box,
) -> Option<Act> {
    let ink = paint.ink;
    let mut y = at.y;
    for line in paint
        .wrap(&last.said(), Weight::Regular, size::SMALL, at.w - 20.0)
        .iter()
        .take(2)
    {
        paint.say_at(at.x, y + 4.0, line, Weight::Regular, size::SMALL, ink.quiet);
        y += 16.0;
    }
    let where_ = Box::new(at.x, y + 8.0, at.w - 20.0, 30.0);
    let label = paint.elide(
        &format!("Host {} again", last.name()),
        Weight::Regular,
        size::BODY,
        at.w - 40.0,
    );
    ui::button(paint, mouse, where_, &label, Kind::Ordinary).then_some(Act::HostAgain)
}

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
    // What a stop gives back, said on the button: the weights and the
    // cache the hold reserved, which is what the daemon will be asked to
    // free (§3.15).
    let stop_label = desk
        .hosted_model()
        .zip(desk.hosted.as_ref().and_then(|hosting| hosting.context))
        .and_then(|(held, context)| reserve_of(held, context))
        .and_then(|(_, total)| total)
        .map_or_else(
            || "Stop server".to_owned(),
            |total| format!("Stop server — frees {}", gigabytes(total)),
        );
    let mut actions = action_buttons(desk, this_one, stop_label);
    actions.push((
        "Contents".to_owned(),
        Kind::Ordinary,
        Act::Tab(crate::Tab::Contents),
    ));
    for (label, kind, what) in actions {
        let where_ = Box::new(area.x, y, list - 20.0, 30.0);
        let needs_one = true;
        // Host waits on a build where none of the engines here can run
        // the model: the button is drawn quiet and does nothing.
        let waits = what == Act::HostIt && desk.needs_engine.is_some();
        if ui::button(paint, mouse, where_, &label, kind)
            && (!needs_one || desk.chosen.is_some())
            && !waits
        {
            act = Some(what);
        }
        y += 36.0;
    }
    // Where a caller reaches it. The one fact an API is for.
    // What was last held, where nothing is now and nothing is loading: one
    // line and one press to hold it again (A1).
    if desk.hosted.is_none()
        && !desk.doing.busy()
        && let Some(last) = &desk.last_hold
    {
        return last_hold_offer(paint, mouse, last, Box::new(area.x, y, list, 0.0)).or(act);
    }
    if let Some(hosting) = &desk.hosted {
        paint.say_at(
            area.x,
            y + 4.0,
            "Endpoint",
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
        let said = job
            .refused
            .clone()
            .or_else(|| desk.loading_line())
            .unwrap_or_default();
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
    let mut y = area.y + 24.0;
    // The search field: what is typed narrows the list as it is typed, and
    // is the words the hub is asked for (D51).
    let _pressed = ui::field(
        paint,
        mouse,
        Box::new(area.x - 6.0, y, list, 30.0),
        &desk.filter,
        "Search models",
        desk.editing.is_none(),
    );
    y += 38.0;
    if let Some(pressed) = filter_rows(paint, desk, mouse, area) {
        act = Some(pressed);
    }
    y += filters_height(desk);
    let shown = desk.library();
    if let Some(note) = match (desk.models.is_empty(), shown.is_empty()) {
        (true, _) => Some("none held"),
        (false, true) => Some("nothing here matches"),
        (false, false) => None,
    } {
        paint.say_at(area.x, y, note, Weight::Regular, size::BODY, ink.faint);
        y += 24.0;
    }
    // The rows scroll under the field, with the hub's under them (B-490).
    let rows = Box::new(
        area.x - 6.0,
        y - 4.0,
        list + 6.0,
        (actions_at - y - 10.0).max(10.0),
    );
    let rolled = scrolled(
        paint,
        mouse,
        desk,
        Region::Library,
        rows,
        |paint, mouse, inner| library_rows(paint, desk, mouse, inner, &shown),
    );
    rolled.or(act)
}

/// The library's rows from the top of a region: one a repository, then the
/// hub's rows. Returns what was pressed.
fn library_rows(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    inner: Box,
    shown: &[crate::Group],
) -> Option<Act> {
    let mut act = None;
    let mut y = inner.y + 4.0;
    let list = inner.w - 6.0;
    for group in shown {
        if let Some(pressed) = group_row(
            paint,
            desk,
            mouse,
            group,
            Box::new(inner.x + 6.0, y, list, 40.0),
        ) {
            act = Some(pressed);
        }
        y += 40.0;
    }
    if let Some(pressed) = hub_rows(paint, desk, mouse, Box::new(inner.x + 6.0, y, list, 4000.0)) {
        act = Some(pressed);
    }
    act
}

/// How tall the filters are under the field: the toggle's line, and the
/// three pickers when open (B-489).
fn filters_height(desk: &Desk) -> f32 {
    if desk.filters.open {
        22.0 + 3.0 * 32.0
    } else {
        22.0
    }
}

/// Where each filter's picker sits, for drawing it and its list alike.
fn filter_boxes(area: Box) -> [Box; 3] {
    let top = area.y + 24.0 + 38.0 + 22.0;
    let x = area.x + 88.0;
    let wide = area.w - 94.0;
    [
        Box::new(x, top, wide, 28.0),
        Box::new(x, top + 32.0, wide, 28.0),
        Box::new(x, top + 64.0, wide, 28.0),
    ]
}

/// The word the toggle shows: *Filters*, with how many are set.
fn filters_word(desk: &Desk) -> String {
    let set = [
        desk.filters.architecture.is_some(),
        desk.filters.fits.is_some(),
        desk.filters.size.is_some(),
    ]
    .iter()
    .filter(|set| **set)
    .count();
    match (desk.filters.open, set) {
        (true, 0) => "Filters ▴".to_owned(),
        (false, 0) => "Filters ▾".to_owned(),
        (true, set) => format!("Filters ▴ · {set} set"),
        (false, set) => format!("Filters ▾ · {set} set"),
    }
}

/// The filters under the search field: a toggle, and when open three
/// pickers — architecture, fits here, size — each *any* until set (D51,
/// B-489).
fn filter_rows(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let y = area.y + 24.0 + 38.0;
    let word = filters_word(desk);
    let hit = Box::new(
        area.x - 6.0,
        y - 4.0,
        paint.measure(&word, Weight::Regular, size::SMALL) + 12.0,
        20.0,
    );
    paint.say_at(
        area.x,
        y,
        &word,
        Weight::Regular,
        size::SMALL,
        if mouse.over(hit) {
            ink.accent
        } else {
            ink.quiet
        },
    );
    if mouse.clicked(hit) {
        return Some(Act::ToggleFilters);
    }
    if !desk.filters.open {
        return None;
    }
    let boxes = filter_boxes(area);
    let labels = ["Architecture", "Fits here", "Size"];
    let values = [
        desk.filters
            .architecture
            .clone()
            .unwrap_or_else(|| "any".to_owned()),
        fits_label(desk.filters.fits).to_owned(),
        size_label(desk.filters.size),
    ];
    let pickers = [Picker::Architecture, Picker::Fits, Picker::Size];
    let mut act = None;
    for ((label, value), (box_of, picker)) in
        labels.iter().zip(values).zip(boxes.iter().zip(pickers))
    {
        paint.say_at(
            area.x,
            box_of.y + 7.0,
            label,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        if ui::picker(paint, mouse, *box_of, &value, desk.open == Some(picker)) {
            act = Some(Act::Open(picker));
        }
    }
    act
}

/// The *fits here* choice's word.
fn fits_label(fits: Option<bool>) -> &'static str {
    match fits {
        None => "any",
        Some(true) => "will run here",
        Some(false) => "will not run here",
    }
}

/// The size choice's word.
fn size_label(size: Option<u64>) -> String {
    size.map_or_else(
        || "any".to_owned(),
        |bytes| format!("up to {}", gigabytes(bytes)),
    )
}

/// The row that searches the hub for the words in the field, quiet while
/// a search goes.
fn search_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
    words: &str,
) -> Option<Act> {
    let ink = paint.ink;
    let searching =
        matches!(&desk.doing, Doing::Listing(job) if !job.finished) && desk.hub_chosen.is_none();
    let label = paint.elide(
        &format!("Search Hugging Face for {words:?}"),
        Weight::Regular,
        size::BODY,
        at.w - 32.0,
    );
    let where_ = Box::new(at.x, at.y, at.w - 20.0, 30.0);
    let pressed = ui::button(
        paint,
        mouse,
        where_,
        &label,
        if searching {
            Kind::Quiet
        } else {
            Kind::Ordinary
        },
    );
    if searching {
        paint.say_at(
            at.x,
            at.y + 36.0,
            "searching…",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    (pressed && !searching).then_some(Act::SearchHub)
}

/// Under what is here: the row that searches the hub for the words in the
/// field, and what the hub answered for them, one row a repository with its
/// downloads, most downloaded first (D51, B-485).
fn hub_rows(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let words = desk.filter.trim();
    if words.is_empty() || area.h < 40.0 {
        return None;
    }
    let mut y = area.y + 6.0;
    let mut act = None;
    if !desk.hub_matches() {
        return search_row(paint, desk, mouse, Box::new(area.x, y, area.w, 30.0), words);
    }
    let hub = desk.hub.as_ref()?;
    spaced(paint, area.x, y, "on the hub", ink.faint);
    y += 24.0;
    if hub.repositories.is_empty() {
        paint.say_at(
            area.x,
            y,
            "nothing on the hub matches",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return None;
    }
    for (at, found) in hub.repositories.iter().enumerate() {
        let where_ = Box::new(area.x - 6.0, y - 4.0, area.w, 40.0);
        let chosen = desk.hub_chosen == Some(at);
        if chosen {
            paint.panel(where_, 6.0, ink.accent_soft, 255);
        } else if mouse.over(where_) {
            paint.panel(where_, 6.0, ink.line, 110);
        }
        let name = paint.elide(&found.id, Weight::Regular, size::BODY, area.w - 14.0);
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
        let downloads = found.downloads.map_or_else(
            || "on the hub".to_owned(),
            |count| format!("on the hub · {} downloads", words::grouped(count)),
        );
        paint.say_at(
            area.x,
            y + 18.0,
            &downloads,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        if mouse.clicked(where_) {
            act = Some(Act::PickHub(at));
        }
        y += 40.0;
    }
    act
}

/// One entry of the library: the repository's name, or the file's where
/// no repository is known, with the figures that decide a choice under it
/// and how many quantizations are here (D51, B-486). Pressing it makes the
/// entry's chosen member the subject, or its first.
fn group_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    group: &crate::Group,
    at: Box,
) -> Option<Act> {
    let ink = paint.ink;
    let member = group
        .members
        .iter()
        .copied()
        .find(|member| desk.chosen == Some(*member))
        .or_else(|| group.members.first().copied())?;
    let held = desk.models.get(member)?;
    let where_ = Box::new(at.x - 6.0, at.y - 4.0, at.w, 40.0);
    let chosen = desk
        .chosen
        .is_some_and(|chosen| group.members.contains(&chosen))
        && desk.hub_chosen.is_none()
        && desk.pending.is_none();
    if chosen {
        paint.panel(where_, 6.0, ink.accent_soft, 255);
    } else if mouse.over(where_) {
        paint.panel(where_, 6.0, ink.line, 110);
    }
    let name = paint.elide(
        &group.name(&desk.models),
        Weight::Regular,
        size::BODY,
        at.w - 80.0,
    );
    paint.say_at(
        at.x,
        at.y,
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
        at.x + at.w - 14.0,
        at.y,
        &held.bytes.map_or_else(
            || UNKNOWN.to_owned(),
            |bytes| format!("{:.2}G", bytes as f64 / 1e9),
        ),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let mut figures = chooses_by(held);
    if group.members.len() > 1 {
        figures = format!("{} quantizations · {figures}", group.members.len());
    }
    let figures = paint.elide(&figures, Weight::Regular, size::SMALL, at.w - 14.0);
    paint.say_at(
        at.x,
        at.y + 18.0,
        &figures,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    mouse.clicked(where_).then_some(Act::Choose(member))
}

/// One model's figures in a line: architecture, trained window, device
/// kind, measured speed — each only where it is known (A7).
fn chooses_by(held: &Model) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(architecture) = &held.architecture {
        parts.push(architecture.clone());
    }
    if let Some(trained) = held.trained {
        parts.push(format!("{} tokens", words::grouped(trained)));
    }
    if held.device.is_some() {
        parts.push(if held.on_a_card { "GPU" } else { "CPU" }.to_owned());
    }
    if let Some(speed) = held.speed {
        parts.push(format!("{speed:.0} tok/s"));
    }
    if held.refused.is_some() {
        parts.push("will not run here".to_owned());
    }
    if parts.is_empty() {
        "not yet read".to_owned()
    } else {
        parts.join(" · ")
    }
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
        let column = label_column_of(area.w, 190.0);
        let shown = paint.elide(value, Weight::Bold, size::BODY, area.w - column - 10.0);
        paint.say_at(area.x + column, y, &shown, Weight::Bold, size::BODY, colour);
    };

    for (name, value) in [
        (
            "Size",
            held.bytes
                .map_or_else(unknown, |bytes| format!("{:.2} GB", bytes as f64 / 1e9)),
        ),
        (
            "Architecture",
            held.architecture.clone().unwrap_or_else(unknown),
        ),
        (
            "Trained context",
            held.trained
                .map_or_else(unknown, |held| format!("{} tokens", words::grouped(held))),
        ),
        (
            "KV cache/token",
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
                ("Engine", held.engine.clone().unwrap_or_else(unknown)),
                ("Device", held.device.clone().unwrap_or_else(unknown)),
                (
                    "Max context",
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
    let wide = area.w;
    let speed = Column {
        head: "speed",
        at: wide,
        right: true,
    };
    let table = Box::new(area.x, y, wide, 0.0);
    y = heads(paint, table, "throughput", &[speed]);
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
            Box::new(area.x, y + 20.0, area.w, 72.0),
            &held.ladder,
        );
    }
    y
}

/// **The Diagnostics page: one card per run** (D50, B-477). Each card names
/// the run, says in one line what it answers, shows the controls the run
/// takes with MCF's recommendation as the default, states what it will cost
/// before its own Run, and shows its step and a Stop while it goes. When a
/// run is done the card says so, and the figures are on the model's
/// Statistics tab. No control here stands for a thing a run does not
/// separately do: the checkbox rows that were one run's results drawn as
/// five choices are gone.
fn diagnostics(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    scrolled(
        paint,
        mouse,
        desk,
        Region::DiagnosticsPage,
        area,
        |paint, mouse, inner| diagnostics_body(paint, desk, mouse, inner),
    )
}

/// The Diagnostics page: the model it runs on, the list of every
/// diagnostic down the left with when each last ran, and the one chosen
/// shown whole to the right (D53).
fn diagnostics_body(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let mut menu: Option<(Picker, Box)> = None;

    // The model every diagnostic runs on, first.
    let chosen_model = desk.pending.as_ref().map_or_else(
        || {
            desk.chosen
                .and_then(|at| desk.models.get(at))
                .map_or_else(|| "none chosen".to_owned(), |held| held.name.clone())
        },
        |pending| {
            format!(
                "{} — not downloaded",
                pending.file.trim_end_matches(".gguf")
            )
        },
    );
    paint.say_at(
        area.x,
        area.y,
        "Model",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    let box_of = Box::new(
        area.x + 110.0,
        area.y - 6.0,
        (area.w - 110.0).max(160.0),
        28.0,
    );
    let open = desk.open == Some(Picker::Model);
    if ui::picker(paint, mouse, box_of, &chosen_model, open) {
        act = Some(Act::Open(Picker::Model));
    }
    if open {
        menu = Some((Picker::Model, box_of));
    }
    let top = area.y + 44.0;

    // The list down the left, the boundary dragged (B-490), the chosen
    // diagnostic whole to the right.
    let list_w = desk.splits.diagnostics.min((area.w - 260.0).max(160.0));
    let list = Box::new(area.x, top, list_w, area.bottom() - top);
    spaced(paint, list.x, list.y, "diagnostics", ink.faint);
    // Every diagnostic in turn, from one press (B-508).
    let a_run = a_run_is_going(desk);
    let run_all = Box::new(list.right() - 78.0, list.y - 7.0, 72.0, 26.0);
    let kind = if a_run { Kind::Quiet } else { Kind::Primary };
    if ui::button(paint, mouse, run_all, "Run all", kind) && !a_run {
        act = Some(download_then(desk, Act::RunAll));
    }
    // The strip under the list: what runs, how far, how long, Stop (B-509).
    let strip = Box::new(list.x, list.bottom() - STRIP, list.w, STRIP);
    if let Some(pressed) = progress_strip(paint, desk, mouse, strip) {
        act = Some(pressed);
    }
    let rows = Box::new(
        list.x - 6.0,
        list.y + 22.0,
        list.w + 6.0,
        (list.h - 22.0 - STRIP - 8.0).max(40.0),
    );
    let picked = scrolled(
        paint,
        mouse,
        desk,
        Region::Checks,
        rows,
        |paint, mouse, inner| diagnostic_rows(paint, desk, mouse, inner),
    );
    act = picked.or(act);
    if let Some(to) = ui::splitter(
        paint,
        mouse,
        Box::new(list.right() + 4.0, top, 16.0, area.bottom() - top),
        true,
        desk.grabbed == Some(Splitter::Diagnostics),
    ) {
        act = Some(Act::Split(Splitter::Diagnostics, whole(to)));
    }
    let pane = Box::new(
        list.right() + 24.0,
        top,
        (area.right() - list.right() - 24.0).max(120.0),
        area.bottom() - top,
    );
    let (pressed, opened) = diagnostic_pane(paint, desk, mouse, pane);
    act = pressed.or(act);
    if let Some(opened) = opened {
        menu = Some(opened);
    }
    if let Some((picker, at)) = menu
        && let Some(picked) = open_menu(paint, desk, mouse, picker, at)
    {
        act = Some(picked);
    }
    act
}

/// How tall the strip under the list is.
const STRIP: f32 = 74.0;

/// The strip under the list: which diagnostic is running and, while a
/// Run all goes, which run of how many; a bar that is the run's own
/// progress where its stream says, or the sequence's; the elapsed time;
/// and a Stop. *Nothing running* otherwise, so the list keeps its shape
/// (B-509, A7).
fn progress_strip(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> Option<Act> {
    let ink = paint.ink;
    paint.rule((at.x, at.y), (at.right(), at.y), ink.line, 255);
    let running = desk.running_diagnostic();
    let job = desk.doing.job().filter(|job| !job.finished);
    let (Some(job), Some(running)) = (job, running.or_else(|| job.map(|_| desk.diagnostic))) else {
        paint.say_at(
            at.x,
            at.y + 14.0,
            "nothing running",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return None;
    };
    let what = match desk.sequence_place() {
        Some((place, of)) => format!("run {place} of {of} · {}", running.name()),
        None => running.name().to_owned(),
    };
    let what = if job.stopping && !job.finished {
        format!("{what} · stopping after the attempt in hand")
    } else {
        what
    };
    let shown = paint.elide(&what, Weight::Bold, size::SMALL, at.w - 96.0);
    paint.say_at(
        at.x,
        at.y + 12.0,
        &shown,
        Weight::Bold,
        size::SMALL,
        ink.ink,
    );
    // The time run, and about how long is left at this run's own pace,
    // said as an estimate (B-572).
    let elapsed = match desk.time_left() {
        Some(left) => format!("{} · about {} left", clock(job.ran()), clock(left)),
        None => clock(job.ran()),
    };
    paint.say_right(
        at.right() - 8.0,
        at.y + 12.0,
        &elapsed,
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let bar = Box::new(at.x, at.y + 34.0, at.w - 86.0, 8.0);
    // A run whose stream does not say how far it is draws the bar without
    // a fill, which is honest about what is known (A7).
    ui::progress(
        paint,
        bar,
        desk.sequence_fraction().or_else(|| desk.run_fraction()),
    );
    let stop = Box::new(at.right() - 74.0, at.y + 26.0, 66.0, 26.0);
    ui::button(paint, mouse, stop, "Stop", Kind::Primary).then_some(Act::Stop)
}

/// The list's rows from the top of its region: each family under its
/// heading with a Run all where the family is one run, one row a
/// diagnostic (D53).
fn diagnostic_rows(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let x = area.x + 6.0;
    let w = area.w - 12.0;
    let mut y = area.y + 6.0;
    let running_now = desk.running_diagnostic();
    let a_run = a_run_is_going(desk);
    for (heading, card, members) in crate::Diagnostic::families() {
        spaced(paint, x, y + 5.0, heading, ink.faint);
        if let Some(card) = card {
            let button = Box::new(x + w - 78.0, y - 3.0, 70.0, 26.0);
            let kind = if a_run { Kind::Quiet } else { Kind::Ordinary };
            if ui::button(paint, mouse, button, "Run all", kind) && !a_run {
                act = Some(download_then(desk, Act::Run(card)));
            }
        }
        y += 30.0;
        for diagnostic in members {
            let row = Box::new(x, y, w, 40.0);
            if let Some(pressed) = diagnostic_row(paint, desk, mouse, diagnostic, row, running_now)
            {
                act = Some(pressed);
            }
            y += 42.0;
        }
        y += 10.0;
    }
    act
}

/// One row of the list: the diagnostic's name, and beneath it when it
/// last ran on this model and through what, *never run*, or that it is
/// running now and which step it is on (D53).
fn diagnostic_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    diagnostic: crate::Diagnostic,
    at: Box,
    running_now: Option<crate::Diagnostic>,
) -> Option<Act> {
    let ink = paint.ink;
    let where_ = Box::new(at.x - 6.0, at.y - 4.0, at.w, 40.0);
    let chosen = desk.diagnostic == diagnostic;
    if chosen {
        paint.panel(where_, 6.0, ink.accent_soft, 255);
    } else if mouse.over(where_) {
        paint.panel(where_, 6.0, ink.line, 110);
    }
    let running = running_now == Some(diagnostic);
    let name = paint.elide(diagnostic.name(), Weight::Regular, size::BODY, at.w - 110.0);
    paint.say_at(
        at.x,
        at.y,
        &name,
        if chosen {
            Weight::Bold
        } else {
            Weight::Regular
        },
        size::BODY,
        if chosen { ink.accent } else { ink.ink },
    );
    let last = desk.last_run(diagnostic);
    let (state, colour) = if running {
        ("running".to_owned(), ink.accent)
    } else {
        match &last {
            Some((when, _)) => (crate::when_said(when), ink.faint),
            None => ("never run".to_owned(), ink.faint),
        }
    };
    paint.say_right(
        at.x + at.w - 14.0,
        at.y,
        &state,
        Weight::Regular,
        size::SMALL,
        colour,
    );
    let under = if running {
        desk.doing
            .job()
            .and_then(crate::job::Job::latest)
            .and_then(|answer| match diagnostic {
                crate::Diagnostic::Probe(_) => probe_step_said(answer),
                crate::Diagnostic::Measure(_) => measure_step_said(answer),
                _ => None,
            })
            .or_else(|| desk.under_way())
            .unwrap_or_else(|| "running".to_owned())
    } else {
        match &last {
            Some((_, Some(engine))) => format!("last run through {engine}"),
            Some((_, None)) => "last run on this model".to_owned(),
            None => match diagnostic {
                crate::Diagnostic::Comparison => "at the command line".to_owned(),
                _ => "never run on this model".to_owned(),
            },
        }
    };
    let under = paint.elide(&under, Weight::Regular, size::SMALL, at.w - 14.0);
    paint.say_at(
        at.x,
        at.y + 18.0,
        &under,
        Weight::Regular,
        size::SMALL,
        if running { ink.accent } else { ink.faint },
    );
    mouse.clicked(where_).then_some(Act::Show(diagnostic))
}

/// The chosen diagnostic, whole, in its own scrolled region: the ladder's
/// card as it was, a run's card with its last finding under it, or a
/// probe's or measurement's own pane (D53). Returns what was pressed and
/// a menu to draw over everything, where one is open.
fn diagnostic_pane(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
) -> (Option<Act>, Option<(Picker, Box)>) {
    let mut menu = None;
    let act = scrolled(
        paint,
        mouse,
        desk,
        Region::Diagnostics,
        area,
        |paint, mouse, inner| {
            let inner = second_copy_line(paint, desk, inner);
            match desk.diagnostic {
                crate::Diagnostic::Throughput => {
                    let (act, opened) = throughput_card(paint, desk, mouse, inner);
                    menu = opened;
                    act
                }
                crate::Diagnostic::CrossCheck
                | crate::Diagnostic::Prompt
                | crate::Diagnostic::Comparison => run_pane(paint, desk, mouse, inner),
                crate::Diagnostic::Probe(_) | crate::Diagnostic::Measure(_) => {
                    one_pane(paint, desk, mouse, inner, desk.diagnostic)
                }
                crate::Diagnostic::Eval(_) => eval_pane(paint, desk, mouse, inner, desk.diagnostic),
            }
        },
    );
    (act, menu)
}

/// The cost of a run on a hosted model, above its pane: the space it takes
/// off the top of the pane is returned as what is left (B-561, F243).
fn second_copy_line(paint: &mut Painter, desk: &Desk, inner: Box) -> Box {
    let Some(said) = desk.second_copy() else {
        return inner;
    };
    let ink = paint.ink;
    let shown = paint.elide(&said, Weight::Regular, size::SMALL, inner.w);
    paint.say_at(
        inner.x,
        inner.y,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.warn,
    );
    Box::new(inner.x, inner.y + 24.0, inner.w, (inner.h - 24.0).max(0.0))
}

/// A coding suite's pane: what it answers, Run — which starts MCF's own
/// `eval` for the suite and reads it as it goes — or Stop while it runs,
/// the command's last line meanwhile, when it last ran, and its readings
/// (B-519, D54).
#[allow(clippy::too_many_lines, reason = "one pane, its rows in order")]
fn eval_pane(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    diagnostic: crate::Diagnostic,
) -> Option<Act> {
    let ink = paint.ink;
    let inner_x = area.x + PAD;
    let inner_w = area.w - 2.0 * PAD;
    let running_one = desk.running_diagnostic() == Some(diagnostic);
    let done = is_done_one(desk, diagnostic);
    let under = if is_the_challenges_row(diagnostic) {
        RUNS_UNDER_HEIGHT
            + if desk.edit_refused.is_some() {
                18.0
            } else {
                0.0
            }
    } else {
        0.0
    };
    // The results so far, a line each, while the suite runs and once it
    // has finished (B-569).
    let so_far = if running_one || done {
        desk.results_so_far()
    } else {
        Vec::new()
    };
    #[allow(clippy::cast_precision_loss, reason = "a count of lines")]
    let so_far_height = if so_far.is_empty() {
        0.0
    } else {
        22.0 + 16.0 * so_far.len() as f32
    };
    let height = head_height_of(paint, diagnostic.answers(), area.w)
        + 44.0
        + under
        + so_far_height
        + if done { 30.0 } else { 0.0 }
        + 12.0;
    let frame = Box::new(area.x, area.y, area.w, height);
    let y = card_head_named(paint, frame, diagnostic.name(), diagnostic.answers());
    let mut act = None;
    let running = a_run_is_going(desk);
    // Stop asks the suite to finish the attempt in hand and close its run;
    // a second press kills it. A row whose newest run did not finish
    // offers Resume, which goes on from what that run has (B-571).
    let stopping = matches!(&desk.doing, Doing::Evaluating(job) if job.stopping && !job.finished);
    let label = if running_one && stopping {
        "Kill"
    } else if running_one {
        "Stop"
    } else if !running && desk.resumable(diagnostic) {
        "Resume"
    } else {
        run_label(desk, running_one)
    };
    let (pressed, button) = ui::fitted(
        paint,
        mouse,
        (inner_x, y),
        label,
        if running && !running_one {
            Kind::Quiet
        } else {
            Kind::Primary
        },
    );
    if pressed && running_one {
        act = Some(Act::Stop);
    } else if pressed && !running {
        act = Some(Act::RunOne(diagnostic));
    }
    if running_one {
        // The command's own last line, as `mcf eval` prints it.
        let said = desk
            .doing
            .job()
            .and_then(|job| {
                job.answers
                    .iter()
                    .rev()
                    .find_map(|answer| answer.get("line").and_then(Value::as_text))
            })
            .map_or_else(|| "starting".to_owned(), |line| line.trim().to_owned());
        let shown = paint.elide(
            &said,
            Weight::Regular,
            size::SMALL,
            inner_w - button.w - 12.0,
        );
        paint.say_at(
            button.right() + 12.0,
            y + 8.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.accent,
        );
    }
    let mut below = y + 44.0;
    // What the catalogue will run under, and the two fields that set it
    // (B-564): under the Run button, before the readings.
    if is_the_challenges_row(diagnostic)
        && let Some(pressed) = runs_under(paint, desk, mouse, inner_x, &mut below)
    {
        act = Some(pressed);
    }
    if !so_far.is_empty() {
        paint.say_at(
            inner_x,
            below + 4.0,
            "so far",
            Weight::Bold,
            size::SMALL,
            ink.faint,
        );
        below += 22.0;
        for line in &so_far {
            let shown = paint.elide(line, Weight::Regular, size::SMALL, inner_w);
            paint.say_at(
                inner_x,
                below,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.quiet,
            );
            below += 16.0;
        }
    }
    if done {
        paint.say_at(
            inner_x,
            below + 8.0,
            "Done — the readings are below",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        below += 30.0;
    }
    let mut y = frame.bottom() + 18.0;
    spaced(paint, area.x, y, "last run", ink.faint);
    y += 24.0;
    let said = match desk.last_run(diagnostic) {
        Some((at, _)) => format!("taken {}", crate::when_said(&at)),
        None => "never run on this model".to_owned(),
    };
    paint.say_at(area.x, y, &said, Weight::Regular, size::SMALL, ink.faint);
    y += 22.0;
    if let Some(job) = desk.doing.job()
        && matches!(desk.doing, Doing::Evaluating(_))
        && job.finished
        && let Some(why) = &job.refused
    {
        for wrapped in paint.wrap(why, Weight::Regular, size::BODY, area.w) {
            paint.say_at(area.x, y, &wrapped, Weight::Regular, size::BODY, ink.ink);
            y += 20.0;
        }
    }
    let _ = below;
    if let Some(run) = desk.readings_of(diagnostic) {
        readings_table(
            paint,
            Box::new(area.x, y + 14.0, area.w, 0.0),
            run,
            a_run_is_going(desk),
        );
    }
    act
}

/// A run's pane: its card as it was, and under it what its last run
/// said and when (D53).
fn run_pane(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let card = desk.diagnostic.card();
    let (act, below) = small_card(
        paint,
        desk,
        mouse,
        Box::new(area.x, area.y, area.w, 0.0),
        card,
    );
    let held = desk.chosen.and_then(|at| desk.models.get(at));
    let mut y = below + 18.0;
    match desk.diagnostic {
        crate::Diagnostic::CrossCheck => {
            spaced(paint, area.x, y, "last cross-check", ink.faint);
            y += 24.0;
            match held {
                Some(held) if !held.cross_checked.is_empty() => {
                    if let Some(at) = &held.cross_checked_at {
                        paint.say_at(
                            area.x,
                            y,
                            &format!("taken {}", crate::when_said(at)),
                            Weight::Regular,
                            size::SMALL,
                            ink.faint,
                        );
                        y += 20.0;
                    }
                    for line in &held.cross_checked {
                        for wrapped in paint.wrap(line, Weight::Regular, size::BODY, area.w) {
                            paint.say_at(area.x, y, &wrapped, Weight::Regular, size::BODY, ink.ink);
                            y += 20.0;
                        }
                        y += 4.0;
                    }
                }
                _ => paint.say_at(
                    area.x,
                    y,
                    "never run on this model",
                    Weight::Regular,
                    size::BODY,
                    ink.faint,
                ),
            }
        }
        crate::Diagnostic::Prompt => {
            let said = match held.and_then(|held| held.prompt_reported_at.as_ref()) {
                Some(at) => format!("last report taken {}", crate::when_said(at)),
                None => "no report taken of this model yet".to_owned(),
            };
            paint.say_at(area.x, y, &said, Weight::Regular, size::SMALL, ink.faint);
        }
        _ => {}
    }
    act
}

/// A probe's or a measurement's pane: what it answers, Apply for a
/// probe, Run with the step while it goes, and under the card its last
/// finding with when it was taken and through what (D53).
fn one_pane(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    diagnostic: crate::Diagnostic,
) -> Option<Act> {
    let ink = paint.ink;
    let inner_x = area.x + PAD;
    let inner_w = area.w - 2.0 * PAD;
    let is_probe = matches!(diagnostic, crate::Diagnostic::Probe(_));
    let running_one = desk.running_diagnostic() == Some(diagnostic);
    let done = is_done_one(desk, diagnostic);
    let height = head_height_of(paint, diagnostic.answers(), area.w)
        + if is_probe { 30.0 } else { 0.0 }
        + 44.0
        + if done { 30.0 } else { 0.0 }
        + 12.0;
    let frame = Box::new(area.x, area.y, area.w, height);
    let mut y = card_head_named(paint, frame, diagnostic.name(), diagnostic.answers());
    let mut act = None;
    if is_probe {
        if tick_box(
            paint,
            mouse,
            (inner_x, y),
            "Apply findings",
            desk.probes_apply,
        ) {
            act = Some(Act::ApplyProbes);
        }
        y += 30.0;
    }
    let running = a_run_is_going(desk);
    let (pressed, button) = ui::fitted(
        paint,
        mouse,
        (inner_x, y),
        run_label(desk, running_one),
        if running && !running_one {
            Kind::Quiet
        } else {
            Kind::Primary
        },
    );
    if pressed && running_one {
        act = Some(Act::Stop);
    } else if pressed && !running {
        act = Some(download_then(desk, Act::RunOne(diagnostic)));
    }
    if running_one {
        let step = desk
            .doing
            .job()
            .and_then(crate::job::Job::latest)
            .and_then(|answer| {
                if is_probe {
                    probe_step_said(answer)
                } else {
                    measure_step_said(answer)
                }
            })
            .unwrap_or_else(|| "starting".to_owned());
        let shown = paint.elide(
            &step,
            Weight::Regular,
            size::SMALL,
            inner_w - button.w - 12.0,
        );
        paint.say_at(
            button.right() + 12.0,
            y + 8.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.accent,
        );
    }
    y += 44.0;
    if done {
        paint.say_at(
            inner_x,
            y + 8.0,
            "Done — the finding is below, and on the model's Statistics tab",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
    }
    finding_rows(paint, desk, area, frame.bottom() + 18.0, diagnostic);
    act
}

/// A probe's or measurement's last finding on the chosen model, under
/// its card: when it was taken and through what, then the lines (D53).
fn finding_rows(
    paint: &mut Painter,
    desk: &Desk,
    area: Box,
    mut y: f32,
    diagnostic: crate::Diagnostic,
) {
    let ink = paint.ink;
    spaced(paint, area.x, y, "last finding", ink.faint);
    y += 24.0;
    let Some(finding) = desk.finding_of(diagnostic) else {
        paint.say_at(
            area.x,
            y,
            "never run on this model",
            Weight::Regular,
            size::BODY,
            ink.faint,
        );
        return;
    };
    let taken = match (&finding.at, &finding.engine) {
        (Some(at), Some(engine)) => format!("taken {} through {engine}", crate::when_said(at)),
        (Some(at), None) => format!("taken {}", crate::when_said(at)),
        (None, _) => "taken, the record did not say when".to_owned(),
    };
    paint.say_at(area.x, y, &taken, Weight::Regular, size::SMALL, ink.faint);
    y += 22.0;
    for line in &finding.lines {
        let line = line.trim_end();
        if line.trim().is_empty() {
            y += 8.0;
            continue;
        }
        let line = columns_said(line);
        for wrapped in paint.wrap(&line, Weight::Regular, size::BODY, area.w) {
            paint.say_at(area.x, y, &wrapped, Weight::Regular, size::BODY, ink.ink);
            y += 20.0;
        }
    }
    // The rows under the sentences: every figure the run read, raw, in
    // one table — what a person compares models by (D54, B-516).
    if let Some(run) = desk.readings_of(diagnostic) {
        readings_table(
            paint,
            Box::new(area.x, y + 14.0, area.w, 0.0),
            run,
            a_run_is_going(desk),
        );
    }
}

/// How many rows of a run's readings the pane shows before it says how
/// many more there are: enough to read a run whole, few enough that a
/// fidelity run of three hundred rows does not push the page a metre down.
const ROWS_SHOWN: usize = 60;

/// A run's readings as a table: the dimensions as columns, then the
/// metric, the value and its unit, one row a reading (D54, B-516).
#[allow(clippy::too_many_lines, reason = "one table, its lines in order")]
fn readings_table(paint: &mut Painter, area: Box, run: &Value, going: bool) {
    let ink = paint.ink;
    let rows = mcf_record::readings::rows_of(run);
    if rows.is_empty() {
        return;
    }
    let dims = mcf_record::readings::dims_of(&rows);
    let mut heads: Vec<String> = dims.clone();
    heads.extend(["metric", "value", "unit"].map(str::to_owned));
    // Each column as wide as its widest cell, the value column set flush
    // right; what does not fit the pane is elided rather than run off it.
    let cells: Vec<Vec<String>> = rows
        .iter()
        .take(ROWS_SHOWN)
        .map(|row| {
            let mut cell: Vec<String> = dims.iter().map(|dim| row.dim(dim)).collect();
            cell.push(row.metric.clone());
            cell.push(words::grouped_signed(row.value));
            cell.push(row.unit.clone());
            cell
        })
        .collect();
    let widths: Vec<f32> = heads
        .iter()
        .enumerate()
        .map(|(at, head)| {
            let widest = cells
                .iter()
                .filter_map(|cell| cell.get(at))
                .map(|text| paint.measure(text, Weight::Regular, size::SMALL))
                .fold(0.0_f32, f32::max);
            (spaced_width(paint, head).max(widest) + 18.0).min(area.w / 2.0)
        })
        .collect();
    let mut y = area.y;
    spaced(paint, area.x, y, "readings", ink.faint);
    y += 22.0;
    // What the run ran under, from its conditions: the engine, the window,
    // the layers, and whatever else the run named, so that the figures
    // below are never read without them (D56).
    let under: Vec<String> = match run.get("conditions") {
        Some(Value::Map(conditions)) => conditions
            .iter()
            .filter_map(|(key, held)| match held {
                Value::List(_) | Value::Map(_) | Value::Null => None,
                held => Some(format!("{key} {}", held.to_line().trim_matches('"'))),
            })
            .collect(),
        _ => Vec::new(),
    };
    if !under.is_empty() {
        let said = paint.elide(
            &format!("under: {}", under.join(" · ")),
            Weight::Regular,
            size::SMALL,
            area.w,
        );
        paint.say_at(area.x, y, &said, Weight::Regular, size::SMALL, ink.faint);
        y += 20.0;
    }
    // A run recorded a part at a time says how it ended, or that it has
    // not: under way while its suite runs, cut off otherwise (B-570).
    if mcf_record::readings::in_parts(run) {
        let ended = match mcf_record::readings::ended_of(run) {
            Some(ended) => ended,
            None if going => format!("under way: {} row(s) so far", rows.len()),
            None => format!("cut off after {} row(s): the run did not close", rows.len()),
        };
        paint.say_at(area.x, y, &ended, Weight::Regular, size::SMALL, ink.faint);
        y += 20.0;
    }
    let mut x = area.x;
    for (head, wide) in heads.iter().zip(&widths) {
        spaced(paint, x, y, head, ink.faint);
        x += wide;
    }
    y += 17.0;
    paint.rule((area.x, y), (area.right(), y), ink.line, 255);
    y += 8.0;
    let value_at = dims.len() + 1;
    for cell in &cells {
        let mut x = area.x;
        for (at, (text, wide)) in cell.iter().zip(&widths).enumerate() {
            let shown = paint.elide(text, Weight::Regular, size::SMALL, wide - 12.0);
            if at == value_at {
                paint.say_right(
                    x + wide - 18.0,
                    y,
                    &shown,
                    Weight::Bold,
                    size::SMALL,
                    ink.ink,
                );
            } else {
                paint.say_at(x, y, &shown, Weight::Regular, size::SMALL, ink.quiet);
            }
            x += wide;
        }
        y += 18.0;
    }
    if rows.len() > ROWS_SHOWN {
        paint.say_at(
            area.x,
            y + 4.0,
            &format!(
                "… and {} more; `mcf data` writes them all",
                rows.len() - ROWS_SHOWN
            ),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
}

/// A line the daemon aligned into columns with runs of spaces, as a
/// proportional face can show it: the columns parted by a dot, since the
/// spaces that lined them up on a terminal collapse here.
pub(crate) fn columns_said(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut spaces = 0_usize;
    for letter in line.trim().chars() {
        if letter == ' ' {
            spaces += 1;
            continue;
        }
        if spaces >= 2 {
            out.push_str(" · ");
        } else if spaces == 1 {
            out.push(' ');
        }
        spaces = 0;
        out.push(letter);
    }
    out
}

/// Whether a probe's or measurement's own run has finished with a
/// finding for it (D53).
fn is_done_one(desk: &Desk, diagnostic: crate::Diagnostic) -> bool {
    if let (crate::Diagnostic::Eval(which), Doing::Evaluating(job)) = (diagnostic, &desk.doing) {
        return job.finished
            && job.refused.is_none()
            && desk.evaluating.is_none_or(|running| running == which)
            && job
                .conclusion()
                .and_then(|end| end.get("exit"))
                .and_then(Value::as_integer)
                == Some(0);
    }
    let ((crate::Diagnostic::Probe(_), Doing::Probing(job))
    | (crate::Diagnostic::Measure(_), Doing::Examining(job))) = (diagnostic, &desk.doing)
    else {
        return false;
    };
    job.finished
        && job.refused.is_none()
        && job.answers.iter().any(|answer| {
            answer
                .get("step")
                .and_then(|step| step.get("name"))
                .and_then(Value::as_text)
                == Some(diagnostic.name())
        })
}

/// A card's frame: the panel, its name and what it answers. Returns where
/// the card's own rows begin.
fn card_head(paint: &mut Painter, at: Box, card: Card) -> f32 {
    card_head_named(paint, at, card.name(), card.answers())
}

/// A card's frame by name: the panel, the name and what it answers.
fn card_head_named(paint: &mut Painter, at: Box, name: &str, answers: &str) -> f32 {
    let ink = paint.ink;
    paint.panel(at, 10.0, ink.card, 255);
    paint.edge(at, 10.0, ink.line, ink.card);
    paint.say_at(
        at.x + PAD,
        at.y + 12.0,
        name,
        Weight::Bold,
        size::BODY,
        ink.ink,
    );
    let mut y = at.y + 31.0;
    for line in paint
        .wrap(answers, Weight::Regular, size::SMALL, at.w - 2.0 * PAD)
        .iter()
        .take(2)
    {
        paint.say_at(at.x + PAD, y, line, Weight::Regular, size::SMALL, ink.faint);
        y += 16.0;
    }
    y + 6.0
}

/// How tall a card's head is: its name and what it answers, wrapped to two
/// lines at most.
fn head_height(paint: &mut Painter, card: Card, w: f32) -> f32 {
    head_height_of(paint, card.answers(), w)
}

/// The same, by what the head says.
fn head_height_of(paint: &mut Painter, answers: &str, w: f32) -> f32 {
    let lines = paint
        .wrap(answers, Weight::Regular, size::SMALL, w - 2.0 * PAD)
        .len()
        .clamp(1, 2);
    #[allow(clippy::cast_precision_loss, reason = "one or two lines")]
    let lines = lines as f32;
    31.0 + 16.0 * lines + 6.0
}

/// A card's Run, or Stop while its run goes, or *Download and run* on a
/// subject not here (B-487).
fn run_label(desk: &Desk, going: bool) -> &'static str {
    if going {
        "Stop"
    } else if desk.pending.is_some() {
        "Download and run"
    } else {
        "Run"
    }
}

/// An act on a subject not here goes after its download (B-487).
fn download_then(desk: &Desk, act: Act) -> Act {
    if desk.pending.is_some() {
        Act::DownloadThen(std::boxed::Box::new(act))
    } else {
        act
    }
}

/// Whether a run is going, so that no card offers a second one meanwhile.
fn a_run_is_going(desk: &Desk) -> bool {
    matches!(
        &desk.doing,
        Doing::Measuring(job)
            | Doing::CrossChecking(job)
            | Doing::Probing(job)
            | Doing::Examining(job)
            | Doing::Evaluating(job)
            if !job.finished
    )
}

/// The throughput card: device and window pickers, where the model lands,
/// the depths the window implies, two ways to run it with their cost, and
/// the run's readings as they come.
fn throughput_card(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
) -> (Option<Act>, Option<(Picker, Box)>) {
    let ink = paint.ink;
    let mut act = None;
    let mut menu = None;
    let running = a_run_is_going(desk);
    let inner = Box::new(at.x + PAD, at.y, at.w - 2.0 * PAD, at.h);
    let mut y = card_head(paint, at, Card::Throughput);
    for (picker, label, value) in [
        (Picker::On, "Device", on_label(desk.on)),
        (
            Picker::Window,
            "Context length",
            format!("{} tokens", words::grouped(desk.window)),
        ),
    ] {
        paint.say_at(inner.x, y, label, Weight::Regular, size::BODY, ink.quiet);
        let column = label_column(inner.w);
        let box_of = Box::new(
            inner.x + column,
            y - 6.0,
            (inner.w - column).max(60.0),
            28.0,
        );
        let open = desk.open == Some(picker);
        if ui::picker(paint, mouse, box_of, &value, open) {
            act = Some(Act::Open(picker));
        }
        if open {
            menu = Some((picker, box_of));
        }
        y += 34.0;
    }
    y = placement_rows(paint, desk, inner.x, y, inner.w);
    if let Some((component, because)) = &desk.card_unused {
        if let Some(pressed) = card_unused(
            paint,
            desk,
            mouse,
            Box::new(inner.x, y, inner.w, 0.0),
            component,
            because,
        ) {
            act = Some(pressed);
        }
        y += card_unused_height(paint, desk, inner.w, because);
    }
    // Choosing a window implies every power of two below it, so the depths
    // are stated rather than offered as a second set of choices.
    paint.say_at(inner.x, y, "Depths", Weight::Regular, size::BODY, ink.quiet);
    let column = label_column(inner.w);
    let depths = paint.elide(
        &desk.ladder_line(),
        Weight::Regular,
        size::BODY,
        inner.w - column,
    );
    paint.say_at(
        inner.x + column,
        y,
        &depths,
        Weight::Regular,
        size::BODY,
        ink.ink,
    );
    y += 34.0;

    let (pressed, below) = throughput_buttons(paint, desk, mouse, (inner.x, y), inner.w, running);
    act = pressed.or(act);
    y = below;
    readings(
        paint,
        desk,
        Box::new(inner.x, y, inner.w, (at.bottom() - y).max(0.0)),
    );
    (act, menu)
}

/// How wide a card's label column is: a hundred and fifty points where
/// there is room, and a share of a narrow card otherwise (B-510).
fn label_column(inner_w: f32) -> f32 {
    label_column_of(inner_w, 150.0)
}

/// The same, with the width the labels want where there is room.
fn label_column_of(inner_w: f32, most: f32) -> f32 {
    (inner_w * 0.4).min(most).max(70.0)
}

/// Quick run and Run, each with its cost; Stop while the ladder climbs; and
/// where the figures went once it has. Returns where the readings begin.
fn throughput_buttons(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    (x, y): (f32, f32),
    wide: f32,
    running: bool,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut act = None;
    let (quick_w, full_w): (f32, f32) = if desk.pending.is_some() {
        (170.0, 160.0)
    } else {
        (130.0, 110.0)
    };
    // Two buttons and a Stop share a narrow card rather than run off it
    // (B-510).
    let room = ((wide - 28.0) / 3.0).max(60.0);
    let (quick_w, full_w) = (quick_w.min(room), full_w.min(room));
    let quick = Box::new(x, y, quick_w, 34.0);
    let full = Box::new(quick.right() + 14.0, y, full_w, 34.0);
    let kind = |primary: bool| {
        if running {
            Kind::Quiet
        } else if primary {
            Kind::Primary
        } else {
            Kind::Ordinary
        }
    };
    // On a subject not here the buttons download first and run after
    // (B-487).
    let then = |act: Act| download_then(desk, act);
    let (quick_label, full_label) = if desk.pending.is_some() {
        ("Download, quick run", "Download and run")
    } else {
        ("Quick run", "Run")
    };
    if ui::button(paint, mouse, quick, quick_label, kind(false)) && !running {
        act = Some(then(Act::Measure {
            deepest: desk.quick_depth(),
        }));
    }
    if ui::button(paint, mouse, full, full_label, kind(true)) && !running {
        act = Some(then(Act::Run(Card::Throughput)));
    }
    let measuring = matches!(&desk.doing, Doing::Measuring(job) if !job.finished);
    if measuring {
        let stop = Box::new(full.right() + 14.0, y, 90.0, 34.0);
        if ui::button(paint, mouse, stop, "Stop", Kind::Primary) {
            act = Some(Act::Stop);
        }
    }
    for (button, quick_one) in [(quick, true), (full, false)] {
        let (low, high) = desk.estimate(quick_one);
        // The caption fits its button, however narrow the card (B-510).
        let caption = paint.elide(
            &span(low, high),
            Weight::Regular,
            size::SMALL,
            button.w + 10.0,
        );
        paint.say_centred(
            Box::new(button.x, button.bottom(), button.w, 20.0),
            &caption,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    let mut below = y + 34.0 + 22.0;
    if let Some(pressed) = done_line(paint, desk, mouse, (x, below), Card::Throughput) {
        act = Some(pressed);
    }
    if is_done(desk, Card::Throughput) {
        below += 30.0;
    }
    (act, below + 12.0)
}

/// Whether the row is the catalogue's, whose pane takes the retries and
/// the window (B-564).
fn is_the_challenges_row(diagnostic: crate::Diagnostic) -> bool {
    matches!(diagnostic, crate::Diagnostic::Eval(at)
        if crate::SUITES.get(at).is_some_and(|(name, _, _)| name.starts_with("challenges")))
}

/// What the Challenges pane's conditions and fields take, in height; a
/// refusal of what was typed takes one line more.
const RUNS_UNDER_HEIGHT: f32 = 18.0 * 3.0 + 34.0 * 3.0 + 8.0;

/// What the catalogue will run under, said before Run is pressed: the
/// engine and device MCF resolved for the model, the answer budget, and
/// the two fields a person can set — the retries and the window (B-564,
/// D56). The rest is the daemon's at the ask, and every row's record
/// names it.
fn runs_under(paint: &mut Painter, desk: &Desk, mouse: &Mouse, x: f32, y: &mut f32) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let model = desk.chosen.and_then(|at| desk.models.get(at));
    let engine = match model {
        Some(held) => match (&held.engine, &held.device) {
            (Some(engine), Some(device)) => format!("{engine} on {device}"),
            (Some(engine), None) => engine.clone(),
            _ => "the daemon's choice; each row's record names it".to_owned(),
        },
        None => "the daemon's choice".to_owned(),
    };
    for line in [
        format!("runs under: {engine}"),
        "budget: 1,400 tokens an answer · seed 0 · every answer run in a container".to_owned(),
        match desk.challenge_window {
            Some(window) => format!("window: {window} tokens, every attempt"),
            None => "window: sized to each turn unless set below".to_owned(),
        },
    ] {
        paint.say_at(x, *y, &line, Weight::Regular, size::SMALL, ink.quiet);
        *y += 18.0;
    }
    for (name, field, now, placeholder) in [
        (
            "Retries",
            crate::Field::Retries,
            desk.retries.to_string(),
            "10",
        ),
        (
            "Window",
            crate::Field::Window,
            desk.challenge_window
                .map_or_else(String::new, |window| window.to_string()),
            "sized to the turn",
        ),
        (
            "Languages",
            crate::Field::Languages,
            desk.challenge_languages.clone().unwrap_or_default(),
            "python, javascript, rust, go",
        ),
    ] {
        paint.say_at(x, *y, name, Weight::Regular, size::BODY, ink.quiet);
        let focused = desk
            .editing
            .as_ref()
            .is_some_and(|(editing, _)| *editing == field);
        let text = if focused {
            desk.being_typed().to_owned()
        } else {
            now
        };
        if ui::field(
            paint,
            mouse,
            Box::new(x + 90.0, *y - 6.0, 160.0, 28.0),
            &text,
            placeholder,
            focused,
        ) {
            act = Some(Act::Edit(field));
        }
        *y += 34.0;
    }
    if let Some(why) = &desk.edit_refused {
        paint.say_at(x, *y, why, Weight::Regular, size::SMALL, ink.warn);
        *y += 18.0;
    }
    *y += 8.0;
    act
}

/// Whether this card's run has finished, with figures to read.
fn is_done(desk: &Desk, card: Card) -> bool {
    match (card, &desk.doing) {
        (Card::Throughput, Doing::Measuring(job))
        | (Card::CrossCheck, Doing::CrossChecking(job))
        | (Card::Capabilities, Doing::Probing(job)) => job.finished && job.refused.is_none(),
        (Card::Performance | Card::Fidelity | Card::Behaviour, Doing::Examining(job)) => {
            job.finished && job.refused.is_none()
        }
        (Card::Coding, Doing::Evaluating(job)) => {
            job.finished
                && job.refused.is_none()
                && job
                    .conclusion()
                    .and_then(|end| end.get("exit"))
                    .and_then(Value::as_integer)
                    == Some(0)
        }
        _ => false,
    }
}

/// *Done — figures on Statistics*, with the button that goes there, where
/// the card's run has finished (D50).
fn done_line(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    (x, y): (f32, f32),
    card: Card,
) -> Option<Act> {
    if !is_done(desk, card) {
        return None;
    }
    let ink = paint.ink;
    paint.say_at(
        x,
        y + 8.0,
        "Done — figures on the model's Statistics tab",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let (pressed, _) = ui::fitted(paint, mouse, (x + 300.0, y), "Statistics", Kind::Ordinary);
    pressed.then_some(Act::SeeStatistics)
}

/// One of the four smaller cards. Returns what was pressed and where the
/// card ends.
fn small_card(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
    card: Card,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let running = a_run_is_going(desk);
    let inner_x = at.x + PAD;
    let inner_w = at.w - 2.0 * PAD;
    // The card's height is known before it is drawn: the frame is painted
    // first and the rows go over it.
    let model = desk
        .chosen
        .and_then(|at| desk.models.get(at))
        .map_or_else(|| "<model>".to_owned(), |held| held.name.clone());
    let height =
        head_height(paint, card, at.w) + small_card_height(paint, desk, card, inner_w, &model);
    let frame = Box::new(at.x, at.y, at.w, height);
    let mut y = card_head(paint, frame, card);
    let mut act = None;
    match card {
        Card::CrossCheck => {
            let checking = matches!(&desk.doing, Doing::CrossChecking(job) if !job.finished);
            let (pressed, button) = ui::fitted(
                paint,
                mouse,
                (inner_x, y),
                run_label(desk, checking),
                if running && !checking {
                    Kind::Quiet
                } else {
                    Kind::Primary
                },
            );
            if pressed && checking {
                act = Some(Act::Stop);
            } else if pressed && !running {
                act = Some(download_then(desk, Act::Run(Card::CrossCheck)));
            }
            let (low, high) = desk.cross_check_estimate();
            paint.say_at(
                button.right() + 12.0,
                y + 8.0,
                &span(low, high),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            y += 44.0;
            if let Some(pressed) = done_line(paint, desk, mouse, (inner_x, y), card) {
                act = Some(pressed);
            }
            if let Doing::CrossChecking(job) = &desk.doing
                && !is_done(desk, card)
            {
                cross_check_progress(paint, job, Box::new(inner_x, y + 12.0, inner_w, 120.0));
            }
        }
        Card::Prompt => {
            let (pressed, _) = ui::fitted(paint, mouse, (inner_x, y), "Open", Kind::Primary);
            if pressed {
                act = Some(Act::Go(Page::Prompt));
            }
        }
        Card::Comparison => {
            if let Some(pressed) = command_rows(paint, mouse, (inner_x, y), inner_w, card, &model) {
                act = Some(pressed);
            }
        }
        // Every coding suite in one run of MCF's own `eval`, read as a job
        // (B-519); each suite is also a row of the list with its own Run.
        Card::Coding => {
            let evaluating = matches!(&desk.doing, Doing::Evaluating(job) if !job.finished);
            let (pressed, _) = ui::fitted(
                paint,
                mouse,
                (inner_x, y),
                run_label(desk, evaluating),
                if running && !evaluating {
                    Kind::Quiet
                } else {
                    Kind::Primary
                },
            );
            if pressed && evaluating {
                act = Some(Act::Stop);
            } else if pressed && !running {
                act = Some(Act::Run(Card::Coding));
            }
            y += 44.0;
            if let Some(pressed) = done_line(paint, desk, mouse, (inner_x, y), card) {
                act = Some(pressed);
            }
        }
        // The probes and the measurements are rows of the list, each shown
        // whole from its own pane (D53); the ladder has its own card.
        Card::Throughput
        | Card::Capabilities
        | Card::Performance
        | Card::Fidelity
        | Card::Behaviour => {}
    }
    (act, frame.bottom())
}

/// A step the daemon announced for the measurements, as one line:
/// *measurement 3 of 14: prefix-reuse* — the same words `mcf examine`
/// prints (A22).
fn measure_step_said(body: &Value) -> Option<String> {
    let step = body.get("step")?;
    let figure = |key: &str| step.get(key).and_then(Value::as_integer);
    let within = body.get("progress").map_or_else(String::new, |progress| {
        let held = |key: &str| progress.get(key).and_then(Value::as_integer).unwrap_or(0);
        format!(
            " — {} of {}, {}",
            held("done"),
            held("of"),
            progress.get("doing").and_then(Value::as_text).unwrap_or("")
        )
    });
    Some(format!(
        "measurement {} of {}: {}{within}",
        figure("count")?,
        figure("of")?,
        step.get("name").and_then(Value::as_text)?
    ))
}

/// A step the daemon announced for the probes, as one line: *probe 3 of
/// 9: stop-conditions* — the same words `mcf probe` prints (A22).
fn probe_step_said(body: &Value) -> Option<String> {
    let step = body.get("step")?;
    let figure = |key: &str| step.get(key).and_then(Value::as_integer);
    Some(format!(
        "probe {} of {}: {}",
        figure("count")?,
        figure("of")?,
        step.get("name").and_then(Value::as_text)?
    ))
}

/// A checkbox with its label. Returns whether it was pressed.
fn tick_box(paint: &mut Painter, mouse: &Mouse, (x, y): (f32, f32), label: &str, on: bool) -> bool {
    let ink = paint.ink;
    let square = Box::new(x, y - 1.0, 16.0, 16.0);
    paint.edge(
        square,
        4.0,
        if on { ink.accent } else { ink.line },
        ink.card,
    );
    if on {
        ui::tick(
            paint,
            Box::new(square.x + 3.0, square.y + 3.0, 10.0, 10.0),
            ink.accent,
        );
    }
    paint.say_at(x + 24.0, y, label, Weight::Regular, size::SMALL, ink.ink);
    let width = 24.0 + paint.measure(label, Weight::Regular, size::SMALL) + 8.0;
    mouse.clicked(Box::new(x - 4.0, y - 5.0, width, 24.0))
}

/// How tall a small card is, from what it shows.
fn small_card_height(
    paint: &mut Painter,
    desk: &Desk,
    card: Card,
    inner_w: f32,
    model: &str,
) -> f32 {
    match card {
        Card::CrossCheck => {
            let progress = match &desk.doing {
                Doing::CrossChecking(_) if !is_done(desk, card) => 120.0,
                _ => 0.0,
            };
            let done = if is_done(desk, card) { 34.0 } else { 0.0 };
            44.0 + progress + done + 12.0
        }
        Card::Prompt => 34.0 + 16.0,
        Card::Coding => {
            let done = if is_done(desk, card) { 34.0 } else { 0.0 };
            44.0 + done + 12.0
        }
        Card::Comparison => {
            let command = card.command(model).map_or(0.0, |command| {
                #[allow(clippy::cast_precision_loss, reason = "a line count")]
                let lines = paint
                    .wrap(&command, Weight::Regular, size::SMALL, inner_w)
                    .len() as f32;
                lines * 16.0
            });
            20.0 + command + 40.0 + 12.0
        }
        Card::Throughput
        | Card::Capabilities
        | Card::Performance
        | Card::Fidelity
        | Card::Behaviour => 0.0,
    }
}

/// The rows of a card whose run is at the command line for now: what the
/// probes last applied, the sentence saying where it runs, the command,
/// and Copy (B-478).
fn command_rows(
    paint: &mut Painter,
    mouse: &Mouse,
    (x, mut y): (f32, f32),
    wide: f32,
    card: Card,
    model: &str,
) -> Option<Act> {
    let ink = paint.ink;
    paint.say_at(
        x,
        y,
        "Runs at the command line for now:",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 20.0;
    let command = card.command(model)?;
    for line in paint.wrap(&command, Weight::Regular, size::SMALL, wide) {
        paint.say_at(x, y, &line, Weight::Regular, size::SMALL, ink.ink);
        y += 16.0;
    }
    let (pressed, _) = ui::fitted(paint, mouse, (x, y + 4.0), "Copy", Kind::Ordinary);
    pressed.then_some(Act::Copy(command))
}

/// Where the model lands — engine and device — as MCF resolved them, in the
/// words the Models page uses for the same two facts. Not pickers: a person
/// who wants them otherwise changes them where they are set. The device is
/// the line this page had nothing of: a run whose page does not say whether
/// it is timing a card or a processor is timing something the reader has to
/// guess at (A7, §3.4). Returns where the next row goes.
fn placement_rows(paint: &mut Painter, desk: &Desk, x: f32, mut y: f32, wide: f32) -> f32 {
    let ink = paint.ink;
    // A subject not here has no engine or device yet: MCF resolves a file
    // it holds, and the last model's would be a claim about this one (A7).
    let placed = if desk.pending.is_some() {
        None
    } else {
        desk.chosen.and_then(|at| desk.models.get(at))
    };
    for (label, value) in [
        ("Engine", placed.and_then(|held| held.engine.clone())),
        (
            "Device",
            placed.and_then(|held| {
                held.device.as_ref().map(|device| {
                    if held.on_a_card {
                        format!("{device} (all layers)")
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
        let column = label_column_of(wide, 190.0);
        let shown = paint.elide(&said, Weight::Regular, size::BODY, wide - column);
        paint.say_at(x + column, y, &shown, Weight::Regular, size::BODY, colour);
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
        None => "Auto".to_owned(),
        Some(mcf_serve::control::On::Processor) => "CPU".to_owned(),
        Some(mcf_serve::control::On::Card) => "GPU (all layers)".to_owned(),
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
        // The Configure tab's lists are drawn by the model page.
        Picker::Placement | Picker::Rope | Picker::Quantization => None,
        Picker::Architecture => {
            let mut labels = vec!["any".to_owned()];
            labels.extend(desk.architectures());
            let now = desk.filters.architecture.as_ref().and_then(|held| {
                desk.architectures()
                    .iter()
                    .position(|found| found == held)
                    .map(|at| at + 1)
            });
            ui::options(paint, mouse, at, &labels, now.or(Some(0))).map(Act::SetArchitecture)
        }
        Picker::Fits => {
            let labels: Vec<String> = crate::FITS_CHOICES
                .iter()
                .map(|fits| fits_label(*fits).to_owned())
                .collect();
            let now = crate::FITS_CHOICES
                .iter()
                .position(|fits| *fits == desk.filters.fits);
            ui::options(paint, mouse, at, &labels, now).map(Act::SetFits)
        }
        Picker::Size => {
            let labels: Vec<String> = crate::SIZE_CHOICES
                .iter()
                .map(|size| size_label(*size))
                .collect();
            let now = crate::SIZE_CHOICES
                .iter()
                .position(|size| *size == desk.filters.size);
            ui::options(paint, mouse, at, &labels, now).map(Act::SetSize)
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

/// One rung as it arrived: the depth, the figure or that there is none,
/// and how many pairs it was read off where fewer than all separated.
/// Returns where the next line goes, or nothing where this was no rung.
fn one_reading(paint: &mut Painter, answer: &Value, x: f32, mut y: f32, wide: f32) -> Option<f32> {
    let ink = paint.ink;
    let reading = answer.get("reading")?;
    let depth = reading
        .get("depth")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    paint.say_at(
        x,
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
    let column = label_column_of(wide, 190.0);
    paint.say_at(x + column, y, &said, Weight::Bold, size::BODY, colour);
    // A figure from one pair of three says so beside itself, quietly,
    // so it does not wear the look of one from three (A7, F174).
    let note = mcf_tui::screens::diagnostics::pairs_note(reading);
    if !note.is_empty() {
        let after = paint.measure(&said, Weight::Bold, size::BODY);
        let shown = paint.elide(
            &note,
            Weight::Regular,
            size::SMALL,
            (wide - column - after - 8.0).max(40.0),
        );
        paint.say_at(
            x + column + after + 8.0,
            y + 1.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    y += 22.0;
    Some(y)
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
        if let Some(next) = one_reading(paint, answer, area.x, y, area.w) {
            y = next;
        }
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
    spaced(paint, area.x, area.y, "add model", ink.faint);
    let mut y = area.y + 28.0;
    paint.say_at(
        area.x,
        y,
        "Search Hugging Face by name, or paste owner/repository or a repository URL.",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    y += 30.0;

    let mut act = None;
    let field = Box::new(area.x, y, (area.w - 130.0).max(160.0), 32.0);
    let _clicked = ui::field(
        paint,
        mouse,
        field,
        &desk.typed,
        "e.g. llama-3, owner/Model-GGUF, or a repository URL",
        true,
    );
    let (looked, _) = ui::fitted(
        paint,
        mouse,
        (field.right() + 10.0, y),
        "Search",
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
                Box::new(area.x, y, area.w, 8.0),
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
            .wrap(why, Weight::Regular, size::BODY, area.w)
            .iter()
            .take(3)
        {
            paint.say_at(area.x, y, line, Weight::Regular, size::BODY, ink.bad);
            y += 20.0;
        }
        return act;
    }
    let found = job.conclusion().or_else(|| job.latest())?;
    if found.get("repositories").is_some() {
        return searched(
            paint,
            mouse,
            Box::new(area.x, y, area.w, area.bottom() - y),
            found,
        )
        .or(act);
    }
    published(
        paint,
        mouse,
        Box::new(area.x, y, area.w, area.bottom() - y),
        found,
    )
    .or(act)
}

/// The repositories the hub listed for a word, one row each, most
/// downloaded first; pressing one looks it up.
fn searched(paint: &mut Painter, mouse: &Mouse, area: Box, found: &Value) -> Option<Act> {
    let ink = paint.ink;
    let query = found.get("query").and_then(Value::as_text).unwrap_or("");
    let listed = found
        .get("repositories")
        .and_then(Value::as_list)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();
    let wide = area.w;
    let mut y = area.y;
    if listed.is_empty() {
        paint.say_at(
            area.x,
            y,
            &format!("No repositories with GGUF files for {query:?}."),
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return None;
    }
    paint.say_at(
        area.x,
        y,
        &format!(
            "{} results for {query:?} · most downloaded first",
            listed.len()
        ),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 22.0;
    let mut act = None;
    for repository in &listed {
        if y > area.bottom() - 26.0 {
            break;
        }
        let Some(id) = repository.get("id").and_then(Value::as_text) else {
            continue;
        };
        let hit = Box::new(area.x - 6.0, y - 4.0, wide + 12.0, 24.0);
        if mouse.over(hit) {
            paint.panel(hit, 6.0, ink.line, 90);
        }
        let shown = paint.elide(id, Weight::Bold, size::BODY, wide - 150.0);
        paint.say_at(area.x, y, &shown, Weight::Bold, size::BODY, ink.ink);
        let downloads = repository
            .get("downloads")
            .and_then(Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
            .map_or_else(String::new, |count| {
                format!("{} downloads", words::grouped(count))
            });
        paint.say_right(
            area.x + wide,
            y,
            &downloads,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        if mouse.clicked(hit) {
            act = Some(Act::Pick(id.to_owned()));
        }
        y += 24.0;
    }
    act
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
    let wide = area.w;
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
    for (at, file) in files.iter().enumerate().take(12) {
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
        } else if mouse.clicked(where_) {
            // The row itself picks the file as the page's subject, not
            // downloaded, with the button that downloads and starts it
            // (B-487).
            act = Some(Act::PickOffered(at));
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
/// The box a question is typed into, the button that sends it, and the
/// turn it goes inside: the same switches `mcf run` takes, so what a person
/// can ask at the prompt they can ask here (A22, B-462). Returns what was
/// pressed and where the next thing goes.
fn ask_box(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
    held: &Model,
) -> (Option<Act>, f32) {
    let mut act = None;
    let field = Box::new(at.x, at.y, (at.w - 120.0).max(160.0), 32.0);
    if ui::field(
        paint,
        mouse,
        field,
        &desk.typed,
        "Message the model",
        desk.caret == Caret::Document,
    ) {
        act = Some(Act::Focus(Caret::Document));
    }
    let (asked, _) = ui::fitted(
        paint,
        mouse,
        (field.right() + 10.0, at.y),
        "Send",
        Kind::Primary,
    );
    if asked
        && !desk.doing.busy()
        && let Some(index) = desk
            .models
            .iter()
            .position(|listed| listed.path == held.path)
    {
        act = Some(Act::Ask { at: index });
    }
    let (turn_act, after) = the_turn(
        paint,
        desk,
        mouse,
        Box::new(at.x, at.y + 42.0, field.w, 0.0),
    );
    (act.or(turn_act), after + 10.0)
}

/// What the held engine is doing, as the engine counts it, before where
/// it answers: the tiles and the rate line. Returns where the next thing
/// goes; nothing is drawn where nothing is held.
fn in_use_block(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let Some(in_use) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
    else {
        return at.y;
    };
    let y = use_tiles(paint, in_use, Box::new(at.x, at.y, at.w, 0.0));
    let y = machine_and_run_tiles(paint, desk, Box::new(at.x, y + 8.0, at.w, 0.0));
    let y = rate_line(paint, &desk.rates, Box::new(at.x, y + 8.0, at.w, 48.0));
    y + 16.0
}

/// The server the daemon holds for a run, as a hosted one is shown: its
/// name, engine and window, its counters as tiles, and what the run has
/// spent — the card's energy summed a second at a time, the tokens it
/// produced meanwhile, and tokens a kilojoule (B-573). Nothing where no
/// run holds a server; returns where the next block begins.
fn under_test_block(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let Some(under) = desk.under_test.as_ref() else {
        return at.y;
    };
    let ink = paint.ink;
    spaced(paint, at.x, at.y, "under test", ink.faint);
    let heading = match under.window {
        Some(window) => format!(
            "{} · {} · window {}",
            under.name(),
            under.engine,
            words::grouped(window)
        ),
        None => format!("{} · {}", under.name(), under.engine),
    };
    let shown = paint.elide(&heading, Weight::Bold, size::BODY, at.w);
    paint.say_at(at.x, at.y + 24.0, &shown, Weight::Bold, size::BODY, ink.ink);
    let y = use_tiles(paint, &under.in_use, Box::new(at.x, at.y + 50.0, at.w, 0.0));
    let y = machine_and_run_tiles(paint, desk, Box::new(at.x, y + 8.0, at.w, 0.0));
    y + 22.0
}

/// What the held engine is doing, as tiles: a short label over a figure,
/// two rows of five. A figure the engine did not publish is drawn as
/// unmeasured, not as nought (A7).
fn use_tiles(paint: &mut Painter, in_use: &crate::Use, at: Box) -> f32 {
    let rate = |held: Option<f32>| held.map(|rate| format!("{rate:.1}"));
    let count = |held: Option<u64>| held.map(words::grouped);
    let bytes = |held: Option<u64>| held.map(gigabytes);
    let tiles: Vec<(&str, Option<String>)> = vec![
        ("Gen tok/s", rate(in_use.generated_per_second)),
        ("Prompt tok/s", rate(in_use.prompted_per_second)),
        ("Tokens out", count(in_use.generated)),
        ("Tokens in", count(in_use.prompted)),
        (
            "KV cache",
            in_use
                .cache_used
                .map(|ratio| format!("{:.0}%", (ratio * 100.0).clamp(0.0, 100.0))),
        ),
        ("KV tokens", count(in_use.cache_tokens)),
        ("Decodes", count(in_use.decodes)),
        ("Active", count(in_use.processing)),
        ("Queued", count(in_use.queued)),
        ("RAM", bytes(in_use.resident)),
        ("VRAM", bytes(in_use.card)),
        ("Uptime", in_use.uptime_seconds.map(crate::ago_said)),
    ];
    tiles_of(paint, &tiles, at)
}

/// The machine and the run beside the engine, a tile each: what the card
/// is doing and drawing now, and what the run going has spent — energy,
/// tokens, tokens a kilojoule, time (B-581, B-573). A figure nobody read is
/// drawn as unmeasured, not as nought (A7).
fn machine_and_run_tiles(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let card = desk.reading.cards.first();
    let spent = desk.spent;
    let mut tiles: Vec<(&str, Option<String>)> = vec![
        (
            "Card load",
            card.and_then(|card| card.load.map(|load| format!("{}%", load.whole()))),
        ),
        (
            "Card temp",
            card.and_then(|card| card.temperature.map(|degrees| format!("{degrees} °C"))),
        ),
        (
            "Card power",
            card.and_then(|card| card.power.map(|watts| format!("{watts} W"))),
        ),
        (
            "Card memory",
            card.and_then(|card| card.used.map(gigabytes)),
        ),
    ];
    if spent.seconds > 0 {
        #[expect(
            clippy::integer_division,
            reason = "whole kilojoules is the unit shown"
        )]
        let kilojoules = spent.millijoules / 1_000_000;
        tiles.push((
            "Run energy",
            (spent.millijoules > 0).then(|| format!("{} kJ", words::grouped(kilojoules))),
        ));
        tiles.push(("Run tokens", spent.tokens().map(words::grouped)));
        tiles.push((
            "Tokens/kJ",
            spent.tokens_per_kilojoule().map(words::grouped),
        ));
        tiles.push(("Run time", Some(clock(spent.seconds))));
    }
    tiles_of(paint, &tiles, at)
}

/// Tiles in rows of five: a short label over a figure, each its own box;
/// returns where the next block begins (B-581).
fn tiles_of(paint: &mut Painter, tiles: &[(&str, Option<String>)], at: Box) -> f32 {
    let ink = paint.ink;
    let across = (at.w - 4.0 * 10.0) / 5.0;
    let mut y = at.y;
    for (index, (label, figure)) in tiles.iter().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "a few tiles: the index is never large enough to lose one"
        )]
        let column = (index % 5) as f32;
        if index > 0 && index % 5 == 0 {
            y += 66.0;
        }
        let tile = Box::new(at.x + (across + 10.0) * column, y, across, 58.0);
        ui::card(paint, tile, false);
        spaced(paint, tile.x + 12.0, tile.y + 12.0, label, ink.faint);
        let (said, colour) = figure.as_ref().map_or_else(
            || (words::UNMEASURED.to_owned(), ink.faint),
            |said| (said.clone(), ink.ink),
        );
        let shown = paint.elide(&said, Weight::Bold, size::HEAD, across - 24.0);
        paint.say_at(
            tile.x + 12.0,
            tile.y + 28.0,
            &shown,
            Weight::Bold,
            size::HEAD,
            colour,
        );
    }
    y + 66.0
}

/// The predicting rate over the last two minutes, one bar a second, newest
/// at the right, against the highest seen in that time.
fn rate_line(paint: &mut Painter, rates: &std::collections::VecDeque<f32>, at: Box) -> f32 {
    let ink = paint.ink;
    let peak = rates.iter().copied().fold(0.0_f32, f32::max);
    spaced(paint, at.x, at.y, "gen tok/s · last 2 min", ink.faint);
    let plot = Box::new(at.x, at.y + 16.0, at.w, at.h - 16.0);
    paint.edge(plot, 6.0, ink.line, ink.card);
    if peak <= 0.0 {
        paint.say_at(
            plot.x + 12.0,
            plot.y + 8.0,
            "no generation yet",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return plot.bottom();
    }
    let slot = (plot.w - 8.0) / 120.0;
    let mut x = plot.right() - 4.0;
    for rate in rates.iter().rev() {
        let tall = ((rate / peak) * (plot.h - 8.0)).max(1.0);
        paint.rect(
            Box::new(
                x - slot + 1.0,
                plot.bottom() - 4.0 - tall,
                (slot - 2.0).max(1.0),
                tall,
            ),
            ink.accent,
        );
        x -= slot;
        if x < plot.x + 4.0 {
            break;
        }
    }
    paint.say_right(
        at.right(),
        at.y - 2.0,
        &format!("peak {peak:.1}"),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    plot.bottom()
}

/// The state of the hold, on the page a person asks the model from: where
/// it answers, with a button that copies the address and a line saying what
/// the address is; or how far the load has got; or the engine being built
/// first; or why the last hold was refused; or what the last stop freed.
/// Returns what was pressed and where the next thing goes. The ask box
/// follows either way: a question goes through MCF's own engine, loaded for
/// it, whether or not the model is held on a port.
fn held_block(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut y = at.y;
    let this_one = desk
        .hosted
        .as_ref()
        .zip(desk.chosen.and_then(|held| desk.models.get(held)))
        .filter(|(hosting, held)| hosting.model == held.path)
        .map(|(hosting, _)| hosting);
    if let Some(hosting) = this_one {
        return where_it_answers(paint, mouse, hosting, desk.hosted_model(), at);
    }
    let said: (String, Rgb) = if let Some(loading) = desk.loading_line() {
        (loading, ink.quiet)
    } else if let Doing::Provisioning(job) = &desk.doing
        && !job.finished
    {
        building(paint, job, at.x, y, at.w);
        return (None, y + 96.0);
    } else if let Some(why) = &desk.host_refused {
        (why.clone(), ink.bad)
    } else if let Some(freed) = &desk.freed {
        (freed.clone(), ink.quiet)
    } else {
        (
            "No server for this model: messages here run through MCF's own engine, loaded per \
             message. Start a server from Configure to serve it over HTTP."
                .to_owned(),
            ink.faint,
        )
    };
    for line in paint
        .wrap(&said.0, Weight::Regular, size::BODY, at.w)
        .iter()
        .take(4)
    {
        paint.say_at(at.x, y, line, Weight::Regular, size::BODY, said.1);
        y += 20.0;
    }
    (None, y + 8.0)
}

/// Where a held model answers: the address with a button that copies it,
/// what the address is, whether a key guards it, its window and since when,
/// and what reaches it.
fn where_it_answers(
    paint: &mut Painter,
    mouse: &Mouse,
    hosting: &crate::Hosted,
    model: Option<&Model>,
    at: Box,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut y = at.y;
    let mut act = None;
    paint.say_at(at.x, y, "Endpoint", Weight::Regular, size::SMALL, ink.quiet);
    let after = paint.measure("Endpoint", Weight::Regular, size::SMALL);
    paint.say_at(
        at.x + after + 8.0,
        y - 1.0,
        &hosting.address,
        Weight::Bold,
        size::BODY,
        ink.accent,
    );
    let address_wide = paint.measure(&hosting.address, Weight::Bold, size::BODY);
    let (copied, _) = ui::fitted(
        paint,
        mouse,
        (at.x + after + address_wide + 22.0, y - 8.0),
        "Copy",
        Kind::Quiet,
    );
    if copied {
        act = Some(Act::Copy(hosting.address.clone()));
    }
    y += 24.0;
    // Where the network reaches it, where the hold is open to it (B-577).
    if let Some(network) = &hosting.network_address {
        paint.say_at(
            at.x,
            y,
            "On the network",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        let after = paint.measure("On the network", Weight::Regular, size::SMALL);
        paint.say_at(
            at.x + after + 8.0,
            y - 1.0,
            network,
            Weight::Bold,
            size::BODY,
            ink.accent,
        );
        let wide = paint.measure(network, Weight::Bold, size::BODY);
        let (copied, _) = ui::fitted(
            paint,
            mouse,
            (at.x + after + wide + 22.0, y - 8.0),
            "Copy",
            Kind::Quiet,
        );
        if copied {
            act = Some(Act::Copy(network.clone()));
        }
        y += 24.0;
    }
    for line in [
        "OpenAI-compatible API · use as base URL".to_owned(),
        if hosting.api_key && hosting.network_address.is_some() {
            "API key: set · required, since the hold answers the network".to_owned()
        } else if hosting.api_key {
            "API key: set".to_owned()
        } else {
            "API key: none (localhost only)".to_owned()
        },
        format!(
            "Context {}   ·   Started {}",
            hosting.context.map_or_else(
                || UNKNOWN.to_owned(),
                |context| format!("{} tokens", words::grouped(context))
            ),
            hosting.since
        ),
        // What the held window costs, where the model is listed here and
        // its cache per token is known (§3.15).
        model
            .zip(hosting.context)
            .and_then(|(model, context)| reserve_line(model, context))
            .unwrap_or_default(),
        takes_line(hosting),
    ] {
        if line.is_empty() {
            continue;
        }
        let shown = paint.elide(&line, Weight::Regular, size::SMALL, at.w);
        paint.say_at(at.x, y, &shown, Weight::Regular, size::SMALL, ink.quiet);
        y += 17.0;
    }
    (act, y + 12.0)
}

fn hosting(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    scrolled(
        paint,
        mouse,
        desk,
        Region::Server,
        area,
        |paint, mouse, inner| hosting_body(paint, desk, mouse, inner),
    )
}

/// The Server page's content, from the top of its region.
fn hosting_body(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    // The model under test first, where a run holds one: what it is doing
    // and what the run has cost, the way a hosted model is shown (B-573).
    let top = under_test_block(paint, desk, Box::new(area.x, area.y, area.w, 0.0));
    let area = Box::new(area.x, top, area.w, (area.h - (top - area.y)).max(0.0));
    // The held model first, whatever is chosen on Models; the chosen one
    // where nothing is held, because a question can still be asked of it
    // through MCF's own engine.
    let held = desk
        .hosted_model()
        .or_else(|| desk.chosen.and_then(|at| desk.models.get(at)));
    // A model can be held that this list does not carry — the daemon's
    // store moved under it, or another client held it — and it is still
    // running: its name comes from the hold, and there is no ask box for
    // it because the box needs the model to be listed.
    let name = match (held, desk.hosted.as_ref()) {
        (Some(held), _) => held.name.clone(),
        (None, Some(hosting)) => hosting.name(),
        (None, None) => {
            spaced(paint, area.x, area.y, "server", ink.faint);
            paint.say_at(
                area.x,
                area.y + 28.0,
                "Nothing is held. Models — choose one — Configure — Host.",
                Weight::Regular,
                size::BODY,
                ink.quiet,
            );
            return None;
        }
    };
    spaced(paint, area.x, area.y, "server", ink.faint);
    let name = paint.elide(&name, Weight::Bold, size::HEAD, area.w);
    paint.say_at(
        area.x,
        area.y + 24.0,
        &name,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    let mut y = in_use_block(paint, desk, Box::new(area.x, area.y + 62.0, area.w, 0.0));

    let mut act = None;
    let Some(held) = held else {
        // Held and not listed: where it answers, and nothing to ask it with.
        if let Some(hosting) = desk.hosted.as_ref() {
            let (pressed, _) = where_it_answers(
                paint,
                mouse,
                hosting,
                None,
                Box::new(area.x, y, area.w, 0.0),
            );
            act = pressed;
        }
        return act;
    };
    // What is held, or how the hold is going, or why it is not: the one
    // place for it, before anything can be asked (A2, A7).
    let (held_act, after) = held_block(paint, desk, mouse, Box::new(area.x, y, area.w, 0.0));
    act = held_act.or(act);
    y = after;
    let (asked, after) = ask_box(paint, desk, mouse, Box::new(area.x, y, area.w, 0.0), held);
    act = asked.or(act);
    y = after;

    if let Doing::Answering(job) = &desk.doing
        && let Some(why) = &job.refused
    {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w)
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
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, area.w);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::SMALL, ink.quiet);
        y += 16.0;
    }
    let panel = Box::new(area.x, y, area.w, (area.bottom() - y).max(60.0));
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
        // **Which generation, of how many.** The daemon says before each
        // what it is about to ask — *generation 3 of 12: without part 2 of
        // 4* — and the page says the same, with a bar of the count and the
        // seconds so far; before the first is announced it says it is
        // waiting for that, not a sentence about generations in general
        // (B-479, A7).
        let step = job
            .latest()
            .and_then(mcf_serve::prompt::step_said)
            .unwrap_or_else(|| {
                "waiting for the daemon to say which generation it is on".to_owned()
            });
        paint.say_at(
            area.x,
            at + 20.0,
            &step,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        ui::progress(
            paint,
            Box::new(area.x, at + 42.0, area.w, 8.0),
            job.latest().and_then(step_fraction),
        );
        paint.say_at(
            area.x,
            at + 58.0,
            &format!("{} s so far", job.ran()),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return None;
    }
    let job = desk.doing.job()?;
    if let Some(why) = &job.refused {
        for line in paint
            .wrap(why, Weight::Regular, size::BODY, area.w)
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

/// How far a report is, as the generation announced of those planned;
/// `None` where the daemon has not said (A7).
fn step_fraction(latest: &Value) -> Option<f32> {
    let step = latest.get("step")?;
    let figure = |key: &str| {
        step.get(key)
            .and_then(Value::as_integer)
            .and_then(|found| u16::try_from(found).ok())
    };
    let (count, of) = (figure("count")?, figure("of")?);
    if of == 0 {
        return None;
    }
    // A generation index is a small count, exact in f32.
    Some((f32::from(count.saturating_sub(1)) / f32::from(of)).clamp(0.0, 1.0))
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
    // While a report runs, the button that started it is the one that cuts
    // it short (B-479, B-468).
    let running = matches!(&desk.doing, Doing::Reporting(job) if !job.finished);
    let (asked, button) = ui::fitted(
        paint,
        mouse,
        (area.x, under),
        if running { "Stop" } else { "Analyse" },
        Kind::Primary,
    );
    if asked && running {
        act = Some(Act::Stop);
    } else if asked && !desk.doing.busy() && desk.chosen.is_some() {
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
        area.w,
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
    let offset = desk.scrolled(Region::Prompt);
    let seen = &mouse.within(body);
    paint.clip(body);
    paint.mark();
    let pressed = prompt_report(paint, desk, seen, area, top - offset);
    let content = (paint.lowest() - (top - offset)).max(0.0);
    paint.unclip();
    let moved = ui::scroll_region(paint, mouse, body, offset, content);
    pressed
        .or(moved.map(|to| Act::Scroll(Region::Prompt, whole(to))))
        .or(act)
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
    let wide = area.w;
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
    let title = paint.elide(&title, Weight::Regular, size::SMALL, area.w);
    spaced(paint, area.x, area.y, &title, ink.faint);
    let room = area.w;
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
    let field = Box::new(area.x, area.y + 52.0, area.w, height);
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
    let wide = area.w;
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
    anatomy_body(
        paint,
        Box::new(area.x, top, area.w, area.bottom() - top),
        said,
    );
    switched
}

/// The tensor directory's figures, agreements, arithmetic and shares.
fn anatomy_body(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) {
    let top = area.y;
    // Two columns: the figures and the header check on the left, the tables
    // on the right. The window does not scroll, so a table that does not fit
    // is cut at a row that says so rather than drawn over the edge.
    // A narrow pane stacks the two columns instead of squeezing them.
    let stacked = area.w < 900.0;
    let figures = if stacked {
        area.w
    } else {
        460.0_f32.min(area.w / 2.0)
    };
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
    let after = arithmetic(
        paint,
        Box::new(area.x, after + 24.0, figures, area.bottom() - after - 24.0),
        said,
    );
    let right = if stacked {
        Box::new(
            area.x,
            after + 32.0,
            area.w,
            (area.bottom() - after - 32.0).max(0.0),
        )
    } else {
        Box::new(
            area.x + figures + 40.0,
            top,
            (area.w - figures - 40.0).max(200.0),
            area.bottom() - top,
        )
    };
    if right.h < 40.0 {
        return;
    }
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
    for line in paint.wrap(&why, Weight::Regular, size::BODY, area.w) {
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
    vocabulary_body(
        paint,
        Box::new(area.x, top, area.w, area.bottom() - top),
        said,
    );
    switched
}

/// The vocabulary's figures, its template, and its named tokens.
fn vocabulary_body(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) {
    let top = area.y;
    let spoken = &said.vocabulary;
    let stacked = area.w < 900.0;
    let figures = if stacked {
        area.w
    } else {
        560.0_f32.min(area.w / 2.0)
    };
    let left = Box::new(area.x, top, figures, area.bottom() - top);
    let after = spoken_figures(paint, left, spoken);
    let after = template(
        paint,
        Box::new(area.x, after + 24.0, figures, area.bottom() - after - 24.0),
        spoken,
    );
    let right = if stacked {
        Box::new(
            area.x,
            after + 32.0,
            area.w,
            (area.bottom() - after - 32.0).max(0.0),
        )
    } else {
        Box::new(
            area.x + figures + 40.0,
            top,
            (area.w - figures - 40.0).max(200.0),
            area.bottom() - top,
        )
    };
    if right.h >= 40.0 {
        named_tokens(paint, right, spoken);
    }
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
