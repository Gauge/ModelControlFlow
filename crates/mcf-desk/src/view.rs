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

/// The vertical rhythm. Every gap down a tab is one of these, so that blocks of the same
/// kind sit the same distance apart wherever they appear rather than each place picking a
/// number that looked right on its own.
mod gap {
    /// One wrapped line of small text.
    pub(super) const LINE: f32 = 16.0;
    /// One row of a list that names a thing on the left and shows it on the right.
    pub(super) const ROW: f32 = 20.0;
    /// How tall a button, a field or a chip is.
    pub(super) const CONTROL: f32 = 28.0;
    /// Between one block within a section and the next.
    pub(super) const BLOCK: f32 = 12.0;
}

pub const UNKNOWN: &str = "Unknown";

/// What stands in for a figure the machine did not offer. A dash, because a figure MCF
/// was not given is not a figure of nought.
const UNKNOWN_FIGURE: &str = "—";

pub fn draw(paint: &mut Painter, desk: &Desk, mouse: &Mouse) -> Option<Act> {
    let (width, height) = paint.size();
    paint.begin();
    let side = desk.splits.side;
    let mut act = side_bar(paint, desk, mouse, height, side);

    // Every page gives up the foot of the window to the strip, so that what MCF is
    // saying is in one place whatever is being looked at.
    let opened = desk.notices_open && !desk.notices.is_empty();
    let listed = if opened {
        RECENT_TALL.min((height - 220.0).max(80.0))
    } else {
        0.0
    };
    let said = STRIP + listed;
    let main = Box::new(
        side + PAD,
        PAD,
        (width - side - PAD * 2.0).max(10.0),
        (height - PAD * 2.0 - said).max(10.0),
    );
    let went = match desk.page {
        Page::Host | Page::Models => host(paint, desk, mouse, main),
        Page::Adding => adding(paint, desk, mouse, main),
        // The server page is handed the whole page as well as the padded box inside it:
        // its rail runs to the window's own edges, top, bottom and right. A rail inset by
        // the page's padding floats, and reads as a card that happens to be tall.
        Page::Hosting => {
            let whole = Box::new(
                side + 1.0,
                0.0,
                (width - side - 1.0).max(10.0),
                (height - said).max(10.0),
            );
            hosting(paint, desk, mouse, main, whole)
        }
        Page::Anatomy => anatomy(paint, desk, mouse, main),
        Page::Vocabulary => vocabulary(paint, desk, mouse, main),
        Page::Downloads => downloads(paint, desk, mouse, main),
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

    // Drawn last, and over everything, because it is the one thing always in view.
    let strip = Box::new(side, height - STRIP, (width - side).max(10.0), STRIP);
    if opened {
        let list = Box::new(side, strip.y - listed, strip.w, listed);
        if let Some(pressed) = notice_list(paint, desk, mouse, list) {
            act = Some(pressed);
        }
    }
    if let Some(pressed) = notice_strip(paint, desk, mouse, strip) {
        act = Some(pressed);
    }
    paint.end();
    act
}

/// How tall the strip along the foot of the window is.
const STRIP: f32 = 34.0;

/// How tall the list opens to, at most.
const RECENT_TALL: f32 = 292.0;

/// The ink a tone is said in.
fn tone_ink(tone: crate::notice::Tone, ink: &crate::paint::Ink) -> Rgb {
    match tone {
        crate::notice::Tone::Working => ink.accent,
        crate::notice::Tone::Refused => ink.bad,
        crate::notice::Tone::Warning => ink.warn,
        crate::notice::Tone::Done => ink.good,
    }
}

/// The mark beside a notice, so its kind is never colour alone.
fn tone_mark(paint: &mut Painter, tone: crate::notice::Tone, x: f32, y: f32) {
    let ink = paint.ink;
    let colour = tone_ink(tone, &ink);
    let middle = (x + 6.0, y + 6.0);
    match tone {
        // An arc with a gap: a ring that is plainly unfinished.
        crate::notice::Tone::Working => {
            for at in 0..10 {
                #[expect(clippy::cast_precision_loss, reason = "ten steps around a circle")]
                let turn = at as f32 / 12.0 * std::f32::consts::TAU;
                let (dx, dy) = (turn.cos() * 5.0, turn.sin() * 5.0);
                paint.rect(
                    Box::new(middle.0 + dx - 1.0, middle.1 + dy - 1.0, 2.0, 2.0),
                    colour,
                );
            }
        }
        crate::notice::Tone::Done => {
            for at in 0..4 {
                #[expect(clippy::cast_precision_loss, reason = "four steps of a tick")]
                let step = at as f32;
                paint.rect(
                    Box::new(middle.0 - 4.0 + step, middle.1 + step - 1.0, 2.0, 2.0),
                    colour,
                );
            }
            for at in 0..6 {
                #[expect(clippy::cast_precision_loss, reason = "six steps of a tick")]
                let step = at as f32;
                paint.rect(
                    Box::new(middle.0 - 1.0 + step, middle.1 + 2.0 - step, 2.0, 2.0),
                    colour,
                );
            }
        }
        crate::notice::Tone::Refused => {
            for at in 0..9 {
                #[expect(clippy::cast_precision_loss, reason = "nine steps of a cross")]
                let step = at as f32 - 4.0;
                paint.rect(
                    Box::new(middle.0 + step - 1.0, middle.1 + step - 1.0, 2.0, 2.0),
                    colour,
                );
                paint.rect(
                    Box::new(middle.0 + step - 1.0, middle.1 - step - 1.0, 2.0, 2.0),
                    colour,
                );
            }
        }
        // A bar and a dot: an exclamation, which is what a warning is.
        crate::notice::Tone::Warning => {
            paint.rect(Box::new(middle.0 - 1.0, middle.1 - 5.0, 2.0, 7.0), colour);
            paint.rect(Box::new(middle.0 - 1.0, middle.1 + 4.0, 2.0, 2.0), colour);
        }
    }
}

/// The strip along the foot: what is happening, and what has just been said.
///
/// One place, always the same place, for everything that used to be said in eleven
/// different ones. What it shows is [`Notices::foremost`]: work first, because work is now.
fn notice_strip(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    paint.rect(at, ink.card);
    paint.rule((at.x, at.y), (at.right(), at.y), ink.line, 255);

    let refusals = desk.notices.refusals();
    let all = desk.notices.len();
    let mut right = at.right() - 16.0;

    // The way in and out of the list, at the end of the strip where it stays put.
    if all > 0 {
        let said = match (all, desk.notices_open) {
            (_, true) => "Close".to_owned(),
            (1, false) => "1 recent".to_owned(),
            (all, false) => format!("{all} recent"),
        };
        let wide = paint.measure(&said, Weight::Regular, size::SMALL) + 24.0;
        let hit = Box::new(right - wide, at.y + 4.0, wide, at.h - 8.0);
        let over = mouse.over(hit);
        if over {
            paint.panel(hit, 6.0, ink.sunk, 255);
        }
        paint.say_right(
            right - 16.0,
            at.y + 10.0,
            &said,
            Weight::Regular,
            size::SMALL,
            if over { ink.ink } else { ink.quiet },
        );
        let arrow = if desk.notices_open { "▾" } else { "▴" };
        paint.say_right(
            right - 4.0,
            at.y + 9.0,
            arrow,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        if mouse.clicked(hit) {
            act = Some(Act::OpenNotices);
        }
        right -= wide + 6.0;
    }

    // How many want answering, said outright rather than left to be found in the list.
    if refusals > 0 && !desk.notices_open {
        let said = if refusals == 1 {
            "1 refused".to_owned()
        } else {
            format!("{refusals} refused")
        };
        paint.say_right(
            right,
            at.y + 10.0,
            &said,
            Weight::Bold,
            size::SMALL,
            ink.bad,
        );
        right -= paint.measure(&said, Weight::Bold, size::SMALL) + 16.0;
    }

    let Some(foremost) = desk.notices.foremost() else {
        paint.say_at(
            at.x + 22.0,
            at.y + 10.0,
            "Nothing to report",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return act;
    };

    tone_mark(paint, foremost.tone, at.x + 22.0, at.y + 11.0);
    let mut x = at.x + 40.0;
    let room = (right - x - 12.0).max(80.0);
    let shown = paint.elide(&foremost.what, Weight::Regular, size::BODY, room);
    paint.say_at(x, at.y + 9.0, &shown, Weight::Regular, size::BODY, ink.ink);
    x += paint.measure(&shown, Weight::Regular, size::BODY) + 14.0;

    // The share, where it is known. Where it is not, nothing is drawn rather than a bar
    // at nought, which would say the work had not started.
    if let Some(share) = foremost.share
        && x + 150.0 < right
    {
        let bar = Box::new(x, at.y + 15.0, 120.0, 4.0);
        ui::progress(paint, bar, Some(share));
        x = bar.right() + 12.0;
    }
    if let Some(detail) = &foremost.detail
        && x + 60.0 < right
    {
        let shown = paint.elide(detail, Weight::Regular, size::SMALL, right - x - 8.0);
        paint.say_at(
            x,
            at.y + 10.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    act
}

/// The strip, opened out: what MCF has said, newest first.
fn notice_list(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    paint.rect(at, ink.card);
    paint.rule((at.x, at.y), (at.right(), at.y), ink.line, 255);
    let inside = Box::new(at.x + 22.0, at.y + 15.0, (at.w - 44.0).max(120.0), at.h);

    spaced(
        paint,
        inside.x,
        inside.y,
        "recent · what MCF has said",
        ink.faint,
    );
    let clear = "Dismiss the settled";
    let wide = paint.measure(clear, Weight::Regular, size::SMALL) + 20.0;
    let hit = Box::new(inside.right() - wide, inside.y - 6.0, wide, 22.0);
    let over = mouse.over(hit);
    paint.say_right(
        inside.right() - 10.0,
        inside.y - 2.0,
        clear,
        Weight::Regular,
        size::SMALL,
        if over { ink.ink } else { ink.faint },
    );
    if mouse.clicked(hit) {
        act = Some(Act::DismissNotices);
    }
    paint.say_right(
        inside.right() - wide - 14.0,
        inside.y - 2.0,
        "refusals are in the record too — mcf failures",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );

    let mut y = inside.y + 22.0;
    for held in desk.notices.recent() {
        if y + 34.0 > at.bottom() {
            break;
        }
        paint.say_at(
            inside.x,
            y + 2.0,
            &held.clock_said(),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        tone_mark(paint, held.tone, inside.x + 52.0, y + 3.0);
        let room = inside.w - 78.0;
        let shown = paint.elide(&held.what, Weight::Regular, size::BODY, room);
        paint.say_at(
            inside.x + 70.0,
            y,
            &shown,
            Weight::Regular,
            size::BODY,
            ink.ink,
        );
        y += 19.0;
        if let Some(detail) = &held.detail {
            let shown = paint.elide(detail, Weight::Regular, size::SMALL, room);
            paint.say_at(
                inside.x + 70.0,
                y,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.quiet,
            );
            y += 17.0;
        }
        y += 5.0;
    }
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
        // The count rides on the label, because a person who has started a download and
        // walked to another page has no other way to know it is still going.
        let under_way = desk.transfers_under_way();
        let label = if *page == Page::Downloads && under_way > 0 {
            format!("{label} ({under_way})")
        } else {
            (*label).to_owned()
        };
        if ui::nav(paint, mouse, area, &label, desk.page.section() == *page) {
            act = Some(Act::Go(*page));
        }
    }
    // What MCF is doing was said here too, in its own words and its own colour. The strip
    // along the foot says it now, for every page at once, so saying it again in the corner
    // of the side bar was the same thing twice and often not the same thing.
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
    // As the server page does, the model page is handed the box its rail may run into as
    // well as the padded one its content sits in.
    let whole = Box::new(pane.x, pane.y, pane.w + PAD, pane.h + PAD);
    model_page(paint, desk, mouse, pane, whole, held).or(act)
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

    // What is not going. A repository's quantizations are separate files, and somebody
    // about to remove one is owed the plain statement that the others stay.
    // Only where this is about one model. Removing several at once is a disk being
    // cleared, and what is left of each repository is not the question then.
    if let Some((others, repository)) = removing
        .only()
        .and_then(|model| desk.others_of_the_repository(model))
    {
        let said = match others {
            1 => format!("The other quantization of {repository} stays where it is."),
            many => format!("The other {many} quantizations of {repository} stay where they are."),
        };
        for line in paint.wrap(&said, Weight::Regular, size::SMALL, wide) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.quiet);
            y += 16.0;
        }
        y += 12.0;
    }

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

/// The downloads page: what the disk holds, what is arriving, and what can go.
///
/// It was the transfer queue and nothing else, which meant the one page about files on a
/// disk said nothing about the disk. What is on the shelf outlives what is arriving by a
/// long way — four hundred and fifty gigabytes of it here — so the shelf is the page and
/// the queue is a band across the top of it when there is one.
fn downloads(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    // What is about to be removed takes the page, because it is a question and the rest
    // of the page is the answer to a different one.
    if desk.removing.is_some() {
        return removal_page(paint, desk, mouse, area);
    }
    spaced(paint, area.x, area.y, "downloads", ink.faint);
    let weights = desk.weights.bytes;
    paint.say_at(
        area.x,
        area.y + 22.0,
        &words::size_in_words(Some(weights)).unwrap_or_else(|| words::UNMEASURED.to_owned()),
        Weight::Bold,
        26.0,
        ink.ink,
    );
    let after = paint.measure(
        &words::size_in_words(Some(weights)).unwrap_or_default(),
        Weight::Bold,
        26.0,
    );
    paint.say_at(
        area.x + after + 11.0,
        area.y + 30.0,
        "of model weights on this disk",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );

    let mut y = area.y + 68.0;
    y = storage_block(paint, desk, Box::new(area.x, y, area.w, 0.0));
    if let Some(pressed) = orphan_block(paint, desk, mouse, Box::new(area.x, y, area.w, 0.0)) {
        act = Some(pressed);
    }
    if !desk.weights.orphans.is_empty() {
        y += 46.0;
    }
    if desk.transfers_under_way() > 0 || !desk.transfers.is_empty() {
        let (pressed, below) = arriving_block(paint, desk, mouse, Box::new(area.x, y, area.w, 0.0));
        act = act.or(pressed);
        y = below;
    }
    let (pressed, below) = shelf_head(paint, desk, mouse, Box::new(area.x, y, area.w, 0.0));
    act = act.or(pressed);
    y = below;
    let list = Box::new(area.x, y, area.w, (area.bottom() - y).max(80.0));
    ui::card(paint, list, false);
    let inside = Box::new(list.x, list.y, list.w, list.h);
    let rolled = scrolled(
        paint,
        mouse,
        desk,
        Region::Downloads,
        inside,
        |paint, mouse, inner| shelf_rows(paint, desk, mouse, inner),
    );
    act.or(rolled)
}

/// What the disk holds: the weights, everything else, and what is free.
fn storage_block(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let card = Box::new(at.x, at.y, at.w, 74.0);
    ui::card(paint, card, false);
    let inside = Box::new(card.x + 18.0, card.y + 15.0, card.w - 36.0, card.h);
    let Some(volume) = desk.disk else {
        paint.say_at(
            inside.x,
            inside.y + 4.0,
            "MCF could not read what this filesystem holds.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return card.bottom() + 16.0;
    };
    let weights = desk.weights.bytes.min(volume.used());
    let other = volume.used().saturating_sub(weights);
    let bar = Box::new(inside.x, inside.y, inside.w, 10.0);
    paint.panel(bar, 5.0, ink.sunk, 255);
    #[expect(
        clippy::cast_precision_loss,
        reason = "byte counts of a filesystem, drawn to the nearest pixel"
    )]
    let share = |held: u64| (held as f32 / volume.total.max(1) as f32) * bar.w;
    let wide = share(weights);
    if wide >= 1.0 {
        paint.panel(Box::new(bar.x, bar.y, wide, bar.h), 5.0, ink.accent, 255);
    }
    let rest = share(other);
    if rest >= 1.0 {
        paint.rect(Box::new(bar.x + wide, bar.y, rest, bar.h), ink.line);
    }

    let mut x = inside.x;
    for (colour, label, held) in [
        (ink.accent, "Model weights", Some(weights)),
        (ink.line, "Everything else", Some(other)),
        (ink.sunk, "Free", Some(volume.free)),
    ] {
        paint.panel(Box::new(x, inside.y + 28.0, 8.0, 8.0), 2.0, colour, 255);
        paint.say_at(
            x + 14.0,
            inside.y + 25.0,
            label,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        let after = x + 14.0 + paint.measure(label, Weight::Regular, size::SMALL) + 8.0;
        let said = held
            .and_then(|held| words::size_in_words(Some(held)))
            .unwrap_or_else(|| words::UNMEASURED.to_owned());
        paint.say_at(
            after,
            inside.y + 25.0,
            &said,
            Weight::Bold,
            size::SMALL,
            ink.ink,
        );
        x = after + paint.measure(&said, Weight::Bold, size::SMALL) + 24.0;
    }
    // Both shares, because they answer different questions: whether the models are what
    // is filling the disk, and whether the disk is full.
    if let (Some(of_used), Some(of_disk)) = (
        volume.share_of_what_is_used(weights),
        volume.share_of_the_disk(weights),
    ) {
        paint.say_right(
            inside.right(),
            inside.y + 25.0,
            &format!("{of_used}% of what is used · {of_disk}% of the disk"),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    card.bottom() + 16.0
}

/// Projectors with no model beside them, and how much they would give back.
fn orphan_block(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> Option<Act> {
    let ink = paint.ink;
    if desk.weights.orphans.is_empty() {
        return None;
    }
    let card = Box::new(at.x, at.y, at.w, 38.0);
    paint.edge(card, 10.0, ink.warn_soft, ink.warn_soft);
    let count = desk.weights.orphans.len();
    let said = match count {
        1 => "1 multimodal projector with no model beside it".to_owned(),
        many => format!("{many} multimodal projectors with no model beside them"),
    };
    paint.say_at(
        card.x + 16.0,
        card.y + 11.0,
        &said,
        Weight::Regular,
        size::SMALL,
        ink.warn,
    );
    let after = card.x + 16.0 + paint.measure(&said, Weight::Regular, size::SMALL) + 12.0;
    if let Some(back) = words::size_in_words(Some(desk.weights.reclaimable())) {
        paint.say_at(
            after,
            card.y + 11.0,
            &format!("{back} reclaimable"),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    let (pressed, _drawn) = ui::fitted(
        paint,
        mouse,
        (card.right() - 122.0, card.y + 4.0),
        "Tick them all",
        Kind::Quiet,
    );
    pressed.then_some(Act::PickTheOrphans)
}

/// The queue, as a band above the shelf.
fn arriving_block(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut act = None;
    let under_way = desk.transfers_under_way();
    let said = match under_way {
        0 => "Arriving".to_owned(),
        1 => "1 file arriving".to_owned(),
        many => format!("{many} files arriving"),
    };
    spaced(paint, at.x, at.y, &said, ink.faint);
    let settled = desk.transfers.iter().filter(|held| held.settled()).count();
    if settled > 0 {
        let (pressed, _drawn) = ui::fitted(
            paint,
            mouse,
            (at.right() - 140.0, at.y - 8.0),
            "Clear finished",
            Kind::Quiet,
        );
        if pressed {
            act = Some(Act::ForgetTransfers);
        }
    }
    let mut y = at.y + 20.0;
    for transfer in desk.transfers.iter().take(3) {
        let (below, pressed) = transfer_row(
            paint,
            mouse,
            Box::new(at.x, y, at.w, TRANSFER_ROW),
            y,
            transfer,
        );
        act = act.or(pressed);
        y = below;
    }
    if desk.transfers.len() > 3 {
        paint.say_at(
            at.x,
            y,
            &format!("and {} more", desk.transfers.len() - 3),
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        y += 18.0;
    }
    (act, y + 10.0)
}

/// The heading over the shelf, and what is to be done with what is ticked.
fn shelf_head(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut act = None;
    spaced(paint, at.x, at.y, "on this disk · largest first", ink.faint);
    if desk.picked.is_empty() {
        paint.say_right(
            at.right(),
            at.y - 2.0,
            "tick a variant to remove it",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return (act, at.y + 22.0);
    }
    let (remove, drawn) = ui::fitted(
        paint,
        mouse,
        (at.right() - 104.0, at.y - 9.0),
        "Remove…",
        Kind::Ordinary,
    );
    if remove {
        act = Some(Act::RemovePicked);
    }
    let (cleared, _drawn) = ui::fitted(
        paint,
        mouse,
        (drawn.x - 74.0, at.y - 9.0),
        "Clear",
        Kind::Quiet,
    );
    if cleared {
        act = Some(Act::ClearPicked);
    }
    let said = format!(
        "{} chosen · {}",
        desk.picked.len(),
        words::size_in_words(Some(desk.picked_bytes())).unwrap_or_else(|| "—".to_owned())
    );
    paint.say_right(
        drawn.x - 84.0,
        at.y - 2.0,
        &said,
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    // Said before it is pressed, not after it is refused: the hold is the one thing on
    // this page that cannot simply go.
    if let Some(name) = desk.picked_the_served() {
        let after = at.x + spaced_width(paint, "on this disk · largest first") + 16.0;
        paint.say_at(
            after,
            at.y - 2.0,
            &format!("{name} is being served — stop the server first"),
            Weight::Regular,
            size::SMALL,
            ink.warn,
        );
    }
    (act, at.y + 22.0)
}

/// How tall one transfer's row is. Fixed, so that a row does not jump as its words change
/// length while it is being read.
const TRANSFER_ROW: f32 = 92.0;

/// The shelf: one row a repository, largest first, opening out into its variants.
///
/// Largest first because the page is for getting room back, and the row that answers that
/// is the biggest one. A repository of a hundred and fifty gigabytes sorted alphabetically
/// under G is a row nobody finds.
fn shelf_rows(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let mut groups = desk.library();
    if groups.is_empty() && desk.weights.orphans.is_empty() {
        paint.say_at(
            area.x + 16.0,
            area.y + 16.0,
            "Nothing on this disk yet. Models — look one up — Download.",
            Weight::Regular,
            size::BODY,
            ink.faint,
        );
        return None;
    }
    let bytes_of = |group: &crate::Group| {
        group
            .members
            .iter()
            .filter_map(|at| desk.models.get(*at))
            .filter_map(|held| held.bytes)
            .fold(0, u64::saturating_add)
    };
    // Largest first: the page is for getting room back, so the row that answers that
    // comes first. Reversed rather than negated, because a byte count has no negative.
    groups.sort_by_key(&bytes_of);
    groups.reverse();
    let widest = groups.iter().map(bytes_of).max().unwrap_or(0);

    let mut y = area.y + 4.0;
    for group in &groups {
        let (below, pressed) = shelf_group(
            paint,
            desk,
            mouse,
            group,
            (bytes_of(group), widest),
            Box::new(area.x + 14.0, y, area.w - 28.0, 0.0),
        );
        act = act.or(pressed);
        y = below;
    }
    if !desk.weights.orphans.is_empty() {
        let (_below, pressed) = orphan_rows(
            paint,
            desk,
            mouse,
            Box::new(area.x + 14.0, y, area.w - 28.0, 0.0),
        );
        act = act.or(pressed);
    }
    act
}

/// One repository, and its variants where it is opened out.
fn shelf_group(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    group: &crate::Group,
    (bytes, widest): (u64, u64),
    at: Box,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let mut y = at.y;
    paint.rule(
        (at.x - 14.0, y - 5.0),
        (at.right() + 14.0, y - 5.0),
        ink.line,
        120,
    );

    let opened = desk.is_opened_out(group.repository.as_deref()) || group.members.len() == 1;
    let name = group.name(&desk.models);
    let arrow = if group.members.len() > 1 {
        if opened { "▾" } else { "▸" }
    } else {
        " "
    };
    paint.say_at(
        at.x,
        y + 2.0,
        arrow,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let shown = paint.elide(&name, Weight::Bold, size::BODY, at.w * 0.4);
    paint.say_at(at.x + 16.0, y, &shown, Weight::Bold, size::BODY, ink.ink);
    let after = at.x + 16.0 + paint.measure(&shown, Weight::Bold, size::BODY) + 12.0;
    let variants = match group.members.len() {
        1 => "1 variant".to_owned(),
        many => format!("{many} variants"),
    };
    paint.say_at(
        after,
        y + 1.0,
        &variants,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );

    // A bar against the largest repository, so the row that is worth removing looks it.
    let bar = Box::new(at.right() - 176.0, y + 6.0, 90.0, 5.0);
    paint.panel(bar, 2.5, ink.sunk, 255);
    if widest > 0 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "byte counts of a shelf, drawn to the nearest pixel"
        )]
        let wide = (bytes as f32 / widest as f32) * bar.w;
        if wide >= 1.0 {
            paint.panel(Box::new(bar.x, bar.y, wide, bar.h), 2.5, ink.accent, 255);
        }
    }
    paint.say_right(
        at.right(),
        y,
        &words::size_in_words(Some(bytes)).unwrap_or_else(|| words::UNMEASURED.to_owned()),
        Weight::Bold,
        size::BODY,
        ink.ink,
    );
    let row = Box::new(at.x - 8.0, y - 4.0, at.w - 190.0, 24.0);
    if group.members.len() > 1
        && mouse.clicked(row)
        && let Some(repository) = group.repository.as_ref()
    {
        act = Some(Act::OpenOut(repository.clone()));
    }
    y += 26.0;

    if !opened {
        return (y + 4.0, act);
    }
    for member in &group.members {
        let Some(held) = desk.models.get(*member) else {
            continue;
        };
        let (below, pressed) = variant_row(
            paint,
            desk,
            mouse,
            held,
            Box::new(at.x + 18.0, y, at.w - 18.0, 0.0),
        );
        act = act.or(pressed);
        y = below;
    }
    (y + 6.0, act)
}

/// One variant, with the tick box that chooses it.
fn variant_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    held: &Model,
    at: Box,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let ticked = desk.picked.contains(&held.path);
    let row = Box::new(at.x - 6.0, at.y - 3.0, at.w + 6.0, 22.0);
    if ticked {
        paint.panel(row, 6.0, ink.accent_soft, 255);
    } else if mouse.over(row) {
        paint.panel(row, 6.0, ink.line, 90);
    }
    let square = Box::new(at.x, at.y + 3.0, 13.0, 13.0);
    if ui::check(paint, mouse, square, ticked) {
        act = Some(Act::PickOnDisk(held.path.clone()));
    }
    let said = match held.parts {
        Some(parts) if parts > 1 => without_the_part(&quantization_said(held)),
        _ => quantization_said(held),
    };
    let shown = paint.elide(&said, Weight::Regular, size::SMALL, at.w * 0.45);
    paint.say_at(
        at.x + 24.0,
        at.y,
        &shown,
        Weight::Regular,
        size::SMALL,
        ink.ink,
    );
    let mut after = at.x + 24.0 + paint.measure(&shown, Weight::Regular, size::SMALL) + 12.0;
    // Split files are one variant in several pieces, and saying so keeps somebody from
    // reading three parts as three models.
    if let Some(parts) = held.parts.filter(|parts| *parts > 1) {
        let said = format!("{parts} parts");
        paint.say_at(after, at.y, &said, Weight::Regular, size::SMALL, ink.faint);
        after += paint.measure(&said, Weight::Regular, size::SMALL) + 12.0;
    }
    if desk
        .hosted
        .as_ref()
        .is_some_and(|hosting| hosting.model == held.path)
    {
        paint.say_at(
            after,
            at.y,
            "serving",
            Weight::Bold,
            size::SMALL,
            ink.accent,
        );
    }
    paint.say_right(
        at.right() - 14.0,
        at.y,
        &held
            .bytes
            .and_then(|bytes| words::size_in_words(Some(bytes)))
            .unwrap_or_else(|| words::UNMEASURED.to_owned()),
        Weight::Regular,
        size::SMALL,
        ink.ink,
    );
    (at.y + 21.0, act)
}

/// The projectors nothing uses, as their own group at the foot of the shelf.
fn orphan_rows(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let mut y = at.y;
    paint.rule(
        (at.x - 14.0, y - 5.0),
        (at.right() + 14.0, y - 5.0),
        ink.line,
        120,
    );
    paint.say_at(
        at.x + 16.0,
        y,
        "Projectors with no model",
        Weight::Bold,
        size::BODY,
        ink.warn,
    );
    paint.say_right(
        at.right(),
        y,
        &words::size_in_words(Some(desk.weights.reclaimable()))
            .unwrap_or_else(|| words::UNMEASURED.to_owned()),
        Weight::Bold,
        size::BODY,
        ink.ink,
    );
    y += 26.0;
    for orphan in &desk.weights.orphans {
        let ticked = desk.picked.contains(&orphan.path);
        let row = Box::new(at.x + 12.0, y - 3.0, at.w - 12.0, 22.0);
        if ticked {
            paint.panel(row, 6.0, ink.accent_soft, 255);
        } else if mouse.over(row) {
            paint.panel(row, 6.0, ink.line, 90);
        }
        let square = Box::new(at.x + 18.0, y + 3.0, 13.0, 13.0);
        if ui::check(paint, mouse, square, ticked) {
            act = Some(Act::PickOnDisk(orphan.path.clone()));
        }
        let shown = paint.elide(&orphan.name(), Weight::Regular, size::SMALL, at.w * 0.4);
        paint.say_at(
            at.x + 42.0,
            y,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.ink,
        );
        let after = at.x + 42.0 + paint.measure(&shown, Weight::Regular, size::SMALL) + 12.0;
        let beside = orphan.beside();
        let shown = paint.elide(&beside, Weight::Regular, size::SMALL, at.w * 0.3);
        paint.say_at(after, y, &shown, Weight::Regular, size::SMALL, ink.faint);
        paint.say_right(
            at.right() - 14.0,
            y,
            &words::size_in_words(Some(orphan.bytes))
                .unwrap_or_else(|| words::UNMEASURED.to_owned()),
            Weight::Regular,
            size::SMALL,
            ink.ink,
        );
        y += 21.0;
    }
    (y + 6.0, act)
}

fn transfer_row(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    y: f32,
    transfer: &crate::Transfer,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let buttons = 240.0_f32.min(area.w * 0.5);
    let room = (area.w - buttons - 16.0).max(80.0);
    let name = paint.elide(&transfer.name(), Weight::Bold, size::BODY, room);
    paint.say_at(area.x, y, &name, Weight::Bold, size::BODY, ink.ink);
    let whose = paint.elide(&transfer.reference, Weight::Regular, size::SMALL, room);
    paint.say_at(
        area.x,
        y + 20.0,
        &whose,
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    ui::progress(
        paint,
        Box::new(area.x, y + 42.0, room, 8.0),
        transfer.fraction(),
    );
    let said = paint.elide(&transfer.said(), Weight::Regular, size::SMALL, room);
    let colour = match transfer.state.as_str() {
        "failed" => ink.warn,
        "done" => ink.accent,
        _ => ink.quiet,
    };
    paint.say_at(
        area.x,
        y + 58.0,
        &said,
        Weight::Regular,
        size::SMALL,
        colour,
    );
    if let Some(arrived) = words::size_in_words(Some(transfer.arrived)) {
        let of = words::size_in_words(Some(transfer.whole))
            .map_or_else(|| arrived.clone(), |whole| format!("{arrived} of {whole}"));
        paint.say_at(
            area.x,
            y + 74.0,
            &of,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }

    // Only what this state can actually be asked.
    let mut x = area.right() - buttons;
    if transfer.under_way() {
        let (pressed, drawn) = ui::fitted(paint, mouse, (x, y + 10.0), "Pause", Kind::Ordinary);
        if pressed {
            act = Some(Act::PauseTransfer(transfer.id));
        }
        x = drawn.right() + 8.0;
    } else if !matches!(transfer.state.as_str(), "done") {
        let (pressed, drawn) = ui::fitted(paint, mouse, (x, y + 10.0), "Carry on", Kind::Primary);
        if pressed {
            act = Some(Act::ResumeTransfer(transfer.id));
        }
        x = drawn.right() + 8.0;
    }
    if !transfer.settled() {
        let (pressed, _drawn) = ui::fitted(paint, mouse, (x, y + 10.0), "Give up", Kind::Quiet);
        if pressed {
            act = Some(Act::GiveUpTransfer(transfer.id));
        }
    }
    paint.rule(
        (area.x, y + TRANSFER_ROW - 10.0),
        (area.right(), y + TRANSFER_ROW - 10.0),
        ink.line,
        120,
    );
    (y + TRANSFER_ROW, act)
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
    // While this file is in the queue, this shows the queue's own account of it rather
    // than a second one: there is one transfer, and the downloads page shows the same.
    if let Some(transfer) = desk
        .transfer_of_the_pending()
        .filter(|held| !held.settled())
    {
        paint.say_at(
            area.x,
            y,
            &format!("Getting {}", transfer.name()),
            Weight::Bold,
            size::BODY,
            ink.ink,
        );
        ui::progress(
            paint,
            Box::new(area.x, y + 24.0, area.w, 8.0),
            transfer.fraction(),
        );
        paint.say_at(
            area.x,
            y + 46.0,
            &transfer.said(),
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        let (seen, _drawn) = ui::fitted(
            paint,
            mouse,
            (area.x, y + 68.0),
            "See all downloads",
            Kind::Quiet,
        );
        if seen {
            act = Some(Act::Go(Page::Downloads));
        }
    } else {
        let (start, drawn) = ui::fitted(
            paint,
            mouse,
            (area.x, y),
            "Download and start server",
            Kind::Primary,
        );
        if start {
            act = Some(Act::DownloadThen(std::boxed::Box::new(Act::HostIt)));
        }
        let (get, _) = ui::fitted(
            paint,
            mouse,
            (drawn.right() + 12.0, y),
            "Download",
            Kind::Ordinary,
        );
        // Nothing asks whether the window is busy. A transfer is queued in the daemon, so
        // there is nothing for it to be busy with.
        if get {
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
    whole: Box,
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
    // Optimize places a rail and a run bar of its own, so it is handed the room whole and
    // scrolls only the column inside it. Every other tab is one scrolling page.
    if desk.tab == crate::Tab::Optimize {
        let room = Box::new(
            below.x,
            below.y,
            below.w,
            (whole.bottom() - below.y).max(120.0),
        );
        return optimize_tab(paint, desk, mouse, room, whole).or(act);
    }
    let drawn = scrolled(
        paint,
        mouse,
        desk,
        Region::Page,
        below,
        |paint, mouse, inner| match desk.tab {
            crate::Tab::Configure => configure_tab(paint, desk, mouse, inner, held),
            crate::Tab::Optimize => None,
            crate::Tab::Statistics => {
                statistics_tab(paint, desk, inner, held);
                None
            }
            crate::Tab::Contents => contents_tab(paint, desk, mouse, inner),
        },
    );
    drawn.or(act)
}

/// Where the value half of a labelled row starts. One column for every row that names a
/// thing on the left and shows it on the right, so the names line up down the tab instead of
/// each row choosing its own margin.
const NAMED: f32 = 140.0;

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
        y + 7.0,
        "Or my own",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    // As wide as it can be up to its own width, and never past the edge of the rail.
    let field_at = Box::new(area.x + NAMED, y, (area.w - NAMED).clamp(48.0, 120.0), 28.0);
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
    let unit = dial.unit();
    if !unit.is_empty() {
        paint.say_at(
            field_at.right() + 12.0,
            y + 7.0,
            unit,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    y += gap::CONTROL + gap::BLOCK;
    if let Some(why) = &desk.optimizing.custom_refused {
        for line in paint.wrap(why, Weight::Regular, size::SMALL, area.w - 20.0) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.warn);
            y += gap::LINE;
        }
        y += gap::BLOCK;
    }
    (y, act)
}

const ROW: f32 = 19.0;

/// How much of a template the box shows at once. Enough to read a block of it without
/// pushing every setting under it off the page.
const TEMPLATE_TALL: f32 = 200.0;

/// The one line that says what the sweep is doing: the button, then what is happening now,
/// then how far along it is. Nothing here moves sideways when a sweep starts or stops — a
/// line that jumps as it updates is harder to read than one that stays put.
/// What the sweep makes of everything measured so far, and — once it has finished — the
/// decision it leaves to the reader. A sweep measures; it does not move the settings above
/// it until somebody says to.
fn what_it_found(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    mut y: f32,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    if let Some(said) = &desk.optimizing.adopted {
        for line in paint.wrap(said, Weight::Regular, size::SMALL, area.w - 20.0) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.accent);
            y += gap::LINE;
        }
        return (y + gap::BLOCK, act);
    }
    let dial = desk.optimizing.sweep.dial;
    let Some(best) = desk.optimizing.report.best_by(desk.optimizing.measure) else {
        return (y, act);
    };
    let reading = match desk.optimizing.measure {
        mcf_optimize::reading::Measure::Speed => format!(
            "{} tok/s",
            best.tokens_a_second()
                .map_or_else(|| "—".to_owned(), |rate| format!("{rate:.0}"))
        ),
        mcf_optimize::reading::Measure::Correctness => format!("{}/{}", best.passed, best.of),
    };
    let settled = desk.optimizing.settled.filter(|held| *held == best.step);
    let heading = if settled.is_some() {
        "Best"
    } else {
        "Best so far"
    };
    paint.say_at(
        area.x,
        y,
        &format!(
            "{heading}: {} at {reading}",
            dial.said_among(best.step, &desk.optimizing.named)
        ),
        Weight::Bold,
        size::SMALL,
        ink.ink,
    );
    y += gap::ROW;
    if settled.is_none() {
        return (y, act);
    }
    let asks = format!("Set {} to this?", dial.label().to_lowercase());
    paint.say_at(
        area.x,
        y + 8.0,
        &asks,
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    let at = area.x + paint.measure(&asks, Weight::Regular, size::SMALL) + 14.0;
    let (taken, drawn) = ui::fitted(paint, mouse, (at, y), "Use it", Kind::Primary);
    if taken {
        act = Some(Act::AdoptBest);
    }
    let (kept, _where) = ui::fitted(
        paint,
        mouse,
        (drawn.right() + 10.0, y),
        "Leave it",
        Kind::Quiet,
    );
    if kept {
        act = Some(Act::KeepAsIs);
    }
    (y + gap::CONTROL + gap::BLOCK, act)
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
    let columns = mcf_optimize::reading::Report::columns_of(dial, desk.optimizing.measure);
    let across = u16::try_from(columns.len().max(1)).unwrap_or(9);
    let wide = ((area.w - 28.0 - BESIDE) / f32::from(across)).max(52.0);
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
    for (at, head) in columns.iter().enumerate() {
        paint.say_at(
            area.x + 28.0 + wide * at as f32,
            *y,
            head,
            Weight::Bold,
            size::LABEL,
            ink.faint,
        );
    }
    paint.say_at(
        area.right() - BESIDE - 12.0,
        *y,
        "One row",
        Weight::Bold,
        size::LABEL,
        ink.faint,
    );
    *y += 20.0;
    let band = paint.clipped();
    let mut why = None;
    for row in &desk.optimizing.rows {
        act = one_row(
            paint,
            desk,
            mouse,
            RowAt {
                area,
                y,
                wide,
                dial,
                band,
                why: &mut why,
            },
            row,
        )
        .or(act);
    }
    if let Some(said) = why {
        *y += 6.0;
        for line in paint.wrap(&said, Weight::Regular, size::SMALL, area.w - 20.0) {
            paint.say_at(area.x, *y, &line, Weight::Regular, size::SMALL, ink.warn);
            *y += gap::LINE;
        }
    }
    act
}

struct RowAt<'a> {
    area: Box,
    y: &'a mut f32,
    wide: f32,
    dial: mcf_optimize::dial::Dial,
    band: Option<Box>,
    /// Where the reason the row under the mouse ended as it did is put, to be shown under
    /// the table. A reading that failed and does not say what it hit is a reading nobody
    /// can act on.
    why: &'a mut Option<String>,
}

fn one_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    where_it_goes: RowAt<'_>,
    row: &mcf_optimize::ledger::Row,
) -> Option<Act> {
    let ink = paint.ink;
    let RowAt {
        area,
        y,
        wide,
        dial,
        band,
        why,
    } = where_it_goes;
    let mut act = None;
    let below = *y + ROW;
    let seen = band.is_none_or(|held| below >= held.y && *y <= held.bottom());
    let where_ = Box::new(area.x, *y - 3.0, area.w - 12.0, ROW);
    let again = Box::new(where_.right() - BESIDE, *y - 3.0, 54.0, ROW);
    let gone = Box::new(where_.right() - BESIDE + 58.0, *y - 3.0, 54.0, ROW);
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
        let cells = mcf_optimize::reading::Report::cells_of(
            &row.reading,
            dial,
            desk.optimizing.measure,
            &desk.optimizing.named,
        );
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
        beside_a_row(paint, mouse, again, "Again", ink.accent);
        beside_a_row(paint, mouse, gone, "Forget", ink.warn);
        if mouse.over(where_)
            && let Some(said) = &row.reading.why
        {
            *why = Some(said.clone());
        }
    }
    if mouse.clicked(again) {
        act = Some(Act::RerunRow(row.at));
    } else if mouse.clicked(gone) {
        act = Some(Act::ForgetRow(row.at));
    } else if mouse.clicked(where_) {
        act = Some(Act::PickRow(row.at));
    }
    *y += ROW;
    act
}

const BESIDE: f32 = 118.0;

fn beside_a_row(paint: &mut Painter, mouse: &Mouse, area: Box, said: &str, colour: Rgb) {
    let ink = paint.ink;
    let over = mouse.over(area);
    if over {
        paint.edge(area, 4.0, colour, ink.card);
    }
    paint.say_at(
        area.x + 7.0,
        area.y + 3.0,
        said,
        Weight::Bold,
        size::LABEL,
        if over { colour } else { ink.faint },
    );
}

/// Optimize, laid out like the server page: a fixed rail on the right holding the sweep
/// and the button that starts it, a scrolling column holding what was measured, and — only
/// while a sweep runs — a bar pinned across the foot of that column carrying the progress.
///
/// The rail and the bar never both offer to run the sweep. Idle, the rail's foot starts it;
/// running, the bar owns pausing and stopping, because that is where the eye already is.
fn optimize_tab(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    whole: Box,
) -> Option<Act> {
    // Narrow enough and a rail would leave the column too thin for a table, so the page
    // stacks — the same move, at the same width, that the server page makes.
    if whole.w < RAIL_STACKS_UNDER {
        return optimize_stacked(paint, desk, mouse, area);
    }
    let rail = Box::new(whole.right() - RAIL, area.y, RAIL, whole.bottom() - area.y);
    let column = Box::new(
        area.x,
        area.y,
        (rail.x - RAIL_GUTTER - area.x).max(280.0),
        area.h,
    );
    let meter = if desk.optimizing.running {
        meter_height(paint, desk, column.w)
    } else {
        0.0
    };
    let body = Box::new(column.x, column.y, column.w, (column.h - meter).max(120.0));
    let mut act = scrolled(
        paint,
        mouse,
        desk,
        Region::Page,
        body,
        |paint, mouse, inner| what_was_measured(paint, desk, mouse, inner),
    );
    if meter > 0.0
        && let Some(state) = meter_of(desk)
    {
        act = sweep_meter(
            paint,
            mouse,
            Box::new(column.x, column.bottom() - meter, column.w, meter),
            &state,
        )
        .or(act);
    }
    let (pressed, menu) = optimize_rail(paint, desk, mouse, rail);
    act = pressed.or(act);
    // Drawn last and clipped to nothing, so a dozen dials can hang over the column rather
    // than being cut off by the rail they were opened in.
    if let Some(field) = menu {
        let wide = field.w.max(200.0);
        let at = Box::new((field.x + field.w - wide).max(0.0), field.y, wide, field.h);
        if let Some(picked) = open_menu(paint, desk, mouse, Picker::Dial, at) {
            act = Some(picked);
        }
    }
    act
}

/// The same page with the rail's contents run in under the column instead of beside it.
fn optimize_stacked(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let mut stacked_menu = None;
    let meter = if desk.optimizing.running {
        meter_height(paint, desk, area.w)
    } else {
        0.0
    };
    let body = Box::new(area.x, area.y, area.w, (area.h - meter).max(120.0));
    let mut act = scrolled(
        paint,
        mouse,
        desk,
        Region::Page,
        body,
        |paint, mouse, inner| {
            let mut act = what_was_measured(paint, desk, mouse, inner);
            let mut y = paint.lowest() + 28.0;
            paint.rule(
                (inner.x, y - 14.0),
                (inner.right(), y - 14.0),
                paint.ink.line,
                255,
            );
            let (below, picked, opened) =
                the_sweep(paint, desk, mouse, Box::new(inner.x, y, inner.w, 0.0));
            act = act.or(picked);
            stacked_menu = opened;
            y = below;
            if !desk.optimizing.running {
                let wide = inner.w.min(260.0);
                if ui::button(
                    paint,
                    mouse,
                    Box::new(inner.x, y + 10.0, wide, ui::BUTTON),
                    start_label(desk),
                    start_kind(desk),
                ) {
                    act = Some(Act::Sweep);
                }
                y += ui::BUTTON + 10.0;
            }
            paint.reaches(y);
            act
        },
    );
    if meter > 0.0
        && let Some(state) = meter_of(desk)
    {
        act = sweep_meter(
            paint,
            mouse,
            Box::new(area.x, area.bottom() - meter, area.w, meter),
            &state,
        )
        .or(act);
    }
    if let Some(field) = stacked_menu
        && let Some(picked) = open_menu(paint, desk, mouse, Picker::Dial, field)
    {
        act = Some(picked);
    }
    act
}

fn start_label(desk: &Desk) -> &'static str {
    if desk
        .optimizing
        .run
        .as_ref()
        .is_some_and(mcf_optimize::running::Running::stopping)
    {
        return "Stopping";
    }
    if desk.optimizing.known > 0 {
        "Carry on"
    } else {
        "Run sweep"
    }
}

fn start_kind(desk: &Desk) -> Kind {
    let idle = desk
        .why_the_dial_does_nothing(desk.optimizing.sweep.dial)
        .is_some();
    let stopping = desk
        .optimizing
        .run
        .as_ref()
        .is_some_and(mcf_optimize::running::Running::stopping);
    if idle || stopping {
        Kind::Ordinary
    } else {
        Kind::Primary
    }
}

/// The rail: what the sweep is, and the button that starts it, pinned at the foot exactly
/// where the server page pins `Stop server`.
fn optimize_rail(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
) -> (Option<Act>, Option<Box>) {
    let ink = paint.ink;
    paint.rect(at, ink.card);
    paint.rule((at.x, at.y), (at.x, at.bottom()), ink.line, 255);
    let inside = Box::new(at.x + 22.0, at.y + PAD, (at.w - 44.0).max(80.0), at.h);
    let button = if desk.optimizing.running {
        0.0
    } else {
        ui::BUTTON + 20.0
    };
    let room = Box::new(
        inside.x,
        inside.y,
        inside.w,
        (at.bottom() - button - inside.y).max(80.0),
    );
    let mut menu = None;
    let mut act = scrolled(
        paint,
        mouse,
        desk,
        Region::Sweep,
        room,
        |paint, mouse, inner| {
            let (below, picked, opened) = the_sweep(paint, desk, mouse, inner);
            paint.reaches(below);
            menu = opened;
            picked
        },
    );
    if button > 0.0
        && ui::button(
            paint,
            mouse,
            Box::new(
                inside.x,
                at.bottom() - ui::BUTTON - 14.0,
                inside.w,
                ui::BUTTON,
            ),
            start_label(desk),
            start_kind(desk),
        )
    {
        act = Some(Act::Sweep);
    }
    (act, menu)
}

/// Everything that decides what the sweep will do, in the order the decisions are made:
/// which dial moves, how it is searched, what it is ranked by, what it is run against, and
/// finally the settings every trial is held under.
fn the_sweep(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
) -> (f32, Option<Act>, Option<Box>) {
    let mut act = None;
    let (below, picked, menu) = which_setting_moves(paint, desk, mouse, at);
    act = act.or(picked);
    let mut y = below;
    let dial = desk.optimizing.sweep.dial;

    // A list has no span to search: every value in it is run, so there is no choice here.
    if !dial.values_are_a_list() {
        y = rail_section(paint, at, y, "search");
        let ways: Vec<(String, bool)> = mcf_optimize::hunt::Way::ALL
            .iter()
            .map(|way| (way.label().to_owned(), *way == desk.optimizing.way))
            .collect();
        let (below, picked) = segmented(paint, mouse, Box::new(at.x, y, at.w, 0.0), &ways);
        if let Some(chosen) = picked {
            act = act.or(Some(Act::SweepWay(chosen)));
        }
        y = below + 10.0;
    }
    let (below, act) = the_rest_of_the_sweep(paint, desk, mouse, Box::new(at.x, y, at.w, 0.0), act);
    (below, act, menu)
}

/// Which dial the sweep moves, and what moving it actually does.
fn which_setting_moves(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
) -> (f32, Option<Act>, Option<Box>) {
    let ink = paint.ink;
    let mut act = None;
    let mut menu = None;
    let mut y = rail_section(paint, at, at.y, "setting");
    let dial = desk.optimizing.sweep.dial;
    let open = desk.open == Some(Picker::Dial);
    let field = Box::new(at.x, y, at.w, 30.0);
    if ui::picker(paint, mouse, field, dial.label(), open) {
        act = Some(Act::Open(Picker::Dial));
    }
    if open {
        menu = Some(field);
    }
    y = field.bottom() + 10.0;
    let why = if let Some(why) = desk.why_the_dial_does_nothing(dial) {
        why
    } else if !desk.optimizing.measure.needs_the_answers_run() {
        format!(
            "{} cannot change what a model answers, only how fast — so each value is timed \
             over {} tokens.",
            dial.flag().unwrap_or(dial.label()),
            if dial.times_reading_the_prompt() {
                mcf_optimize::trial::TOKENS_PREFILLED
            } else {
                mcf_optimize::trial::TOKENS_TIMED
            }
        )
    } else if dial.is_named_by_the_model() {
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
    let idle = desk.why_the_dial_does_nothing(dial).is_some();
    for line in paint.wrap(&why, Weight::Regular, size::SMALL, at.w) {
        paint.say_at(
            at.x,
            y,
            &line,
            Weight::Regular,
            size::SMALL,
            if idle { ink.warn } else { ink.faint },
        );
        y += gap::LINE;
    }
    (y + 10.0, act, menu)
}

/// What the sweep is ranked by, the values it runs, and what it runs against.
fn the_rest_of_the_sweep(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    at: Box,
    mut act: Option<Act>,
) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let dial = desk.optimizing.sweep.dial;
    let mut y = at.y;
    // A setting that cannot change an answer is only ever ranked by speed, and the line
    // above it already says so. Two buttons, one of which is refused, are not a choice.
    if !dial.cannot_change_an_answer() {
        y = rail_section(paint, at, y, "ranked by");
        let measures: Vec<(String, bool)> = mcf_optimize::reading::Measure::ALL
            .iter()
            .map(|measure| {
                (
                    measure.label().to_owned(),
                    *measure == desk.optimizing.measure,
                )
            })
            .collect();
        let (below, picked) = segmented(paint, mouse, Box::new(at.x, y, at.w, 0.0), &measures);
        if let Some(chosen) = picked {
            act = act.or(Some(Act::SweepMeasure(chosen)));
        }
        y = below + 6.0;
    }
    let span = dial.span();
    let said = if dial.is_named_by_the_model() {
        format!(
            "Every level this model names: {}.",
            desk.optimizing.named.join(", ")
        )
    } else if dial.values_are_a_list() {
        "Runs the values ticked below, and nothing else.".to_owned()
    } else if desk.optimizing.way == mcf_optimize::hunt::Way::Halving {
        format!(
            "Starts at {} and doubles until a value comes back worse, then halves either \
             side of the best down to steps of {}.",
            dial.step_of(dial.climbs_from()).said(),
            dial.step_of(span.finest).said()
        )
    } else {
        format!(
            "Runs the values below and nothing else, from {} to {}.",
            dial.step_of(span.floor).said(),
            dial.step_of(span.ceiling).said()
        )
    };
    for line in paint.wrap(&said, Weight::Regular, size::SMALL, at.w) {
        paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
        y += gap::LINE;
    }
    y += 10.0;

    if !dial.is_named_by_the_model() && desk.optimizing.way != mcf_optimize::hunt::Way::Halving {
        y = rail_section(paint, at, y, "values");
        let offered_steps: Vec<mcf_optimize::dial::Step> = dial.suggested();
        let values: Vec<(String, bool)> = offered_steps
            .iter()
            .map(|step| {
                (
                    dial.said_among(*step, &desk.optimizing.named),
                    desk.optimizing.sweep.steps.iter().any(|held| held == step),
                )
            })
            .collect();
        let (below, picked) =
            option_chips(paint, mouse, Box::new(at.x, y, at.w, 0.0), &values, true);
        if let Some(chosen) = picked {
            act = act.or(Some(Act::SweepValue(chosen)));
        }
        y = below + 4.0;
        // A list offers every value it has, so there is nothing of one's own to type.
        if dial.values_are_a_list() {
            y += 6.0;
        } else {
            let (below, typed) =
                a_value_of_my_own(paint, desk, mouse, Box::new(at.x, y, at.w, 0.0), y);
            act = act.or(typed);
            y = below;
        }
    }

    let (below, picked) = the_test_set(paint, desk, mouse, Box::new(at.x, y, at.w, 0.0));
    act = act.or(picked);
    y = below;
    (
        what_it_runs_under(paint, desk, Box::new(at.x, y, at.w, 0.0)),
        act,
    )
}

/// What the sweep is run against, and how many times each value is taken.
///
/// A timed trial reads nothing from a test set — it is handed a prompt to continue or a
/// prompt to read, the same one every time — so a timed sweep shows only its takes.
fn the_test_set(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (f32, Option<Act>) {
    let ink = paint.ink;
    let mut act = None;
    let marked = desk.optimizing.measure.needs_the_answers_run();
    let mut y = if marked {
        let mut y = rail_section(paint, at, at.y, "test set");
        let said = format!(
            "All {} questions, asked one at a time, every time — so two readings can be set \
             beside each other.",
            questions_in(&desk.optimizing.sweep.sets)
        );
        for line in paint.wrap(&said, Weight::Regular, size::SMALL, at.w) {
            paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
            y += gap::LINE;
        }
        y += 6.0;
        paint.say_at(
            at.x,
            y,
            "Takes of each",
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        y + 18.0
    } else {
        rail_section(paint, at, at.y, "takes of each")
    };
    let takes: Vec<(String, bool)> = (1..=3_u8)
        .map(|held| (held.to_string(), held == desk.optimizing.sweep.repeats))
        .collect();
    let (below, picked) = segmented(paint, mouse, Box::new(at.x, y, at.w, 0.0), &takes);
    if let Some(chosen) = picked {
        act = act.or(Some(Act::Takes(chosen.saturating_add(1))));
    }
    y = below + 6.0;
    let said = what_the_sweep_runs(desk);
    for line in paint.wrap(&said, Weight::Regular, size::SMALL, at.w) {
        paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
        y += gap::LINE;
    }
    (y + 10.0, act)
}

/// How many questions the sets named hold between them.
fn questions_in(sets: &[usize]) -> usize {
    sets.iter()
        .filter_map(|number| mcf_optimize::corpus::Set::numbered(*number))
        .map(|set| set.tasks.len())
        .sum()
}

/// What one press of the button runs, in the terms that sweep is counted in: questions
/// for a marked sweep, timed runs for a timed one. An automatic search chooses its own
/// values as it goes, so it is counted per value rather than against a list it has not
/// chosen yet.
fn what_the_sweep_runs(desk: &Desk) -> String {
    let sweep = &desk.optimizing.sweep;
    let takes = usize::from(sweep.repeats.max(1));
    let marked = desk.optimizing.measure.needs_the_answers_run();
    let each = if marked {
        format!(
            "{} questions",
            questions_in(&sweep.sets).saturating_mul(takes)
        )
    } else if takes == 1 {
        "One timed run".to_owned()
    } else {
        format!("{takes} timed runs")
    };
    let automatic =
        !sweep.dial.values_are_a_list() && desk.optimizing.way == mcf_optimize::hunt::Way::Halving;
    if automatic {
        return format!("{each} for each value the search tries.");
    }
    if sweep.steps.is_empty() {
        return "Nothing to run: tick at least one value.".to_owned();
    }
    let values = sweep
        .steps
        .iter()
        .map(|step| sweep.dial.said_among(*step, &desk.optimizing.named))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{each} for each of {values}.")
}

/// The settings every trial is held under, and anything the sweep refused.
fn what_it_runs_under(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let mut y = rail_section(paint, at, at.y, "base");
    if let Some(settings) = &desk.settings {
        let dialled = desk.optimizing.sweep.dial;
        let rows: [(&str, String, bool); 6] = [
            ("engine", settings.engine.clone(), false),
            ("context", format!("{}", settings.context), false),
            (
                "prompt batch",
                settings.batch.to_string(),
                dialled == mcf_optimize::dial::Dial::Batch,
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
            paint.say_at(at.x, y, name, Weight::Regular, size::SMALL, ink.quiet);
            let said = if moving {
                "the sweep moves this".to_owned()
            } else {
                value
            };
            let shown = paint.elide(&said, Weight::Regular, size::SMALL, at.w - 96.0);
            paint.say_right(
                at.right(),
                y,
                &shown,
                Weight::Regular,
                size::SMALL,
                if moving { ink.accent } else { ink.ink },
            );
            y += gap::ROW;
        }
    } else {
        for line in paint.wrap(
            "MCF has not worked out what this model would run under yet.",
            Weight::Regular,
            size::SMALL,
            at.w,
        ) {
            paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
            y += gap::LINE;
        }
    }
    if let Some(why) = &desk.optimizing.refused {
        y += 6.0;
        for line in paint.wrap(why, Weight::Regular, size::SMALL, at.w) {
            paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, ink.warn);
            y += gap::LINE;
        }
    }
    y + gap::BLOCK
}

/// The panel across the foot of the column while a sweep runs: the two buttons that act
/// on it, how far along it is, and what it has come to so far.
#[derive(Debug)]
pub struct Meter {
    pub stopping: bool,
    pub waiting: bool,
    /// What the sweep is on right now: the clock, the value, the set, the question.
    pub doing: String,
    /// What the value on screen has scored so far, and how long it has left — said as each
    /// question is marked, not once a set of them has finished.
    pub so_far: Option<String>,
    /// What it has counted so far.
    pub along: String,
    pub fraction: Option<f32>,
    pub best: Option<String>,
}

fn meter_of(desk: &Desk) -> Option<Meter> {
    let run = desk.optimizing.run.as_ref()?;
    let stopping = run.stopping();
    let best = desk
        .optimizing
        .report
        .best_by(desk.optimizing.measure)
        .map(|best| {
            let at = desk
                .optimizing
                .sweep
                .dial
                .said_among(best.step, &desk.optimizing.named);
            match desk.optimizing.measure {
                mcf_optimize::reading::Measure::Speed => format!(
                    "best {} at {at}",
                    best.tokens_a_second().map_or_else(
                        || UNKNOWN_FIGURE.to_owned(),
                        |rate| format!("{rate:.0} tok/s")
                    )
                ),
                mcf_optimize::reading::Measure::Correctness => {
                    format!("best {}/{} at {at}", best.passed, best.of)
                }
            }
        });
    Some(Meter {
        stopping,
        waiting: run.asked_to_wait(),
        doing: if stopping {
            "stopping — this trial is being thrown away".to_owned()
        } else {
            run.label(
                &desk.optimizing.named,
                desk.optimizing.sweep.dial,
                desk.optimizing.measure,
                desk.optimizing.ceiling_of_a_trial(),
            )
        },
        so_far: run.so_far(&desk.optimizing.named, desk.optimizing.sweep.dial),
        along: run.far_along(),
        fraction: if stopping {
            None
        } else {
            how_far_through(desk, run)
        },
        best,
    })
}

/// How much of the bar to fill. An automatic search has no total to count towards, so a
/// sweep asked a question at a time fills it across the value on screen; one by hand fills
/// it across the whole sweep, with the trial under way counted a question at a time rather
/// than all at once when it finishes.
fn how_far_through(desk: &Desk, run: &mcf_optimize::running::Running) -> Option<f32> {
    let automatic = desk.optimizing.way == mcf_optimize::hunt::Way::Halving
        && !desk.optimizing.sweep.dial.values_are_a_list();
    if automatic {
        return run
            .through_the_value()
            .or_else(|| desk.optimizing.fraction());
    }
    let whole = desk.optimizing.fraction()?;
    let Some((question, of)) = run.question else {
        return Some(whole);
    };
    let trials = u16::try_from(desk.optimizing.sweep.trials().max(1)).ok()?;
    let within = f32::from(u16::try_from(question.saturating_sub(1)).ok()?)
        / f32::from(u16::try_from(of.max(1)).ok()?);
    Some((whole + within / f32::from(trials)).min(1.0))
}

/// The lines under the bar, as they will be drawn at this width: what the sweep is on,
/// what the value on screen has come to, and what the sweep has counted. Each wraps rather
/// than being cut off, because a line elided to fit is the part of it that was news.
fn meter_lines(paint: &mut Painter, meter: &Meter, wide: f32) -> Vec<(String, Weight, bool)> {
    let mut lines = Vec::new();
    for line in paint.wrap(&meter.doing, Weight::Bold, size::SMALL, wide) {
        lines.push((line, Weight::Bold, true));
    }
    if let Some(so_far) = &meter.so_far {
        for line in paint.wrap(so_far, Weight::Regular, size::SMALL, wide) {
            lines.push((line, Weight::Regular, true));
        }
    }
    for line in paint.wrap(&meter.along, Weight::Regular, size::SMALL, wide) {
        lines.push((line, Weight::Regular, false));
    }
    lines
}

const METER_PAD: f32 = 12.0;
const METER_BAR: f32 = 8.0;

/// How tall the panel is at this width. Measured before the page is laid out, so the
/// column above it gives up exactly the room the panel takes and no more.
fn meter_height(paint: &mut Painter, desk: &Desk, wide: f32) -> f32 {
    let Some(meter) = meter_of(desk) else {
        return 0.0;
    };
    let lines = meter_lines(paint, &meter, (wide - 2.0 * METER_PAD).max(80.0)).len();
    let lines = u16::try_from(lines).unwrap_or(u16::MAX);
    METER_PAD + ui::BUTTON + 10.0 + METER_BAR + 10.0 + f32::from(lines) * gap::LINE + METER_PAD
}

pub fn sweep_meter(paint: &mut Painter, mouse: &Mouse, at: Box, meter: &Meter) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    paint.rect(at, ink.card);
    paint.rule((at.x, at.y), (at.right(), at.y), ink.line, 255);
    let Meter {
        stopping, waiting, ..
    } = *meter;
    let left = at.x + METER_PAD;
    let wide = (at.w - 2.0 * METER_PAD).max(80.0);
    let y = at.y + METER_PAD;

    let (pressed, paused) = ui::fitted(
        paint,
        mouse,
        (left, y),
        if waiting { "Carry on" } else { "Pause" },
        if stopping {
            Kind::Quiet
        } else if waiting {
            Kind::Primary
        } else {
            Kind::Ordinary
        },
    );
    if pressed && !stopping {
        act = Some(Act::PauseSweep);
    }
    let (pressed, stopped) = ui::fitted(
        paint,
        mouse,
        (paused.right() + 8.0, y),
        if stopping { "Stopping" } else { "Stop" },
        Kind::Quiet,
    );
    if pressed {
        act = Some(Act::Sweep);
    }
    if let Some(best) = &meter.best {
        let room = (left + wide - stopped.right() - 18.0).max(0.0);
        if room > 60.0 {
            let shown = paint.elide(best, Weight::Regular, size::SMALL, room);
            paint.say_right(
                left + wide,
                y + (ui::BUTTON - gap::LINE) / 2.0,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.quiet,
            );
        }
    }

    // The meter itself. A sweep that cannot say how far it has to go is drawn as an
    // unmeasured bar rather than a full one.
    let mut y = y + ui::BUTTON + 10.0;
    ui::progress(paint, Box::new(left, y, wide, METER_BAR), meter.fraction);
    y += METER_BAR + 10.0;
    for (line, weight, strong) in meter_lines(paint, meter, wide) {
        paint.say_at(
            left,
            y,
            &line,
            weight,
            size::SMALL,
            if strong { ink.ink } else { ink.faint },
        );
        y += gap::LINE;
    }
    act
}

/// A track holding every option with the taken one raised out of it.
///
/// `ui::nav` draws nothing at all for an option that is not the one taken, so a row of them
/// reads as one button beside some captions rather than as a choice. The track is what says
/// a choice is being offered, before a single label is read.
fn segmented(
    paint: &mut Painter,
    mouse: &Mouse,
    at: Box,
    labels: &[(String, bool)],
) -> (f32, Option<usize>) {
    let ink = paint.ink;
    if labels.is_empty() {
        return (at.y, None);
    }
    let mut picked = None;
    let widths: Vec<f32> = labels
        .iter()
        .map(|(label, _)| paint.measure(label, Weight::Bold, size::BODY) + 26.0)
        .collect();
    let whole: f32 = widths.iter().sum();
    // Wider than the room it has, the track gives every option the same share rather than
    // running off the edge.
    let widths: Vec<f32> = if whole > at.w {
        let each = (at.w / labels.len() as f32).max(40.0);
        labels.iter().map(|_| each).collect()
    } else {
        widths
    };
    let track = Box::new(at.x, at.y, widths.iter().sum(), gap::CONTROL + 2.0);
    paint.edge(track, 9.0, ink.line, ink.sunk);
    let mut x = track.x;
    for (at_index, ((label, on), wide)) in labels.iter().zip(&widths).enumerate() {
        let seg = Box::new(x, track.y, *wide, track.h);
        if *on {
            paint.panel(seg.inset(3.0), 6.0, ink.accent, 255);
        } else if mouse.over(seg) {
            paint.panel(seg.inset(3.0), 6.0, ink.line, 120);
        }
        let after_another_unpicked = at_index > 0
            && !*on
            && labels
                .get(at_index.saturating_sub(1))
                .is_some_and(|(_, before)| !*before);
        if after_another_unpicked {
            paint.rule((x, seg.y + 8.0), (x, seg.bottom() - 8.0), ink.line, 255);
        }
        let shown = paint.elide(label, Weight::Bold, size::BODY, seg.w - 10.0);
        paint.say_centred(
            seg,
            &shown,
            if *on { Weight::Bold } else { Weight::Regular },
            size::BODY,
            if *on { ink.accent_ink } else { ink.quiet },
        );
        if mouse.clicked(seg) {
            picked = Some(at_index);
        }
        x += wide;
    }
    (track.bottom() + 6.0, picked)
}

/// Chips that each keep their own edge and a box to tick, for the sets where more than one
/// can be on at once. A track would be wrong here: these are not alternatives.
fn option_chips(
    paint: &mut Painter,
    mouse: &Mouse,
    at: Box,
    labels: &[(String, bool)],
    many: bool,
) -> (f32, Option<usize>) {
    let ink = paint.ink;
    let mut picked = None;
    let mut x = at.x;
    let mut line = at.y;
    for (index, (label, on)) in labels.iter().enumerate() {
        let lead = if many { 18.0 } else { 0.0 };
        let wide = paint.measure(label, Weight::Bold, size::BODY) + 26.0 + lead;
        if x + wide > at.right() && x > at.x {
            x = at.x;
            line += gap::CONTROL + 8.0;
        }
        let chip = Box::new(x, line, wide, gap::CONTROL + 2.0);
        if *on {
            paint.edge(chip, 8.0, ink.accent, ink.accent);
        } else if mouse.over(chip) {
            paint.edge(chip, 8.0, ink.faint, ink.card);
        } else {
            paint.edge(chip, 8.0, ink.line, ink.card);
        }
        if many {
            let mark = Box::new(chip.x + 10.0, chip.y + 9.0, 12.0, 12.0);
            if *on {
                ui::tick(paint, mark, ink.accent_ink);
            } else {
                paint.edge(mark, 3.0, ink.faint, ink.card);
            }
        }
        paint.say_at(
            chip.x + 13.0 + lead,
            chip.y + 7.0,
            label,
            if *on { Weight::Bold } else { Weight::Regular },
            size::BODY,
            if *on { ink.accent_ink } else { ink.quiet },
        );
        if mouse.clicked(chip) {
            picked = Some(index);
        }
        x += wide + 8.0;
    }
    (line + gap::CONTROL + 2.0 + gap::BLOCK, picked)
}

/// The column: what the sweep found, the shape of it, and then every reading behind it.
fn what_was_measured(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let ink = paint.ink;
    let mut act = None;
    let mut y = area.y;
    if desk.chosen.and_then(|at| desk.models.get(at)).is_none() {
        paint.say_at(
            area.x,
            y,
            "Choose a model on the left and its readings show here.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return None;
    }
    spaced(paint, area.x, y, "what the sweep found", ink.faint);
    y += 22.0;
    y = found_tiles(paint, desk, Box::new(area.x, y, area.w, 0.0));
    let (below, looked) = now_asking(paint, desk, mouse, Box::new(area.x, y + 14.0, area.w, 0.0));
    act = act.or(looked);
    y = below;
    let (below, decided) = what_it_found(paint, desk, mouse, area, y + 14.0);
    act = act.or(decided);
    y = below;

    if desk.optimizing.rows.is_empty() {
        for line in paint.wrap(
            "Nothing measured against this configuration yet. Every reading is written down \
             as it finishes, and every one taken under exactly these settings shows here — \
             from this sweep and from any before it.",
            Weight::Regular,
            size::SMALL,
            area.w.min(620.0),
        ) {
            paint.say_at(area.x, y, &line, Weight::Regular, size::SMALL, ink.faint);
            y += gap::LINE;
        }
        paint.reaches(y);
        return act;
    }

    y = readings_chart(paint, desk, Box::new(area.x, y + 6.0, area.w, 190.0));
    y += 20.0;
    spaced(paint, area.x, y, "readings", ink.faint);
    y += 22.0;
    act = act.or(rows_of_the_record(paint, desk, mouse, area, &mut y));
    paint.reaches(y);
    act
}

/// How many lines of a reply the panel shows: the latest of what the model is thinking, and
/// the latest of what it has answered.
const THOUGHT_LINES: usize = 4;
const ANSWER_LINES: usize = 3;

/// The question being asked and what has come of it: what it asks, the model's reply as it
/// arrives, the answer wanted, and the verdict the moment it is marked — with the whole set
/// beneath it as a strip of squares, one a question, that can be pressed to look back.
///
/// Drawn for a sweep that marks answers and nothing else: a timed trial continues a prompt
/// nobody asked, so there is no question to show.
fn now_asking(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> (f32, Option<Act>) {
    let Some(run) = desk.optimizing.run.as_ref() else {
        return (at.y, None);
    };
    if !desk.optimizing.measure.needs_the_answers_run() || run.questions.is_empty() {
        return (at.y, None);
    }
    let Some(shown) = desk.optimizing.shown_question() else {
        return (at.y, None);
    };
    let Some(seen) = run.questions.get(shown) else {
        return (at.y, None);
    };
    let ink = paint.ink;
    let inner = (at.w - 2.0 * ASKING_PAD).max(120.0);
    let following = run.following() == Some(shown);

    // Everything is measured before anything is drawn, so the card is as tall as it holds.
    let asked = paint.wrap(&seen.asked, Weight::Regular, size::BODY, inner);
    let reply = reply_lines(paint, seen, inner - 24.0);
    let reply_h = 12.0 + gap::LINE * (1.0 + count_of(reply.len().max(1))) + 10.0;
    let square = ((inner - 4.0 * 24.0) / 25.0).clamp(8.0, 18.0);
    let rows = run.questions.len().div_ceil(25).max(1);
    let strip_h = 22.0 + (square + 4.0) * count_of(rows);
    let asked_h = ASKED_LINE * count_of(asked.len());
    let height = 2.0 * ASKING_PAD + 26.0 + asked_h + 10.0 + reply_h + 12.0 + 40.0 + strip_h;
    let card = Box::new(at.x, at.y, at.w, height);
    paint.edge(
        card,
        10.0,
        if following { ink.accent } else { ink.line },
        ink.card,
    );

    let left = at.x + ASKING_PAD;
    let mut y = asking_heading(
        paint,
        desk,
        Box::new(left, at.y + ASKING_PAD, inner, 0.0),
        shown,
        following,
    );
    for line in &asked {
        paint.say_at(left, y, line, Weight::Regular, size::BODY, ink.ink);
        y += ASKED_LINE;
    }
    y += 10.0;
    reply_well(paint, seen, &reply, Box::new(left, y, inner, reply_h));
    y += reply_h + 12.0;
    verdict_row(paint, seen, Box::new(left, y, inner, 0.0));
    y += 40.0;
    let act = question_strip(
        paint,
        mouse,
        run,
        Box::new(left, y, inner, square),
        (shown, following),
    );
    (card.bottom(), act)
}

const ASKING_PAD: f32 = 16.0;
const ASKED_LINE: f32 = 21.0;

fn count_of(held: usize) -> f32 {
    f32::from(u16::try_from(held).unwrap_or(u16::MAX))
}

/// The label that says whether the panel is following the sweep or looking back, and where
/// in the sweep the question shown stands.
fn asking_heading(paint: &mut Painter, desk: &Desk, at: Box, shown: usize, following: bool) -> f32 {
    let ink = paint.ink;
    spaced(
        paint,
        at.x,
        at.y,
        if following {
            "now asking"
        } else {
            "looking back"
        },
        if following { ink.accent } else { ink.faint },
    );
    let Some(run) = desk.optimizing.run.as_ref() else {
        return at.y + 26.0;
    };
    let of = run.questions.len();
    let dial = desk.optimizing.sweep.dial;
    let mut said = run.doing.map_or_else(String::new, |doing| {
        format!(
            "{} {} · set {}",
            dial.label(),
            dial.said_among(doing.step, &desk.optimizing.named),
            doing.set
        )
    });
    if let Some((place, sets)) = run.place.filter(|(_, sets)| *sets > 1) {
        let _wrote = std::fmt::Write::write_fmt(&mut said, format_args!(" ({place} of {sets})"));
    }
    if !said.is_empty() {
        said.push_str(" · ");
    }
    let _wrote = std::fmt::Write::write_fmt(
        &mut said,
        format_args!("question {} of {of}", shown.saturating_add(1)),
    );
    let shown_where = paint.elide(
        &said,
        Weight::Regular,
        size::SMALL,
        (at.w - 110.0).max(60.0),
    );
    paint.say_right(
        at.right(),
        at.y,
        &shown_where,
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    at.y + 26.0
}

/// The end of the reply as it will be drawn: the latest of the thinking, then the latest of
/// the answer, each marked with which it is.
fn reply_lines(
    paint: &mut Painter,
    seen: &mcf_optimize::running::Seen,
    room: f32,
) -> Vec<(String, bool)> {
    let mut thought = paint.wrap(seen.thought.trim(), Weight::Regular, size::SMALL, room);
    let skip = thought.len().saturating_sub(THOUGHT_LINES);
    thought.drain(..skip);
    let mut answer = paint.wrap(seen.answer.trim(), Weight::Bold, size::SMALL, room);
    let skip = answer.len().saturating_sub(ANSWER_LINES);
    answer.drain(..skip);
    thought
        .into_iter()
        .map(|line| (line, false))
        .chain(answer.into_iter().map(|line| (line, true)))
        .collect()
}

/// The reply, in a well of its own: how far it has got, the end of the thinking dimmed, and
/// the end of the answer.
fn reply_well(
    paint: &mut Painter,
    seen: &mcf_optimize::running::Seen,
    lines: &[(String, bool)],
    well: Box,
) {
    let ink = paint.ink;
    paint.panel(well, 8.0, ink.sunk, 255);
    let count = match (&seen.verdict, seen.milliseconds) {
        (None, _) if !seen.sent => "not asked yet".to_owned(),
        (None, _) if seen.answer.trim().is_empty() => {
            format!("thinking · {} tokens", seen.produced)
        }
        (None, _) => format!("answering · {} tokens", seen.produced),
        (Some(_), Some(held)) => format!(
            "answered · {} tokens · {:.1} s",
            seen.produced,
            f64::from(u32::try_from(held).unwrap_or(u32::MAX)) / 1000.0
        ),
        (Some(_), None) => format!("answered · {} tokens", seen.produced),
    };
    let left = well.x + 12.0;
    let mut y = well.y + 10.0;
    paint.say_at(left, y, &count, Weight::Regular, size::SMALL, ink.faint);
    for (line, answering) in lines {
        y += gap::LINE;
        if *answering {
            paint.say_at(left, y, line, Weight::Bold, size::SMALL, ink.ink);
        } else {
            paint.say_at(left, y, line, Weight::Regular, size::SMALL, ink.faint);
        }
    }
}

/// The verdict, with what was wanted beside it, so a marking fault and a wrong answer look
/// different.
fn verdict_row(paint: &mut Painter, seen: &mcf_optimize::running::Seen, at: Box) {
    use mcf_optimize::running::Verdict;
    let ink = paint.ink;
    let (said, colour) = match &seen.verdict {
        None => ("Asking".to_owned(), ink.accent),
        Some(Verdict::Right) => ("Right".to_owned(), ink.good),
        Some(Verdict::Wrong(given)) => (
            given.as_ref().map_or_else(
                || "Wrong — no answer line".to_owned(),
                |given| format!("Wrong — said {given}"),
            ),
            ink.bad,
        ),
        Some(Verdict::RanAway(_)) => ("Ran away — marked wrong".to_owned(), ink.warn),
    };
    let chip_w = paint.measure(&said, Weight::Bold, size::SMALL) + 20.0;
    let chip = Box::new(at.x, at.y, chip_w, 24.0);
    paint.panel(chip, 12.0, colour, 40);
    paint.say_at(
        at.x + 10.0,
        at.y + 4.0,
        &said,
        Weight::Bold,
        size::SMALL,
        colour,
    );
    let wanted = format!("Wanted {}", seen.wanted);
    let room = (at.w - chip_w - 16.0).max(40.0);
    let wanted = paint.elide(&wanted, Weight::Regular, size::SMALL, room);
    paint.say_at(
        chip.right() + 12.0,
        at.y + 4.0,
        &wanted,
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
}

/// The set, a square a question, coloured by what came of it. A square is pressed to look
/// back at its question.
fn question_strip(
    paint: &mut Painter,
    mouse: &Mouse,
    run: &mcf_optimize::running::Running,
    at: Box,
    (shown, following): (usize, bool),
) -> Option<Act> {
    use mcf_optimize::running::Verdict;
    let ink = paint.ink;
    let square = at.h;
    paint.say_at(
        at.x,
        at.y,
        "This set, question by question",
        Weight::Regular,
        size::SMALL,
        ink.quiet,
    );
    paint.say_right(
        at.right(),
        at.y,
        "press one to look back",
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    let top = at.y + 22.0;
    let mut act = None;
    for (index, each) in run.questions.iter().enumerate() {
        let column = count_of(index.rem_euclid(25));
        let row = count_of(index.div_euclid(25));
        let place = Box::new(
            at.x + column * (square + 4.0),
            top + row * (square + 4.0),
            square,
            square,
        );
        let fill = match &each.verdict {
            Some(Verdict::Right) => Some(ink.good),
            Some(Verdict::Wrong(_)) => Some(ink.bad),
            Some(Verdict::RanAway(_)) => Some(ink.warn),
            None => None,
        };
        if index == shown && !following {
            paint.edge(place.inset(-3.0), 6.0, ink.ink, ink.card);
        }
        match fill {
            Some(colour) => paint.panel(place, 4.0, colour, 255),
            None if each.open() => paint.edge(place, 4.0, ink.accent, ink.card),
            None => paint.edge(place, 4.0, ink.line, ink.sunk),
        }
        if each.sent && mouse.clicked(place) {
            act = Some(Act::LookAt(index));
        }
    }
    act
}

/// The three figures a sweep exists to produce, as tiles. While a sweep runs they say what
/// it is finding as it finds it — the value being measured, its score so far, and how fast
/// it is going — rather than waiting for a set of twenty-five to be written down before
/// they say anything at all.
fn found_tiles(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let tiles = what_the_tiles_say(desk);
    let across = 3.0;
    let wide = ((at.w - 12.0 * (across - 1.0)) / across).max(90.0);
    for (index, (label, figure, note, lift)) in tiles.iter().enumerate() {
        let (label, note) = (label.as_str(), note.as_str());
        let tile = Box::new(at.x + (wide + 12.0) * index as f32, at.y, wide, 84.0);
        ui::card(paint, tile, *lift && figure.is_some());
        spaced(paint, tile.x + 14.0, tile.y + 13.0, label, ink.faint);
        let (said, colour) = figure.as_ref().map_or_else(
            || (UNKNOWN_FIGURE.to_owned(), ink.faint),
            |said| (said.clone(), if *lift { ink.accent } else { ink.ink }),
        );
        let shown = paint.elide(&said, Weight::Bold, 24.0, tile.w - 28.0);
        paint.say_at(
            tile.x + 14.0,
            tile.y + 28.0,
            &shown,
            Weight::Bold,
            24.0,
            colour,
        );
        if paint.measure(note, Weight::Regular, size::SMALL) <= tile.w - 28.0 {
            paint.say_at(
                tile.x + 14.0,
                tile.y + 60.0,
                note,
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        }
    }
    at.y + 84.0
}

type Tile = (String, Option<String>, String, bool);

fn what_the_tiles_say(desk: &Desk) -> [Tile; 3] {
    let best = desk.optimizing.report.best_by(desk.optimizing.measure);
    let dial = desk.optimizing.sweep.dial;
    let named = &desk.optimizing.named;
    let run = desk.optimizing.run.as_ref();
    let tally = run.map(|run| run.tally).filter(|tally| tally.asked > 0);
    let settled = best
        .as_ref()
        .is_some_and(|best| desk.optimizing.settled == Some(best.step));
    let correctness = desk.optimizing.measure == mcf_optimize::reading::Measure::Correctness;

    let first = match (&best, run.and_then(|run| run.doing)) {
        (Some(best), _) => (
            if settled { "best value" } else { "best so far" }.to_owned(),
            Some(dial.said_among(best.step, named)),
            dial.label().to_owned(),
            true,
        ),
        (None, Some(doing)) => (
            "measuring".to_owned(),
            Some(dial.said_among(doing.step, named)),
            "no value has finished yet".to_owned(),
            false,
        ),
        (None, None) => (
            "best so far".to_owned(),
            None,
            dial.label().to_owned(),
            true,
        ),
    };

    let second = match (&best, tally) {
        (Some(best), _) if correctness => (
            "correctness".to_owned(),
            best.passed
                .saturating_mul(100)
                .checked_div(best.of)
                .map(|share| format!("{share}%")),
            format!("{} of {} right", best.passed, best.of),
            false,
        ),
        (Some(best), _) => (
            "speed".to_owned(),
            best.tokens_a_second().map(|rate| format!("{rate:.0}")),
            "tokens a second".to_owned(),
            false,
        ),
        (None, Some(tally)) => (
            "right so far".to_owned(),
            tally.share().map(|share| format!("{share}%")),
            format!(
                "{} of {} at {}",
                tally.right,
                tally.asked,
                tally
                    .value
                    .map_or_else(String::new, |step| dial.said_among(step, named))
            ),
            false,
        ),
        (None, None) => (
            desk.optimizing.measure.label().to_owned(),
            None,
            if correctness {
                "of the questions asked"
            } else {
                "tokens a second"
            }
            .to_owned(),
            false,
        ),
    };

    let third = match tally.and_then(|tally| Some((tally, tally.a_question()?))) {
        Some((tally, each)) => (
            "pace".to_owned(),
            Some(format!("{:.1} s", each.as_secs_f64())),
            tally.tokens_a_second().map_or_else(
                || "a question".to_owned(),
                |rate| format!("a question · {rate} tok/s"),
            ),
            false,
        ),
        None => (
            "readings".to_owned(),
            Some(desk.optimizing.rows.len().to_string()),
            "under exactly this configuration".to_owned(),
            false,
        ),
    };
    [first, second, third]
}

/// How long a bar is, for a figure against the largest of them.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a bar's length in points, bounded by the width of a card"
)]
fn along_by(room: f32, figure: f64, most: f64) -> f32 {
    if most <= 0.0 {
        return 3.0;
    }
    ((f64::from(room) * (figure / most)) as f32).max(3.0)
}

/// Every reading drawn as a bar, so the shape of the sweep can be seen before the numbers
/// are read. One bar per value, the best one in the accent.
/// One bar per value, taking the best reading at each — a value run three times is one
/// bar, not three.
fn bars_of(desk: &Desk) -> Vec<(mcf_optimize::dial::Step, f64, String)> {
    let measure = desk.optimizing.measure;
    let mut bars: Vec<(mcf_optimize::dial::Step, f64, String)> = Vec::new();
    for row in &desk.optimizing.rows {
        let Some(figure) = (match measure {
            mcf_optimize::reading::Measure::Speed => row.reading.tokens_a_second(),
            mcf_optimize::reading::Measure::Correctness => (row.reading.of > 0)
                .then(|| f64::from(row.reading.passed) / f64::from(row.reading.of)),
        }) else {
            continue;
        };
        let said = match measure {
            mcf_optimize::reading::Measure::Speed => format!("{figure:.0}"),
            mcf_optimize::reading::Measure::Correctness => {
                format!("{}/{}", row.reading.passed, row.reading.of)
            }
        };
        match bars
            .iter_mut()
            .find(|(step, _, _)| *step == row.reading.step)
        {
            Some(held) if figure > held.1 => {
                held.1 = figure;
                held.2 = said;
            }
            Some(_) => {}
            None => bars.push((row.reading.step, figure, said)),
        }
    }
    bars.sort_by_key(|(step, _, _)| match *step {
        mcf_optimize::dial::Step::Whole(value) | mcf_optimize::dial::Step::Thousandths(value) => {
            value
        }
    });
    bars
}

fn readings_chart(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let dial = desk.optimizing.sweep.dial;
    let measure = desk.optimizing.measure;
    let bars = bars_of(desk);
    if bars.len() < 2 {
        return at.y;
    }
    let most = bars
        .iter()
        .map(|(_, figure, _)| *figure)
        .fold(0.0_f64, f64::max);
    if most <= 0.0 {
        return at.y;
    }
    let best = desk
        .optimizing
        .report
        .best_by(measure)
        .map(|best| best.step);
    let tall = 22.0;
    let card = Box::new(at.x, at.y, at.w, 34.0 + tall * bars.len() as f32 + 12.0);
    ui::card(paint, card, false);
    spaced(
        paint,
        card.x + 16.0,
        card.y + 14.0,
        &format!("{} by {}", measure.label(), dial.label().to_lowercase()),
        ink.faint,
    );
    let mut y = card.y + 34.0;
    let named = &desk.optimizing.named;
    for (step, figure, said) in &bars {
        let label = dial.said_among(*step, named);
        let shown = paint.elide(&label, Weight::Regular, size::SMALL, 76.0);
        paint.say_at(
            card.x + 16.0,
            y + 3.0,
            &shown,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        let from = card.x + 100.0;
        let room = (card.w - 100.0 - 32.0 - 72.0).max(40.0);
        let on_top = best == Some(*step);
        paint.panel(
            Box::new(from, y + 2.0, room, tall - 10.0),
            4.0,
            ink.sunk,
            255,
        );
        paint.panel(
            Box::new(from, y + 2.0, along_by(room, *figure, most), tall - 10.0),
            4.0,
            if on_top { ink.accent } else { ink.line },
            255,
        );
        paint.say_at(
            from + room + 10.0,
            y + 3.0,
            said,
            if on_top {
                Weight::Bold
            } else {
                Weight::Regular
            },
            size::SMALL,
            if on_top { ink.accent } else { ink.quiet },
        );
        y += tall;
    }
    card.bottom()
}

/// The live figures for this model, if this model is the one being run. A model that is
/// not held has nothing live to show, and saying so is the honest thing; a model that is
/// held has the same figures the server page shows, because they are the same figures.
fn live_for<'a>(desk: &'a Desk, held: &Model) -> Option<&'a crate::Use> {
    if let Some(hosting) = desk.hosted.as_ref()
        && hosting.model == held.path
    {
        return hosting.in_use.as_ref();
    }
    desk.under_test
        .as_ref()
        .filter(|under| under.model == held.path)
        .map(|under| &under.in_use)
}

fn statistics_tab(paint: &mut Painter, desk: &Desk, area: Box, held: &Model) {
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
    let Some(in_use) = live_for(desk, held) else {
        section(
            paint,
            &mut y,
            "Use",
            &[
                "Nothing running for this model.".to_owned(),
                "MCF records what a hold actually did — the settings, this machine, and \
                 the rates that followed — as it holds it. Host this one and the figures \
                 appear here as they are measured."
                    .to_owned(),
            ],
            ink.faint,
        );
        return;
    };
    spaced(paint, right.x, y, "use", ink.faint);
    y += 20.0;
    let y = use_tiles(paint, in_use, Box::new(right.x, y, right.w, 0.0));
    let y = machine_and_run_tiles(paint, desk, Box::new(right.x, y + 8.0, right.w, 0.0));
    let _below = token_totals(
        paint,
        desk,
        &desk.tallies,
        Box::new(right.x, y + 8.0, right.w, 108.0),
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
    Idle(String),
    Pick(Picker, String),
    Words(crate::Field, String, &'static str),
    Number(crate::Field, String, String),
    Flip(crate::Switch, bool),
}

/// The settings this model's own chat template reads, as controls.
///
/// Named as the template names them, because that is what the model will read and MCF
/// does not know what any of them mean — only that the template asks for them. The line
/// under each says what the template does when nobody says anything, so leaving a control
/// alone and setting it to that are visibly the same thing.
fn what_the_template_takes(desk: &Desk) -> Vec<(String, Control, String)> {
    let mut rows = Vec::new();
    for (at, held) in desk.template_takes().into_iter().enumerate() {
        let Ok(which) = u8::try_from(at) else {
            break;
        };
        let sent = desk.what_is_taken(&held.name);
        let control = match &held.takes {
            mcf_serve::parameters::Takes::Switch { on_unless_asked } => {
                let on = sent
                    .and_then(|held| match held {
                        mcf_record::json::Value::Bool(on) => Some(*on),
                        _ => None,
                    })
                    .unwrap_or_else(|| on_unless_asked.unwrap_or(false));
                Control::Flip(crate::Switch::TemplateTakes(which), on)
            }
            mcf_serve::parameters::Takes::Word { allowed, .. } if !allowed.is_empty() => {
                Control::Pick(
                    crate::Picker::TemplateWord(which),
                    sent.and_then(|held| held.as_text())
                        .map_or_else(|| "the template's own".to_owned(), str::to_owned),
                )
            }
            mcf_serve::parameters::Takes::Count { .. } => Control::Number(
                crate::Field::TemplateWord(which),
                sent.and_then(mcf_record::json::Value::as_integer)
                    .map_or_else(String::new, |whole| whole.to_string()),
                held.default_said(),
            ),
            mcf_serve::parameters::Takes::Word { .. } => Control::Words(
                crate::Field::TemplateWord(which),
                sent.and_then(|held| held.as_text())
                    .map_or_else(String::new, str::to_owned),
                "the template's own",
            ),
        };
        rows.push((held.name.clone(), control, held.default_said()));
    }
    rows
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

struct Row<'a> {
    name: &'a str,
    /// Why the setting is there, in MCF's words. A setting read off a model's own template
    /// has none: MCF knows the model will read it and not what it means, and inventing an
    /// explanation for somebody else's parameter would be inventing it.
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
        |paint: &mut Painter, y: f32, row: Row<'_>, hovered: &mut Option<(&'static str, f32)>| {
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
            name: "Chat template",
            because: "the Jinja the engine turns a conversation into tokens with. This is \
                      the one packed into the model's own file, which is what it was \
                      published to be addressed with; edit it and MCF holds the model \
                      under what you leave here instead",
        },
        &mut hovered,
    );
    {
        let focused = desk
            .editing
            .as_ref()
            .is_some_and(|(editing, _)| *editing == crate::Field::ChatTemplate);
        let held = if focused {
            desk.typing_now().clone()
        } else {
            crate::typing::Typing::of(desk.template_now())
        };
        let box_of = Box::new(column, y - 6.0, control.max(240.0), TEMPLATE_TALL);
        let (touched, moved) = ui::lines(
            paint,
            mouse,
            box_of,
            &held,
            desk.scrolled(crate::Region::Template),
            focused,
        );
        if touched != ui::Touched::No {
            act = Some(Act::Edit(crate::Field::ChatTemplate, touched));
        }
        if (moved - desk.scrolled(crate::Region::Template)).abs() > 0.5 {
            act = act.or(Some(Act::Scroll(
                crate::Region::Template,
                moved.round() as i32,
            )));
        }
        y += TEMPLATE_TALL + 6.0;
        if desk.template_is_the_model_s_own() {
            paint.say_at(
                column,
                y,
                "as the file has it",
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        } else {
            let (back, _where) =
                ui::fitted(paint, mouse, (column, y - 6.0), "Put it back", Kind::Quiet);
            if back {
                act = Some(Act::TemplateAsPublished);
            }
        }
        y += 30.0;
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
    let number = |field: crate::Field, now: String| Control::Number(field, now, String::new());
    // What the model's own file asks for, in the words `mcf settings` uses for it. A blank
    // box is otherwise the same blank whether the file said nothing or MCF lost what it said.
    let published = |of: &dyn Fn(&mcf_serve::hosting::Hosting) -> Option<String>| -> String {
        desk.recommended
            .as_ref()
            .and_then(of)
            .unwrap_or_else(|| "the engine's own".to_owned())
    };
    let sampled = |field: crate::Field,
                   now: String,
                   of: &dyn Fn(&mcf_serve::hosting::Hosting) -> Option<String>| {
        Control::Number(field, now, published(of))
    };
    let flip = |held: crate::Switch, on: bool| Control::Flip(held, on);
    let mut sections: Vec<(&'static str, Vec<(String, Control)>)> = vec![
        (
            "Where it runs",
            vec![
                (
                    "Split mode".to_owned(),
                    pick(Picker::SplitMode, settings.spread.split.as_str().to_owned()),
                ),
                (
                    "Experts".to_owned(),
                    pick(Picker::Experts, settings.spread.experts.said()),
                ),
                (
                    "Main device".to_owned(),
                    number(
                        crate::Field::MainDevice,
                        settings.spread.main_device.to_string(),
                    ),
                ),
                (
                    "Devices".to_owned(),
                    words(
                        crate::Field::Devices,
                        settings.spread.devices.clone().unwrap_or_default(),
                        "every one MCF found",
                    ),
                ),
                (
                    "Dense layers on the processor".to_owned(),
                    number(
                        crate::Field::DenseLayersOnCpu,
                        settings.spread.ffn_layers_on_processor.to_string(),
                    ),
                ),
                (
                    "Tensors placed by hand".to_owned(),
                    words(
                        crate::Field::OverrideTensors,
                        settings.spread.override_tensors.clone().unwrap_or_default(),
                        "none",
                    ),
                ),
                (
                    "Cache in system memory".to_owned(),
                    flip(
                        crate::Switch::CacheOnProcessor,
                        settings.spread.cache_on_processor,
                    ),
                ),
                (
                    "Memory lock".to_owned(),
                    flip(crate::Switch::KeepResident, settings.keep_resident),
                ),
                (
                    "How it loads".to_owned(),
                    pick(Picker::Loading, settings.loading.as_str().to_owned()),
                ),
                (
                    "Large tensors".to_owned(),
                    pick(Picker::LargeTensors, settings.lazily.as_str().to_owned()),
                ),
            ],
        ),
        (
            "Speed",
            vec![
                (
                    "Prompt batch".to_owned(),
                    number(crate::Field::Batch, settings.batch.to_string()),
                ),
                (
                    "Micro-batch".to_owned(),
                    number(crate::Field::Ubatch, settings.ubatch.to_string()),
                ),
                (
                    "Threads".to_owned(),
                    number(crate::Field::Threads, settings.threads.to_string()),
                ),
                (
                    "Threads for reading a prompt".to_owned(),
                    number(
                        crate::Field::ThreadsBatch,
                        settings.threads_batch.to_string(),
                    ),
                ),
                (
                    "Flash attention".to_owned(),
                    flip(crate::Switch::FlashAttention, settings.flash_attention),
                ),
            ],
        ),
        (
            "Thinking",
            vec![
                (
                    "Thinking budget".to_owned(),
                    number(
                        crate::Field::ThinkingBudget,
                        settings
                            .started
                            .thinking
                            .map_or_else(String::new, |held| held.to_string()),
                    ),
                ),
                (
                    "Thinking level".to_owned(),
                    if desk.levels_of_the_model().is_empty() {
                        Control::Idle(
                            "this model's template neither names a level nor marks a \
                             thinking section"
                                .to_owned(),
                        )
                    } else {
                        pick(
                            Picker::ThinkingLevel,
                            settings
                                .started
                                .effort
                                .clone()
                                .unwrap_or_else(|| "the model's own".to_owned()),
                        )
                    },
                ),
                (
                    "Draft depth".to_owned(),
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
            "Sampling",
            vec![
                (
                    "Temperature".to_owned(),
                    sampled(
                        crate::Field::Temperature,
                        settings
                            .started
                            .temperature
                            .map_or_else(String::new, |held| held.to_string()),
                        &|held| held.started.temperature.map(|it| it.to_string()),
                    ),
                ),
                (
                    "Top-p".to_owned(),
                    sampled(
                        crate::Field::TopP,
                        settings
                            .started
                            .top_p
                            .map_or_else(String::new, |held| held.to_string()),
                        &|held| held.started.top_p.map(|it| it.to_string()),
                    ),
                ),
                (
                    "Top-k".to_owned(),
                    sampled(
                        crate::Field::TopK,
                        settings
                            .started
                            .top_k
                            .map_or_else(String::new, |held| held.to_string()),
                        &|held| held.started.top_k.map(|it| it.to_string()),
                    ),
                ),
            ],
        ),
        (
            "Repetition",
            mcf_serve::sampling::Knob::ALL
                .into_iter()
                .map(|knob| {
                    (
                        knob.label().to_owned(),
                        sampled(
                            crate::Field::Sampling(knob),
                            settings
                                .started
                                .sampling
                                .get(knob)
                                .map_or_else(String::new, |held| knob.said(held)),
                            &move |held| held.started.sampling.get(knob).map(|it| knob.said(it)),
                        ),
                    )
                })
                .collect(),
        ),
        (
            "Reuse between messages",
            vec![
                (
                    "Prompt cache".to_owned(),
                    flip(crate::Switch::PromptCache, settings.reuse.prompt_cache),
                ),
                (
                    "Prompt cache memory".to_owned(),
                    number(
                        crate::Field::PromptCacheMib,
                        settings.reuse.prompt_cache_mib.to_string(),
                    ),
                ),
                (
                    "Prefix reuse".to_owned(),
                    number(
                        crate::Field::CacheReuse,
                        settings.reuse.cache_reuse.to_string(),
                    ),
                ),
                (
                    "Keep idle slots".to_owned(),
                    flip(crate::Switch::IdleSlots, settings.reuse.idle_slots),
                ),
                (
                    "Checkpoints".to_owned(),
                    number(
                        crate::Field::Checkpoints,
                        settings.reuse.checkpoints.to_string(),
                    ),
                ),
                (
                    "Checkpoint spacing".to_owned(),
                    number(
                        crate::Field::CheckpointSpacing,
                        settings.reuse.checkpoint_min_step.to_string(),
                    ),
                ),
                (
                    "Context shift".to_owned(),
                    flip(crate::Switch::ContextShift, settings.reuse.context_shift),
                ),
                (
                    "Tokens kept in front".to_owned(),
                    number(crate::Field::Keep, settings.reuse.keep.to_string()),
                ),
            ],
        ),
        (
            "Serving",
            vec![
                (
                    "Name callers use".to_owned(),
                    words(
                        crate::Field::Alias,
                        settings.alias.clone().unwrap_or_default(),
                        "its file, without the suffix",
                    ),
                ),
                (
                    "Conversations at once".to_owned(),
                    number(crate::Field::Slots, settings.slots.to_string()),
                ),
                (
                    "Port".to_owned(),
                    number(crate::Field::Port, settings.port.to_string()),
                ),
                (
                    "Reachable from the network".to_owned(),
                    flip(crate::Switch::Open, settings.open),
                ),
                (
                    "Answer kind".to_owned(),
                    pick(Picker::Answers, settings.answers.as_str().to_owned()),
                ),
                (
                    "Pooling".to_owned(),
                    pick(Picker::Pooling, settings.pooling.as_str().to_owned()),
                ),
            ],
        ),
    ];
    // Last, and only where there is something to show: a model whose template takes
    // nothing gets no heading saying so, because an empty section reads as a thing that
    // failed rather than as a model that is simply addressed plainly.
    let taken = what_the_template_takes(desk);
    if !taken.is_empty() {
        sections.push((
            "What this template takes",
            taken
                .into_iter()
                .map(|(name, control, _)| (name, control))
                .collect(),
        ));
    }

    for (heading, rows) in sections {
        y = a_section(paint, area, y, heading);
        for (name, shape) in rows {
            let name = name.as_str();
            label(
                paint,
                y,
                Row {
                    name,
                    because: because_of(name),
                },
                &mut hovered,
            );
            match shape {
                Control::Idle(said) => {
                    let shown = paint.elide(&said, Weight::Regular, size::SMALL, control - 8.0);
                    paint.say_at(column, y, &shown, Weight::Regular, size::SMALL, ink.faint);
                }
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
                Control::Number(field, now, empty) => {
                    let (touched, _) = typed_in(paint, mouse, y, field, now, &empty);
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
    let saved = desk
        .last_settings
        .as_ref()
        .is_some_and(|(last, _)| settings.differs_from(last).is_empty());
    let (remember, drawn) = ui::fitted(
        paint,
        mouse,
        (x, y),
        if saved {
            "Saved"
        } else {
            "Save for this model"
        },
        Kind::Quiet,
    );
    if remember && !saved {
        act = Some(Act::RememberSettings);
    }
    x += drawn.w + 12.0;
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
        Picker::TemplateWord(which) => {
            let takes = desk.template_takes();
            let held = takes.get(usize::from(which))?;
            let mcf_serve::parameters::Takes::Word { allowed, .. } = &held.takes else {
                return None;
            };
            let mut labels = vec!["the template's own".to_owned()];
            labels.extend(allowed.iter().cloned());
            let now = desk.what_is_taken(&held.name).map_or(0, |sent| {
                allowed
                    .iter()
                    .position(|word| Some(word.as_str()) == sent.as_text())
                    .map_or(0, |at| at.saturating_add(1))
            });
            ui::options(paint, mouse, at, &labels, Some(now))
                .map(|chosen| Act::TemplateWord(which, chosen))
        }
        Picker::ThinkingLevel => {
            let mut labels = vec!["the model's own".to_owned()];
            labels.extend(desk.levels_of_the_model());
            let now = desk.settings.as_ref().map(|settings| {
                settings.started.effort.as_ref().map_or(0, |held| {
                    desk.levels_of_the_model()
                        .iter()
                        .position(|named| named == held)
                        .map_or(0, |at| at.saturating_add(1))
                })
            });
            ui::options(paint, mouse, at, &labels, now).map(Act::ThinkingLevel)
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
        // The dial belongs to Optimize, which draws its own menu over the whole page
        // rather than inside the tab that opened it.
        Picker::Dial
        | Picker::Model
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
    // Removing a model is not one of the things a model's own page does. It is a disk
    // being cleared, and that is the downloads page — where what is on the disk, what it
    // comes to, and what removing it would give back are all in view at once.
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
        // Opened out, each quantization of the repository gets a row of its own, because
        // each of them is a separate file to choose, hold and remove.
        if group.members.len() > 1 && desk.is_opened_out(group.repository.as_deref()) {
            for member in &group.members {
                if let Some(pressed) = quant_row(
                    paint,
                    desk,
                    mouse,
                    *member,
                    Box::new(inner.x + 6.0, y, list, QUANT_ROW),
                ) {
                    act = Some(pressed);
                }
                y += QUANT_ROW;
            }
            y += 6.0;
        }
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
    let many = group.members.len() > 1;
    let opened = desk.is_opened_out(group.repository.as_deref());
    let mut figures = chooses_by(held);
    if many {
        figures = format!("{} variants here · {figures}", group.members.len());
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
    // A repository holding more than one quantization opens out into them. Without
    // something to press, only the first of them could ever be chosen — and so only the
    // first of them could ever be held or removed.
    if many {
        let word = if opened {
            "▾ each variant"
        } else {
            "▸ each variant"
        };
        let hit = Box::new(at.x + at.w - 96.0, at.y + 14.0, 90.0, 18.0);
        paint.say_right(
            at.x + at.w - 14.0,
            at.y + 18.0,
            word,
            Weight::Regular,
            size::SMALL,
            if mouse.over(hit) {
                ink.accent
            } else {
                ink.quiet
            },
        );
        if mouse.clicked(hit) {
            return group
                .repository
                .as_ref()
                .map(|held| Act::OpenOut(held.clone()));
        }
    }
    mouse.clicked(where_).then_some(Act::Choose(member))
}

/// How tall one quantization's row is, opened out under its repository.
const QUANT_ROW: f32 = 30.0;

/// One quantization of a repository, opened out under it.
///
/// It is the thing chosen, so it is the thing the actions beside the page act on: holding
/// it holds this file, and removing it removes this file and no other.
fn quant_row(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    member: usize,
    at: Box,
) -> Option<Act> {
    let ink = paint.ink;
    let held = desk.models.get(member)?;
    let where_ = Box::new(at.x + 12.0, at.y - 4.0, at.w - 12.0, at.h);
    let chosen = desk.chosen == Some(member) && desk.hub_chosen.is_none() && desk.pending.is_none();
    if chosen {
        paint.panel(where_, 6.0, ink.accent_soft, 255);
    } else if mouse.over(where_) {
        paint.panel(where_, 6.0, ink.line, 90);
    }
    // The quantization is what the part of the name after the repository says, so that is
    // what is shown: the repository's own name is on the row above.
    let said = quantization_said(held);
    let name = paint.elide(&said, Weight::Regular, size::SMALL, at.w - 90.0);
    paint.say_at(
        at.x + 22.0,
        at.y + 3.0,
        &name,
        if chosen {
            Weight::Bold
        } else {
            Weight::Regular
        },
        size::SMALL,
        if chosen { ink.accent } else { ink.ink },
    );
    paint.say_right(
        at.x + at.w - 14.0,
        at.y + 3.0,
        &held.bytes.map_or_else(|| UNKNOWN.to_owned(), gigabytes),
        Weight::Regular,
        size::SMALL,
        ink.faint,
    );
    mouse.clicked(where_).then_some(Act::Choose(member))
}

/// The `-00001-of-00003` a split variant's first file carries.
///
/// Trimmed off where the row says how many parts there are beside it: the same fact twice
/// on one row pushes the part that differs off the end.
fn without_the_part(said: &str) -> String {
    let Some((before, of)) = said.rsplit_once("-of-") else {
        return said.to_owned();
    };
    let Some((prefix, at)) = before.rsplit_once('-') else {
        return said.to_owned();
    };
    if at.is_empty()
        || of.is_empty()
        || !at.chars().all(|held| held.is_ascii_digit())
        || !of.chars().all(|held| held.is_ascii_digit())
    {
        return said.to_owned();
    }
    prefix.to_owned()
}

/// What distinguishes this file from the others in its repository. The repository's name is
/// on the row above, so repeating it here would push the part that differs off the end.
pub(crate) fn quantization_said(held: &Model) -> String {
    let file = held.file.trim_end_matches(".gguf");
    let repository = held
        .repository
        .as_deref()
        .and_then(|held| held.rsplit('/').next())
        .unwrap_or_default();
    let shortened = repository
        .strip_suffix("-GGUF")
        .or(Some(repository))
        .filter(|held| !held.is_empty())
        .and_then(|held| {
            file.strip_prefix(held)
                .map(|rest| rest.trim_start_matches(['-', '.', '_']))
        })
        .filter(|rest| !rest.is_empty());
    shortened.unwrap_or(file).to_owned()
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
        Picker::Dial => {
            let offered = desk.dials_offered();
            let labels: Vec<String> = offered
                .iter()
                .map(|held| {
                    // The dials that would do nothing on this model are still offered —
                    // saying why is more use than hiding them — but they say so.
                    if desk.why_the_dial_does_nothing(*held).is_some() {
                        format!("{} ·", held.label())
                    } else {
                        held.label().to_owned()
                    }
                })
                .collect();
            if labels.is_empty() {
                return None;
            }
            let chosen = offered
                .iter()
                .position(|held| *held == desk.optimizing.sweep.dial);
            ui::options(paint, mouse, at, &labels, chosen).map(Act::Dial)
        }
        Picker::TemplateWord(_)
        | Picker::Placement
        | Picker::Rope
        | Picker::ThinkingLevel
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
    let mut tiles: Vec<(&str, Option<String>)> = vec![
        // Averages over the time the engine says it spent, for the same reason the server
        // page states them: a rate taken between two readings of counters that only move
        // when a request finishes reads 0.0 while the model is working.
        (
            "Generation tok/s",
            rate(
                in_use
                    .generation_average()
                    .or(in_use.generating_per_second()),
            ),
        ),
        (
            "Prefill tok/s",
            rate(in_use.prefill_average().or(in_use.prompting_per_second())),
        ),
        ("Requests served", count(in_use.requests_served)),
        (
            "Generated tokens",
            count(in_use.generated_live.or(in_use.generated)),
        ),
        ("Prefill tokens", count(in_use.prompted)),
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
    // Only the cache figures the engine in front of us actually publishes. An engine that
    // reports how full its cache is gets that tile; one that reports the deepest sequence
    // it has seen gets that instead. A tile that can never fill is worse than no tile:
    // it reads as a measurement that failed.
    let mut cache: Vec<(&str, Option<String>)> = Vec::new();
    if let Some(ratio) = in_use.cache_used {
        cache.push((
            "KV cache",
            Some(format!("{:.0}%", (ratio * 100.0).clamp(0.0, 100.0))),
        ));
    }
    if in_use.cache_tokens.is_some() {
        cache.push(("KV tokens", count(in_use.cache_tokens)));
    }
    if in_use.deepest.is_some() {
        cache.push(("Peak sequence", count(in_use.deepest)));
    }
    if in_use.prompt_reused.is_some() {
        cache.push(("Cached tokens", count(in_use.prompt_reused)));
    }
    // Only where a draft head has actually proposed something. A model held without one
    // has not failed at drafting, and a tile reading 0% would say that it had.
    if let Some(share) = in_use.draft_taken_share() {
        cache.push((
            "Draft acceptance",
            Some(format!("{:.0}%", (share * 100.0).clamp(0.0, 100.0))),
        ));
    }
    let at_four = tiles.len().min(4);
    let rest = tiles.split_off(at_four);
    tiles.extend(cache);
    tiles.extend(rest);
    tiles_of(paint, &tiles, at)
}

/// One of the three counts a hold keeps: what it is called, and how to read it off a
/// reading.
type Series = (&'static str, fn(&crate::Tally) -> u64);

/// How many bands the fill under a curve is drawn in. The curve only ever climbs, so each
/// band is one rectangle reaching to the right edge — which is what makes a fading fill
/// cheap enough to draw every frame. Enough of them that the fade reads as a fade rather
/// than as a stack of slabs.
const BANDS: usize = 16;

/// How strong the fill is where it meets the curve at the top of the plot.
const FILL: f32 = 74.0;

/// Tokens through the hold: what came in, what the model read, what it wrote.
///
/// Totals, drawn as they stand rather than as a rate. These counters only move when a
/// request finishes, so a difference between two readings a second apart is nought almost
/// always and a whole answer's worth once in a while — a rate worked out from them says
/// the model is idle while it is working. The total itself has no such trouble: it stands
/// level while nothing finishes and climbs where something did, so the shape says when the
/// work happened and the slope says how fast it went.
///
/// Three plots rather than three lines on one: reading a prompt runs an order of magnitude
/// faster than writing an answer, and on a shared scale the writing is a line along the
/// floor.
fn token_totals(
    paint: &mut Painter,
    desk: &Desk,
    tallies: &std::collections::VecDeque<crate::Tally>,
    at: Box,
) -> f32 {
    let ink = paint.ink;
    spaced(paint, at.x, at.y, "tokens · this hold", ink.faint);
    let plot = Box::new(at.x, at.y + 16.0, at.w, (at.h - 16.0).max(30.0));
    let span = tallies
        .front()
        .zip(tallies.back())
        .map(|(first, last)| last.at.saturating_duration_since(first.at).as_secs());
    match span {
        Some(seconds) if seconds > 0 => {
            paint.say_right(
                at.right(),
                at.y - 2.0,
                &format!(
                    "a step is a request · over the last {}",
                    crate::ago_said(seconds)
                ),
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        }
        _ => {
            paint.edge(plot, 6.0, ink.line, ink.card);
            paint.say_at(
                plot.x + 12.0,
                plot.y + 8.0,
                "waiting for a second reading — a plot is drawn between two",
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            return plot.bottom();
        }
    }
    let in_use = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref());
    // Each plot names its phase and states its own total, so a plot answers how much and
    // how often without looking anywhere else.
    let of: [(Series, Option<u64>); 3] = [
        (
            ("Prompt", |tally| tally.asked),
            in_use.and_then(|in_use| {
                in_use
                    .prompt_reused
                    .map(|reused| reused.saturating_add(in_use.prompted.unwrap_or(0)))
            }),
        ),
        (
            ("Prefilled", |tally| tally.processed),
            in_use.and_then(|in_use| in_use.prompted),
        ),
        (
            ("Generated", |tally| tally.written),
            in_use.and_then(|in_use| in_use.generated_live.or(in_use.generated)),
        ),
    ];
    let across = (plot.w - 20.0) / 3.0;
    for (index, ((label, reading), total)) in of.into_iter().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "three plots: the index cannot lose one"
        )]
        let column = index as f32;
        let cell = Box::new(plot.x + (across + 10.0) * column, plot.y, across, plot.h);
        let each = in_use
            .and_then(|in_use| in_use.a_request(total))
            .map(|each| format!("{} a request", words::grouped(each.round().max(0.0) as u64)));
        one_total(paint, tallies, reading, label, total, each, cell);
    }
    plot.bottom()
}

/// One count as it stands: the total in the head, the climb in the plot, what one request
/// came to underneath.
#[allow(
    clippy::too_many_arguments,
    reason = "one plot: the count to draw, what to call it, and the two figures beside it"
)]
fn one_total(
    paint: &mut Painter,
    tallies: &std::collections::VecDeque<crate::Tally>,
    reading: fn(&crate::Tally) -> u64,
    label: &str,
    total: Option<u64>,
    each: Option<String>,
    at: Box,
) {
    let ink = paint.ink;
    paint.edge(at, 6.0, ink.line, ink.card);
    spaced(paint, at.x + 12.0, at.y + 11.0, label, ink.faint);
    let said = total.map_or_else(|| words::UNMEASURED.to_owned(), words::grouped);
    let named = spaced_width(paint, label);
    let shown = paint.elide(&said, Weight::Bold, size::SMALL, at.w - 24.0 - named);
    paint.say_right(
        at.right() - 12.0,
        at.y + 10.0,
        &shown,
        Weight::Bold,
        size::SMALL,
        if total.is_some() { ink.ink } else { ink.faint },
    );
    let foot = 18.0;
    let plot = Box::new(
        at.x + 11.0,
        at.y + 32.0,
        at.w - 22.0,
        (at.h - 32.0 - foot).max(12.0),
    );
    if let Some(each) = each {
        paint.say_at(
            plot.x + 1.0,
            plot.bottom() + 4.0,
            &each,
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
    }
    paint.rect(Box::new(plot.x, plot.bottom() - 1.0, plot.w, 1.0), ink.line);
    draw_the_climb(paint, tallies, reading, plot);
}

/// The shape of one count climbing across the window.
///
/// Drawn against the window's own first reading rather than against nought: a hold that
/// has served millions of tokens would otherwise draw this morning's work as a flat line
/// at the top of the plot, saying nothing about the last hour. The total in the head is
/// the absolute figure; the plot is what has happened since the window opened.
/// The curve, a column a pixel: how far up the plot the count stood at each column, as a
/// share of everything it climbed across the window.
fn the_shape_of_it(
    tallies: &std::collections::VecDeque<crate::Tally>,
    reading: fn(&crate::Tally) -> u64,
    base: u64,
    climb: u64,
    wide: f32,
) -> Vec<f32> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a count of columns across a plot, far inside usize"
    )]
    let columns = (wide.max(2.0) as usize).max(2);
    // A running maximum, so that a counter which stalls or an engine started again under
    // the hold cannot make the curve fall. A total that fell would not be a total.
    let mut climbed = 0.0_f32;
    let mut shape: Vec<f32> = Vec::with_capacity(columns);
    for column in 0..columns {
        let at_reading = column
            .saturating_mul(tallies.len().saturating_sub(1))
            .checked_div(columns.saturating_sub(1))
            .unwrap_or(0);
        let held = tallies
            .get(at_reading)
            .map_or(0, |tally| reading(tally).saturating_sub(base));
        #[expect(
            clippy::cast_precision_loss,
            reason = "token counts, as a share of the window's own climb"
        )]
        let of_the_climb = held as f32 / climb as f32;
        climbed = climbed.max(of_the_climb.clamp(0.0, 1.0));
        shape.push(climbed);
    }
    shape
}

fn draw_the_climb(
    paint: &mut Painter,
    tallies: &std::collections::VecDeque<crate::Tally>,
    reading: fn(&crate::Tally) -> u64,
    plot: Box,
) {
    let ink = paint.ink;
    let Some((first, last)) = tallies.front().zip(tallies.back()) else {
        return;
    };
    let base = reading(first);
    let climb = reading(last).saturating_sub(base);
    if climb == 0 {
        return;
    }
    let shape = the_shape_of_it(tallies, reading, base, climb, plot.w);
    #[allow(
        clippy::cast_precision_loss,
        reason = "a count of bands, a single digit"
    )]
    let bands = BANDS as f32;
    for band in 0..BANDS {
        #[allow(clippy::cast_precision_loss, reason = "a band index, a single digit")]
        let from_top = band as f32;
        // The fill is one rectangle a band, because the curve only climbs: everything
        // right of where it first reaches this band's height is under the curve.
        let above = 1.0 - (from_top + 0.5) / bands;
        let Some(first) = shape.iter().position(|share| *share >= above) else {
            continue;
        };
        #[allow(
            clippy::cast_precision_loss,
            reason = "a column index across a plot, far inside f32"
        )]
        let x = plot.x + first as f32;
        let top = plot.bottom() - plot.h * (1.0 - from_top / bands);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an alpha, worked out to fall inside a byte"
        )]
        let alpha = (FILL * (1.0 - from_top / bands)).clamp(0.0, 255.0) as u8;
        paint.wash(
            Box::new(x, top, (plot.right() - x).max(0.0), plot.h / bands),
            ink.accent,
            alpha,
        );
    }
    // A line a column was eight hundred of them a frame for a shape that is flat between
    // requests. One line a run of equal height draws the same staircase in a few dozen:
    // a tread where nothing finished, a riser where something did.
    let y = |share: f32| plot.bottom() - share * plot.h;
    #[allow(
        clippy::cast_precision_loss,
        reason = "a column index across a plot, far inside f32"
    )]
    let x = |column: usize| plot.x + column as f32;
    let mut run = 0_usize;
    for column in 1..shape.len() {
        let held = shape.get(column).copied().unwrap_or(0.0);
        let standing = shape.get(run).copied().unwrap_or(0.0);
        // Within half a pixel is the same height: a riser nobody can see is not a step,
        // and drawing it costs a line either way.
        if (y(held) - y(standing)).abs() < 0.5 && column + 1 < shape.len() {
            continue;
        }
        let ended = column.saturating_sub(1).max(run);
        if ended > run {
            paint.rule(
                (x(run), y(standing)),
                (x(ended), y(standing)),
                ink.accent,
                255,
            );
        }
        paint.rule(
            (x(ended), y(standing)),
            (x(column), y(held)),
            ink.accent,
            255,
        );
        run = column;
    }
    if run == 0 {
        // One height throughout: a hold that has done nothing since the window opened is
        // still a reading, and a flat line is what it looks like.
        let standing = shape.first().copied().unwrap_or(0.0);
        paint.rule(
            (x(0), y(standing)),
            (x(shape.len().saturating_sub(1)), y(standing)),
            ink.accent,
            255,
        );
    }
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
    // No tiles takes no room. A row's worth of height was being reserved for an empty
    // set, which opened a band of nothing wherever a block had nothing to say.
    if tiles.is_empty() {
        return at.y;
    }
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

/// The token counts over the life of the hold: what came in, what the model read, and
/// what it wrote.
///
/// Whatever the hold has to say about itself that is not a figure: that it is loading,
/// that it was refused, that it was let go, or that this model has no server at all.
///
/// Where the hold is answering it says nothing — the addresses and what it takes to reach
/// them are in the rail, and saying them twice on one page was how they came to disagree.
fn hold_status(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let mut y = at.y;
    let this_one = desk
        .hosted
        .as_ref()
        .zip(desk.chosen.and_then(|held| desk.models.get(held)))
        .filter(|(hosting, held)| hosting.model == held.path)
        .map(|(hosting, _)| hosting);
    if this_one.is_some() {
        return y;
    }
    let said: (String, Rgb) = if let Some(loading) = desk.loading_line() {
        (loading, ink.quiet)
    } else if let Doing::Provisioning(job) = &desk.doing
        && !job.finished
    {
        building(paint, job, at.x, y, at.w);
        return y + 96.0;
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
    y + 8.0
}

/// How wide the rail down the right of the server page is.
///
/// The page is about two things — the model and the machine it runs on — and they used to
/// be stacked, so three tables of machine readings took the top third of the page and
/// pushed the model's own figures and the message box below the fold. Side by side, each
/// gets the shape it wants: the model a column that grows, the machine a list that does
/// not.
const RAIL: f32 = 296.0;

/// Between the rail and the column beside it.
const RAIL_GUTTER: f32 = 28.0;

/// Below this the page stacks instead. A rail that has taken a third of the width is not
/// a rail, and the column beside it stops being able to hold a line of prose.
const RAIL_STACKS_UNDER: f32 = 920.0;

/// One line of the rail: what it is on the left, what it says on the right.
///
/// A second figure goes after the first rather than in a column of its own — the rail is
/// too narrow to keep columns aligned, and the quiet figure is the one that gives way.
fn rail_row(
    paint: &mut Painter,
    at: Box,
    y: f32,
    label: &str,
    loud: &str,
    quiet: Option<&str>,
) -> f32 {
    let ink = paint.ink;
    let named = paint.measure(label, Weight::Regular, size::SMALL);
    paint.say_at(at.x, y, label, Weight::Regular, size::SMALL, ink.quiet);
    let mut right = at.right();
    if let Some(quiet) = quiet {
        paint.say_right(right, y, quiet, Weight::Regular, size::SMALL, ink.faint);
        right -= paint.measure(quiet, Weight::Regular, size::SMALL) + 8.0;
    }
    let room = (right - at.x - named - 10.0).max(24.0);
    let shown = paint.elide(loud, Weight::Bold, size::SMALL, room);
    paint.say_right(right, y, &shown, Weight::Bold, size::SMALL, ink.ink);
    y + 19.0
}

/// A heading in the rail, with a rule above it wherever it is not the first.
fn rail_section(paint: &mut Painter, at: Box, y: f32, title: &str) -> f32 {
    let ink = paint.ink;
    if y > at.y + 1.0 {
        paint.rule((at.x, y - 10.0), (at.right(), y - 10.0), ink.line, 150);
    }
    spaced(paint, at.x, y, title, ink.faint);
    y + 19.0
}

/// The machine, the memory and the disks, as the rail says them.
///
/// Every figure the three stacked tables used to carry is here — load, temperature, clock,
/// cores, watts, used and total and free memory, read and write and temperature per
/// disk — in a quarter of the room, because a name beside a figure needs no column.
fn machine_rows(paint: &mut Painter, desk: &Desk, at: Box, mut y: f32) -> f32 {
    let reading = &desk.reading;
    let dash = || UNKNOWN_FIGURE.to_owned();
    y = rail_section(paint, at, y, "machine");

    let cpu = &reading.processor;
    let load = cpu
        .load
        .map_or_else(dash, |load| format!("{} %", load.whole()));
    let beside = [
        cpu.temperature.map(|held| format!("{held} °C")),
        cpu.clock
            .map(|mhz| format!("{:.2} GHz", f64::from(mhz) / 1000.0)),
        cpu.cores.map(|held| format!("{held} cores")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(" · ");
    y = rail_row(
        paint,
        at,
        y,
        "CPU",
        &load,
        (!beside.is_empty()).then_some(&beside),
    );

    for card in &reading.cards {
        let load = card
            .load
            .map_or_else(dash, |load| format!("{} %", load.whole()));
        let beside = [
            card.temperature.map(|held| format!("{held} °C")),
            card.power.map(|watts| format!("{watts} W")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" · ");
        y = rail_row(
            paint,
            at,
            y,
            &format!("GPU {}", card.name),
            &load,
            (!beside.is_empty()).then_some(&beside),
        );
    }
    memory_and_disk_rows(paint, desk, at, y)
}

/// What the memory and the disks read, under the same heading as the processors: they are
/// all the machine, and each is a name beside a figure.
fn memory_and_disk_rows(paint: &mut Painter, desk: &Desk, at: Box, mut y: f32) -> f32 {
    let reading = &desk.reading;
    let dash = || UNKNOWN_FIGURE.to_owned();
    let system = &reading.memory;
    let used = match (system.total, system.available) {
        (Some(total), Some(free)) => Some(total.saturating_sub(free)),
        _ => None,
    };
    y = rail_row(
        paint,
        at,
        y,
        "System memory",
        &used.map_or_else(dash, gigabytes),
        system
            .total
            .map(|total| format!("of {}", gigabytes(total)))
            .as_deref(),
    );
    if let Some(free) = system.available {
        y = rail_row(paint, at, y, "Free", &gigabytes(free), None);
    }
    for card in &reading.cards {
        let free = match (card.total, card.used) {
            (Some(total), Some(used)) => Some(total.saturating_sub(used)),
            _ => None,
        };
        y = rail_row(
            paint,
            at,
            y,
            "Graphics memory",
            &card.used.map_or_else(dash, gigabytes),
            card.total
                .map(|total| format!("of {}", gigabytes(total)))
                .as_deref(),
        );
        if let Some(free) = free {
            y = rail_row(paint, at, y, "Free", &gigabytes(free), None);
        }
    }
    for disk in &reading.disks {
        let rate = |bytes: Option<u64>| {
            bytes.map_or_else(dash, |held| format!("{:.1} MB/s", held as f64 / 1e6))
        };
        let beside = [
            Some(format!("write {}", rate(disk.written))),
            disk.temperature.map(|held| format!("{held} °C")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" · ");
        y = rail_row(
            paint,
            at,
            y,
            &disk.name,
            &rate(disk.read),
            (!beside.is_empty()).then_some(&beside),
        );
    }
    y + 10.0
}

/// What the hold is taking, and what it has drawn.
fn held_and_energy_rows(paint: &mut Painter, desk: &Desk, at: Box, mut y: f32) -> f32 {
    let Some(in_use) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
    else {
        return y;
    };
    let dash = || UNKNOWN_FIGURE.to_owned();
    y = rail_section(paint, at, y, "held");
    y = rail_row(
        paint,
        at,
        y,
        "VRAM",
        &in_use.card.map_or_else(dash, gigabytes),
        None,
    );
    y = rail_row(
        paint,
        at,
        y,
        "RAM",
        &in_use.resident.map_or_else(dash, gigabytes),
        None,
    );
    if let Some(window) = desk.hosted.as_ref().and_then(|hosting| hosting.context) {
        y = rail_row(
            paint,
            at,
            y,
            "Context length",
            &words::grouped(window),
            Some("tokens"),
        );
    }
    if let Some(deepest) = in_use.deepest {
        y = rail_row(
            paint,
            at,
            y,
            "Peak sequence",
            &words::grouped(deepest),
            None,
        );
    }
    if let Some((cache, whole)) = desk
        .hosted
        .as_ref()
        .zip(desk.hosted_model())
        .and_then(|(hosting, model)| reserve_of(model, hosting.context?, hosting.cache))
    {
        y = rail_row(
            paint,
            at,
            y,
            "KV cache",
            &gigabytes(cache),
            whole
                .map(|whole| format!("{} with weights", gigabytes(whole)))
                .as_deref(),
        );
    }
    y = rail_row(
        paint,
        at,
        y,
        "Uptime",
        &in_use.uptime_seconds.map_or_else(dash, crate::ago_said),
        None,
    );
    energy_rows(paint, in_use, at, y + 10.0)
}

/// What the hold has drawn, and what that costs where a price is set.
fn energy_rows(paint: &mut Painter, in_use: &crate::Use, at: Box, mut y: f32) -> f32 {
    let dash = || UNKNOWN_FIGURE.to_owned();
    y = rail_section(paint, at, y, "energy");
    y = rail_row(
        paint,
        at,
        y,
        if in_use.power_named.as_deref() == Some("package") {
            "Package now"
        } else {
            "Card now"
        },
        &in_use
            .card_power_watts
            .map_or_else(dash, |watts| format!("{watts:.1} W")),
        None,
    );
    if let Some(joules) = in_use.card_energy_joules {
        let drawn = if joules >= 1_000.0 {
            format!("{:.1} kJ", joules / 1_000.0)
        } else {
            format!("{joules:.0} J")
        };
        y = rail_row(paint, at, y, "Drawn", &drawn, None);
    }
    if let Some(seconds) = in_use.card_energy_over_seconds {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a span in seconds, shown as a clock"
        )]
        let whole = seconds.max(0.0) as u64;
        y = rail_row(paint, at, y, "Over", &clock(whole), None);
    }
    y = rail_row(
        paint,
        at,
        y,
        "Cost",
        &in_use.card_energy_cost_millionths.map_or_else(
            || "no price set".to_owned(),
            |millionths| mcf_core::price::Cost { millionths }.to_string(),
        ),
        None,
    );
    y + 10.0
}

/// The rail: everything about the machine and the endpoint, out of the model's way.
fn server_rail(paint: &mut Painter, desk: &Desk, mouse: &Mouse, at: Box) -> Option<Act> {
    let ink = paint.ink;
    paint.rect(at, ink.card);
    paint.rule((at.x, at.y), (at.x, at.bottom()), ink.line, 255);
    let inside = Box::new(at.x + 22.0, at.y + PAD, (at.w - 42.0).max(80.0), at.h);
    let mut act = None;
    let mut y = inside.y;

    if let Some(hosting) = desk.hosted.as_ref() {
        y = rail_section(paint, inside, y, "endpoint");
        let (pressed, after) = rail_endpoint(paint, mouse, hosting, inside, y);
        act = pressed.or(act);
        y = after + 10.0;
    }
    y = held_and_energy_rows(paint, desk, inside, y);
    let _after = machine_rows(paint, desk, inside, y);

    if desk.hosted.is_some() {
        let button = Box::new(inside.x, at.bottom() - 46.0, inside.w, ui::BUTTON);
        if ui::button(paint, mouse, button, "Stop server", Kind::Quiet) {
            act = Some(Act::StopHosting);
        }
    }
    act
}

/// The addresses the hold answers on, and what it takes to reach them.
fn rail_endpoint(
    paint: &mut Painter,
    mouse: &Mouse,
    hosting: &crate::Hosted,
    at: Box,
    mut y: f32,
) -> (Option<Act>, f32) {
    let ink = paint.ink;
    let mut act = None;
    for (label, address) in [
        ("This computer", Some(&hosting.address)),
        ("On the network", hosting.network_address.as_ref()),
    ] {
        let Some(address) = address else { continue };
        paint.say_at(at.x, y, label, Weight::Regular, size::SMALL, ink.quiet);
        y += 16.0;
        let shown = paint.elide(address, Weight::Bold, size::SMALL, at.w - 46.0);
        paint.say_at(at.x, y, &shown, Weight::Bold, size::SMALL, ink.accent);
        let (copied, _) = ui::fitted(
            paint,
            mouse,
            (at.right() - 44.0, y - 8.0),
            "Copy",
            Kind::Quiet,
        );
        if copied {
            act = Some(Act::Copy(address.clone()));
        }
        y += 24.0;
    }
    let (said, colour) = match (hosting.api_key, hosting.open) {
        (true, true) => (
            "API key set · required, since the hold answers the network".to_owned(),
            ink.quiet,
        ),
        (true, false) => ("API key set".to_owned(), ink.quiet),
        // Said plainly, and in the colour of a warning, because a hold that answers the
        // network without a key is reachable by anything that can route to this machine.
        (false, true) => (
            "No API key, and the hold answers the network: anything that can reach this \
             machine can use it"
                .to_owned(),
            ink.warn,
        ),
        (false, false) => ("No API key · this computer only".to_owned(), ink.faint),
    };
    for line in paint.wrap(&said, Weight::Regular, size::SMALL, at.w) {
        paint.say_at(at.x, y, &line, Weight::Regular, size::SMALL, colour);
        y += 16.0;
    }
    for line in [
        "OpenAI-compatible · use as base URL".to_owned(),
        format!("Started {}", hosting.since),
        takes_line(hosting),
    ] {
        for line in paint.wrap(&line, Weight::Regular, size::SMALL, at.w) {
            paint.say_at(
                at.x,
                y + 3.0,
                &line,
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            y += 16.0;
        }
    }
    (act, y + 8.0)
}

/// The pill beside the model's name that says how long it has been held.
///
/// It sits next to the name because every figure on this page is a figure for this hold
/// and no other: an engine's counters start at nought when it starts. A page of noughts
/// beside "held 3 s" is a hold that has just started; the same page with no such mark was
/// read as a page of broken figures.
fn held_chip(paint: &mut Painter, desk: &Desk, x: f32, y: f32) -> f32 {
    let ink = paint.ink;
    let Some(seconds) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
        .and_then(|in_use| in_use.uptime_seconds)
    else {
        return x;
    };
    let said = format!("held {}", crate::ago_said(seconds));
    let wide = paint.measure(&said, Weight::Bold, size::SMALL) + 28.0;
    let pill = Box::new(x, y, wide, 21.0);
    paint.panel(pill, 10.5, ink.accent_soft, 255);
    paint.panel(Box::new(x + 10.0, y + 7.5, 6.0, 6.0), 3.0, ink.accent, 255);
    paint.say_at(
        x + 20.0,
        y + 4.0,
        &said,
        Weight::Bold,
        size::SMALL,
        ink.accent,
    );
    pill.right()
}

/// How long a span of work is said. Seconds while a hold is young, because "0 min" for
/// the first minute of a hold reads as nothing having happened.
fn spent_said(seconds: f32) -> String {
    if seconds >= 60.0 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a span of seconds, shown as a clock"
        )]
        let whole = seconds.max(0.0) as u64;
        clock(whole)
    } else {
        format!("{seconds:.1} s")
    }
}

/// One card across the top of the column: what it is called, the figure, the unit the
/// figure is in, the line underneath, and whether the figure is the loud one.
type Card = (
    &'static str,
    Option<String>,
    Option<&'static str>,
    Option<String>,
    bool,
);

/// What the hold has served, and what it has averaged, as four cards across the top.
///
/// Averages rather than rates: both figures are a total over the time actually spent on
/// it, which is exact and reads true from the first finished request. The rates worked out
/// between two readings sat at 0.0 almost always, because these counters only move when a
/// request finishes.
fn rate_cards(paint: &mut Painter, desk: &Desk, at: Box) -> f32 {
    let ink = paint.ink;
    let Some(in_use) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
    else {
        return at.y;
    };
    let across = (at.w - 30.0) / 4.0;
    let cards: [Card; 4] = [
        (
            "Requests served",
            in_use.requests_served.map(words::grouped),
            None,
            // Only where the engine says: "not measured yet in flight, not measured yet
            // queued" filled the line and said nothing at all.
            match (in_use.processing, in_use.queued) {
                (None, None) => None,
                (in_flight, queued) => Some(format!(
                    "{} in flight, {} queued",
                    words::grouped(in_flight.unwrap_or(0)),
                    words::grouped(queued.unwrap_or(0)),
                )),
            },
            true,
        ),
        (
            "Generation",
            in_use.generation_average().map(|rate| format!("{rate:.1}")),
            Some("tok/s"),
            in_use
                .engine_generating_seconds
                .map(|spent| format!("avg · {} generating", spent_said(spent))),
            false,
        ),
        (
            "Prefill",
            in_use.prefill_average().map(|rate| format!("{rate:.0}")),
            Some("tok/s"),
            in_use
                .engine_prompt_seconds
                .map(|spent| format!("avg · {} prefilling", spent_said(spent))),
            false,
        ),
        (
            "Cache hit",
            in_use
                .cache_share()
                .map(|share| format!("{:.0}%", share * 100.0)),
            None,
            in_use
                .prompt_reused
                .map(|reused| format!("{} tokens reused", words::grouped(reused))),
            false,
        ),
    ];
    for (index, (label, figure, unit, under, loud)) in cards.into_iter().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "four cards: the index cannot lose one"
        )]
        let column = index as f32;
        let card = Box::new(at.x + (across + 10.0) * column, at.y, across, 78.0);
        ui::card(paint, card, false);
        spaced(paint, card.x + 14.0, card.y + 12.0, label, ink.faint);
        // A figure MCF was not given is said small. At the figure's own size the words
        // standing in for it filled the card and read as though they were the figure.
        let (said, colour, tall) = match figure {
            Some(said) => (said, if loud { ink.accent } else { ink.ink }, 26.0),
            None => (words::UNMEASURED.to_owned(), ink.faint, size::BODY),
        };
        let shown = paint.elide(&said, Weight::Bold, tall, card.w - 28.0);
        let top = card.y + 30.0 + (26.0 - tall) * 0.55;
        paint.say_at(card.x + 14.0, top, &shown, Weight::Bold, tall, colour);
        // The unit sits on the figure's baseline rather than its top, so that a small
        // word beside a large number does not read as a superscript.
        if let Some(unit) = unit.filter(|_| tall > size::BODY) {
            let after = card.x + 14.0 + paint.measure(&shown, Weight::Bold, tall) + 6.0;
            if after + paint.measure(unit, Weight::Regular, size::SMALL) <= card.right() - 12.0 {
                paint.say_at(
                    after,
                    top + 12.0,
                    unit,
                    Weight::Regular,
                    size::SMALL,
                    ink.faint,
                );
            }
        }
        if let Some(under) = under {
            let shown = paint.elide(&under, Weight::Regular, size::SMALL, card.w - 28.0);
            paint.say_at(
                card.x + 14.0,
                card.y + 58.0,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
        }
    }
    at.y + 88.0
}

/// What one request came to, on average.
///
/// The figure a reader actually wants from a hold that is idle at the moment they look:
/// not what is happening this second, but what happens when something does.
fn per_request_rows(paint: &mut Painter, desk: &Desk, at: Box, mut y: f32) -> f32 {
    let ink = paint.ink;
    let Some(in_use) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
    else {
        return y;
    };
    if in_use.requests_served.unwrap_or(0) == 0 {
        return y;
    }
    spaced(paint, at.x, y, "per request", ink.faint);
    y += 21.0;
    let whole = |each: Option<f32>| {
        each.map(|each| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "an average of token counts, shown whole"
            )]
            let whole = each.round().max(0.0) as u64;
            words::grouped(whole)
        })
    };
    let asked = in_use
        .prompt_reused
        .map(|reused| reused.saturating_add(in_use.prompted.unwrap_or(0)));
    let mut rows: Vec<(&str, String, Option<String>)> = Vec::new();
    if let Some(said) = whole(in_use.a_request(asked)) {
        rows.push(("Prompt", said, Some("tokens".to_owned())));
    }
    if let Some(said) = whole(in_use.a_request(in_use.prompt_reused)) {
        let share = in_use
            .cache_share()
            .map(|share| format!("{:.0}%", share * 100.0));
        rows.push(("Cached", said, share));
    }
    if let Some(said) = whole(in_use.a_request(in_use.generated)) {
        rows.push(("Generated", said, Some("tokens".to_owned())));
    }
    if let Some(rate) = in_use.prefill_average() {
        rows.push(("Prefill", format!("{rate:.0}"), Some("tok/s".to_owned())));
    }
    if let Some(rate) = in_use.generation_average() {
        rows.push(("Generation", format!("{rate:.1}"), Some("tok/s".to_owned())));
    }
    if let Some(seconds) = in_use.seconds_working()
        && let Some(requests) = in_use.requests_served.filter(|served| *served > 0)
    {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of requests, shown against a span of seconds"
        )]
        let requests = requests as f32;
        rows.push(("Time", spent_said(seconds / requests), None));
    }
    figure_rows(paint, at, y, &rows)
}

/// Where the hold's time has gone, in rows rather than in boxes.
///
/// Figures the size of a headline, seventeen of them, said nothing about which mattered.
/// These are the counters: worth reading, not worth watching, so they are quiet. The
/// token totals are not among them any more — each one stands in the head of its own plot.
fn work_rows(paint: &mut Painter, desk: &Desk, at: Box, mut y: f32) -> f32 {
    let ink = paint.ink;
    let Some(in_use) = desk
        .hosted
        .as_ref()
        .and_then(|hosting| hosting.in_use.as_ref())
    else {
        return y;
    };
    spaced(paint, at.x, y, "time spent", ink.faint);
    y += 21.0;
    let mut rows: Vec<(&str, String, Option<String>)> = Vec::new();
    if let Some(spent) = in_use.engine_generating_seconds {
        rows.push(("Generating", spent_said(spent), None));
    }
    if let Some(spent) = in_use.engine_prompt_seconds {
        rows.push(("Prefilling", spent_said(spent), None));
    }
    if let Some(decodes) = in_use.decodes {
        rows.push(("Decode calls", words::grouped(decodes), None));
    }
    if let Some(share) = in_use.draft_taken_share() {
        rows.push((
            "Draft acceptance",
            format!("{:.0}%", (share * 100.0).clamp(0.0, 100.0)),
            in_use
                .drafted
                .map(|drafted| format!("of {} proposed", words::grouped(drafted))),
        ));
    }
    if let Some(ratio) = in_use.cache_used {
        rows.push((
            "KV cache",
            format!("{:.0}%", (ratio * 100.0).clamp(0.0, 100.0)),
            None,
        ));
    }
    if rows.is_empty() {
        paint.say_at(
            at.x,
            y,
            "Nothing measured yet — the engine's counters start at nought with the hold.",
            Weight::Regular,
            size::SMALL,
            ink.faint,
        );
        return y + 22.0;
    }
    figure_rows(paint, at, y, &rows)
}

/// A block of quiet figures, two to a line: what it is called on the left, the figure on
/// the right, and whatever qualifies the figure just inside it.
fn figure_rows(
    paint: &mut Painter,
    at: Box,
    mut y: f32,
    rows: &[(&str, String, Option<String>)],
) -> f32 {
    let ink = paint.ink;
    if rows.is_empty() {
        return y;
    }
    let across = (at.w - 26.0) / 2.0;
    for (index, (label, figure, beside)) in rows.iter().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "a handful of rows: the index cannot lose one"
        )]
        let column = (index % 2) as f32;
        let row = Box::new(at.x + (across + 26.0) * column, y, across, 19.0);
        paint.say_at(row.x, row.y, label, Weight::Regular, size::SMALL, ink.quiet);
        let mut right = row.right();
        if let Some(beside) = beside {
            let shown = paint.elide(beside, Weight::Regular, size::SMALL, across * 0.62);
            paint.say_right(
                right,
                row.y,
                &shown,
                Weight::Regular,
                size::SMALL,
                ink.faint,
            );
            right -= paint.measure(&shown, Weight::Regular, size::SMALL) + 8.0;
        }
        paint.say_right(right, row.y, figure, Weight::Bold, size::SMALL, ink.ink);
        if column > 0.0 || index + 1 == rows.len() {
            y += 20.0;
        }
    }
    y + 8.0
}

/// The server page: the model down the left, the machine down the right.
///
/// `area` is the padded box every page is drawn in; `whole` is the page right of the
/// sidebar, edge to edge, which is what the rail is drawn against.
fn hosting(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box, whole: Box) -> Option<Act> {
    if desk.hosted.is_none() && desk.chosen.is_none() && desk.hosted_model().is_none() {
        return nothing_is_held(paint, desk, mouse, area, whole);
    }
    // Narrow enough and the rail would leave the column too thin to hold a line of prose,
    // so the page stacks: the model first, the machine under it.
    if whole.w < RAIL_STACKS_UNDER {
        return scrolled(
            paint,
            mouse,
            desk,
            Region::Server,
            area,
            |paint, mouse, inner| {
                let mut act = None;
                let mut y = server_column_body(paint, desk, inner);
                // Stacked, the message box rides the flow with the model it belongs to
                // rather than being pinned: there is no column for it to sit at the foot
                // of. Leaving it out of this branch left a narrow window unable to ask
                // the model anything at all.
                if let Some(held) = desk
                    .hosted_model()
                    .or_else(|| desk.chosen.and_then(|at| desk.models.get(at)))
                {
                    let (asked, after) = ask_box(
                        paint,
                        desk,
                        mouse,
                        Box::new(inner.x, y + 8.0, inner.w, 0.0),
                        held,
                    );
                    act = asked;
                    y = after;
                }
                let below = Box::new(inner.x, y + 14.0, inner.w, inner.h);
                let mut y = held_and_energy_rows(paint, desk, below, below.y);
                y = machine_rows(paint, desk, below, y);
                if let Some(hosting) = desk.hosted.as_ref() {
                    y = rail_section(paint, below, y, "endpoint");
                    let (pressed, _after) = rail_endpoint(paint, mouse, hosting, below, y);
                    act = act.or(pressed);
                }
                act
            },
        );
    }

    let rail = Box::new(whole.right() - RAIL, whole.y, RAIL, whole.h);
    let column = Box::new(
        area.x,
        area.y,
        (rail.x - RAIL_GUTTER - area.x).max(300.0),
        area.h,
    );
    let mut act = server_rail(paint, desk, mouse, rail);
    if let Some(pressed) = server_column(paint, desk, mouse, column) {
        act = Some(pressed);
    }
    act
}

fn nothing_is_held(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    whole: Box,
) -> Option<Act> {
    let ink = paint.ink;
    let rail = Box::new(whole.right() - RAIL, whole.y, RAIL, whole.h);
    let wide = whole.w >= RAIL_STACKS_UNDER;
    let act = if wide {
        server_rail(paint, desk, mouse, rail)
    } else {
        None
    };
    spaced(paint, area.x, area.y, "server", ink.faint);
    paint.say_at(
        area.x,
        area.y + 28.0,
        "Nothing is held. Models — choose one — Configure — Host.",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    act
}

/// The column: the model, what it is doing, and what was last asked of it. The message box
/// is pinned to the foot rather than riding the scroll, so it is where it was left.
fn server_column(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Act> {
    let held = desk
        .hosted_model()
        .or_else(|| desk.chosen.and_then(|at| desk.models.get(at)));
    let composer = if held.is_some() { 48.0 } else { 0.0 };
    let body = Box::new(area.x, area.y, area.w, (area.h - composer).max(90.0));
    let mut act = scrolled(
        paint,
        mouse,
        desk,
        Region::Server,
        body,
        |paint, _mouse, inner| {
            let _after = server_column_body(paint, desk, inner);
            None
        },
    );
    if let Some(held) = held {
        let (asked, _after) = ask_box(
            paint,
            desk,
            mouse,
            Box::new(area.x, area.bottom() - composer + 8.0, area.w, 0.0),
            held,
        );
        act = asked.or(act);
    }
    act
}

fn server_column_body(paint: &mut Painter, desk: &Desk, area: Box) -> f32 {
    let ink = paint.ink;
    let top = under_test_block(paint, desk, Box::new(area.x, area.y, area.w, 0.0));
    let held = desk
        .hosted_model()
        .or_else(|| desk.chosen.and_then(|at| desk.models.get(at)));
    let name = match (held, desk.hosted.as_ref()) {
        (Some(held), _) => held.name.clone(),
        (None, Some(hosting)) => hosting.name(),
        (None, None) => return top,
    };
    spaced(paint, area.x, top, "server", ink.faint);
    let shown = paint.elide(&name, Weight::Bold, size::HEAD, area.w - 130.0);
    paint.say_at(
        area.x,
        top + 22.0,
        &shown,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );
    let after = paint.measure(&shown, Weight::Bold, size::HEAD);
    let _ended = held_chip(paint, desk, area.x + after + 14.0, top + 24.0);

    let mut y = rate_cards(paint, desk, Box::new(area.x, top + 56.0, area.w, 0.0));
    y = token_totals(
        paint,
        desk,
        &desk.tallies,
        Box::new(area.x, y + 6.0, area.w, 108.0),
    );
    y = per_request_rows(
        paint,
        desk,
        Box::new(area.x, y + 14.0, area.w, 0.0),
        y + 14.0,
    );
    y = work_rows(paint, desk, Box::new(area.x, y + 6.0, area.w, 0.0), y + 6.0);
    y = machine_and_run_tiles(paint, desk, Box::new(area.x, y + 4.0, area.w, 0.0));

    y = hold_status(paint, desk, Box::new(area.x, y, area.w, 0.0));
    what_was_asked(
        paint,
        desk,
        Box::new(area.x, y, area.w, (area.bottom() - y).max(90.0)),
    )
}

/// The last thing asked and what came back.
///
/// MCF keeps one exchange, not a history, so this says exactly that rather than pretending
/// to a transcript it does not have.
fn what_was_asked(paint: &mut Painter, desk: &Desk, area: Box) -> f32 {
    let ink = paint.ink;
    let mut y = area.y;
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
        return y;
    }
    if desk.asked.is_empty() && desk.said.is_empty() {
        if desk.doing.busy() {
            paint.say_at(
                area.x,
                y,
                "thinking",
                Weight::Regular,
                size::BODY,
                ink.quiet,
            );
            return y + 22.0;
        }
        // Said, rather than left blank. The room below belongs to the exchange, and an
        // empty half-page above a message box reads as a page that failed to draw.
        paint.say_at(
            area.x,
            y,
            "Ask the model something — what you asked and what it answered show here.",
            Weight::Regular,
            size::BODY,
            ink.faint,
        );
        return y + 22.0;
    }
    spaced(paint, area.x, y, "last exchange", ink.faint);
    y += 22.0;
    if !desk.asked.is_empty() {
        let room = (area.w * 0.74).max(160.0);
        let lines = paint.wrap(&desk.asked, Weight::Regular, size::BODY, room - 30.0);
        let tall = 20.0 * lines.len().min(4) as f32 + 18.0;
        let bubble = Box::new(area.right() - room, y, room, tall);
        paint.panel(bubble, 10.0, ink.accent_soft, 255);
        let mut at = bubble.y + 9.0;
        for line in lines.iter().take(4) {
            paint.say_at(
                bubble.x + 15.0,
                at,
                line,
                Weight::Regular,
                size::BODY,
                ink.ink,
            );
            at += 20.0;
        }
        y = bubble.bottom() + 10.0;
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
            y += 22.0;
        }
        return y;
    }
    for said in what_it_ran_under(desk) {
        let shown = paint.elide(&said, Weight::Regular, size::SMALL, area.w);
        paint.say_at(area.x, y, &shown, Weight::Regular, size::SMALL, ink.faint);
        y += 16.0;
    }
    let room = (area.w * 0.88).max(200.0);
    let panel = Box::new(area.x, y, room, (area.bottom() - y).max(64.0));
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
    panel.bottom()
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
