use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};

pub mod png;

const SIDE: u32 = 448;

const GROUND: [u8; 3] = [250, 249, 247];

const RED: [u8; 3] = [200, 60, 45];
const BLUE: [u8; 3] = [40, 90, 190];

pub const QUESTION: &str =
    "What shape is in this image, and what colour is it? Answer in one sentence.";

fn in_triangle(x: i64, y: i64) -> bool {
    const APEX: (i64, i64) = (224, 90);
    const LEFT: (i64, i64) = (88, 340);
    const RIGHT: (i64, i64) = (360, 340);
    let edge = |a: (i64, i64), b: (i64, i64)| (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
    let one = edge(APEX, RIGHT);
    let two = edge(RIGHT, LEFT);
    let three = edge(LEFT, APEX);
    (one >= 0 && two >= 0 && three >= 0) || (one <= 0 && two <= 0 && three <= 0)
}

fn in_circle(x: i64, y: i64) -> bool {
    const CENTRE: (i64, i64) = (224, 224);
    const RADIUS_SQUARED: i64 = 112 * 112;
    let (dx, dy) = (x - CENTRE.0, y - CENTRE.1);
    dx * dx + dy * dy <= RADIUS_SQUARED
}

#[must_use]
pub fn image(shape: Shape) -> Vec<u8> {
    image_of(shape, SIDE)
}

#[must_use]
pub fn image_of(shape: Shape, side: u32) -> Vec<u8> {
    let side = side.max(1);
    let scaled = |at: u32| {
        #[expect(
            clippy::integer_division,
            reason = "a pixel's place on the probe's own grid, floored to a pixel"
        )]
        let on_the_grid = u64::from(at) * u64::from(SIDE) / u64::from(side);
        i64::try_from(on_the_grid).unwrap_or(i64::MAX)
    };
    let mut rows = Vec::with_capacity(side as usize);
    for y in 0..side {
        let mut row = Vec::with_capacity(side as usize * 3);
        for x in 0..side {
            let (inside, ink) = match shape {
                Shape::Triangle => (in_triangle(scaled(x), scaled(y)), RED),
                Shape::Circle => (in_circle(scaled(x), scaled(y)), BLUE),
            };
            row.extend_from_slice(if inside { &ink } else { &GROUND });
        }
        rows.push(row);
    }
    png::encode(side, side, &rows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Triangle,
    Circle,
}

impl Shape {
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Triangle => "a red triangle",
            Self::Circle => "a blue circle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sees {
    pub declared: Declared,
    pub answers_differ: bool,
    pub about_triangle: String,
    pub about_circle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub architecture: String,
    pub projector: Option<String>,
}

pub const VISION: Method = Method {
    name: "vision",
    asks: "one question about each of two pictures that differ in the plainest way two pictures \
           can, with the seed held still, and reports whether the two answers differ — which is \
           whether the pixels reached the model at all",
    decides: "whether MCF may put an image to this artifact and have it participate — and \
              nothing about how well it sees, which is a judgement a graded task makes (D42, \
              §XIII)",
};

pub type Look<'a> = &'a mut dyn FnMut(&[u8], &str) -> Option<(String, usize)>;

pub const NO_TOOL: &str = "no provisioned engine here takes an image";

#[must_use]
pub fn without_a_tool(model: &Path) -> Probed<Sees> {
    Probed::inconclusive(
        VISION,
        "no provisioned engine here carries a tool that takes an image — `mcf provision \
         llama.cpp` builds one. MCF reports that it could not ask, rather than that this model \
         cannot see (A21)",
        0,
        0,
        super::conditions(&VISION, model, NO_TOOL),
    )
}

#[must_use]
pub fn vision(
    model: &Path,
    bytes: &[u8],
    projector: Option<&Path>,
    engine: &str,
    look: Look<'_>,
) -> Probed<Sees> {
    let conditions = super::conditions(&VISION, model, engine);
    let Ok(file) = mcf_standin::gguf::parse(bytes) else {
        return Probed::inconclusive(
            VISION,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let architecture = file
        .get("general.architecture")
        .and_then(mcf_standin::gguf::Value::as_text)
        .unwrap_or("unstated")
        .to_owned();
    let declared = Declared {
        architecture,
        projector: projector.map(|at| at.display().to_string()),
    };

    if projector.is_none() {
        return Probed::inconclusive(
            VISION,
            "no projector was found beside this artifact, and a projector is what turns pixels \
             into something a text transformer can read. MCF reports that it did not find one, \
             rather than that this model cannot see (A7)",
            0,
            0,
            conditions,
        );
    }

    let Some((about_triangle, first_spent)) = look(&image(Shape::Triangle), QUESTION) else {
        return Probed::inconclusive(
            VISION,
            "the engine did not answer with the first picture in front of it, so whether an \
             image reaches this model is not a question that was put",
            1,
            0,
            conditions,
        );
    };
    let Some((about_circle, second_spent)) = look(&image(Shape::Circle), QUESTION) else {
        return Probed::inconclusive(
            VISION,
            "the first picture produced an answer and the second did not, so the two cannot be \
             compared and nothing is concluded",
            2,
            first_spent,
            conditions,
        );
    };

    Probed {
        method: VISION,
        outcome: Outcome::Observed(Sees {
            declared,
            answers_differ: about_triangle.trim() != about_circle.trim(),
            about_triangle,
            about_circle,
        }),
        trials: 2,
        tokens: first_spent.saturating_add(second_spent),
        conditions,
    }
}

#[cfg(test)]
mod tests;
