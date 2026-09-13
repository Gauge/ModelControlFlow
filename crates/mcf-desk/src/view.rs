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
use crate::{Act, Caret, Desk, Doing, Model, Page, Picker, Region, Splitter, windows};
use mcf_record::json::Value;
use mcf_serve::anatomy::SaidVocabulary;

const PAD: f32 = 26.0;

mod size {
    pub(super) const LABEL: f32 = 9.5;
    pub(super) const SMALL: f32 = 12.0;
    pub(super) const BODY: f32 = 13.5;
    pub(super) const HEAD: f32 = 17.0;
}

pub const UNKNOWN: &str = "Unknown";

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
        Page::Host | Page::Models => host(paint, desk, mouse, main),
        Page::Adding => adding(paint, desk, mouse, main),
        Page::Hosting => hosting(paint, desk, mouse, main),
        Page::Anatomy => anatomy(paint, desk, mouse, main),
        Page::Vocabulary => vocabulary(paint, desk, mouse, main),
        Page::Exit => leaving(paint, mouse, main),
    };
    act = match (went, act) {
        (Some(Act::Scroll(..)), Some(pressed)) => Some(pressed),
        (went, act) => went.or(act),
    };
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

fn whole(at: f32) -> i32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a point on the screen, far inside i32"
    )]
    let rounded = at.round() as i32;
    rounded
}

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
    let mine = crate::Desk::build_said();
    paint.say_at(18.0, 38.0, &mine, Weight::Regular, size::SMALL, ink.faint);
    if let Some(theirs) = desk.daemon_build.as_ref().filter(|held| **held != mine) {
        paint.say_at(
            18.0,
            54.0,
            &format!("daemon {theirs}"),
            Weight::Regular,
            size::SMALL,
            ink.accent,
        );
    }

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

struct Column {
    head: &'static str,
    at: f32,
    right: bool,
}

const TABLE_DESIGNED_FOR: f32 = 700.0;

fn column_edge(area: Box, at: f32) -> f32 {
    area.x + at * (area.w / TABLE_DESIGNED_FOR).max(1.0)
}

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

fn spaced(paint: &mut Painter, x: f32, y: f32, text: &str, colour: Rgb) {
    let shouted = text.to_uppercase();
    let mut pen = x;
    for letter in shouted.chars() {
        let one = letter.to_string();
        paint.say_at(pen, y, &one, Weight::Bold, size::LABEL, colour);
        pen += paint.measure(&one, Weight::Bold, size::LABEL) + 0.7;
    }
}

fn spaced_width(paint: &mut Painter, text: &str) -> f32 {
    text.to_uppercase()
        .chars()
        .map(|letter| paint.measure(&letter.to_string(), Weight::Bold, size::LABEL) + 0.7)
        .sum()
}

#[must_use]
pub fn reserve_of(
    held: &Model,
    context: u64,
    width: mcf_core::configuration::CacheType,
) -> Option<(u64, Option<u64>)> {
    let per_token = held
        .cache_elements_per_token
        .and_then(|elements| width.bytes_for(elements))
        .or(held.cache_per_token)?;
    let cache = per_token.saturating_mul(context);
    Some((cache, held.bytes.map(|held| held.saturating_add(cache))))
}

#[must_use]
pub fn gigabytes(bytes: u64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a memory figure shown to one decimal place"
    )]
    let held = bytes as f64 / 1e9;
    format!("{held:.1} GB")
}

#[must_use]
pub fn reserve_line(
    held: &Model,
    context: u64,
    width: mcf_core::configuration::CacheType,
) -> Option<String> {
    let (cache, total) = reserve_of(held, context, width)?;
    Some(match total {
        Some(total) => format!(
            "KV cache {} · total {} with weights",
            gigabytes(cache),
            gigabytes(total)
        ),
        None => format!("KV cache {}", gigabytes(cache)),
    })
}

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

fn host(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let list = desk.splits.list.min((area.w - 300.0).max(150.0));
    let right = area.x + list + 40.0;
    let mut act = None;
    let actions_at = area.bottom() - 200.0;
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
    if desk.removing.is_some() {
        return removal_page(paint, desk, mouse, pane).or(act);
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

fn removal_done(paint: &mut Painter, mouse: &Mouse, said: &str, area: Box, y: f32) -> Option<Act> {
    let ink = paint.ink;
    let mut y = y;
    let wide = (area.w - 20.0).max(160.0);
    for line in paint.wrap(said, Weight::Regular, size::BODY, wide) {
        paint.say_at(area.x, y, &line, Weight::Regular, size::BODY, ink.ink);
        y += 20.0;
    }
    ui::button(
        paint,
        mouse,
        Box::new(area.x, y + 12.0, 160.0, ui::BUTTON),
        "Done",
        Kind::Primary,
    )
    .then_some(Act::CancelRemove)
}

fn what_would_go(paint: &mut Painter, removing: &crate::Removing, area: Box, y: f32) -> f32 {
    let ink = paint.ink;
    let mut y = y;
    let wide = (area.w - 20.0).max(160.0);
    if removing.files.is_empty() {
        return y;
    }
    let total = removing
        .bytes
        .map_or_else(|| "an unstatable total".to_owned(), gigabytes);
    paint.say_at(
        area.x,
        y,
        &format!("{} file(s), {total}", removing.files.len()),
        Weight::Bold,
        size::SMALL,
        ink.ink,
    );
    y += 22.0;
    for gone in removing.files.iter().take(6) {
        let shown = paint.elide(&gone.path, Weight::Regular, size::SMALL, wide - 90.0);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::SMALL, ink.faint);
        paint.say_at(
            area.right() - 90.0,
            y,
            &gigabytes(gone.bytes),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        y += 18.0;
    }
    if removing.files.len() > 6 {
        paint.say_at(
            area.x,
            y,
            &format!("and {} more", removing.files.len().saturating_sub(6)),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        y += 18.0;
    }
    y + 12.0
}

fn removal_page(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let removing = desk.removing.as_ref()?;
    let mut act = None;
    let wide = (area.w - 20.0).max(160.0);
    let name = paint.elide(&removing.name, Weight::Bold, size::HEAD, wide);
    paint.say_at(area.x, area.y, &name, Weight::Bold, size::HEAD, ink.ink);
    let mut y = area.y + 36.0;

    if let Some(said) = &removing.done {
        return removal_done(paint, mouse, said, area, y);
    }

    let ending = if removing.purge {
        "This deletes the files. Nothing is shelved and nothing comes back."
    } else if removing.reversible {
        "The files move to the shelf below. Nothing is deleted, and they can be moved back."
    } else {
        "The shelf is on another filesystem, so this copies rather than moves, and cannot be \
         undone in place."
    };
    for line in paint.wrap(ending, Weight::Regular, size::BODY, wide) {
        paint.say_at(area.x, y, &line, Weight::Regular, size::BODY, ink.quiet);
        y += 20.0;
    }
    y += 10.0;

    y = what_would_go(paint, removing, area, y);

    paint.say_at(area.x, y, "Why", Weight::Regular, size::BODY, ink.quiet);
    y += 22.0;
    let touched = ui::field(
        paint,
        mouse,
        Box::new(area.x, y, wide.min(420.0), 28.0),
        &removing.reason,
        "what this is making room for",
        true,
    );
    if touched != ui::Touched::No {
        act = Some(Act::RemoveReason(touched));
    }
    y += 42.0;

    let square = Box::new(area.x, y, 20.0, 20.0);
    if ui::check(paint, mouse, square, removing.purge) {
        act = Some(Act::PurgeToggle);
    }
    paint.say_at(
        area.x + 30.0,
        y + 2.0,
        "Delete the files instead of shelving them",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    y += 36.0;

    if let Some(why) = &removing.refused {
        for line in paint.wrap(why, Weight::Regular, size::SMALL, wide) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.warn);
            y += 16.0;
        }
        y += 8.0;
    }

    let doing = if removing.purge {
        "Delete it"
    } else {
        "Shelve it"
    };
    if ui::button(
        paint,
        mouse,
        Box::new(area.x, y, 160.0, ui::BUTTON),
        doing,
        Kind::Primary,
    ) {
        act = Some(Act::DoRemove);
    }
    if ui::button(
        paint,
        mouse,
        Box::new(area.x + 172.0, y, 120.0, ui::BUTTON),
        "Keep it",
        Kind::Quiet,
    ) {
        act = Some(Act::CancelRemove);
    }
    act
}

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

fn no_settings(paint: &mut Painter, desk: &Desk, area: Box, why: &str) {
    let ink = paint.ink;
    spaced(paint, area.x, area.y, "settings", ink.faint);
    let lines = paint.wrap(why, Weight::Regular, size::SMALL, area.w);
    let mut y = area.y + 24.0;
    for line in lines.iter().take(3) {
        paint.say_at(area.x, y, line, Weight::Regular, size::SMALL, ink.warn);
        y += 16.0;
    }
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
            crate::Tab::Optimize => optimize_tab(paint, desk, mouse, inner),
            crate::Tab::Statistics => {
                statistics_tab(paint, inner, held);
                None
            }
            crate::Tab::Contents => contents_tab(paint, desk, mouse, inner),
        },
    );
    drawn.or(act)
}

fn section(paint: &mut Painter, area: Box, y: f32, title: &str, because: &str) -> f32 {
    let ink = paint.ink;
    paint.say_at(area.x, y, title, Weight::Bold, size::BODY, ink.ink);
    let titled = paint.measure(title, Weight::Bold, size::BODY);
    let room = area.w - titled - 18.0;
    let shown = paint.elide(because, Weight::Regular, size::SMALL, room.max(20.0));
    paint.say_at(
        area.x + titled + 12.0,
        y + 2.0,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y + 26.0
}

fn chips(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    y: f32,
    labels: &[(String, bool)],
) -> (f32, Option<usize>) {
    let mut picked = None;
    let mut x = area.x;
    let mut line = y;
    for (at, (label, on)) in labels.iter().enumerate() {
        let wide = paint.measure(label, Weight::Bold, size::SMALL) + 24.0;
        if x + wide > area.right() {
            x = area.x;
            line += 34.0;
        }
        if ui::nav(paint, mouse, Box::new(x, line, wide, 28.0), label, *on) {
            picked = Some(at);
        }
        x += wide + 6.0;
    }
    (line + 38.0, picked)
}

fn base_configuration(paint: &mut Painter, desk: &Desk, area: Box, mut y: f32) -> f32 {
    let ink = paint.ink;
    y = section(
        paint,
        area,
        y,
        "Base configuration",
        "every trial runs against exactly this — MCF holds the model itself",
    );
    let Some(held) = desk.chosen.and_then(|at| desk.models.get(at)) else {
        paint.say_at(
            area.x,
            y,
            "Choose a model on the left and its settings become the base here.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return y + 30.0;
    };
    let Some(settings) = &desk.settings else {
        paint.say_at(
            area.x,
            y,
            "MCF has not worked out what this model would run under yet.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return y + 30.0;
    };
    let dialled = desk.optimizing.sweep.dial;
    let rows: [(&str, String, bool); 6] = [
        ("model", held.name.clone(), false),
        ("engine", settings.engine.clone(), false),
        (
            "context window",
            format!("{} tokens", settings.context),
            false,
        ),
        (
            "micro-batch",
            settings.ubatch.to_string(),
            dialled == mcf_optimize::dial::Dial::MicroBatch,
        ),
        ("key cache", format!("{:?}", settings.cache), false),
        (
            "draft depth",
            settings
                .started
                .drafted
                .map_or_else(|| "off".to_owned(), |held| held.to_string()),
            dialled == mcf_optimize::dial::Dial::DraftDepth,
        ),
    ];
    for (name, value, moving) in rows {
        paint.say_at(area.x, y, name, Weight::Regular, size::SMALL, ink.faint);
        let said = if moving {
            format!("{value} — the sweep moves this")
        } else {
            value
        };
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, area.w - 150.0);
        paint.say_at(
            area.x + 140.0,
            y,
            &shown,
            Weight::Regular,
            size::SMALL,
            if moving { ink.accent } else { ink.ink },
        );
        y += 20.0;
    }
    y + 12.0
}

fn how_it_searches(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let dial = desk.optimizing.sweep.dial;
    let span = dial.span();

    paint.say_at(
        area.x,
        y + 7.0,
        "Search",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let ways: Vec<(String, bool)> = mcf_optimize::hunt::Way::ALL
        .iter()
        .map(|way| (way.label().to_owned(), *way == desk.optimizing.way))
        .collect();
    let inset = Box::new(area.x + 110.0, area.y, (area.w - 110.0).max(120.0), area.h);
    let (below, picked) = chips(paint, mouse, inset, y, &ways);
    if let Some(at) = picked {
        act = Some(Act::SweepWay(at));
    }
    y = below;

    paint.say_at(
        area.x,
        y + 7.0,
        "Ranked by",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let measures: Vec<(String, bool)> = mcf_optimize::reading::Measure::ALL
        .iter()
        .map(|measure| {
            (
                measure.label().to_owned(),
                *measure == desk.optimizing.measure,
            )
        })
        .collect();
    let (below, picked) = chips(paint, mouse, inset, y, &measures);
    if let Some(at) = picked {
        act = Some(Act::SweepMeasure(at));
    }
    y = below + 4.0;

    let automatic = desk.optimizing.way == mcf_optimize::hunt::Way::Halving;
    let said = if automatic {
        format!(
            "Starts at {}, then halves the gap around whichever wins, down to steps of {}.",
            dial.coarse()
                .iter()
                .map(|step| step.said())
                .collect::<Vec<_>>()
                .join(", "),
            dial.step_of(span.finest).said()
        )
    } else {
        format!(
            "Runs the values below and nothing else, anywhere from {} to {}.",
            dial.step_of(span.floor).said(),
            dial.step_of(span.ceiling).said()
        )
    };
    for line in paint.wrap(&said, Weight::Regular, size::SMALL, area.w - 20.0) {
        paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
        y += 16.0;
    }
    (y + 10.0, act)
}

fn setting_to_optimize(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    y = section(
        paint,
        area,
        y,
        "Setting to optimize",
        "one dial moves, everything above stays where it is",
    );
    let offered = desk.dials_worth_offering();
    let dials: Vec<(String, bool)> = offered
        .iter()
        .map(|dial| (dial.label().to_owned(), *dial == desk.optimizing.sweep.dial))
        .collect();
    let (below, picked) = chips(paint, mouse, area, y, &dials);
    if let Some(at) = picked {
        act = Some(Act::Dial(at));
    }
    y = below;
    let dial = desk.optimizing.sweep.dial;
    let how = if dial.is_named_by_the_model() {
        desk.declared
            .as_ref()
            .map_or_else(String::new, |held| held.thinking.said())
    } else if dial.reloads_the_engine() {
        format!(
            "{} is a launch flag, so MCF holds the model again for each value.",
            dial.flag().unwrap_or("")
        )
    } else {
        format!(
            "{} rides in each request, so the model is held once for the whole sweep.",
            dial.field().unwrap_or("")
        )
    };
    for line in paint.wrap(&how, Weight::Regular, size::SMALL, area.w - 20.0) {
        paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
        y += 16.0;
    }
    y += 10.0;
    let (below, chosen) = how_it_searches(paint, desk, mouse, area, y);
    act = act.or(chosen);
    y = below;
    if desk.optimizing.way == mcf_optimize::hunt::Way::Halving
        && !desk.optimizing.sweep.dial.is_named_by_the_model()
    {
        return (y, act);
    }
    let (below, picked) = the_values_by_hand(paint, desk, mouse, area, y);
    (below, act.or(picked))
}

fn the_values_by_hand(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let mut act = None;
    let dial = desk.optimizing.sweep.dial;
    let offered: Vec<mcf_optimize::dial::Step> = if dial.is_named_by_the_model() {
        (0..desk.optimizing.named.len())
            .filter_map(|at| u32::try_from(at).ok())
            .map(mcf_optimize::dial::Step::Whole)
            .collect()
    } else {
        dial.suggested()
    };
    let values: Vec<(String, bool)> = offered
        .iter()
        .map(|step| {
            (
                dial.said_among(*step, &desk.optimizing.named),
                desk.optimizing.sweep.steps.iter().any(|held| held == step),
            )
        })
        .collect();
    let (below, picked) = chips(paint, mouse, area, y, &values);
    if let Some(at) = picked {
        act = Some(Act::SweepValue(at));
    }
    y = below + 6.0;
    let (below, typed) = a_value_of_my_own(paint, desk, mouse, area, y);
    (below, act.or(typed))
}

fn a_value_of_my_own(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let dial = desk.optimizing.sweep.dial;
    paint.say_at(
        area.x,
        y + 6.0,
        "Or a value of my own",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let field_at = Box::new(area.x + 160.0, y, 120.0, 28.0);
    let touched = ui::field(
        paint,
        mouse,
        field_at,
        &desk.optimizing.custom,
        dial.step_of(dial.span().finest).said().as_str(),
        desk.optimizing.custom_focused,
    );
    if touched != ui::Touched::No {
        act = Some(Act::CustomValue(touched));
    }
    if ui::button(
        paint,
        mouse,
        Box::new(area.x + 292.0, y, 80.0, 28.0),
        "Add",
        Kind::Ordinary,
    ) {
        act = Some(Act::AddCustom);
    }
    let unit = dial.unit();
    if !unit.is_empty() {
        paint.say_at(
            area.x + 382.0,
            y + 6.0,
            unit,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    y += 34.0;
    if let Some(why) = &desk.optimizing.custom_refused {
        for line in paint.wrap(why, Weight::Regular, size::SMALL, area.w - 20.0) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.warn);
            y += 16.0;
        }
        y += 4.0;
    }
    (y, act)
}

fn test_set(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    y = section(
        paint,
        area,
        y,
        "Test set",
        "sixty-four checked programming tasks in eight prompts of eight",
    );
    let sets: Vec<(String, bool)> = mcf_optimize::corpus::Set::all()
        .iter()
        .map(|set| {
            (
                format!("Set {}", set.number),
                desk.optimizing.sweep.sets.contains(&set.number),
            )
        })
        .collect();
    let (below, picked) = chips(paint, mouse, area, y, &sets);
    if let Some(at) = picked {
        act = Some(Act::TestSet(at.saturating_add(1)));
    }
    y = below;
    let repeats = format!("Repeats: {}", desk.optimizing.sweep.repeats);
    let (pressed, area_of) = ui::fitted(paint, mouse, (area.x, y), &repeats, Kind::Ordinary);
    if pressed {
        act = Some(Act::Repeats);
    }
    paint.say_at(
        area_of.right() + 12.0,
        y + 9.0,
        &desk.optimizing.sweep.said(),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    (y + 44.0, act)
}

const ROW: f32 = 19.0;

fn run_row(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box, y: f32) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let label = if desk.optimizing.running {
        "Stop".to_owned()
    } else if desk.optimizing.known > 0 {
        "Carry on".to_owned()
    } else {
        "Run sweep".to_owned()
    };
    let kind = if desk.optimizing.running {
        Kind::Ordinary
    } else {
        Kind::Primary
    };
    let (pressed, button) = ui::fitted(paint, mouse, (area.x, y), &label, kind);
    if pressed {
        act = Some(Act::Sweep);
    }
    if desk.optimizing.running {
        ui::progress(
            paint,
            Box::new(button.right() + 14.0, y + 13.0, 180.0, 6.0),
            desk.optimizing.fraction(),
        );
    }
    let doing = desk.optimizing.run.as_ref().map_or_else(
        || desk.optimizing.standing(),
        mcf_optimize::running::Running::said,
    );
    let at = if desk.optimizing.running {
        button.right() + 206.0
    } else {
        button.right() + 14.0
    };
    let shown = paint.elide(
        &doing,
        Weight::Regular,
        size::SMALL,
        area.right() - at - 130.0,
    );
    paint.say_at(at, y + 9.0, &shown, Weight::Regular, size::SMALL, ink.faint);
    if !desk.optimizing.running && desk.optimizing.known > 0 {
        let (pressed, _box) = ui::fitted(
            paint,
            mouse,
            (area.right() - 120.0, y),
            "Forget them",
            Kind::Quiet,
        );
        if pressed {
            act = Some(Act::ForgetReadings);
        }
    }
    act
}

fn sweep_report(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    y = section(
        paint,
        area,
        y,
        "Report",
        "one row for every prompt run, exactly as it was measured",
    );
    act = act.or(run_row(paint, desk, mouse, area, y));
    if let Some(why) = &desk.optimizing.refused {
        let shown = paint.elide(why, Weight::Regular, size::SMALL, area.w - 20.0);
        paint.say_at(
            area.x,
            y + 40.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.warn,
        );
    }
    y += 52.0;
    let best = desk.optimizing.report.best_by(desk.optimizing.measure);
    if let Some(best) = best {
        let dial = desk.optimizing.sweep.dial;
        let said = match desk.optimizing.measure {
            mcf_optimize::reading::Measure::Speed => format!(
                "Best so far: {} at {} tok/s over {} trial(s)",
                dial.said(best.step),
                best.tokens_a_second()
                    .map_or_else(|| "—".to_owned(), |rate| format!("{rate:.1}")),
                best.trials
            ),
            mcf_optimize::reading::Measure::Correctness => format!(
                "Best so far: {} at {}/{} over {} trial(s)",
                dial.said(best.step),
                best.passed,
                best.of,
                best.trials
            ),
        };
        paint.say_at(area.x, y, &said, Weight::Bold, size::SMALL, ink.accent);
        y += 22.0;
    }
    if desk.optimizing.rows.is_empty() {
        paint.say_at(
            area.x,
            y,
            "Nothing measured against this configuration yet. Every reading is written down \
             as it finishes, and every one taken under exactly these settings shows here — \
             from this sweep and from any before it.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return act;
    }
    act = act.or(rows_of_the_record(paint, desk, mouse, area, &mut y));
    paint.reaches(y);
    act
}

fn rows_of_the_record(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    y: &mut f32,
) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let dial = desk.optimizing.sweep.dial;
    let wide = ((area.w - 28.0) / 9.0).max(58.0);
    paint.say_at(
        area.x,
        *y,
        &format!(
            "{} reading(s) under exactly this configuration{}",
            desk.optimizing.rows.len(),
            if desk.optimizing.picked.is_empty() {
                " · click a row to pick it".to_owned()
            } else {
                format!(" · {} picked", desk.optimizing.picked.len())
            }
        ),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    *y += 20.0;
    if !desk.optimizing.picked.is_empty() && !desk.optimizing.running {
        let (pressed, held) = ui::fitted(
            paint,
            mouse,
            (area.x, *y),
            &format!("Run {} again", desk.optimizing.picked.len()),
            Kind::Primary,
        );
        if pressed {
            act = Some(Act::RerunPicked);
        }
        let (cleared, _where) = ui::fitted(
            paint,
            mouse,
            (held.right() + 10.0, *y),
            "Pick none",
            Kind::Quiet,
        );
        if cleared {
            act = Some(Act::PickNone);
        }
        *y += 40.0;
    }
    for (at, head) in mcf_optimize::reading::Report::COLUMNS.iter().enumerate() {
        paint.say_at(
            area.x + 28.0 + wide * at as f32,
            *y,
            head,
            Weight::Bold,
            size::LABEL,
            ink.faint,
        );
    }
    *y += 20.0;
    let band = paint.clipped();
    for row in &desk.optimizing.rows {
        let below = *y + ROW;
        let seen = band.is_none_or(|held| below >= held.y && *y <= held.bottom());
        let where_ = Box::new(area.x, *y - 3.0, area.w - 12.0, ROW);
        if seen {
            let picked = desk.optimizing.picked.contains(&row.at);
            if picked {
                paint.wash(where_, ink.accent, 40);
            } else if mouse.over(where_) {
                paint.wash(where_, ink.line, 60);
            }
            if picked {
                ui::tick(
                    paint,
                    Box::new(area.x + 4.0, *y + 1.0, 12.0, 12.0),
                    ink.accent,
                );
            }
            let cells =
                mcf_optimize::reading::Report::cells_of(&row.reading, dial, &desk.optimizing.named);
            for (column, cell) in cells.iter().enumerate() {
                let shown = paint.elide(cell, Weight::Regular, size::SMALL, wide - 8.0);
                paint.say_at(
                    area.x + 28.0 + wide * column as f32,
                    *y,
                    &shown,
                    Weight::Regular,
                    size::SMALL,
                    ink.ink,
                );
            }
        }
        if mouse.clicked(where_) {
            act = Some(Act::PickRow(row.at));
        }
        *y += ROW;
    }
    act
}

fn optimize_tab(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let mut act = None;
    let mut y = area.y;
    y = base_configuration(paint, desk, area, y);
    let (below, picked) = setting_to_optimize(paint, desk, mouse, area, y);
    act = act.or(picked);
    y = below;
    let (below, picked) = test_set(paint, desk, mouse, area, y);
    act = act.or(picked);
    y = below;
    act.or(sweep_report(paint, desk, mouse, area, y))
}

fn statistics_tab(paint: &mut Painter, area: Box, held: &Model) {
    let ink = paint.ink;
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
    section(
        paint,
        &mut y,
        "Use",
        &[
            "Nothing recorded for this model yet.".to_owned(),
            "MCF records what a hold actually did — the settings, this machine, and \
             the rates that followed — as it holds it."
                .to_owned(),
        ],
        ink.faint,
    );
}

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

enum Control {
    Pick(Picker, String),
    Words(crate::Field, String, &'static str),
    Number(crate::Field, String),
    Flip(crate::Switch, bool),
}

fn a_section(paint: &mut Painter, area: Box, y: f32, heading: &str) -> f32 {
    let ink = paint.ink;
    let top = y + 10.0;
    paint.rule((area.x, top), (area.right(), top), ink.line, 140);
    paint.say_at(
        area.x,
        top + 10.0,
        heading,
        Weight::Bold,
        size::SMALL,
        ink.accent,
    );
    top + 34.0
}

struct Row {
    name: &'static str,
    because: &'static str,
}

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
            .find(|setting| setting.name.eq_ignore_ascii_case(name))
            .map_or("", |setting| setting.because)
    };
    let recommended_for = |name: &str| {
        listed
            .iter()
            .find(|setting| setting.name.eq_ignore_ascii_case(name))
            .filter(|setting| setting.value != setting.recommended)
            .map(|setting| setting.recommended.clone())
    };
    let column = area.x + label_column_of(area.w, 190.0);
    let control = (area.w - 190.0).max(120.0);
    let mut act = None;
    let mut hovered: Option<(&'static str, f32)> = None;
    let mut menu: Option<(Picker, Box)> = None;

    let label =
        |paint: &mut Painter, y: f32, row: Row, hovered: &mut Option<(&'static str, f32)>| {
            let hit = Box::new(area.x, y - 4.0, area.w, 26.0);
            if mouse.over(hit) && !row.because.is_empty() {
                *hovered = Some((row.because, y));
            }
            let over = mouse.over(hit);
            paint.say_at(
                area.x,
                y,
                row.name,
                Weight::Regular,
                size::BODY,
                if over { ink.ink } else { ink.quiet },
            );
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
     -> (ui::Touched, String) {
        let focused = desk
            .editing
            .as_ref()
            .is_some_and(|(editing, _)| *editing == field);
        let text = if focused {
            desk.being_typed().to_owned()
        } else {
            now
        };
        let held = if focused {
            desk.typing_now().clone()
        } else {
            crate::typing::Typing::of(text.clone())
        };
        let touched = ui::field(
            paint,
            mouse,
            Box::new(column, y - 6.0, control, 28.0),
            &held,
            placeholder,
            focused,
        );
        (touched, text)
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
    y = a_section(paint, area, y, "The model");
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

    y = a_section(paint, area, y, "Context and cache");
    label(
        paint,
        y,
        Row {
            name: "Context window",
            because: because_of("context window"),
        },
        &mut hovered,
    );
    let (touched, text) = typed_in(
        paint,
        mouse,
        y,
        crate::Field::Context,
        settings.context.to_string(),
        "tokens",
    );
    if touched != ui::Touched::No {
        act = Some(Act::Edit(crate::Field::Context, touched));
    }
    y += 34.0;
    let typed_window = text.trim().replace([',', '_'], "").parse::<u64>().ok();
    if let Some(said) = reserve_line(
        held,
        typed_window.unwrap_or(settings.context),
        settings.cache,
    ) {
        paint.say_at(column, y, &said, Weight::Regular, size::SMALL, ink.faint);
        y += 18.0;
    }
    recommends(paint, &mut y, "context window");

    label(
        paint,
        y,
        Row {
            name: "Cache width",
            because: because_of("cache width"),
        },
        &mut hovered,
    );
    let cache_box = Box::new(column, y - 6.0, 150.0, 28.0);
    let cache_open = desk.open == Some(Picker::Cache);
    if ui::picker(paint, mouse, cache_box, settings.cache.as_str(), cache_open) {
        act = Some(Act::Open(Picker::Cache));
    }
    if cache_open {
        menu = Some((Picker::Cache, cache_box));
    }
    paint.say_at(
        column + 160.0,
        y,
        settings.cache.said(),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    y += 34.0;
    recommends(paint, &mut y, "cache width");

    let pick = |held: Picker, shown: String| Control::Pick(held, shown);
    let words =
        |field: crate::Field, now: String, empty: &'static str| Control::Words(field, now, empty);
    let number = |field: crate::Field, now: String| Control::Number(field, now);
    let flip = |held: crate::Switch, on: bool| Control::Flip(held, on);
    let sections: Vec<(&'static str, Vec<(&'static str, Control)>)> = vec![
        (
            "Where it runs",
            vec![
                (
                    "Split mode",
                    pick(Picker::SplitMode, settings.spread.split.as_str().to_owned()),
                ),
                (
                    "Experts",
                    pick(Picker::Experts, settings.spread.experts.said()),
                ),
                (
                    "Main device",
                    number(
                        crate::Field::MainDevice,
                        settings.spread.main_device.to_string(),
                    ),
                ),
                (
                    "Devices",
                    words(
                        crate::Field::Devices,
                        settings.spread.devices.clone().unwrap_or_default(),
                        "every one MCF found",
                    ),
                ),
                (
                    "Dense layers on the processor",
                    number(
                        crate::Field::DenseLayersOnCpu,
                        settings.spread.ffn_layers_on_processor.to_string(),
                    ),
                ),
                (
                    "Tensors placed by hand",
                    words(
                        crate::Field::OverrideTensors,
                        settings.spread.override_tensors.clone().unwrap_or_default(),
                        "none",
                    ),
                ),
                (
                    "Cache in system memory",
                    flip(
                        crate::Switch::CacheOnProcessor,
                        settings.spread.cache_on_processor,
                    ),
                ),
                (
                    "Memory lock",
                    flip(crate::Switch::KeepResident, settings.keep_resident),
                ),
                (
                    "How it loads",
                    pick(Picker::Loading, settings.loading.as_str().to_owned()),
                ),
                (
                    "Large tensors",
                    pick(Picker::LargeTensors, settings.lazily.as_str().to_owned()),
                ),
            ],
        ),
        (
            "Speed",
            vec![
                (
                    "Prompt batch",
                    number(crate::Field::Batch, settings.batch.to_string()),
                ),
                (
                    "Micro-batch",
                    number(crate::Field::Ubatch, settings.ubatch.to_string()),
                ),
                (
                    "Threads",
                    number(crate::Field::Threads, settings.threads.to_string()),
                ),
                (
                    "Threads for reading a prompt",
                    number(
                        crate::Field::ThreadsBatch,
                        settings.threads_batch.to_string(),
                    ),
                ),
                (
                    "Flash attention",
                    flip(crate::Switch::FlashAttention, settings.flash_attention),
                ),
            ],
        ),
        (
            "Thinking",
            vec![
                (
                    "Thinking budget",
                    number(
                        crate::Field::ThinkingBudget,
                        settings
                            .started
                            .thinking
                            .map_or_else(String::new, |held| held.to_string()),
                    ),
                ),
                (
                    "Draft depth",
                    number(
                        crate::Field::DraftDepth,
                        settings
                            .started
                            .drafted
                            .map_or_else(String::new, |held| held.to_string()),
                    ),
                ),
            ],
        ),
        (
            "Reuse between messages",
            vec![
                (
                    "Prompt cache",
                    flip(crate::Switch::PromptCache, settings.reuse.prompt_cache),
                ),
                (
                    "Prompt cache memory",
                    number(
                        crate::Field::PromptCacheMib,
                        settings.reuse.prompt_cache_mib.to_string(),
                    ),
                ),
                (
                    "Prefix reuse",
                    number(
                        crate::Field::CacheReuse,
                        settings.reuse.cache_reuse.to_string(),
                    ),
                ),
                (
                    "Keep idle slots",
                    flip(crate::Switch::IdleSlots, settings.reuse.idle_slots),
                ),
                (
                    "Checkpoints",
                    number(
                        crate::Field::Checkpoints,
                        settings.reuse.checkpoints.to_string(),
                    ),
                ),
                (
                    "Checkpoint spacing",
                    number(
                        crate::Field::CheckpointSpacing,
                        settings.reuse.checkpoint_min_step.to_string(),
                    ),
                ),
                (
                    "Context shift",
                    flip(crate::Switch::ContextShift, settings.reuse.context_shift),
                ),
                (
                    "Tokens kept in front",
                    number(crate::Field::Keep, settings.reuse.keep.to_string()),
                ),
            ],
        ),
        (
            "Serving",
            vec![
                (
                    "Name callers use",
                    words(
                        crate::Field::Alias,
                        settings.alias.clone().unwrap_or_default(),
                        "its file, without the suffix",
                    ),
                ),
                (
                    "Conversations at once",
                    number(crate::Field::Slots, settings.slots.to_string()),
                ),
                (
                    "Port",
                    number(crate::Field::Port, settings.port.to_string()),
                ),
                (
                    "Reachable from the network",
                    flip(crate::Switch::Open, settings.open),
                ),
                (
                    "Answer kind",
                    pick(Picker::Answers, settings.answers.as_str().to_owned()),
                ),
                (
                    "Pooling",
                    pick(Picker::Pooling, settings.pooling.as_str().to_owned()),
                ),
            ],
        ),
    ];
    for (heading, rows) in sections {
        y = a_section(paint, area, y, heading);
        for (name, control) in rows {
            label(
                paint,
                y,
                Row {
                    name,
                    because: because_of(name),
                },
                &mut hovered,
            );
            match control {
                Control::Pick(picker, shown) => {
                    let box_of = Box::new(column, y - 6.0, 210.0, 28.0);
                    let open = desk.open == Some(picker);
                    if ui::picker(paint, mouse, box_of, &shown, open) {
                        act = Some(Act::Open(picker));
                    }
                    if open {
                        menu = Some((picker, box_of));
                    }
                }
                Control::Words(field, now, empty) => {
                    let (touched, _) = typed_in(paint, mouse, y, field, now, empty);
                    if touched != ui::Touched::No {
                        act = Some(Act::Edit(field, touched));
                    }
                }
                Control::Number(field, now) => {
                    let (touched, _) = typed_in(paint, mouse, y, field, now, "");
                    if touched != ui::Touched::No {
                        act = Some(Act::Edit(field, touched));
                    }
                }
                Control::Flip(which, on) => {
                    if switch(paint, mouse, y, on) {
                        act = Some(Act::Switch(which));
                    }
                }
            }
            y += 34.0;
            recommends(paint, &mut y, name);
        }
    }

    y = a_section(paint, area, y, "Keys and extras");
    label(
        paint,
        y,
        Row {
            name: "API key",
            because: because_of("API key"),
        },
        &mut hovered,
    );
    let (touched, _) = typed_in(
        paint,
        mouse,
        y,
        crate::Field::ApiKey,
        settings.api_key.clone().unwrap_or_default(),
        "none (open on localhost)",
    );
    if touched != ui::Touched::No {
        act = Some(Act::Edit(crate::Field::ApiKey, touched));
    }
    y += 34.0;

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
    y = a_section(paint, area, y, "Stretching the context");
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
        let held = if focused {
            desk.typing_now().clone()
        } else {
            crate::typing::Typing::of(text.clone())
        };
        let touched = ui::field(
            paint,
            mouse,
            Box::new(column + 160.0, y - 6.0, 100.0, 28.0),
            &held,
            "",
            focused,
        );
        if touched != ui::Touched::No {
            act = Some(Act::Edit(crate::Field::RopeFactor, touched));
        }
    }
    y += 34.0;

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

    if let Some(why) = &desk.edit_refused {
        paint.say_at(area.x, y, why, Weight::Regular, size::SMALL, ink.warn);
        y += 18.0;
    }
    let (pressed, _after) = configure_foot(
        paint,
        desk,
        mouse,
        Box::new(area.x, y, area.w, area.bottom() - y),
        held,
        settings,
    );
    if let Some(pressed) = pressed {
        act = Some(pressed);
    }
    if let Some((because, at)) = hovered {
        an_info_box(paint, Box::new(area.x, at, area.w, 0.0), because);
    }
    if let Some((picker, at)) = menu
        && let Some(picked) = configure_menu(paint, desk, mouse, picker, at)
    {
        act = Some(picked);
    }
    act
}

fn an_info_box(paint: &mut Painter, beside: Box, because: &str) {
    let ink = paint.ink;
    let wide = beside.w.clamp(200.0, 380.0);
    let lines = paint.wrap(because, Weight::Regular, size::SMALL, wide - 24.0);
    let shown: Vec<&String> = lines.iter().take(6).collect();
    let tall = 16.0 + 15.0 * shown.len() as f32;
    let left = (beside.x + beside.w - wide).max(beside.x);
    let top = beside.y + 22.0;
    let where_ = Box::new(left, top, wide, tall);
    paint.wash(Box::new(left + 2.0, top + 3.0, wide, tall), ink.sunk, 120);
    paint.edge(where_, ui::RADIUS, ink.accent, ink.card);
    let mut y = top + 9.0;
    for line in shown {
        paint.say_at(left + 12.0, y, line, Weight::Regular, size::SMALL, ink.ink);
        y += 15.0;
    }
}

fn configure_foot(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    held: &Model,
    settings: &mcf_serve::hosting::Hosting,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut y = area.y + 6.0;
    let mut act = None;
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
    let too_big = held.does_not_fit.is_some();
    let (host, _) = ui::fitted(
        paint,
        mouse,
        (x, y),
        if too_big {
            "Start server (it does not fit)"
        } else if waits {
            "Start server (build the engine first)"
        } else {
            "Start server"
        },
        if waits || too_big {
            Kind::Quiet
        } else {
            Kind::Primary
        },
    );
    if host && !waits && !too_big && !desk.doing.busy() {
        act = Some(Act::HostIt);
    }
    (act, y + 40.0)
}

fn placement_label(placement: &crate::Placement) -> String {
    let where_ = match placement.on.as_str() {
        "resolved" => "Auto",
        "processor" => "CPU",
        "card" => "GPU",
        other => other,
    };
    format!("{where_} — {} on {}", placement.engine, placement.device)
}

fn rope_label(at: usize) -> &'static str {
    match at {
        1 => "Off",
        2 => "Linear",
        3 => "YaRN",
        _ => "Default",
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one picker an arm, which is the readable shape for a list of them"
)]
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
        Picker::Answers => {
            let labels: Vec<String> = mcf_serve::hosting::Answers::ALL
                .iter()
                .map(|held| format!("{} — {}", held.as_str(), held.said()))
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                mcf_serve::hosting::Answers::ALL
                    .iter()
                    .position(|held| *held == settings.answers)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Answers)
        }
        Picker::Pooling => {
            let labels: Vec<String> = mcf_serve::hosting::Pooling::ALL
                .iter()
                .map(|held| held.as_str().to_owned())
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                mcf_serve::hosting::Pooling::ALL
                    .iter()
                    .position(|held| *held == settings.pooling)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Pooling)
        }
        Picker::Loading => {
            let labels: Vec<String> = mcf_serve::hosting::Loading::ALL
                .iter()
                .map(|held| format!("{} — {}", held.as_str(), held.said()))
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                mcf_serve::hosting::Loading::ALL
                    .iter()
                    .position(|held| *held == settings.loading)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Loading)
        }
        Picker::LargeTensors => {
            let labels: Vec<String> = mcf_serve::hosting::Lazily::ALL
                .iter()
                .map(|held| format!("{} — {}", held.as_str(), held.said()))
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                mcf_serve::hosting::Lazily::ALL
                    .iter()
                    .position(|held| *held == settings.lazily)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::LargeTensors)
        }
        Picker::SplitMode => {
            let labels: Vec<String> = mcf_serve::hosting::Split::ALL
                .iter()
                .map(|split| format!("{} — {}", split.as_str(), split.said()))
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                mcf_serve::hosting::Split::ALL
                    .iter()
                    .position(|split| *split == settings.spread.split)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::SplitMode)
        }
        Picker::Experts => {
            let labels: Vec<String> = crate::EXPERT_CHOICES
                .iter()
                .map(|held| held.said())
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                crate::EXPERT_CHOICES
                    .iter()
                    .position(|held| *held == settings.spread.experts)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Experts)
        }
        Picker::Cache => {
            let labels: Vec<String> = crate::CACHE_CHOICES
                .iter()
                .map(|width| format!("{width} — {}", width.said()))
                .collect();
            let now = desk.settings.as_ref().and_then(|settings| {
                crate::CACHE_CHOICES
                    .iter()
                    .position(|width| *width == settings.cache)
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::Cache)
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

fn action_buttons(desk: &Desk, this_one: bool, stop_label: String) -> Vec<(String, Kind, Act)> {
    if this_one {
        return vec![
            ("Chat".to_owned(), Kind::Primary, Act::Go(Page::Hosting)),
            (stop_label, Kind::Ordinary, Act::StopHosting),
        ];
    }
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
    listed
}

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
    spaced(paint, area.x, actions_at, "actions", ink.faint);
    let mut y = actions_at + 26.0;
    let this_one = desk
        .hosted
        .as_ref()
        .zip(desk.chosen.and_then(|at| desk.models.get(at)))
        .is_some_and(|(hosting, held)| hosting.model == held.path);
    let stop_label = desk
        .hosted_model()
        .zip(desk.hosted.as_ref())
        .and_then(|(held, hosting)| reserve_of(held, hosting.context?, hosting.cache))
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
    if !this_one && desk.chosen.is_some() {
        actions.push(("Remove…".to_owned(), Kind::Quiet, Act::AskToRemove));
    }
    for (label, kind, what) in actions {
        let where_ = Box::new(area.x, y, list - 20.0, 30.0);
        let needs_one = true;
        let waits = what == Act::HostIt && desk.needs_engine.is_some();
        if ui::button(paint, mouse, where_, &label, kind)
            && (!needs_one || desk.chosen.is_some())
            && !waits
        {
            act = Some(what);
        }
        y += 36.0;
    }
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

fn model_list(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let list = area.w;
    let actions_at = area.bottom();
    let mut act = None;
    spaced(paint, area.x, area.y, "models", ink.faint);
    let mut y = area.y + 24.0;
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

fn filters_height(desk: &Desk) -> f32 {
    if desk.filters.open {
        22.0 + 3.0 * 32.0
    } else {
        22.0
    }
}

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

fn fits_label(fits: Option<bool>) -> &'static str {
    match fits {
        None => "any",
        Some(true) => "will run here",
        Some(false) => "will not run here",
    }
}

fn size_label(size: Option<u64>) -> String {
    size.map_or_else(
        || "any".to_owned(),
        |bytes| format!("up to {}", gigabytes(bytes)),
    )
}

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
    if held.refused.is_some() {
        parts.push("will not run here".to_owned());
    }
    if parts.is_empty() {
        "not yet read".to_owned()
    } else {
        parts.join(" · ")
    }
}

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
    y
}

fn label_column_of(inner_w: f32, most: f32) -> f32 {
    (inner_w * 0.4).min(most).max(70.0)
}

const ON_CHOICES: [Option<mcf_serve::control::On>; 3] = [
    None,
    Some(mcf_serve::control::On::Processor),
    Some(mcf_serve::control::On::Card),
];

fn on_label(on: Option<mcf_serve::control::On>) -> String {
    match on {
        None => "Auto".to_owned(),
        Some(mcf_serve::control::On::Processor) => "CPU".to_owned(),
        Some(mcf_serve::control::On::Card) => "GPU (all layers)".to_owned(),
    }
}

fn open_menu(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    picker: Picker,
    at: Box,
) -> Option<Act> {
    match picker {
        Picker::Model => {
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
        Picker::Placement
        | Picker::Rope
        | Picker::Cache
        | Picker::SplitMode
        | Picker::Loading
        | Picker::Answers
        | Picker::Pooling
        | Picker::LargeTensors
        | Picker::Experts
        | Picker::Quantization => None,
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

fn clock(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds} s");
    }
    #[expect(clippy::integer_division, reason = "whole minutes and the rest")]
    let (minutes, rest) = (seconds / 60, seconds % 60);
    format!("{minutes} min {rest:02} s")
}

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

fn offered_row(
    paint: &mut Painter,
    mouse: &Mouse,
    row: Box,
    file: &Value,
    repository: &str,
    at: usize,
) -> Option<Act> {
    let ink = paint.ink;
    let (y, wide) = (row.y, row.w);
    let area = row;
    let asked_for = file
        .get("file")
        .and_then(Value::as_text)
        .unwrap_or("?")
        .to_owned();
    let name = file
        .get("name")
        .and_then(Value::as_text)
        .unwrap_or(&asked_for)
        .to_owned();
    let parts = file
        .get("parts")
        .and_then(Value::as_integer)
        .unwrap_or(1)
        .max(1);
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
    if parts > 1 {
        let after = paint.measure(&shown, Weight::Regular, size::BODY);
        paint.say_at(
            area.x + after + 10.0,
            y + 2.0,
            &format!("in {parts} files"),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
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
        return Some(Act::Download {
            reference: repository.to_owned(),
            file: asked_for,
        });
    }
    mouse.clicked(where_).then_some(Act::PickOffered(at))
}

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
    if let Some(terms) = found.get("terms").and_then(Value::as_text) {
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
        let row = Box::new(area.x, y, wide, 30.0);
        act = offered_row(paint, mouse, row, file, &repository, at).or(act);
        y += 32.0;
        if y > area.bottom() - 20.0 {
            break;
        }
    }
    act
}

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
            (Some(_), Some(_)) if held("bytes_whole").is_some() => format!(
                "part {} of {}: {} of {}",
                held("part").unwrap_or(0),
                held("of").unwrap_or(0),
                words::size_in_words(held("arrived_whole")).unwrap_or_default(),
                words::size_in_words(held("bytes_whole")).unwrap_or_default()
            ),
            (Some(arrived), Some(total)) => format!(
                "{} of {}",
                words::size_in_words(Some(arrived)).unwrap_or_default(),
                words::size_in_words(Some(total)).unwrap_or_default()
            ),
            _ => "starting".to_owned(),
        },
    }
}

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
    ) != ui::Touched::No
    {
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
    (act, at.y + 42.0)
}

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

fn use_tiles(paint: &mut Painter, in_use: &crate::Use, at: Box) -> f32 {
    let rate = |held: Option<f32>| held.map(|rate| format!("{rate:.1}"));
    let count = |held: Option<u64>| held.map(words::grouped);
    let bytes = |held: Option<u64>| held.map(gigabytes);
    let tiles: Vec<(&str, Option<String>)> = vec![
        ("Gen tok/s", rate(in_use.generated_per_second)),
        ("Prompt tok/s", rate(in_use.prompted_per_second)),
        (
            "Tokens out",
            count(in_use.generated_live.or(in_use.generated)),
        ),
        ("Tokens in", count(in_use.prompted)),
        (
            "KV cache",
            in_use
                .cache_used
                .map(|ratio| format!("{:.0}%", (ratio * 100.0).clamp(0.0, 100.0))),
        ),
        ("KV tokens", count(in_use.cache_tokens)),
        ("Decodes", count(in_use.decodes)),
        (
            if in_use.power_named.as_deref() == Some("package") {
                "Package watts"
            } else {
                "Card watts"
            },
            rate(in_use.card_power_watts),
        ),
        (
            if in_use.power_named.as_deref() == Some("package") {
                "Package energy"
            } else {
                "Card energy"
            },
            in_use.card_energy_joules.map(|joules| {
                if joules >= 1_000.0 {
                    format!("{:.1} kJ", joules / 1_000.0)
                } else {
                    format!("{joules:.0} J")
                }
            }),
        ),
        (
            "Energy over",
            in_use
                .card_energy_over_seconds
                .map(|seconds| clock(seconds.max(0.0) as u64)),
        ),
        (
            "Cost",
            in_use
                .card_energy_cost_millionths
                .map(|millionths| mcf_core::price::Cost { millionths }.to_string()),
        ),
        ("Active", count(in_use.processing)),
        ("Queued", count(in_use.queued)),
        ("RAM", bytes(in_use.resident)),
        ("VRAM", bytes(in_use.card)),
        ("Uptime", in_use.uptime_seconds.map(crate::ago_said)),
    ];
    tiles_of(paint, &tiles, at)
}

fn machine_and_run_tiles(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let spent = desk.spent;
    let mut tiles: Vec<(&str, Option<String>)> = Vec::new();
    if spent.seconds > 0 {
        #[expect(
            clippy::integer_division,
            reason = "whole kilojoules is the unit shown"
        )]
        let kilojoules = spent.millijoules / 1_000_000;
        tiles.push((
            "Run energy",
            (spent.millijoules > 0).then(|| {
                if kilojoules > 0 {
                    format!("{} kJ", words::grouped(kilojoules))
                } else {
                    #[expect(clippy::integer_division, reason = "whole joules, under a kilojoule")]
                    let joules = spent.millijoules / 1_000;
                    format!("{joules} J")
                }
            }),
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

pub fn saying(paint: &mut Painter, said: &str) {
    let ink = paint.ink;
    paint.begin();
    let (width, height) = paint.size();
    let across = paint.measure(said, Weight::Regular, size::BODY);
    paint.say_at(
        ((width - across) / 2.0).max(12.0),
        (height / 2.0) - 10.0,
        said,
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    paint.end();
}

fn rate_line(paint: &mut Painter, rates: &std::collections::VecDeque<f32>, at: Box) -> f32 {
    let ink = paint.ink;
    let peak = rates.iter().copied().fold(0.0_f32, f32::max);
    spaced(paint, at.x, at.y, "gen tok/s · last 2 min", ink.faint);
    let plot = Box::new(at.x, at.y + 16.0, at.w, at.h - 16.0);
    paint.edge(plot, 6.0, ink.line, ink.card);
    if peak <= 0.0 {
        let said = if rates.is_empty() {
            "waiting for a reading"
        } else {
            "nothing generated while this page has been open"
        };
        paint.say_at(
            plot.x + 12.0,
            plot.y + 8.0,
            said,
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
        model
            .zip(hosting.context)
            .and_then(|(model, context)| reserve_line(model, context, hosting.cache))
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

fn hosting_body(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let top = system_block(paint, desk, Box::new(area.x, area.y, area.w, 0.0));
    let top = under_test_block(paint, desk, Box::new(area.x, top, area.w, 0.0));
    let area = Box::new(area.x, top, area.w, (area.bottom() - top).max(0.0));
    let held = desk
        .hosted_model()
        .or_else(|| desk.chosen.and_then(|at| desk.models.get(at)));
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

fn system_block(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    spaced(paint, at.x, at.y, "system", ink.faint);
    let mut y = at.y + 20.0;
    y = processors_table(paint, Box::new(at.x, y, at.w, 0.0), desk);
    y = memory_table(paint, Box::new(at.x, y + 14.0, at.w, 0.0), desk);
    y = storage_table(paint, Box::new(at.x, y + 14.0, at.w, 130.0), desk);
    y + 26.0
}

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

fn what_it_ran_under(desk: &Desk) -> Vec<String> {
    let Doing::Answering(job) = &desk.doing else {
        return Vec::new();
    };
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

fn anatomy_body(paint: &mut Painter, area: Box, said: &mcf_serve::anatomy::Said) {
    let top = area.y;
    let stacked = area.w < 900.0;
    let figures = if stacked {
        area.w
    } else {
        460.0_f32.min(area.w / 2.0)
    };
    let left = Box::new(area.x, top, figures, area.bottom() - top);
    let after = counted(paint, left, said);
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
    let mut switched = None;
    let mut right = area.right();
    for (label, page) in [
        ("Vocabulary", Page::Vocabulary),
        ("What is in it", Page::Anatomy),
    ] {
        let width = paint.measure(label, Weight::Bold, size::SMALL) + 28.0;
        right -= width;
        let button = Box::new(right, area.y + 18.0, width, 28.0);
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
            let line = paint.elide(&line, Weight::Bold, size::BODY, area.w - 160.0);
            paint.say_at(area.x + 150.0, y, &line, Weight::Bold, size::BODY, ink.ink);
            y += 20.0;
        }
        y += 2.0;
    }
    y
}

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
