#![allow(
    clippy::cast_precision_loss,
    reason = "the step counters in the two drawn glyphs below, which never \
              exceed five"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter, Rgb};

#[derive(Debug, Clone, Copy, Default)]
pub struct Mouse {
    pub at: (f32, f32),
    pub down: bool,
    pub began: Option<(f32, f32)>,
    pub click: Option<(f32, f32)>,
    pub wheel: f32,
}

thread_local! {
    static ASKED: std::cell::RefCell<Option<Vec<Box>>> = const { std::cell::RefCell::new(None) };
}

pub fn boxes_asked<T>(draw: impl FnOnce() -> T) -> (T, Vec<Box>) {
    ASKED.with(|held| *held.borrow_mut() = Some(Vec::new()));
    let drawn = draw();
    let asked = ASKED
        .with(|held| held.borrow_mut().take())
        .unwrap_or_default();
    (drawn, asked)
}

impl Mouse {
    pub fn settle(&mut self) {
        self.click = None;
        self.wheel = 0.0;
    }

    #[must_use]
    pub fn within(&self, area: Box) -> Self {
        let inside = |point: (f32, f32)| area.holds(point);
        Self {
            at: if inside(self.at) {
                self.at
            } else {
                (f32::MIN, f32::MIN)
            },
            began: self.began.filter(|point| inside(*point)),
            click: self.click.filter(|point| inside(*point)),
            ..*self
        }
    }

    #[must_use]
    pub fn over(&self, area: Box) -> bool {
        area.holds(self.at)
    }

    #[must_use]
    pub fn clicked(&self, area: Box) -> bool {
        ASKED.with(|held| {
            if let Some(list) = held.borrow_mut().as_mut() {
                list.push(area);
            }
        });
        let Some(up) = self.click else {
            return false;
        };
        area.holds(up) && self.began.is_some_and(|down| area.holds(down))
    }

    #[must_use]
    pub fn holding(&self, area: Box) -> bool {
        self.down && area.holds(self.at) && self.began.is_some_and(|down| area.holds(down))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Ordinary,
    Quiet,
}

pub const RADIUS: f32 = 8.0;
pub const BUTTON: f32 = 32.0;

pub fn button(paint: &mut Painter, mouse: &Mouse, area: Box, label: &str, kind: Kind) -> bool {
    let hot = mouse.over(area);
    let held = mouse.holding(area);
    let ink = paint.ink;

    match kind {
        Kind::Primary => {
            paint.panel(area, RADIUS, ink.accent, 255);
            if held {
                paint.panel(area, RADIUS, ink.ink, 40);
            } else if hot {
                paint.panel(area, RADIUS, ink.card, 26);
            }
            paint.say_centred(area, label, Weight::Bold, 13.5, ink.accent_ink);
        }
        Kind::Ordinary => {
            let ground = if hot { ink.sunk } else { ink.card };
            paint.edge(area, RADIUS, ink.line, ground);
            if held {
                paint.panel(area, RADIUS, ink.ink, 14);
            }
            paint.say_centred(area, label, Weight::Bold, 13.5, ink.ink);
        }
        Kind::Quiet => {
            if hot {
                paint.panel(area, RADIUS, ink.sunk, 255);
            }
            let colour = if hot { ink.ink } else { ink.quiet };
            paint.say_centred(area, label, Weight::Regular, 13.5, colour);
        }
    }
    mouse.clicked(area)
}

pub fn fitted(
    paint: &mut Painter,
    mouse: &Mouse,
    at: (f32, f32),
    label: &str,
    kind: Kind,
) -> (bool, Box) {
    let width = paint.measure(label, Weight::Bold, 13.5) + 30.0;
    let area = Box::new(at.0, at.1, width, BUTTON);
    (button(paint, mouse, area, label, kind), area)
}

pub fn nav(paint: &mut Painter, mouse: &Mouse, area: Box, label: &str, on: bool) -> bool {
    let ink = paint.ink;
    if on {
        paint.panel(area, RADIUS, ink.accent, 255);
    } else if mouse.over(area) {
        paint.panel(area, RADIUS, ink.line, 130);
    }
    let colour = if on { ink.accent_ink } else { ink.quiet };
    let weight = if on { Weight::Bold } else { Weight::Regular };
    paint.say_at(area.x + 12.0, area.y + 7.0, label, weight, 13.5, colour);
    mouse.clicked(area)
}

pub fn tag(paint: &mut Painter, at: (f32, f32), label: &str, ground: Rgb, colour: Rgb) -> f32 {
    let width = paint.measure(label, Weight::Bold, 10.5) + 16.0;
    let area = Box::new(at.0, at.1, width, 18.0);
    paint.panel(area, 9.0, ground, 255);
    paint.say_centred(area, label, Weight::Bold, 10.5, colour);
    width
}

pub fn label(paint: &mut Painter, x: f32, y: f32, text: &str) {
    let ink = paint.ink;
    let shouted = text.to_uppercase();
    let mut pen = x;
    for ch in shouted.chars() {
        let one = ch.to_string();
        paint.say_at(pen, y, &one, Weight::Bold, 9.5, ink.faint);
        pen += paint.measure(&one, Weight::Bold, 9.5) + 0.6;
    }
}

pub fn field(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    held: &str,
    placeholder: &str,
    focused: bool,
) -> bool {
    let ink = paint.ink;
    let edge = if focused { ink.accent } else { ink.line };
    paint.edge(area, RADIUS, edge, ink.card);
    let inner = area.x + 12.0;
    let room = area.w - 24.0;
    if held.is_empty() {
        let shown = paint.elide(placeholder, Weight::Regular, 13.5, room);
        paint.say_at(
            inner,
            area.y + 8.0,
            &shown,
            Weight::Regular,
            13.5,
            ink.faint,
        );
    } else {
        let width = paint.measure(held, Weight::Regular, 13.5);
        let from = if width > room {
            let mut kept = held;
            while paint.measure(kept, Weight::Regular, 13.5) > room && !kept.is_empty() {
                kept = kept
                    .get(kept.char_indices().nth(1).map_or(kept.len(), |(at, _)| at)..)
                    .unwrap_or("");
            }
            kept.to_owned()
        } else {
            held.to_owned()
        };
        let ended = {
            paint.say_at(inner, area.y + 8.0, &from, Weight::Regular, 13.5, ink.ink);
            inner + paint.measure(&from, Weight::Regular, 13.5)
        };
        if focused {
            paint.wash(
                Box::new(ended + 1.0, area.y + 8.0, 1.5, 17.0),
                ink.accent,
                255,
            );
        }
    }
    mouse.clicked(area)
}

pub fn area(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    held: &str,
    placeholder: &str,
    focused: bool,
) -> bool {
    let ink = paint.ink;
    let edge = if focused { ink.accent } else { ink.line };
    paint.edge(area, RADIUS, edge, ink.card);
    let inner = area.x + 12.0;
    let top = area.y + 8.0;
    let room = area.w - 24.0;
    let step = 18.0;
    if held.is_empty() {
        let shown = paint.elide(placeholder, Weight::Regular, 13.5, room);
        paint.say_at(inner, top, &shown, Weight::Regular, 13.5, ink.faint);
        if focused {
            paint.wash(Box::new(inner, top, 1.5, 17.0), ink.accent, 255);
        }
        return mouse.clicked(area);
    }
    let mut lines: Vec<String> = Vec::new();
    for written in held.split('\n') {
        if written.trim().is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(paint.wrap(written, Weight::Regular, 13.5, room));
        }
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "how many lines fit is a count; a part of a line is not drawn"
    )]
    let fit = ((area.h - 16.0) / step).floor().max(1.0) as usize;
    let from = lines.len().saturating_sub(fit);
    let mut y = top;
    let mut ended = inner;
    for line in lines.iter().skip(from) {
        paint.say_at(inner, y, line, Weight::Regular, 13.5, ink.ink);
        ended = inner + paint.measure(line, Weight::Regular, 13.5);
        y += step;
    }
    if from > 0 {
        let hidden = format!("{from} more lines above");
        let width = paint.measure(&hidden, Weight::Regular, 10.5);
        paint.say_at(
            area.right() - width - 12.0,
            area.y + 6.0,
            &hidden,
            Weight::Regular,
            10.5,
            ink.faint,
        );
    }
    if focused {
        paint.wash(Box::new(ended + 1.0, y - step, 1.5, 17.0), ink.accent, 255);
    }
    mouse.clicked(area)
}

pub fn progress(paint: &mut Painter, area: Box, fraction: Option<f32>) {
    let ink = paint.ink;
    paint.panel(area, area.h / 2.0, ink.sunk, 255);
    if let Some(fraction) = fraction {
        let filled = (area.w * fraction.clamp(0.0, 1.0)).max(0.0);
        if filled > 1.0 {
            paint.panel(
                Box::new(area.x, area.y, filled, area.h),
                area.h / 2.0,
                ink.accent,
                255,
            );
        }
    }
}

pub fn chevron(paint: &mut Painter, at: (f32, f32), colour: Rgb) {
    let (wide, tall) = (9.0_f32, 5.0_f32);
    let rows = 5_i32;
    for step in 0..rows {
        let along = step as f32 / rows as f32;
        paint.wash(
            Box::new(
                at.0 + wide * along / 2.0,
                at.1 + tall * along,
                wide * (1.0 - along),
                tall / rows as f32 + 0.6,
            ),
            colour,
            255,
        );
    }
}

pub fn tick(paint: &mut Painter, area: Box, colour: Rgb) {
    let unit = area.w / 8.0;
    for step in 0..4 {
        let along = step as f32;
        paint.wash(
            Box::new(
                area.x + unit * (1.0 + along * 0.7),
                area.y + unit * (3.4 + along * 0.7),
                unit * 1.5,
                unit * 1.5,
            ),
            colour,
            255,
        );
    }
    for step in 0..5 {
        let along = step as f32;
        paint.wash(
            Box::new(
                area.x + unit * (3.4 + along * 0.8),
                area.y + unit * (5.4 - along * 0.8),
                unit * 1.5,
                unit * 1.5,
            ),
            colour,
            255,
        );
    }
}

pub fn picker(paint: &mut Painter, mouse: &Mouse, area: Box, value: &str, open: bool) -> bool {
    let ink = paint.ink;
    let hot = mouse.over(area);
    paint.edge(
        area,
        RADIUS,
        if open { ink.accent } else { ink.line },
        if hot || open { ink.sunk } else { ink.card },
    );
    let shown = paint.elide(value, Weight::Regular, 13.5, area.w - 44.0);
    paint.say_at(
        area.x + 12.0,
        area.y + 6.0,
        &shown,
        Weight::Regular,
        13.5,
        ink.ink,
    );
    chevron(paint, (area.right() - 22.0, area.y + 13.0), ink.faint);
    mouse.clicked(area)
}

pub const OPTION: f32 = 26.0;

pub fn options(
    paint: &mut Painter,
    mouse: &Mouse,
    below: Box,
    labels: &[String],
    at: Option<usize>,
) -> Option<usize> {
    let ink = paint.ink;
    let tall = OPTION.mul_add(
        f32::from(u16::try_from(labels.len()).unwrap_or(u16::MAX)),
        12.0,
    );
    let list = Box::new(below.x, below.bottom() + 2.0, below.w, tall);
    paint.panel(
        Box::new(list.x + 2.0, list.y + 3.0, list.w, list.h),
        RADIUS,
        ink.ink,
        30,
    );
    paint.edge(list, RADIUS, ink.accent, ink.card);
    let mut chosen = None;
    for (index, label) in labels.iter().enumerate() {
        let row = Box::new(
            list.x + 4.0,
            OPTION.mul_add(
                f32::from(u16::try_from(index).unwrap_or(u16::MAX)),
                list.y + 6.0,
            ),
            list.w - 8.0,
            OPTION,
        );
        if mouse.over(row) {
            paint.panel(row, 6.0, ink.sunk, 255);
        }
        let here = at == Some(index);
        let shown = paint.elide(label, Weight::Regular, 13.5, row.w - 34.0);
        paint.say_at(
            row.x + 22.0,
            row.y + 5.0,
            &shown,
            Weight::Regular,
            13.5,
            ink.ink,
        );
        if here {
            tick(
                paint,
                Box::new(row.x + 5.0, row.y + 7.0, 12.0, 12.0),
                ink.accent,
            );
        }
        if mouse.clicked(row) {
            chosen = Some(index);
        }
    }
    chosen
}

pub fn card(paint: &mut Painter, area: Box, lifted: bool) {
    let ink = paint.ink;
    paint.edge(
        area,
        10.0,
        if lifted { ink.accent } else { ink.line },
        ink.card,
    );
}

pub const WHEEL_STEP: f32 = 48.0;

pub const BAR: f32 = 10.0;

pub fn scroll_region(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    offset: f32,
    content: f32,
) -> Option<f32> {
    let most = (content - area.h).max(0.0).floor();
    let clamp = |wanted: f32| wanted.clamp(0.0, most);
    if most <= 0.0 {
        return (offset > 0.0).then_some(0.0);
    }
    let ink = paint.ink;
    let track = Box::new(area.right() - BAR + 2.0, area.y, BAR - 4.0, area.h);
    paint.panel(track, 3.0, ink.line, 70);
    let thumb_h = (area.h * area.h / content).max(24.0).min(area.h);
    let thumb_y = area.y + (offset / most) * (area.h - thumb_h);
    let thumb = Box::new(track.x, thumb_y, track.w, thumb_h);
    let held = mouse.down && mouse.began.is_some_and(|down| track.holds(down));
    paint.panel(
        thumb,
        3.0,
        if held || mouse.over(track) {
            ink.accent
        } else {
            ink.quiet
        },
        if held { 255 } else { 160 },
    );
    let mut wanted = offset;
    if held {
        let ratio = (mouse.at.1 - area.y - thumb_h / 2.0) / (area.h - thumb_h).max(1.0);
        wanted = ratio.clamp(0.0, 1.0) * most;
    } else if mouse.wheel.abs() > 0.0 && mouse.over(area) {
        wanted = offset - mouse.wheel * WHEEL_STEP;
    }
    let wanted = clamp(wanted);
    ((wanted - offset).abs() > 0.5 || offset > most).then_some(wanted)
}

pub fn splitter(
    paint: &mut Painter,
    mouse: &Mouse,
    band: Box,
    upright: bool,
    grabbed: bool,
) -> Option<f32> {
    let ink = paint.ink;
    let held = mouse.down && (grabbed || mouse.began.is_some_and(|down| band.holds(down)));
    if held || mouse.over(band) {
        let line = if upright {
            Box::new(band.x + band.w / 2.0 - 1.0, band.y, 2.0, band.h)
        } else {
            Box::new(band.x, band.y + band.h / 2.0 - 1.0, band.w, 2.0)
        };
        paint.rect(line, if held { ink.accent } else { ink.quiet });
    }
    held.then_some(if upright { mouse.at.0 } else { mouse.at.1 })
}
