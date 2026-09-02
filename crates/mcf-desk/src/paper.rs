//! The same drawing, into a buffer, with no window and no display.
//!
//! **This exists because the window's appearance was wrong twice and nothing
//! could see it.** The interface was checked by opening it on a machine with a
//! screen, which meant it could not be checked in a test, could not be checked
//! in CI, and could not be checked by anything that had only a terminal. What
//! shipped was a character grid in a window, and every test passed.
//!
//! So [`Paper`] is a second surface for [`crate::paint::Painter`]: the same
//! calls, the same layout code, arriving as bytes instead of pixels on a
//! screen. A test can now render a screen and read what came out — that a card
//! is where it says, that text is inside its panel, that nothing is drawn in
//! the same colour as the thing behind it (A11 in the small: an assertion
//! about the interface comes from the interface, not from a description of
//! it).

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

use crate::sdl::{Held, Rect};

/// An uploaded image, held the way the window holds a texture.
#[derive(Debug)]
struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// A drawing surface that is a buffer of pixels.
#[derive(Debug)]
pub struct Paper {
    /// How wide, in pixels.
    pub width: u32,
    /// How tall, in pixels.
    pub height: u32,
    /// Four bytes a pixel, red first, row-major.
    pub pixels: Vec<u8>,
    /// What this pretends the display's density is.
    pub scale: f32,
    images: Vec<Image>,
    /// Where drawing lands while a clip is set: a pixel outside it is not
    /// touched. In pixels.
    clip: Option<Rect>,
}

impl Paper {
    /// A surface from pixels already drawn, so that a test can carry the
    /// result away from the painter that made it.
    #[must_use]
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            pixels,
            scale: 1.0,
            images: Vec::new(),
            clip: None,
        }
    }

    /// A blank surface.
    #[must_use]
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        let area = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            width,
            height,
            pixels: vec![0; area],
            scale,
            images: Vec::new(),
            clip: None,
        }
    }

    /// Paints every pixel one colour.
    pub fn clear(&mut self, colour: (u8, u8, u8)) {
        for [red, green, blue, alpha] in self.pixels.as_chunks_mut::<4>().0 {
            *red = colour.0;
            *green = colour.1;
            *blue = colour.2;
            *alpha = 255;
        }
    }

    /// The colour at a point, for a test to read.
    #[must_use]
    pub fn at(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y as usize)
            .checked_mul(self.width as usize)?
            .checked_add(x as usize)?
            .checked_mul(4)?;
        let pixel = self.pixels.get(at..at + 3)?;
        match pixel {
            [red, green, blue] => Some((*red, *green, *blue)),
            _ => None,
        }
    }

    /// Confines every drawing that follows to one rectangle, or lifts the
    /// confinement.
    pub fn clip(&mut self, rect: Option<Rect>) {
        self.clip = rect;
    }

    /// Lays one colour over what is there, by its alpha.
    fn blend(&mut self, x: i64, y: i64, colour: (u8, u8, u8), alpha: u8) {
        if x < 0 || y < 0 || alpha == 0 {
            return;
        }
        if let Some(clip) = self.clip {
            let (px, py) = (x as f32, y as f32);
            if px < clip.x.round()
                || py < clip.y.round()
                || px >= (clip.x + clip.w).round()
                || py >= (clip.y + clip.h).round()
            {
                return;
            }
        }
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let Some(at) = (y as usize)
            .checked_mul(self.width as usize)
            .and_then(|row| row.checked_add(x as usize))
            .and_then(|point| point.checked_mul(4))
        else {
            return;
        };
        let Some(pixel) = self.pixels.get_mut(at..at + 4) else {
            return;
        };
        let over = f32::from(alpha) / 255.0;
        let mix = |under: u8, above: u8| -> u8 {
            let blended = f32::from(under).mul_add(1.0 - over, f32::from(above) * over);
            blended.round().clamp(0.0, 255.0) as u8
        };
        if let [red, green, blue, _] = pixel {
            *red = mix(*red, colour.0);
            *green = mix(*green, colour.1);
            *blue = mix(*blue, colour.2);
        }
    }

    /// Fills a rectangle.
    pub fn fill_with(&mut self, rect: Rect, colour: (u8, u8, u8), alpha: u8) {
        let left = rect.x.round() as i64;
        let top = rect.y.round() as i64;
        let right = (rect.x + rect.w).round() as i64;
        let bottom = (rect.y + rect.h).round() as i64;
        for y in top..bottom {
            for x in left..right {
                self.blend(x, y, colour, alpha);
            }
        }
    }

    /// Draws a straight line, one pixel wide.
    ///
    /// Only the horizontal and vertical cases are exact, and those are the
    /// only ones the interface draws — every rule in it is a rule.
    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), colour: (u8, u8, u8), alpha: u8) {
        let steps = ((to.0 - from.0).abs().max((to.1 - from.1).abs()))
            .round()
            .max(1.0);
        for step in 0..=(steps as i64) {
            let along = step as f32 / steps;
            let x = from.0 + (to.0 - from.0) * along;
            let y = from.1 + (to.1 - from.1) * along;
            self.blend(x.round() as i64, y.round() as i64, colour, alpha);
        }
    }

    /// Keeps an image, and answers with the handle the window would have.
    pub fn upload(&mut self, width: u32, height: u32, rgba: &[u8]) -> Option<Held> {
        let wanted = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if rgba.len() != wanted {
            return None;
        }
        self.images.push(Image {
            width,
            height,
            rgba: rgba.to_vec(),
        });
        Some(Held::at(self.images.len().saturating_sub(1)))
    }

    /// Draws part of an image into a rectangle, tinted, scaling by nearest
    /// neighbour.
    ///
    /// Every glyph is drawn at its own size, so nearest is exact for text; the
    /// corner masks are the only thing scaled, and a test looking at a corner
    /// is asking whether it is round rather than how it was resampled.
    pub fn blit(&mut self, held: Held, from: Rect, to: Rect, colour: (u8, u8, u8), alpha: u8) {
        let Some(image) = self.images.get(held.index()) else {
            return;
        };
        let (width, height, rgba) = (image.width, image.height, image.rgba.clone());
        let left = to.x.round() as i64;
        let top = to.y.round() as i64;
        let across = to.w.round().max(0.0);
        let down = to.h.round().max(0.0);
        if across <= 0.0 || down <= 0.0 {
            return;
        }
        for y in 0..(down as i64) {
            for x in 0..(across as i64) {
                let u = from.x + (x as f32 + 0.5) / across * from.w;
                let v = from.y + (y as f32 + 0.5) / down * from.h;
                let (Ok(u), Ok(v)) = (u32::try_from(u as i64), u32::try_from(v as i64)) else {
                    continue;
                };
                if u >= width || v >= height {
                    continue;
                }
                let Some(at) = (v as usize)
                    .checked_mul(width as usize)
                    .and_then(|row| row.checked_add(u as usize))
                    .and_then(|point| point.checked_mul(4))
                else {
                    continue;
                };
                let Some(&coverage) = rgba.get(at + 3) else {
                    continue;
                };
                // Two eight-bit fractions multiplied back down to one. The
                // remainder is under half a step of alpha and cannot be seen.
                #[expect(clippy::integer_division, reason = "fixed-point alpha")]
                let combined = (u16::from(coverage) * u16::from(alpha)) / 255;
                self.blend(
                    left.saturating_add(x),
                    top.saturating_add(y),
                    colour,
                    u8::try_from(combined).unwrap_or(255),
                );
            }
        }
    }

    /// The surface as a portable pixmap, which every image tool reads and
    /// which needs no compression and so no library.
    #[must_use]
    pub fn as_pixmap(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for pixel in self.pixels.as_chunks::<4>().0 {
            out.extend(pixel.get(..3).unwrap_or(&[0, 0, 0]));
        }
        out
    }

    /// How much of the surface is not the colour it started as.
    ///
    /// A crude but load-bearing check: a screen that draws nothing, or draws
    /// everything in the background colour, is a screen nobody can read, and
    /// it is exactly what a broken font or a mistaken palette produces.
    #[must_use]
    pub fn inked(&self, ground: (u8, u8, u8)) -> usize {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|[red, green, blue, _]| (*red, *green, *blue) != ground)
            .count()
    }
}
