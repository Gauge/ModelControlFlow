//! The one thing a terminal cannot draw.
//!
//! A table can say that generation slows with depth. It cannot show the
//! *shape*, and the shape is what makes a fourteen-fold gap between two devices
//! and a rising line legible at the same time. This is the reason the window is
//! worth having, so it is the part that is pixels rather than cells.
//!
//! **Logarithmic on both axes, and said so.** Depth doubles, and the devices
//! are an order of magnitude apart; on linear axes one line would be flat
//! against the floor. A reader who is not told the axis is logarithmic is being
//! misled by a picture, so the label carries it (A6).

use crate::sdl::{Rect, Window};

/// One measured series.
#[derive(Debug, Clone)]
pub struct Series {
    /// What it is: a device, usually.
    pub name: String,
    /// Depth and milliseconds per token, in order.
    pub points: Vec<(f64, f64)>,
    /// What colour to draw it.
    pub colour: (u8, u8, u8),
}

/// Where on the window the plot goes.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// Left edge, in pixels.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// Draws the axes and every series.
///
/// Points are joined by short filled rectangles rather than by a line
/// primitive: SDL has one, and this keeps the drawing to the two calls the
/// binding already makes, which is twelve functions rather than thirteen.
pub fn draw(window: &Window, frame: Frame, series: &[Series], grid: (u8, u8, u8)) {
    let all: Vec<(f64, f64)> = series.iter().flat_map(|s| s.points.clone()).collect();
    if all.len() < 2 {
        return;
    }
    let (mut low_x, mut high_x) = (f64::MAX, f64::MIN);
    let (mut low_y, mut high_y) = (f64::MAX, f64::MIN);
    for (x, y) in &all {
        if *x <= 0.0 || *y <= 0.0 {
            continue;
        }
        low_x = low_x.min(x.log2());
        high_x = high_x.max(x.log2());
        low_y = low_y.min(y.log10());
        high_y = high_y.max(y.log10());
    }
    // A range of nothing would divide by nothing; a single depth is not a
    // curve and is not drawn as one.
    if !(high_x > low_x && high_y > low_y) {
        return;
    }
    // A tenth of the range as margin, so a point never sits on the frame.
    let pad_y = (high_y - low_y) * 0.1;
    let (low_y, high_y) = (low_y - pad_y, high_y + pad_y);

    // A fraction of the frame, which is between nought and one, so narrowing
    // it to the renderer's own width loses nothing a pixel could show.
    #[allow(clippy::cast_possible_truncation, reason = "a fraction, into pixels")]
    let place = |x: f64, y: f64| -> (f32, f32) {
        let across = ((x.log2() - low_x) / (high_x - low_x)) as f32;
        let up = ((y.log10() - low_y) / (high_y - low_y)) as f32;
        (frame.x + across * frame.w, frame.y + frame.h - up * frame.h)
    };

    for step in 0..=4 {
        let y = frame.y + frame.h * (f32::from(u8::try_from(step).unwrap_or(0)) / 4.0);
        window.fill(
            Rect {
                x: frame.x,
                y,
                w: frame.w,
                h: 1.0,
            },
            grid,
        );
    }

    for held in series {
        let mut previous: Option<(f32, f32)> = None;
        for (x, y) in &held.points {
            if *x <= 0.0 || *y <= 0.0 {
                continue;
            }
            let (px, py) = place(*x, *y);
            if let Some((qx, qy)) = previous {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a pixel distance, which is positive and small"
                )]
                let steps = ((px - qx).abs().max((py - qy).abs()) as usize).max(1);
                for step in 0..=steps {
                    let along = f32::from(u8::try_from(step).unwrap_or(u8::MAX))
                        / f32::from(u8::try_from(steps).unwrap_or(u8::MAX));
                    window.fill(
                        Rect {
                            x: qx + (px - qx) * along,
                            y: qy + (py - qy) * along,
                            w: 2.0,
                            h: 2.0,
                        },
                        held.colour,
                    );
                }
            }
            window.fill(
                Rect {
                    x: px - 3.0,
                    y: py - 3.0,
                    w: 6.0,
                    h: 6.0,
                },
                held.colour,
            );
            previous = Some((px, py));
        }
    }
}

#[cfg(test)]
mod tests;
