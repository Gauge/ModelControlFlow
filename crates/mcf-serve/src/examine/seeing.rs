use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::{Found, Reading, Site, as_integer, timed};
use crate::generation::Draw;
use crate::probes::vision::png;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "seeing";

const SIDE: u32 = 448;

const BUDGET: usize = 40;

const GROUND: [u8; 3] = [250, 249, 247];
const INK: [u8; 3] = [30, 30, 34];
const RED: [u8; 3] = [200, 60, 45];
const BLUE: [u8; 3] = [40, 90, 190];
const GREEN: [u8; 3] = [50, 150, 70];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wants {
    Number(i64),
    Word(&'static str),
}

#[derive(Debug)]
pub struct Picture {
    pub name: String,
    pub kind: &'static str,
    pub bytes: Vec<u8>,
    pub asks: &'static str,
    pub wants: Wants,
}

fn blank() -> Vec<Vec<u8>> {
    (0..SIDE)
        .map(|_| {
            let mut row = Vec::with_capacity(SIDE as usize * 3);
            for _ in 0..SIDE {
                row.extend_from_slice(&GROUND);
            }
            row
        })
        .collect()
}

fn disc(rows: &mut [Vec<u8>], centre: (i64, i64), radius: i64, ink: [u8; 3]) {
    for (y, row) in rows.iter_mut().enumerate() {
        let y = i64::try_from(y).unwrap_or(0);
        for x in 0..i64::from(SIDE) {
            let (dx, dy) = (x - centre.0, y - centre.1);
            if dx * dx + dy * dy <= radius * radius {
                let at = usize::try_from(x).unwrap_or(0).saturating_mul(3);
                if let Some(pixel) = row.get_mut(at..at.saturating_add(3)) {
                    pixel.copy_from_slice(&ink);
                }
            }
        }
    }
}

fn block(rows: &mut [Vec<u8>], x0: i64, y0: i64, w: i64, h: i64, ink: [u8; 3]) {
    for (y, row) in rows.iter_mut().enumerate() {
        let y = i64::try_from(y).unwrap_or(0);
        if y < y0 || y >= y0 + h {
            continue;
        }
        for x in x0.max(0)..(x0 + w).min(i64::from(SIDE)) {
            let at = usize::try_from(x).unwrap_or(0).saturating_mul(3);
            if let Some(pixel) = row.get_mut(at..at.saturating_add(3)) {
                pixel.copy_from_slice(&ink);
            }
        }
    }
}

const SEGMENTS: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],
    [false, true, true, false, false, false, false],
    [true, true, false, true, true, false, true],
    [true, true, true, true, false, false, true],
    [false, true, true, false, false, true, true],
    [true, false, true, true, false, true, true],
    [true, false, true, true, true, true, true],
    [true, true, true, false, false, false, false],
    [true, true, true, true, true, true, true],
    [true, true, true, true, false, true, true],
];

#[derive(Debug, Clone, Copy)]
struct Cell {
    left: i64,
    top: i64,
    width: i64,
    height: i64,
    stroke: i64,
}

fn digit(rows: &mut [Vec<u8>], which: usize, cell: Cell) {
    let Some(lit) = SEGMENTS.get(which) else {
        return;
    };
    let Cell {
        left,
        top,
        width,
        height,
        stroke,
    } = cell;
    let half = height.saturating_div(2);
    let strokes = [
        (left, top, width, stroke),
        (left + width - stroke, top, stroke, half),
        (left + width - stroke, top + half, stroke, height - half),
        (left, top + height - stroke, width, stroke),
        (left, top + half, stroke, height - half),
        (left, top, stroke, half),
        (left, top + half - stroke.saturating_div(2), width, stroke),
    ];
    for (on, (sx, sy, sw, sh)) in lit.iter().zip(strokes) {
        if *on {
            block(rows, sx, sy, sw, sh, INK);
        }
    }
}

#[must_use]
pub fn pictures() -> Vec<Picture> {
    let mut out = Vec::new();
    let places = [(112, 112), (336, 112), (112, 336), (336, 336), (224, 224)];
    for count in [1_usize, 2, 3, 4, 5] {
        let mut rows = blank();
        for place in places.iter().take(count) {
            disc(&mut rows, *place, 44, BLUE);
        }
        out.push(Picture {
            name: format!("circles-{count}"),
            kind: "count",
            bytes: png::encode(SIDE, SIDE, &rows),
            asks: "How many circles are in the picture? Answer with the number only.",
            wants: Wants::Number(i64::try_from(count).unwrap_or(0)),
        });
    }
    for number in [305_i64, 472, 918] {
        let mut rows = blank();
        let text = number.to_string();
        let (width, height, stroke, gap) = (80, 160, 18, 40);
        let total = i64::try_from(text.len()).unwrap_or(1) * (width + gap) - gap;
        let first_left = (i64::from(SIDE) - total).saturating_div(2);
        let top = (i64::from(SIDE) - height).saturating_div(2);
        for (at, character) in text.chars().enumerate() {
            let which = character
                .to_digit(10)
                .and_then(|digit| usize::try_from(digit).ok())
                .unwrap_or(0);
            let left = first_left + i64::try_from(at).unwrap_or(0) * (width + gap);
            digit(
                &mut rows,
                which,
                Cell {
                    left,
                    top,
                    width,
                    height,
                    stroke,
                },
            );
        }
        out.push(Picture {
            name: format!("number-{number}"),
            kind: "number",
            bytes: png::encode(SIDE, SIDE, &rows),
            asks: "What number is written in the picture? Answer with the digits only.",
            wants: Wants::Number(number),
        });
    }
    for (word, ink) in [("red", RED), ("blue", BLUE), ("green", GREEN)] {
        let mut rows = blank();
        block(&mut rows, 124, 124, 200, 200, ink);
        out.push(Picture {
            name: format!("square-{word}"),
            kind: "colour",
            bytes: png::encode(SIDE, SIDE, &rows),
            asks: "What colour is the square in the picture? Answer with one word.",
            wants: Wants::Word(word),
        });
    }
    for (side, left_radius, right_radius) in [("left", 90, 40), ("right", 40, 90)] {
        let mut rows = blank();
        disc(&mut rows, (120, 224), left_radius, RED);
        disc(&mut rows, (328, 224), right_radius, RED);
        out.push(Picture {
            name: format!("larger-{side}"),
            kind: "larger",
            bytes: png::encode(SIDE, SIDE, &rows),
            asks: "Two circles: is the larger one on the left or on the right? Answer with one word.",
            wants: Wants::Word(side),
        });
    }
    out
}

#[must_use]
pub fn right(said: &str, wants: &Wants) -> bool {
    match wants {
        Wants::Number(number) => answer_in(said) == Some(*number),
        Wants::Word(word) => {
            let lowered = said.to_lowercase();
            let other = match *word {
                "left" => Some("right"),
                "right" => Some("left"),
                _ => None,
            };
            lowered.contains(word) && other.is_none_or(|other| !lowered.contains(other))
        }
    }
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each picture shown, each answer a row set"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let Some(projector) = site.projector.clone() else {
        return Found::could_not_tell(
            "no projector is beside this model, so a picture has no way in; a text model sees \
             nothing",
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
    let shown = pictures();
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} computed picture(s) of {} × {}, each asked one question, greedy; every answer read \
         by a parser",
        shown.len(),
        SIDE,
        SIDE
    )];
    let mut by_kind: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (at, picture) in shown.iter().enumerate() {
        site.progress(at, shown.len(), &picture.name);
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let text = format!("{}{marker}\n{}{}", frame.before, picture.asks, frame.after);
        let (done, ns) = timed(|| {
            engine.complete(
                Prompt::Shown {
                    text: &text,
                    picture: &picture.bytes,
                },
                BUDGET,
                Draw::greedy(0),
                false,
                site.timed(),
            )
        });
        let completed = match done {
            Ok(completed) => completed,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let was_right = right(&completed.text, &picture.wants);
        let tally = by_kind.entry(picture.kind).or_insert((0, 0));
        tally.0 = tally.0.saturating_add(usize::from(was_right));
        tally.1 = tally.1.saturating_add(1);
        let dims = [
            ("kind", Value::text(picture.kind)),
            ("picture", Value::text(picture.name.clone())),
        ];
        rows.push(Reading::new(&dims, "right", i64::from(was_right), "bool"));
        if let Wants::Number(_) = picture.wants
            && let Some(answer) = answer_in(&completed.text)
        {
            rows.push(Reading::new(&dims, "answer", answer, "count"));
        }
        rows.push(Reading::new(
            &dims,
            "tokens",
            as_integer(completed.predicted),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "ns",
            i64::try_from(ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "picture_bytes",
            as_integer(picture.bytes.len()),
            "bytes",
        ));
        lines.push(format!(
            "  {:<14} {}   said: {}",
            picture.name,
            if was_right { "right" } else { "WRONG" },
            completed.text.trim().chars().take(40).collect::<String>()
        ));
    }
    let mut fields = vec![("pictures", Value::Integer(as_integer(shown.len())))];
    let mut summary = Vec::new();
    let mut right_all = 0_usize;
    for (kind, (right, of)) in &by_kind {
        right_all = right_all.saturating_add(*right);
        summary.push(format!("{kind} {right}/{of}"));
        fields.push((
            match *kind {
                "count" => "count_right",
                "number" => "number_right",
                "colour" => "colour_right",
                _ => "larger_right",
            },
            Value::Integer(as_integer(*right)),
        ));
    }
    fields.push(("right", Value::Integer(as_integer(right_all))));
    lines.push(format!(
        "  {right_all} of {} right: {}",
        shown.len(),
        summary.join(", ")
    ));
    Found {
        lines,
        fields,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{Wants, pictures, right};

    #[test]
    fn every_picture_is_a_png_with_its_answer() {
        let shown = pictures();
        assert_eq!(shown.len(), 13);
        for picture in &shown {
            assert!(picture.bytes.starts_with(b"\x89PNG"), "{}", picture.name);
            assert!(picture.bytes.len() > 100, "{}", picture.name);
        }
        assert_eq!(shown[0].wants, Wants::Number(1));
        assert_eq!(shown[4].wants, Wants::Number(5));
    }

    #[test]
    fn an_answer_is_read_by_kind() {
        assert!(right("There are 3 circles.", &Wants::Number(3)));
        assert!(!right("There are 4 circles.", &Wants::Number(3)));
        assert!(right("The square is Red.", &Wants::Word("red")));
        assert!(!right("blue", &Wants::Word("red")));
        assert!(right("Left.", &Wants::Word("left")));
        assert!(!right(
            "the left one is smaller, so the right",
            &Wants::Word("left")
        ));
    }

    #[test]
    fn two_pictures_of_different_counts_differ() {
        let shown = pictures();
        assert_ne!(shown[0].bytes, shown[1].bytes);
    }
}
