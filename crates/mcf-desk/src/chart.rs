//! The one thing a terminal cannot draw.
//!
//! **A table says a model slows down; it does not say by how much it
//! matters.** The console's MEASURED rows are exact and answer *what is the
//! cost at this depth*. What they cannot answer is the question somebody
//! actually has — *does this get slow if I talk to it for a while* — because
//! that is a shape, and a shape needs an area rather than a column.
//!
//! **The vertical axis starts at zero, and that is the whole honesty of it.**
//! A chart that began at the smallest reading would draw this machine's
//! 1.333 to 1.412 milliseconds a token as a cliff, and it is a rise of six
//! per cent. Suppressing a zero is how a picture tells a lie the numbers under
//! it do not (A6, A11) — so the axis starts at nothing, and a fall-off that
//! looks flat is one that is.
//!
//! **The horizontal axis is the ladder, evenly spaced.** The depths double, so
//! even spacing *is* a logarithmic axis; there is no second scale to explain
//! and nothing to mislabel.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a count of readings and a fraction of a plot's height: both are \
              bounded by the size of a window and exact in f32"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter};

/// One reading: how deep, and what it cost there.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    /// Tokens of context.
    pub depth: u64,
    /// Milliseconds a token at that depth.
    pub ms: f64,
}

/// Draws the fall-off, and returns the row under it.
///
/// Nothing is drawn where there is nothing to draw: one reading is a point and
/// not a shape, and a chart of it would be a picture asserting a trend from a
/// single measurement (A11).
pub fn falloff(paint: &mut Painter, area: Box, readings: &[Reading]) -> f32 {
    let ink = paint.ink;
    if readings.len() < 2 {
        return area.y;
    }
    let deepest = readings.iter().map(|held| held.ms).fold(0.0_f64, f64::max);
    if deepest <= 0.0 {
        return area.y;
    }
    // Headroom, so the topmost point is not against the frame.
    let ceiling = deepest * 1.15;

    let plot = Box::new(area.x, area.y, area.w, area.h);
    paint.rule(
        (plot.x, plot.bottom()),
        (plot.right(), plot.bottom()),
        ink.line,
        255,
    );
    // Three faint lines, so a reader can see how far up a point is without a
    // scale down the side taking room the shape wants.
    for at in 1..=3_u8 {
        let y = plot.bottom() - plot.h * (f32::from(at) / 4.0);
        paint.rule((plot.x, y), (plot.right(), y), ink.line, 90);
    }

    let steps = (readings.len() - 1).max(1) as f32;
    let across = plot.w / steps;
    let mut last: Option<(f32, f32)> = None;
    for (at, reading) in readings.iter().enumerate() {
        let x = plot.x + across * at as f32;
        let up = (reading.ms / ceiling) as f32;
        let y = plot.bottom() - plot.h * up.clamp(0.0, 1.0);
        if let Some((from_x, from_y)) = last {
            paint.rule((from_x, from_y), (x, y), ink.accent, 255);
        }
        // A point, so a reading is visible as a reading rather than only as a
        // bend in a line.
        paint.panel(Box::new(x - 2.5, y - 2.5, 5.0, 5.0), 2.5, ink.accent, 255);
        last = Some((x, y));
    }

    // What the two ends were, at the two ends, because a shape with no
    // magnitude anywhere is a picture rather than a measurement (A6).
    let shallow = readings.first().map(|held| held.ms).unwrap_or_default();
    let deep = readings.last().map(|held| held.ms).unwrap_or_default();
    paint.say_at(
        plot.x,
        plot.bottom() + 6.0,
        &format!("{shallow:.2} ms at {} tokens", tokens(readings.first())),
        Weight::Regular,
        10.5,
        ink.faint,
    );
    paint.say_right(
        plot.right(),
        plot.bottom() + 6.0,
        &format!("{deep:.2} ms at {} tokens", tokens(readings.last())),
        Weight::Regular,
        10.5,
        ink.faint,
    );
    plot.bottom() + 26.0
}

/// The depth of a reading, grouped for a person.
fn tokens(reading: Option<&Reading>) -> String {
    reading.map_or_else(|| "?".to_owned(), |held| crate::words::grouped(held.depth))
}
