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

#[derive(Debug)]
pub enum Surface {
    Live(Window),
    Paper(Paper),
}

impl Surface {
    fn clear(&mut self, colour: Rgb) {
        match self {
            Self::Live(window) => window.clear(colour),
            Self::Paper(paper) => paper.clear(colour),
        }
    }

    fn fill_with(&mut self, rect: Rect, colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.fill_with(rect, colour, alpha),
            Self::Paper(paper) => paper.fill_with(rect, colour, alpha),
        }
    }

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

    fn line(&mut self, from: (f32, f32), to: (f32, f32), colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.line(from, to, colour, alpha),
            Self::Paper(paper) => paper.line(from, to, colour, alpha),
        }
    }

    fn upload(&mut self, width: u32, height: u32, rgba: &[u8]) -> Option<Held> {
        match self {
            Self::Live(window) => window.upload(width, height, rgba),
            Self::Paper(paper) => paper.upload(width, height, rgba),
        }
    }

    fn blit(&mut self, held: Held, from: Rect, to: Rect, colour: Rgb, alpha: u8) {
        match self {
            Self::Live(window) => window.blit(held, from, to, colour, alpha),
            Self::Paper(paper) => paper.blit(held, from, to, colour, alpha),
        }
    }

    fn size(&self) -> (i32, i32) {
        match self {
            Self::Live(window) => window.size(),
            Self::Paper(paper) => (
                i32::try_from(paper.width).unwrap_or(i32::MAX),
                i32::try_from(paper.height).unwrap_or(i32::MAX),
            ),
        }
    }

    fn scale(&self) -> f32 {
        match self {
            Self::Live(window) => window.scale(),
            Self::Paper(paper) => paper.scale,
        }
    }

    fn present(&self) {
        if let Self::Live(window) = self {
            window.present();
        }
    }
}

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, Copy)]
pub struct Ink {
    pub ground: Rgb,
    pub card: Rgb,
    pub sunk: Rgb,
    pub ink: Rgb,
    pub quiet: Rgb,
    pub faint: Rgb,
    pub line: Rgb,
    pub accent: Rgb,
    pub accent_ink: Rgb,
    pub accent_soft: Rgb,
    pub warn: Rgb,
    pub warn_soft: Rgb,
    pub good: Rgb,
    pub bad: Rgb,
    pub bad_soft: Rgb,
}

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

pub const NIGHT: Ink = Ink {
    ground: (0x13, 0x13, 0x16),
    card: (0x1C, 0x1C, 0x20),
    sunk: (0x23, 0x23, 0x29),
    ink: (0xEC, 0xEC, 0xEE),
    quiet: (0x9B, 0x97, 0x91),
    faint: (0x6E, 0x6A, 0x66),
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

const MASK: u32 = 64;

#[derive(Debug)]
pub struct Painter {
    surface: Surface,
    text: Text,
    atlases: BTreeMap<(Weight, u32), Held>,
    corner: Option<Held>,
    pub scale: f32,
    pub ink: Ink,
    lowest: std::cell::Cell<f32>,
    clipped: Option<Box>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Box {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn holds(&self, at: (f32, f32)) -> bool {
        at.0 >= self.x && at.0 < self.x + self.w && at.1 >= self.y && at.1 < self.y + self.h
    }

    #[must_use]
    pub fn inset(&self, by: f32) -> Self {
        Self {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - by * 2.0).max(0.0),
            h: (self.h - by * 2.0).max(0.0),
        }
    }

    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

impl Painter {
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
            clipped: None,
        };
        painter.make_the_corner_mask();
        Ok(painter)
    }

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
            clipped: None,
        };
        painter.make_the_corner_mask();
        Ok(painter)
    }

    #[must_use]
    pub fn paper(&self) -> Option<&Paper> {
        match &self.surface {
            Surface::Paper(paper) => Some(paper),
            Surface::Live(_) => None,
        }
    }

    #[must_use]
    pub fn face(&self) -> String {
        self.text.source().file_stem().map_or_else(
            || "unknown".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    #[must_use]
    pub fn window(&self) -> Option<&Window> {
        match &self.surface {
            Surface::Live(window) => Some(window),
            Surface::Paper(_) => None,
        }
    }

    #[must_use]
    pub fn size(&self) -> (f32, f32) {
        let (width, height) = self.surface.size();
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        (width as f32 / scale, height as f32 / scale)
    }

    pub fn rescale(&mut self) {
        self.scale = self.surface.scale();
    }

    pub fn begin(&mut self) {
        let ground = self.ink.ground;
        self.surface.clear(ground);
    }

    pub fn end(&self) {
        self.surface.present();
    }

    pub fn mark(&self) {
        self.lowest.set(0.0);
    }

    pub fn mark_at(&self, top: f32) {
        self.lowest.set(top);
    }

    #[must_use]
    pub fn lowest(&self) -> f32 {
        self.lowest.get()
    }

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

    pub fn reaches(&self, bottom: f32) {
        self.reach(bottom);
    }

    #[must_use]
    pub fn clipped(&self) -> Option<Box> {
        self.clipped
    }

    pub fn clip(&mut self, area: Box) {
        self.clipped = Some(area);
        let where_ = Rect {
            x: area.x * self.scale,
            y: area.y * self.scale,
            w: area.w * self.scale,
            h: area.h * self.scale,
        };
        self.surface.clip(Some(where_));
    }

    pub fn unclip(&mut self) {
        self.clipped = None;
        self.surface.clip(None);
    }

    pub fn rect(&mut self, area: Box, colour: Rgb) {
        let where_ = self.physical(area);
        self.surface.fill_with(where_, colour, 255);
    }

    pub fn wash(&mut self, area: Box, colour: Rgb, alpha: u8) {
        let where_ = self.physical(area);
        self.surface.fill_with(where_, colour, alpha);
    }

    pub fn rule(&mut self, from: (f32, f32), to: (f32, f32), colour: Rgb, alpha: u8) {
        self.reach(from.1.max(to.1));
        self.surface.line(
            (from.0 * self.scale, from.1 * self.scale),
            (to.0 * self.scale, to.1 * self.scale),
            colour,
            alpha,
        );
    }

    fn make_the_corner_mask(&mut self) {
        let side = MASK as usize;
        let radius = MASK as f32 / 2.0;
        let mut rgba = vec![0_u8; side.saturating_mul(side).saturating_mul(4)];
        for y in 0..side {
            for x in 0..side {
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

    pub fn panel(&mut self, area: Box, radius: f32, colour: Rgb, alpha: u8) {
        let radius = radius.min(area.w / 2.0).min(area.h / 2.0).max(0.0);
        if radius <= 0.5 {
            self.wash(area, colour, alpha);
            return;
        }
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

    pub fn edge(&mut self, area: Box, radius: f32, colour: Rgb, inside: Rgb) {
        self.panel(area, radius, colour, 255);
        let thickness = 1.0 / self.scale.max(0.1);
        self.panel(area.inset(thickness), radius - thickness, inside, 255);
    }

    fn ensure(&mut self, weight: Weight, pixels: f32) -> Option<(Held, u32)> {
        let key = (weight, tenths(pixels));
        if let Some(&held) = self.atlases.get(&key) {
            return Some((held, key.1));
        }
        let atlas = self.text.at(weight, pixels).ok()?;
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

    pub fn measure(&mut self, text: &str, weight: Weight, size: f32) -> f32 {
        let pixels = size * self.scale;
        self.text
            .at(weight, pixels)
            .map_or(0.0, |atlas| atlas.width_of(text) / self.scale.max(0.1))
    }

    pub fn line_height(&mut self, weight: Weight, size: f32) -> f32 {
        let pixels = size * self.scale;
        self.text
            .at(weight, pixels)
            .map_or(size * 1.3, |atlas| atlas.line / self.scale.max(0.1))
    }

    pub fn elide(&mut self, text: &str, weight: Weight, size: f32, room: f32) -> String {
        let pixels = size * self.scale;
        self.text.at(weight, pixels).map_or_else(
            |_| text.to_owned(),
            |atlas| atlas.elide(text, room * self.scale),
        )
    }

    pub fn wrap(&mut self, text: &str, weight: Weight, size: f32, room: f32) -> Vec<String> {
        let pixels = size * self.scale;
        self.text.at(weight, pixels).map_or_else(
            |_| vec![text.to_owned()],
            |atlas| atlas.wrap(text, room * self.scale),
        )
    }

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
        self.reach(y + size * 0.35);
        let Some((held, _)) = self.ensure(weight, pixels) else {
            return x;
        };
        let Ok(atlas) = self.text.at(weight, pixels) else {
            return x;
        };
        let (mut pen, base) = (x * self.scale, y * self.scale);
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
        for (at, base, wide) in boxes {
            self.hollow(at, base, wide, above, colour);
        }
        pen / self.scale.max(0.1)
    }

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

    pub fn say_at(&mut self, x: f32, top: f32, text: &str, weight: Weight, size: f32, colour: Rgb) {
        let pixels = size * self.scale;
        let ascent = self
            .text
            .at(weight, pixels)
            .map_or(size, |atlas| atlas.ascent / self.scale.max(0.1));
        let _ended = self.say(x, top + ascent, text, weight, size, colour);
    }

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

fn tenths(size: f32) -> u32 {
    let scaled = size * 10.0;
    if scaled.is_finite() && scaled > 0.0 {
        scaled.round() as u32
    } else {
        0
    }
}
