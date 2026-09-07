//! The pointer, and the handful of things it can press.
//!
//! **Immediate mode**: a widget is a function that draws itself and says
//! whether it was pressed, and nothing keeps a tree of objects between frames.
//! Retained widgets buy incremental redrawing, and MCF has nothing to spend
//! that on — the whole interface is a few hundred rectangles and it redraws
//! only when something happened.
//!
//! What that leaves is this file: hit testing against boxes, and the drawing
//! of the six things a person can press.

#![allow(
    clippy::cast_precision_loss,
    reason = "the step counters in the two drawn glyphs below, which never \
              exceed five"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter, Rgb};

/// Where the pointer is and what it has just done.
///
/// Rebuilt every frame from the event queue. `click` is set for exactly one
/// frame, on the frame the button came back up.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mouse {
    /// Where the pointer is now.
    pub at: (f32, f32),
    /// Whether the button is down.
    pub down: bool,
    /// Where the press began, while one is in progress.
    ///
    /// A click is a press and a release inside the same thing. Keeping the
    /// origin is what lets somebody press a button, think better of it, drag
    /// off it and release — and not have pressed it.
    pub began: Option<(f32, f32)>,
    /// Set on the frame a click completed, at the point it completed.
    pub click: Option<(f32, f32)>,
    /// How far the wheel turned this frame.
    pub wheel: f32,
}

thread_local! {
    /// Every box the mouse was asked about while a frame was being recorded,
    /// on this thread; `None` while nothing is recording.
    static ASKED: std::cell::RefCell<Option<Vec<Box>>> = const { std::cell::RefCell::new(None) };
}

/// Draws a frame and returns every box the mouse was asked whether it
/// clicked while it was drawn: the controls of that frame, where the window
/// put them. A test that wants to press a control presses one of these
/// rather than sweeping the screen for it — a sweep is a frame a probe and
/// thousands of probes, and a frame is milliseconds; this is one frame and
/// a press a control.
pub fn boxes_asked<T>(draw: impl FnOnce() -> T) -> (T, Vec<Box>) {
    ASKED.with(|held| *held.borrow_mut() = Some(Vec::new()));
    let drawn = draw();
    let asked = ASKED
        .with(|held| held.borrow_mut().take())
        .unwrap_or_default();
    (drawn, asked)
}

impl Mouse {
    /// Clears what is only true for one frame. Called at the top of each.
    pub fn settle(&mut self) {
        self.click = None;
        self.wheel = 0.0;
    }

    /// This mouse as a region under a clip sees it: the pointer and any
    /// click outside `area` are not there, so what is drawn under the clip
    /// cannot be pressed through what covers it.
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

    /// Whether the pointer is over a box.
    #[must_use]
    pub fn over(&self, area: Box) -> bool {
        area.holds(self.at)
    }

    /// Whether a box was clicked this frame: released inside it, having been
    /// pressed inside it.
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

    /// Whether a box is being held down right now, for the pressed look.
    #[must_use]
    pub fn holding(&self, area: Box) -> bool {
        self.down && area.holds(self.at) && self.began.is_some_and(|down| area.holds(down))
    }
}

/// What a button is for, which decides how loudly it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The one obvious thing to do here. Filled with the accent.
    Primary,
    /// A reasonable other thing. Outlined.
    Ordinary,
    /// Available, and not being encouraged. Text only.
    Quiet,
}

/// The corner radius everything rounded shares, in points.
pub const RADIUS: f32 = 8.0;
/// How tall a button is, in points.
pub const BUTTON: f32 = 32.0;

/// Draws a button and says whether it was pressed.
pub fn button(paint: &mut Painter, mouse: &Mouse, area: Box, label: &str, kind: Kind) -> bool {
    let hot = mouse.over(area);
    let held = mouse.holding(area);
    let ink = paint.ink;

    match kind {
        Kind::Primary => {
            // Pressing darkens by laying the ink over the accent rather than
            // by carrying a second accent colour that would have to be kept
            // in step with the first.
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

/// A button sized to its own label, at a given left edge. Returns the box it
/// took, so a row of them can be laid out by passing the right edge on.
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

/// One entry in the navigation column.
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

/// A small rounded label: *Ready*, *Not measured*.
///
/// Colour carries the meaning as well as the word, so that the state of a
/// dozen models reads at a glance rather than a dozen readings.
pub fn tag(paint: &mut Painter, at: (f32, f32), label: &str, ground: Rgb, colour: Rgb) -> f32 {
    let width = paint.measure(label, Weight::Bold, 10.5) + 16.0;
    let area = Box::new(at.0, at.1, width, 18.0);
    paint.panel(area, 9.0, ground, 255);
    paint.say_centred(area, label, Weight::Bold, 10.5, colour);
    width
}

/// The label above a figure: small, spaced, and quiet.
pub fn label(paint: &mut Painter, x: f32, y: f32, text: &str) {
    let ink = paint.ink;
    let shouted = text.to_uppercase();
    // Letter-spacing by hand, because there is no such thing in a run of
    // glyphs — the pen is simply moved on a little further between them.
    let mut pen = x;
    for ch in shouted.chars() {
        let one = ch.to_string();
        paint.say_at(pen, y, &one, Weight::Bold, 9.5, ink.faint);
        pen += paint.measure(&one, Weight::Bold, 9.5) + 0.6;
    }
}

/// A line of text somebody is typing into.
///
/// Immediate mode, like everything else: the caller owns the string and this
/// draws it. Focus is the caller's too — there is one field on a screen at a
/// time, and a focus stack would be machinery for a case that does not exist.
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
        // The *end* of what has been typed, not the beginning: somebody
        // typing a long name needs to see the characters they are putting in.
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

/// A document somebody is typing or pasting into.
///
/// **Many lines, and the end of them.** The prompt somebody analyses is a
/// persona or an instruction sheet, and a line that scrolled sideways held a
/// paragraph as one run nobody could read back (B-430). Each of the
/// document's own lines is wrapped to the width and they are laid out in
/// order; what is shown is the tail that fits, because the end is where typing
/// goes and where a paste just landed. There is no selection and no cursor to
/// move: Ctrl+C takes the whole document and Ctrl+V adds to its end, which is
/// what a field holding one document needs and nothing more.
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
    // The document's lines, each wrapped on its own so a paragraph keeps
    // its shape; an empty line is a line.
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

/// How far along something is.
///
/// `None` is drawn as a track with no fill and the word beside it — a bar at
/// zero says *nothing has happened yet*, and *MCF cannot say how far along
/// this is* is a different thing that must not be drawn as the first (A7).
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

/// The triangle on a dropdown, drawn rather than set.
///
/// **A control's furniture is not text.** Asking the font for `▾` worked on a
/// face that has it and drew a notdef box on the one this machine offers —
/// which is how a chevron becomes a narrow rectangle nobody recognises. Three
/// rows of rectangle always look like a triangle, on every face, at every
/// size.
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

/// The mark in a checked box, drawn rather than set, for the same reason.
pub fn tick(paint: &mut Painter, area: Box, colour: Rgb) {
    // Two strokes: down-right, then up-right and longer. Drawn as a run of
    // small squares so that the diagonal has no gaps at any size.
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

/// A card: the ground everything on these screens sits on.
/// Draws a closed dropdown — the value, and the chevron that promises a list.
///
/// **Returns whether it was clicked**, which is the caller's cue to open it.
/// The chevron is the whole contract: a control wearing one that does anything
/// but expand is teaching the reader that the furniture is decoration.
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

/// How tall one row of an open dropdown is, in points.
pub const OPTION: f32 = 26.0;

/// Draws an open dropdown's list and says which option was chosen.
///
/// **It is drawn over whatever is beneath it**, which is why the caller defers
/// it to the end of the screen: in immediate mode the last thing painted is
/// the thing on top, and a list that drew in place would appear under the
/// table it is supposed to cover.
///
/// **The option that is already set is marked**, because a list that did not
/// say which one you are on makes the reader open it to find out and close it
/// no wiser.
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
    // A shadow, so the list reads as sitting above the page rather than as a
    // second panel that happens to be there.
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

/// A card: the ground everything on these screens sits on.
pub fn card(paint: &mut Painter, area: Box, lifted: bool) {
    let ink = paint.ink;
    paint.edge(
        area,
        10.0,
        if lifted { ink.accent } else { ink.line },
        ink.card,
    );
}

/// How far one turn of the wheel moves a region, in points.
pub const WHEEL_STEP: f32 = 48.0;

/// The width a scroll bar takes at a region's right edge.
pub const BAR: f32 = 10.0;

/// A region that scrolls: the bar at its right edge where its content is
/// taller than it, the wheel over it, and the thumb dragged. Returns the
/// offset the region should be at where that differs from `offset` —
/// clamped to what the content allows, so a region whose content shrank
/// comes back up (B-490).
pub fn scroll_region(
    paint: &mut Painter,
    mouse: &Mouse,
    area: Box,
    offset: f32,
    content: f32,
) -> Option<f32> {
    // Whole points, as the offset an act carries is: a fractional ceiling
    // rounded up on the way back would sit above itself and ask to be
    // clamped again every frame (B-575).
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

/// A splitter between two areas: a band a person drags to move the
/// boundary. Returns where the pointer is along the band's axis while it is
/// dragged, for the caller to set the split by (B-490).
///
/// `grabbed` says a press already took hold of this splitter: the band
/// moves with the line, so once the line is further from where the press
/// began than the band is wide the press is no longer inside it, and a
/// drag that was judged by the band alone dropped the line after a few
/// points and had to be picked up again (F195). A held splitter follows
/// the pointer until the button is let go.
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
