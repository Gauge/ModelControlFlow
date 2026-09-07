//! Everything that puts a pixel down, in units the screens can think in.
//!
//! **Two things distinguish this from the grid it replaces.** Sizes are
//! logical, so a dense display gets more pixels rather than smaller furniture;
//! and a rectangle has a radius, so a panel has a corner rather than a
//! character. Neither is possible on a character cell, which is why the first
//! window could not be made to look like an application by adjusting it.
//!
//! Callers work in points. This multiplies by the display's scale on the way
//! out, and nothing above it ever sees a physical pixel.

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

use crate::font::{Text, Weight};
use crate::paper::Paper;
use crate::sdl::{Held, Rect, Window};
use std::collections::BTreeMap;

/// Where the drawing goes.
///
/// **Two, so that the interface can be looked at without a screen.** The
/// window is what a person sees; the paper is what a test reads. Everything
/// above this enum is written once and draws to both, which is the only way an
/// assertion about the interface can be about the interface rather than about
/// a description of it.
#[derive(Debug)]
pub enum Surface {
    /// A window on a display.
    Live(Window),
    /// A buffer of pixels, and no display at all.
    Paper(Paper),
}

impl Surface {
    /// Paints everything one colour.
    fn clear(&mut self, colour: Rgb) {
        match self {
            Self::Live(window) => window.clear(colour),
            Self::Paper(paper) => paper.clear(colour),
        }
    }

    /// Fills a rectangle.
    fn fill_with(&mut self, rect: Rect, colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.fill_with(rect, colour, alpha),
            Self::Paper(paper) => paper.fill_with(rect, colour, alpha),
        }
    }

    /// Confines what follows to a rectangle, or lifts the confinement.
    fn clip(&mut self, rect: Option<Rect>) {
        match self {
            Self::Live(window) => window.clip(rect.map(|rect| {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "pixel edges of a window, rounded; far inside i32"
                )]
                crate::sdl::ClipRect {
                    x: rect.x.round() as i32,
                    y: rect.y.round() as i32,
                    w: rect.w.round() as i32,
                    h: rect.h.round() as i32,
                }
            })),
            Self::Paper(paper) => paper.clip(rect),
        }
    }

    /// Draws a line.
    fn line(&mut self, from: (f32, f32), to: (f32, f32), colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.line(from, to, colour, alpha),
            Self::Paper(paper) => paper.line(from, to, colour, alpha),
        }
    }

    /// Keeps an image.
    fn upload(&mut self, width: u32, height: u32, rgba: &[u8]) -> Option<Held> {
        match self {
            Self::Live(window) => window.upload(width, height, rgba),
            Self::Paper(paper) => paper.upload(width, height, rgba),
        }
    }

    /// Draws part of an image.
    fn blit(&mut self, held: Held, from: Rect, to: Rect, colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.blit(held, from, to, colour, alpha),
            Self::Paper(paper) => paper.blit(held, from, to, colour, alpha),
        }
    }

    /// How many pixels there are.
    fn size(&self) -> (i32, i32) {
        match self {
            Self::Live(window) => window.size(),
            Self::Paper(paper) => (
                i32::try_from(paper.width).unwrap_or(i32::MAX),
                i32::try_from(paper.height).unwrap_or(i32::MAX),
            ),
        }
    }

    /// The display's density.
    fn scale(&self) -> f32 {
        match self {
            Self::Live(window) => window.scale(),
            Self::Paper(paper) => paper.scale,
        }
    }

    /// Shows the frame. The paper has nothing to show it to.
    fn present(&self) {
        if let Self::Live(window) = self {
            window.present();
        }
    }
}

/// A colour, as the screen wants it.
pub type Rgb = (u8, u8, u8);

/// The palette. Named for the job each colour does, not for what it looks
/// like, so that a theme is a table of values rather than a set of decisions
/// spread through the drawing code.
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    /// Behind everything.
    pub ground: Rgb,
    /// A panel or a card, sitting on the ground.
    pub card: Rgb,
    /// A well: the navigation column, an inset field.
    pub sunk: Rgb,
    /// Body text and headings.
    pub ink: Rgb,
    /// Secondary text — a subtitle, a caption, a unit.
    pub quiet: Rgb,
    /// Labels above figures, and text that is deliberately recessive.
    pub faint: Rgb,
    /// Borders and rules.
    pub line: Rgb,
    /// The one colour that means *this is the thing to press*.
    pub accent: Rgb,
    /// Text on the accent.
    pub accent_ink: Rgb,
    /// A wash of the accent, for a tag or a selected row.
    pub accent_soft: Rgb,
    /// Something is not measured, or wants attention but is not wrong.
    pub warn: Rgb,
    /// A wash of the warning colour.
    pub warn_soft: Rgb,
    /// Something is as it should be.
    pub good: Rgb,
    /// Something will not work. Distinct from [`Ink::warn`], because *this
    /// will not run* and *nobody has measured this* are the two states a
    /// person most needs to tell apart.
    pub bad: Rgb,
    /// A wash of the refusal colour.
    pub bad_soft: Rgb,
}

/// The light palette. A warm neutral ground rather than a grey one, because a
/// grey with no hue in it reads as unfinished beside anything that has one.
pub const DAY: Ink = Ink {
    ground: (0xFA, 0xF9, 0xF7),
    card: (0xFF, 0xFF, 0xFF),
    sunk: (0xF1, 0xEF, 0xEC),
    ink: (0x1A, 0x19, 0x17),
    quiet: (0x6B, 0x66, 0x60),
    faint: (0x9A, 0x95, 0x8D),
    line: (0xE7, 0xE3, 0xDD),
    accent: (0x0E, 0x6E, 0x63),
    accent_ink: (0xFF, 0xFF, 0xFF),
    accent_soft: (0xE2, 0xF1, 0xEE),
    warn: (0xA8, 0x62, 0x1F),
    warn_soft: (0xFB, 0xF0, 0xE4),
    good: (0x2D, 0x6A, 0x4A),
    bad: (0xA0, 0x3D, 0x2C),
    bad_soft: (0xFA, 0xE9, 0xE4),
};

/// The dark palette. Not an inversion — the accent is lifted so it still reads
/// as the thing to press against a dark ground, and the neutrals keep their
/// warmth rather than going blue.
pub const NIGHT: Ink = Ink {
    ground: (0x13, 0x13, 0x16),
    card: (0x1C, 0x1C, 0x20),
    sunk: (0x23, 0x23, 0x29),
    ink: (0xEC, 0xEC, 0xEE),
    quiet: (0x9B, 0x97, 0x91),
    faint: (0x6E, 0x6A, 0x66),
    // Lifted from 0x2B2B31, which was eight steps from the well it divides —
    // a hairline nobody could see. A rule is meant to be quiet and not absent.
    line: (0x35, 0x35, 0x3C),
    accent: (0x4F, 0xBF, 0xAE),
    accent_ink: (0x0B, 0x1A, 0x18),
    accent_soft: (0x12, 0x31, 0x2E),
    warn: (0xD9, 0x9A, 0x55),
    warn_soft: (0x2E, 0x24, 0x17),
    good: (0x6C, 0xBF, 0x95),
    bad: (0xE0, 0x8B, 0x78),
    bad_soft: (0x33, 0x1E, 0x1A),
};

/// How large the corner mask is rasterised. Any radius is drawn by scaling a
/// quadrant of it, so this is a quality setting and nothing else.
const MASK: u32 = 64;

/// The window, the type, and the one mask everything rounded is drawn from.
#[derive(Debug)]
pub struct Painter {
    surface: Surface,
    text: Text,
    /// One uploaded atlas per baked size.
    atlases: BTreeMap<(Weight, u32), Held>,
    corner: Option<Held>,
    /// How much larger this display draws things.
    pub scale: f32,
    /// The palette in force.
    pub ink: Ink,
    /// The lowest edge anything has reached since [`Self::mark`], in
    /// points: how tall a region's content is, read after it is drawn, so a
    /// region that overflows can be scrolled by exactly its overflow.
    lowest: std::cell::Cell<f32>,
}

/// A size in points, before the display's scale is applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box {
    /// Left edge, in points.
    pub x: f32,
    /// Top edge, in points.
    pub y: f32,
    /// Width, in points.
    pub w: f32,
    /// Height, in points.
    pub h: f32,
}

impl Box {
    /// A box from its edges.
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Whether a point is inside. What every hit test is made of.
    #[must_use]
    pub fn holds(&self, at: (f32, f32)) -> bool {
        at.0 >= self.x && at.0 < self.x + self.w && at.1 >= self.y && at.1 < self.y + self.h
    }

    /// The same box, inset on every side.
    #[must_use]
    pub fn inset(&self, by: f32) -> Self {
        Self {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - by * 2.0).max(0.0),
            h: (self.h - by * 2.0).max(0.0),
        }
    }

    /// The right edge.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

impl Painter {
    /// Opens the window and finds a face to draw with.
    ///
    /// # Errors
    ///
    /// What SDL said, or what the font search found — including the case where
    /// it found nothing, which is reported rather than papered over.
    pub fn open(title: &str, width: i32, height: i32, ink: Ink) -> Result<Self, String> {
        let window = Window::open(title, width, height)?;
        let text = Text::found()?;
        let scale = window.scale();
        let mut painter = Self {
            surface: Surface::Live(window),
            text,
            atlases: BTreeMap::new(),
            corner: None,
            scale,
            ink,
            lowest: std::cell::Cell::new(0.0),
        };
        painter.make_the_corner_mask();
        Ok(painter)
    }

    /// A painter that draws into a buffer, with no window and no display.
    ///
    /// The same code paints both, which is the point: what a test reads here
    /// is what a person would have seen.
    ///
    /// # Errors
    ///
    /// That no font could be found on this computer.
    pub fn on_paper(width: u32, height: u32, scale: f32, ink: Ink) -> Result<Self, String> {
        let text = Text::found()?;
        let mut painter = Self {
            surface: Surface::Paper(Paper::new(width, height, scale)),
            text,
            atlases: BTreeMap::new(),
            corner: None,
            scale,
            ink,
            lowest: std::cell::Cell::new(0.0),
        };
        painter.make_the_corner_mask();
        Ok(painter)
    }

    /// The pixels drawn so far, where this painter is drawing to a buffer.
    #[must_use]
    pub fn paper(&self) -> Option<&Paper> {
        match &self.surface {
            Surface::Paper(paper) => Some(paper),
            Surface::Live(_) => None,
        }
    }

    /// The face MCF is drawing with, so that it can be named.
    #[must_use]
    pub fn face(&self) -> String {
        self.text.source().file_stem().map_or_else(
            || "unknown".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    /// The window, for the event pump. `None` where this draws to a buffer.
    #[must_use]
    pub fn window(&self) -> Option<&Window> {
        match &self.surface {
            Surface::Live(window) => Some(window),
            Surface::Paper(_) => None,
        }
    }

    /// How big the drawing area is, in points.
    #[must_use]
    pub fn size(&self) -> (f32, f32) {
        let (width, height) = self.surface.size();
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        (width as f32 / scale, height as f32 / scale)
    }

    /// Re-reads the display's scale. Called when the window moves between
    /// displays, which changes what a point is worth without changing the
    /// layout above.
    pub fn rescale(&mut self) {
        self.scale = self.surface.scale();
    }

    /// Clears to the ground colour.
    pub fn begin(&mut self) {
        let ground = self.ink.ground;
        self.surface.clear(ground);
    }

    /// Shows the frame.
    pub fn end(&self) {
        self.surface.present();
    }

    /// Points to pixels.
    /// Forgets the lowest edge reached, before a region is drawn.
    pub fn mark(&self) {
        self.lowest.set(0.0);
    }

    /// Starts counting how far down things reach from `top`, which may be
    /// above the window: a scrolled region's content begins above what
    /// shows, and a mark at nought would read a region scrolled past its
    /// content as one whose content is the scroll (B-575).
    pub fn mark_at(&self, top: f32) {
        self.lowest.set(top);
    }

    /// The lowest edge anything has reached since the mark, in points.
    #[must_use]
    pub fn lowest(&self) -> f32 {
        self.lowest.get()
    }

    /// Notes an edge something was drawn to.
    fn reach(&self, bottom: f32) {
        if bottom.is_finite() && bottom > self.lowest.get() {
            self.lowest.set(bottom);
        }
    }

    fn physical(&self, area: Box) -> Rect {
        self.reach(area.bottom());
        Rect {
            x: area.x * self.scale,
            y: area.y * self.scale,
            w: area.w * self.scale,
            h: area.h * self.scale,
        }
    }

    /// Confines every drawing until [`Self::unclip`] to `area`: what a
    /// scrolled page draws above or below its window is not drawn.
    pub fn clip(&mut self, area: Box) {
        // A clip is a bound, not a thing drawn: it reaches nowhere.
        let where_ = Rect {
            x: area.x * self.scale,
            y: area.y * self.scale,
            w: area.w * self.scale,
            h: area.h * self.scale,
        };
        self.surface.clip(Some(where_));
    }

    /// Lifts the confinement [`Self::clip`] set.
    pub fn unclip(&mut self) {
        self.surface.clip(None);
    }

    /// A plain rectangle.
    pub fn rect(&mut self, area: Box, colour: Rgb) {
        let where_ = self.physical(area);
        self.surface.fill_with(where_, colour, 255);
    }

    /// A plain rectangle, seen through.
    pub fn wash(&mut self, area: Box, colour: Rgb, alpha: u8) {
        let where_ = self.physical(area);
        self.surface.fill_with(where_, colour, alpha);
    }

    /// A straight line one pixel wide.
    pub fn rule(&mut self, from: (f32, f32), to: (f32, f32), colour: Rgb, alpha: u8) {
        self.reach(from.1.max(to.1));
        self.surface.line(
            (from.0 * self.scale, from.1 * self.scale),
            (to.0 * self.scale, to.1 * self.scale),
            colour,
            alpha,
        );
    }

    /// Rasterises one antialiased rounded square, whose four quadrants are the
    /// four corners of every rounded rectangle drawn afterwards.
    ///
    /// Coverage is computed by sampling a four-by-four grid inside each pixel
    /// and counting how much of it falls within the disc. Sixteen samples is
    /// past the point where another one can be seen at these sizes, and the
    /// whole thing happens once.
    fn make_the_corner_mask(&mut self) {
        let side = MASK as usize;
        let radius = MASK as f32 / 2.0;
        let mut rgba = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
        for y in 0..side {
            for x in 0..side {
                // The centre of the disc is the middle of the square; a pixel
                // is inside when it is within `radius` of it.
                let mut inside = 0_u32;
                for sy in 0..4_u32 {
                    for sx in 0..4_u32 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                        let dx = px - radius;
                        let dy = py - radius;
                        if dx.mul_add(dx, dy * dy) <= radius * radius {
                            inside += 1;
                        }
                    }
                }
                // Exact: sixteen samples, so the numerator is a multiple of
                // 16 times 255 divided by 16 — no remainder is possible.
                #[expect(clippy::integer_division, reason = "sixteenths, exactly")]
                let coverage = u8::try_from(inside.saturating_mul(255) / 16).unwrap_or(255);
                let at = y.saturating_mul(side).saturating_add(x).saturating_mul(4);
                if let Some(pixel) = rgba.get_mut(at..at + 4) {
                    pixel.fill(255);
                    if let Some(alpha) = pixel.get_mut(3) {
                        *alpha = coverage;
                    }
                }
            }
        }
        self.corner = self.surface.upload(MASK, MASK, &rgba);
    }

    /// A rectangle with rounded corners.
    ///
    /// Five straight fills and four scaled quadrants of the mask. The corners
    /// are the only part that is not a rectangle, and they are the only part
    /// that needs to be smooth.
    pub fn panel(&mut self, area: Box, radius: f32, colour: Rgb, alpha: u8) {
        let radius = radius.min(area.w / 2.0).min(area.h / 2.0).max(0.0);
        if radius <= 0.5 {
            self.wash(area, colour, alpha);
            return;
        }
        // The cross: a full-width band through the middle, and two columns
        // between the corners.
        self.wash(
            Box::new(area.x, area.y + radius, area.w, area.h - radius * 2.0),
            colour,
            alpha,
        );
        self.wash(
            Box::new(area.x + radius, area.y, area.w - radius * 2.0, radius),
            colour,
            alpha,
        );
        self.wash(
            Box::new(
                area.x + radius,
                area.bottom() - radius,
                area.w - radius * 2.0,
                radius,
            ),
            colour,
            alpha,
        );
        let Some(mask) = self.corner else {
            return;
        };
        let half = MASK as f32 / 2.0;
        let corners = [
            (0.0, 0.0, area.x, area.y),
            (half, 0.0, area.right() - radius, area.y),
            (0.0, half, area.x, area.bottom() - radius),
            (half, half, area.right() - radius, area.bottom() - radius),
        ];
        for (sx, sy, dx, dy) in corners {
            let to = self.physical(Box::new(dx, dy, radius, radius));
            self.surface.blit(
                mask,
                Rect {
                    x: sx,
                    y: sy,
                    w: half,
                    h: half,
                },
                to,
                colour,
                alpha,
            );
        }
    }

    /// A one-pixel border around a rounded rectangle.
    ///
    /// Drawn as a filled panel with a slightly smaller panel of the background
    /// colour inside it. Stroking a rounded outline properly would want a
    /// second mask; a border is one pixel and this is indistinguishable.
    pub fn edge(&mut self, area: Box, radius: f32, colour: Rgb, inside: Rgb) {
        self.panel(area, radius, colour, 255);
        let thickness = 1.0 / self.scale.max(0.1);
        self.panel(area.inset(thickness), radius - thickness, inside, 255);
    }

    /// Makes sure a size is rasterised and uploaded, and returns its handle.
    fn ensure(&mut self, weight: Weight, pixels: f32) -> Option<(Held, u32)> {
        let key = (weight, tenths(pixels));
        if let Some(&held) = self.atlases.get(&key) {
            return Some((held, key.1));
        }
        let atlas = self.text.at(weight, pixels).ok()?;
        // Coverage is one byte a pixel; the texture wants four. White
        // everywhere, with the coverage as alpha, so that the tint at draw
        // time is the only thing that decides the colour.
        let mut rgba = vec![0_u8; atlas.coverage.len().saturating_mul(4)];
        for (at, &cover) in atlas.coverage.iter().enumerate() {
            if let Some(pixel) = rgba.get_mut(at.saturating_mul(4)..at.saturating_mul(4) + 4) {
                pixel.fill(255);
                if let Some(alpha) = pixel.get_mut(3) {
                    *alpha = cover;
                }
            }
        }
        let (width, height) = (atlas.width, atlas.height);
        let held = self.surface.upload(width, height, &rgba)?;
        let _placed = self.atlases.insert(key, held);
        Some((held, key.1))
    }

    /// How wide a string will be, in points.
    pub fn measure(&mut self, text: &str, weight: Weight, size: f32) -> f32 {
        let pixels = size * self.scale;
        self.text
            .at(weight, pixels)
            .map_or(0.0, |atlas| atlas.width_of(text) / self.scale.max(0.1))
    }

    /// What one line of this size occupies, top to top, in points.
    pub fn line_height(&mut self, weight: Weight, size: f32) -> f32 {
        let pixels = size * self.scale;
        self.text
            .at(weight, pixels)
            .map_or(size * 1.3, |atlas| atlas.line / self.scale.max(0.1))
    }

    /// Shortens text with an ellipsis if it will not fit in `room` points.
    pub fn elide(&mut self, text: &str, weight: Weight, size: f32, room: f32) -> String {
        let pixels = size * self.scale;
        self.text.at(weight, pixels).map_or_else(
            |_| text.to_owned(),
            |atlas| atlas.elide(text, room * self.scale),
        )
    }

    /// Breaks text into lines that fit in `room` points.
    pub fn wrap(&mut self, text: &str, weight: Weight, size: f32, room: f32) -> Vec<String> {
        let pixels = size * self.scale;
        self.text.at(weight, pixels).map_or_else(
            |_| vec![text.to_owned()],
            |atlas| atlas.wrap(text, room * self.scale),
        )
    }

    /// Draws one line of text, with `y` the position of its baseline.
    ///
    /// Returns where the pen ended up, so that a run of differently-styled
    /// pieces can be laid out by handing one call's answer to the next.
    pub fn say(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        weight: Weight,
        size: f32,
        colour: Rgb,
    ) -> f32 {
        let pixels = size * self.scale;
        // A line of text reaches a little under its baseline.
        self.reach(y + size * 0.35);
        let Some((held, _)) = self.ensure(weight, pixels) else {
            return x;
        };
        let Ok(atlas) = self.text.at(weight, pixels) else {
            return x;
        };
        let (mut pen, base) = (x * self.scale, y * self.scale);
        // Where a character has no glyph, a box is drawn in its place and the
        // pen moves by the box's width. Skipping it would make a name shorter
        // than the name and two different models look identical (A1, A2,
        // B-411).
        let mut boxes: Vec<(f32, f32, f32)> = Vec::new();
        let missing = atlas.missing_width();
        let above = atlas.ascent;
        for ch in text.chars() {
            let Some(glyph) = atlas.glyph(ch) else {
                boxes.push((pen, base, missing));
                pen += missing;
                continue;
            };
            let width = f32::from(glyph.x1 - glyph.x0);
            let height = f32::from(glyph.y1 - glyph.y0);
            if width > 0.0 && height > 0.0 {
                self.surface.blit(
                    held,
                    Rect {
                        x: f32::from(glyph.x0),
                        y: f32::from(glyph.y0),
                        w: width,
                        h: height,
                    },
                    Rect {
                        x: (pen + glyph.x_off).round(),
                        y: (base + glyph.y_off).round(),
                        w: width,
                        h: height,
                    },
                    colour,
                    255,
                );
            }
            pen += glyph.advance;
        }
        // After the glyphs, so a box is never half-covered by a neighbour's
        // overhang.
        for (at, base, wide) in boxes {
            self.hollow(at, base, wide, above, colour);
        }
        pen / self.scale.max(0.1)
    }

    /// An empty rectangle where a character MCF cannot draw would have been.
    ///
    /// The universal convention for it, and the point is that it is *visible*:
    /// somebody reading a name with three of them in it knows there are three
    /// characters they are not being shown, which is the whole difference
    /// between a limit and a lie (A1).
    fn hollow(&mut self, pen: f32, base: f32, wide: f32, above: f32, colour: Rgb) {
        let inset = wide * 0.12;
        let left = pen + inset;
        let width = (wide - inset * 2.0).max(1.0);
        let top = base - above * 0.78;
        let height = above * 0.78;
        let edge = self.scale.max(1.0);
        for side in [
            Rect {
                x: left,
                y: top,
                w: width,
                h: edge,
            },
            Rect {
                x: left,
                y: top + height - edge,
                w: width,
                h: edge,
            },
            Rect {
                x: left,
                y: top,
                w: edge,
                h: height,
            },
            Rect {
                x: left + width - edge,
                y: top,
                w: edge,
                h: height,
            },
        ] {
            self.surface.fill_with(side, colour, 170);
        }
    }

    /// Draws one line of text with its baseline set from the top of a box —
    /// how nearly every label in the interface is placed, because a caller
    /// thinks about where a line sits and not about where its baseline is.
    pub fn say_at(&mut self, x: f32, top: f32, text: &str, weight: Weight, size: f32, colour: Rgb) {
        let pixels = size * self.scale;
        let ascent = self
            .text
            .at(weight, pixels)
            .map_or(size, |atlas| atlas.ascent / self.scale.max(0.1));
        let _ended = self.say(x, top + ascent, text, weight, size, colour);
    }

    /// Draws one line of text ending at `right`.
    pub fn say_right(
        &mut self,
        right: f32,
        top: f32,
        text: &str,
        weight: Weight,
        size: f32,
        colour: Rgb,
    ) {
        let width = self.measure(text, weight, size);
        self.say_at(right - width, top, text, weight, size, colour);
    }

    /// Draws one line of text centred in a box, vertically as well.
    pub fn say_centred(&mut self, area: Box, text: &str, weight: Weight, size: f32, colour: Rgb) {
        let width = self.measure(text, weight, size);
        let pixels = size * self.scale;
        let (ascent, descent) = self.text.at(weight, pixels).map_or((size, 0.0), |atlas| {
            (
                atlas.ascent / self.scale.max(0.1),
                atlas.descent / self.scale.max(0.1),
            )
        });
        let baseline = area.y + (area.h - (ascent - descent)) / 2.0 + ascent;
        let _ended = self.say(
            area.x + (area.w - width) / 2.0,
            baseline,
            text,
            weight,
            size,
            colour,
        );
    }
}

/// A size as tenths of a pixel, for a map key.
fn tenths(size: f32) -> u32 {
    let scaled = size * 10.0;
    if scaled.is_finite() && scaled > 0.0 {
        scaled.round() as u32
    } else {
        0
    }
}
