#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "a count of readings and a fraction of a plot's height: both are \
              bounded by the size of a window and exact in f32"
)]

use crate::font::Weight;
use crate::paint::{Box, Painter};

#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub depth: u64,
    pub ms: f64,
}

pub fn falloff(paint: &mut Painter, area: Box, readings: &[Reading]) -> f32 {
    let ink = paint.ink;
    if readings.len() < 2 {
        return area.y;
    }
    let deepest = readings.iter().map(|held| held.ms).fold(0.0_f64, f64::max);
    if deepest <= 0.0 {
        return area.y;
    }
    let ceiling = deepest * 1.15;

    let plot = Box::new(area.x, area.y, area.w, area.h);
    paint.rule(
        (plot.x, plot.bottom()),
        (plot.right(), plot.bottom()),
        ink.line,
        255,
    );
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
        paint.panel(Box::new(x - 2.5, y - 2.5, 5.0, 5.0), 2.5, ink.accent, 255);
        last = Some((x, y));
    }

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

fn tokens(reading: Option<&Reading>) -> String {
    reading.map_or_else(|| "?".to_owned(), |held| crate::words::grouped(held.depth))
}
