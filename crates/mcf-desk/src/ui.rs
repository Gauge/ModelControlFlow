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

impl Mouse {
    /// Clears what is only true for one frame. Called at the top of each.
    pub fn settle(&mut self) {
        self.click = None;
        self.wheel = 0.0;
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
