//! Image cost: what a picture adds to a prompt, in tokens and in time
//! (B-503, D52, B-452).
//!
//! A vision model charges a picture to the same window as its words,
//! and how much depends on the picture's size and the projector's
//! stride — a figure any budget on a vision model needs and none had. A
//! computed picture at three sides goes in with a one-token turn; the
//! tokens the engine read less the words alone is the picture's cost,
//! and the time the prefill took is its price.

use mcf_record::json::Value;

use super::{Found, Site, as_integer, as_ms, timed, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "image-cost";

/// The sides tried, in pixels.
pub const SIDES: [u32; 3] = [224, 448, 896];

/// What is asked beside the picture.
const ASK: &str = "Describe the picture.";

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let Some(projector) = site.projector.clone() else {
        return Found::could_not_tell(
            "no projector is beside this model, so a picture has no way in; a text model has \
             no image cost",
        );
    };
    let engine = match site.server(&Startup {
        projector: Some(projector),
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let Some(marker) = engine.media_marker.clone() else {
        return Found::could_not_tell(
            "the engine places its own marker, so the text cannot say where the picture goes",
        );
    };
    let frame = match crate::turn::frame(&engine, &crate::turn::Turn::default()) {
        Ok(frame) => frame,
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let words_alone = format!("{}{ASK}{}", frame.before, frame.after);
    let text_tokens = match engine.tokenize(&words_alone, true, true) {
        Ok(tokens) => tokens.len(),
        Err(failure) => return Found::could_not_tell(&said(failure)),
    };
    let shown = format!("{}{marker}\n{ASK}{}", frame.before, frame.after);
    let mut lines = vec![format!(
        "  the words alone are {text_tokens} token(s); a computed picture goes in beside them at \
         each side"
    )];
    let mut rows = Vec::new();
    let mut measured = 0_usize;
    let mut largest = 0_usize;
    for side in SIDES {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let picture = crate::probes::vision::image_of(crate::probes::vision::Shape::Triangle, side);
        let (done, ns) = timed(|| {
            engine.complete(
                Prompt::Shown {
                    text: &shown,
                    picture: &picture,
                },
                1,
                Draw::greedy(0),
                true,
                site.timed(),
            )
        });
        match done {
            Ok(completed) => {
                let cost = completed.evaluated.saturating_sub(text_tokens);
                measured = measured.saturating_add(1);
                largest = cost;
                lines.push(format!(
                    "  {side:>4} × {side:<4}   {cost:>5} token(s) for the picture   {:>8} ms to \
                     read the turn",
                    as_ms(ns)
                ));
                rows.push(Value::map([
                    ("side", Value::Integer(i64::from(side))),
                    ("picture_bytes", Value::Integer(as_integer(picture.len()))),
                    (
                        "prompt_tokens",
                        Value::Integer(as_integer(completed.evaluated)),
                    ),
                    ("picture_tokens", Value::Integer(as_integer(cost))),
                    ("prefill_ns", whole(ns)),
                    ("measured", Value::Bool(true)),
                ]));
            }
            Err(failure) => {
                lines.push(format!(
                    "  {side:>4} × {side:<4}   not measured: {}",
                    said(failure)
                ));
                rows.push(Value::map([
                    ("side", Value::Integer(i64::from(side))),
                    ("measured", Value::Bool(false)),
                ]));
            }
        }
    }
    if measured == 0 {
        return Found::could_not_tell("no side could be measured");
    }
    Found {
        lines,
        fields: vec![
            ("text_tokens", Value::Integer(as_integer(text_tokens))),
            ("sides", Value::List(rows)),
            ("measured", Value::Integer(as_integer(measured))),
            ("largest_tokens", Value::Integer(as_integer(largest))),
        ],
    }
}
