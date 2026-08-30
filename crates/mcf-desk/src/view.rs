//! The screens, drawn in pixels and operated with a pointer.
//!
//! **Nothing here is shared with the terminal console, and that is the
//! point.** The first window drew the console's character grid at a larger
//! scale, which made it a terminal somebody had put a mouse pointer over. A
//! console screen is a table of cells that reads top to bottom; these are
//! cards, panels and buttons that are scanned and clicked. The two surfaces
//! now share their *data* and their *daemon* and nothing about their drawing.
//!
//! **What is on a card is what somebody wants to know.** Not the file size,
//! the engine or the context length — the two questions a person actually
//! arrives with, which are whether a model will run and whether it is quick.
//! Everything else is a disclosure away, and nothing is thrown out (A1).

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "every number here is a screen coordinate, a pixel count or a \
              font size. They are bounded by the size of a window — a few \
              thousand — which is exactly representable in f32 and far inside \
              i64, and the conversions are between the integer a buffer is \
              indexed by and the float geometry is done in. A cast that could \
              actually lose something would be a coordinate larger than any \
              display, and `Painter` clamps and `Paper` bounds-checks before \
              any of them reaches memory"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter};
use crate::ui::{self, Kind, Mouse};
use crate::words;
use crate::{Desk, Model, Page};

/// How wide the navigation column is, in points.
const SIDEBAR: f32 = 208.0;
/// The breathing room around the content of a screen.
const PAD: f32 = 30.0;

/// The type scale. Five sizes, and an interface that wants a sixth wants
/// rethinking rather than another number.
mod size {
    /// The label over a figure.
    pub(super) const LABEL: f32 = 9.5;
    /// A caption, a unit, a footnote.
    pub(super) const SMALL: f32 = 12.0;
    /// Body text.
    pub(super) const BODY: f32 = 13.5;
    /// A card's name, a section heading.
    pub(super) const HEAD: f32 = 17.0;
    /// The one figure a screen is about.
    pub(super) const DISPLAY: f32 = 38.0;
}

/// Draws everything, and returns the page to be on next.
pub fn draw(paint: &mut Painter, desk: &Desk, mouse: &Mouse) -> Option<Page> {
    let (width, height) = paint.size();
    let ink = paint.ink;
    paint.begin();

    // The navigation column, which is a well rather than a panel: it sits
    // behind the content instead of on top of it.
    paint.rect(Box::new(0.0, 0.0, SIDEBAR, height), ink.sunk);
    paint.rule((SIDEBAR, 0.0), (SIDEBAR, height), ink.line, 255);

    let mut going = sidebar(paint, desk, mouse, height);

    let main = Box::new(
        SIDEBAR + PAD,
        PAD,
        (width - SIDEBAR - PAD * 2.0).max(10.0),
        (height - PAD * 2.0).max(10.0),
    );
    let went = match desk.page {
        Page::Models => models(paint, desk, mouse, main),
        Page::Model(at) => one_model(paint, desk, mouse, main, at),
        Page::Speed => speed(paint, desk, mouse, main),
        Page::Computer => computer(paint, desk, mouse, main),
    };
    going = went.or(going);

    paint.end();
    going
}

/// The navigation column.
fn sidebar(paint: &mut Painter, desk: &Desk, mouse: &Mouse, height: f32) -> Option<Page> {
    let ink = paint.ink;
    paint.say_at(20.0, 22.0, "MCF", Weight::Bold, size::HEAD, ink.ink);

    let mut going = None;
    let mut y = 66.0;
    for (page, label) in Page::MENU {
        let area = Box::new(10.0, y, SIDEBAR - 20.0, 34.0);
        // A model's own page is reached from the list, so the list stays lit
        // while you are on it — the column shows where you are, and a person
        // who has clicked into a model has not left their models.
        let here = desk.page.section() == *page;
        if ui::nav(paint, mouse, area, label, here) {
            going = Some(*page);
        }
        y += 38.0;
    }

    // What this is, in one sentence, at the bottom where a footer goes. It is
    // the single most reassuring fact about MCF and most people will never
    // read the documentation that says it.
    let note = paint.wrap(
        "Everything runs on this computer. Nothing is sent anywhere.",
        Weight::Regular,
        size::SMALL,
        SIDEBAR - 40.0,
    );
    let mut foot = height - 28.0 - note.len() as f32 * 16.0;
    paint.rule(
        (20.0, foot - 16.0),
        (SIDEBAR - 20.0, foot - 16.0),
        ink.line,
        255,
    );
    for line in note {
        paint.say_at(20.0, foot, &line, Weight::Regular, size::SMALL, ink.faint);
        foot += 16.0;
    }
    going
}

/// A sentence with its first letter raised.
///
/// A refusal arrives from the daemon in the daemon's own words and is never
/// rewritten — but where it is shown it begins a sentence, and starting one in
/// lower case reads as a fragment. This changes how it looks and not what it
/// says.
fn as_a_sentence(said: &str) -> String {
    let mut letters = said.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + letters.as_str()
    })
}

/// A screen's heading and its one-line subtitle.
fn heading(paint: &mut Painter, area: Box, title: &str, sub: &str) -> f32 {
    let ink = paint.ink;
    paint.say_at(area.x, area.y, title, Weight::Bold, 25.0, ink.ink);
    paint.say_at(
        area.x,
        area.y + 34.0,
        sub,
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    area.y + 66.0
}

/// A row of facts in one panel, divided by rules rather than made of separate
/// panels — four boxes read as four things, and these are four aspects of one.
fn fact_panel(paint: &mut Painter, at: Box, facts: &[(&str, String)]) -> f32 {
    let ink = paint.ink;
    let panel = Box::new(at.x, at.y, at.w, 72.0);
    ui::card(paint, panel, false);
    if facts.is_empty() {
        return panel.bottom();
    }
    let cell = panel.w / facts.len() as f32;
    for (which, (name, value)) in facts.iter().enumerate() {
        let x = panel.x + cell * which as f32;
        if which > 0 {
            paint.rule(
                (x, panel.y + 12.0),
                (x, panel.bottom() - 12.0),
                ink.line,
                255,
            );
        }
        paint.say_at(
            x + 16.0,
            panel.y + 15.0,
            &name.to_uppercase(),
            Weight::Bold,
            size::LABEL,
            ink.faint,
        );
        let shortened = paint.elide(value, Weight::Bold, size::BODY, cell - 32.0);
        paint.say_at(
            x + 16.0,
            panel.y + 38.0,
            &shortened,
            Weight::Bold,
            size::BODY,
            ink.ink,
        );
    }
    panel.bottom()
}

/// The one figure a model's page is about, or the fact that there is not one.
///
/// The unit goes beside the number rather than above it, because a figure is
/// read as part of a sentence and *116 words a second* is the sentence.
fn headline_speed(paint: &mut Painter, area: Box, y: f32, held: &Model) -> f32 {
    let ink = paint.ink;
    if let (Some(figure), Some(compared)) = (
        words::speed_figure(held.speed),
        words::against_reading(held.speed),
    ) {
        let end = paint.say(
            area.x,
            y + 34.0,
            &figure,
            Weight::Bold,
            size::DISPLAY,
            ink.ink,
        );
        paint.say_at(
            end + 10.0,
            y + 18.0,
            &format!("words a second — {compared}"),
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return y + 62.0;
    }
    paint.say_at(area.x, y, words::UNMEASURED, Weight::Bold, 22.0, ink.warn);
    paint.say_at(
        area.x,
        y + 28.0,
        "MCF has not timed this model on your computer.",
        Weight::Regular,
        size::BODY,
        ink.quiet,
    );
    y + 58.0
}

/// **Screen one.** Every model, as cards.
fn models(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Page> {
    let ink = paint.ink;
    let ready = desk.models.iter().filter(|held| held.will_run()).count();
    let sub = match desk.models.len() {
        0 => "None on this computer yet.".to_owned(),
        1 if ready == 1 => "One on this computer. It will run.".to_owned(),
        held => format!(
            "{held} on this computer. {} will run.",
            if ready == held {
                "All of them".to_owned()
            } else {
                format!("{ready} of them")
            }
        ),
    };
    let top = heading(paint, area, "Your models", &sub);

    if let Some(why) = &desk.refusal {
        return refusal(paint, Box::new(area.x, top, area.w, area.h), why);
    }
    if desk.models.is_empty() {
        paint.say_at(
            area.x,
            top + 8.0,
            "When you add one it will appear here.",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return None;
    }

    // As many columns as fit at a comfortable card width, and never so many
    // that a card becomes a strip.
    let widest = 300.0_f32;
    let gap = 14.0;
    let columns = ((area.w + gap) / (widest + gap)).floor().max(1.0);
    let card_width = (area.w - gap * (columns - 1.0)) / columns;
    let card_height = 126.0;

    let mut going = None;
    for (at, held) in desk.models.iter().enumerate() {
        let column = (at as f32) % columns;
        let row = ((at as f32) / columns).floor();
        let where_ = Box::new(
            area.x + column * (card_width + gap),
            top + row * (card_height + gap) - desk.scroll,
            card_width,
            card_height,
        );
        if where_.bottom() < top - 20.0 || where_.y > area.bottom() {
            continue;
        }
        if model_card(paint, mouse, where_, held) {
            going = Some(Page::Model(at));
        }
    }
    going
}

/// One card. Returns true if it was opened.
fn model_card(paint: &mut Painter, mouse: &Mouse, area: Box, held: &Model) -> bool {
    let ink = paint.ink;
    let hot = mouse.over(area);
    ui::card(paint, area, hot);

    let inner = area.x + 16.0;
    let room = area.w - 32.0;

    // The state, as a colour and a word at once.
    // Three states, three colours. A model that cannot run and a model
    // nobody has timed are not the same thing, and drawing them alike was
    // exactly the failure the console's palette test was written against: a
    // refusal must not look like anything else.
    let (ground, colour, word) = if !held.will_run() {
        (ink.bad_soft, ink.bad, "Will not run")
    } else if held.speed.is_none() {
        (ink.warn_soft, ink.warn, "Not measured")
    } else {
        (ink.accent_soft, ink.accent, "Ready")
    };
    let _width = ui::tag(paint, (inner, area.y + 14.0), word, ground, colour);

    let name = paint.elide(&held.name, Weight::Bold, size::HEAD, room);
    paint.say_at(
        inner,
        area.y + 40.0,
        &name,
        Weight::Bold,
        size::HEAD,
        ink.ink,
    );

    // The sentence that says what this model is for, on this machine.
    let sentence = as_a_sentence(&held.in_a_sentence());
    let lines = paint.wrap(&sentence, Weight::Regular, size::SMALL, room);
    let mut y = area.y + 66.0;
    for line in lines.iter().take(2) {
        paint.say_at(inner, y, line, Weight::Regular, size::SMALL, ink.quiet);
        y += 17.0;
    }

    let button = Box::new(inner, area.bottom() - 12.0 - 26.0, 100.0, 26.0);
    let pressed = ui::button(paint, mouse, button, "Details", Kind::Ordinary);
    // The whole card is a target, not only the button on it — a card that
    // looks like one thing should behave like one thing, and reaching for the
    // name is what people try first.
    pressed || mouse.clicked(area)
}

/// **Screen two.** One model, opening with what it means rather than what it is.
fn one_model(
    paint: &mut Painter,
    desk: &Desk,
    mouse: &Mouse,
    area: Box,
    at: usize,
) -> Option<Page> {
    let ink = paint.ink;
    let Some(held) = desk.models.get(at) else {
        return Some(Page::Models);
    };
    let where_ = as_a_sentence(&held.where_it_runs());
    let top = heading(paint, area, &held.name, &where_);

    let mut y = headline_speed(paint, area, top, held);

    let facts = [
        ("Memory", held.memory_sentence()),
        (
            "Remembers",
            words::remembers(held.context).unwrap_or_else(|| words::UNMEASURED.to_owned()),
        ),
        (
            "Wakes up in",
            words::wakes_in(held.wakes).unwrap_or_else(|| words::UNMEASURED.to_owned()),
        ),
        (
            "Long conversations",
            words::holds_up(held.fastest, held.slowest)
                .unwrap_or_else(|| words::UNMEASURED.to_owned()),
        ),
    ];
    y = fact_panel(paint, Box::new(area.x, y, area.w.min(720.0), 0.0), &facts) + 20.0;

    let mut going = None;
    let (measure, area_of) = ui::fitted(
        paint,
        mouse,
        (area.x, y),
        if held.speed.is_some() {
            "Measure it again"
        } else {
            "Measure it"
        },
        Kind::Primary,
    );
    if measure {
        going = Some(Page::Speed);
    }
    let (back, _) = ui::fitted(
        paint,
        mouse,
        (area_of.right() + 10.0, y),
        "Back",
        Kind::Ordinary,
    );
    if back {
        going = Some(Page::Models);
    }
    y += ui::BUTTON + 26.0;

    // Everything the interface decided not to lead with. Nothing is thrown
    // away; it is ordered (A1).
    paint.rule((area.x, y), (area.x + area.w.min(720.0), y), ink.line, 255);
    y += 14.0;
    paint.say_at(
        area.x,
        y,
        "Technical details",
        Weight::Bold,
        size::SMALL,
        ink.quiet,
    );
    y += 24.0;
    for (name, value) in held.technical() {
        if value.is_empty() {
            continue;
        }
        paint.say_at(area.x, y, &name, Weight::Regular, size::SMALL, ink.quiet);
        let shortened = paint.elide(&value, Weight::Regular, size::SMALL, area.w - 220.0);
        paint.say_at(
            area.x + 210.0,
            y,
            &shortened,
            Weight::Regular,
            size::SMALL,
            ink.ink,
        );
        y += 19.0;
        if y > area.bottom() - 20.0 {
            break;
        }
    }
    going
}

/// **Screen three.** Measuring, and what came of it.
fn speed(paint: &mut Painter, desk: &Desk, mouse: &Mouse, area: Box) -> Option<Page> {
    let ink = paint.ink;
    let top = heading(
        paint,
        area,
        "Speed tests",
        "Timing a model on your graphics card and your processor, so you can see the difference.",
    );
    let mut y = top;

    paint.say_at(
        area.x,
        y,
        "Nothing has been measured on this computer yet.",
        Weight::Bold,
        size::BODY,
        ink.ink,
    );
    y += 22.0;
    let lines = paint.wrap(
        "A measurement takes a model, runs it at several conversation lengths and \
         times each one. MCF will show how long it expects to take before it starts, \
         and you can stop it at any point.",
        Weight::Regular,
        size::BODY,
        area.w.min(560.0),
    );
    for line in lines {
        paint.say_at(area.x, y, &line, Weight::Regular, size::BODY, ink.quiet);
        y += 19.0;
    }
    y += 14.0;

    let mut going = None;
    if desk.models.is_empty() {
        paint.say_at(
            area.x,
            y,
            "Add a model first.",
            Weight::Regular,
            size::BODY,
            ink.quiet,
        );
        return None;
    }
    let (chose, _) = ui::fitted(paint, mouse, (area.x, y), "Choose a model", Kind::Primary);
    if chose {
        going = Some(Page::Models);
    }

    // The conditions of every sentence this application prints about speed.
    // A6: the numbers are conversions and the constants are stated.
    let note = paint.wrap(
        &words::basis(),
        Weight::Regular,
        size::SMALL,
        area.w.min(560.0),
    );
    let mut foot = area.bottom() - note.len() as f32 * 16.0;
    for line in note {
        paint.say_at(area.x, foot, &line, Weight::Regular, size::SMALL, ink.faint);
        foot += 16.0;
    }
    going
}

/// **Screen four.** What this computer can do, with the gauges below it.
fn computer(paint: &mut Painter, desk: &Desk, _mouse: &Mouse, area: Box) -> Option<Page> {
    let ink = paint.ink;
    let reading = &desk.reading;

    let headline = desk.capability_sentence();
    let top = heading(paint, area, "Your computer", &headline);
    let mut y = top;

    // The four things that decide what will run here.
    let facts = [
        ("Graphics card", desk.card_sentence()),
        ("Memory", desk.memory_sentence()),
        ("Processor", desk.processor_sentence()),
        ("Right now", desk.doing_sentence()),
    ];
    y = fact_panel(paint, Box::new(area.x, y, area.w.min(760.0), 0.0), &facts) + 26.0;

    // The gauges, which are what the old Monitor screen was made entirely of.
    // They are here because when something is wrong they are what tells you,
    // and below the fold because when nothing is wrong they tell you nothing.
    paint.say_at(
        area.x,
        y,
        "Live readings",
        Weight::Bold,
        size::SMALL,
        ink.quiet,
    );
    y += 26.0;

    let rows: [(&str, String); 4] = [
        ("Processor", reading_cpu(reading)),
        ("Graphics card", reading_gpu(reading)),
        ("Memory", reading_memory(reading)),
        ("Disk", reading_disk(reading)),
    ];
    for (name, value) in rows {
        paint.say_at(area.x, y, name, Weight::Regular, size::BODY, ink.quiet);
        paint.say_at(
            area.x + 150.0,
            y,
            &value,
            Weight::Regular,
            size::BODY,
            ink.ink,
        );
        y += 24.0;
    }
    None
}

/// What the processor is doing, in one line.
fn reading_cpu(reading: &mcf_tui::machine::Reading) -> String {
    let mut said = Vec::new();
    if let Some(load) = reading.processor.load {
        said.push(format!("{}% busy", load.whole()));
    }
    if let Some(temperature) = reading.processor.temperature {
        said.push(format!("{temperature} °C"));
    }
    if let Some(clock) = reading.processor.clock {
        said.push(format!("{:.2} GHz", f64::from(clock) / 1000.0));
    }
    if said.is_empty() {
        words::UNMEASURED.to_owned()
    } else {
        said.join(" · ")
    }
}

/// What the graphics card is doing.
fn reading_gpu(reading: &mcf_tui::machine::Reading) -> String {
    let Some(card) = reading.cards.first() else {
        return "None found".to_owned();
    };
    let mut said = Vec::new();
    if let Some(load) = card.load {
        said.push(format!("{}% busy", load.whole()));
    }
    if let Some(temperature) = card.temperature {
        said.push(format!("{temperature} °C"));
    }
    if let Some(power) = card.power {
        said.push(format!("{power} W"));
    }
    if said.is_empty() {
        words::UNMEASURED.to_owned()
    } else {
        said.join(" · ")
    }
}

/// How much memory is in use.
fn reading_memory(reading: &mcf_tui::machine::Reading) -> String {
    match (reading.memory.total, reading.memory.available) {
        (Some(total), Some(free)) => format!(
            "{} in use of {}",
            words::size_in_words(Some(total.saturating_sub(free))).unwrap_or_default(),
            words::size_in_words(Some(total)).unwrap_or_default()
        ),
        _ => words::UNMEASURED.to_owned(),
    }
}

/// What the disks are doing.
fn reading_disk(reading: &mcf_tui::machine::Reading) -> String {
    let Some(disk) = reading.disks.first() else {
        return words::UNMEASURED.to_owned();
    };
    let mut said = Vec::new();
    if let Some(read) = disk.read {
        said.push(format!("{:.1} MB/s read", read as f64 / 1e6));
    }
    if let Some(written) = disk.written {
        said.push(format!("{:.1} MB/s written", written as f64 / 1e6));
    }
    if let Some(temperature) = disk.temperature {
        said.push(format!("{temperature} °C"));
    }
    if said.is_empty() {
        words::UNMEASURED.to_owned()
    } else {
        said.join(" · ")
    }
}

/// What is drawn when MCF itself could not be reached.
///
/// It says what happened and what it means, and does not print the socket path
/// at somebody who has never heard of one.
fn refusal(paint: &mut Painter, area: Box, why: &str) -> Option<Page> {
    let ink = paint.ink;
    let panel = Box::new(area.x, area.y, area.w.min(560.0), 116.0);
    paint.panel(panel, 10.0, ink.warn_soft, 255);
    paint.say_at(
        panel.x + 20.0,
        panel.y + 18.0,
        "MCF is not answering",
        Weight::Bold,
        size::HEAD,
        ink.warn,
    );
    let lines = paint.wrap(why, Weight::Regular, size::SMALL, panel.w - 40.0);
    let mut y = panel.y + 48.0;
    for line in lines.iter().take(3) {
        paint.say_at(
            panel.x + 20.0,
            y,
            line,
            Weight::Regular,
            size::SMALL,
            ink.quiet,
        );
        y += 17.0;
    }
    None
}
