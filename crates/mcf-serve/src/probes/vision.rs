//! Whether an image reaches the model at all (B-057, D42, §X, A21, A19, B-320).
//!
//! **The question is not whether the model sees well.** *Well* is a judgement
//! about what came out, and grading it needs a rater and a graded task, which
//! is a laboratory's work and not a probe's (§XIII). Vision was declined as a
//! modality on exactly that ground, and on a second one that has since gone
//! away: nothing in the build could put the question. A provisioned engine now
//! takes an image, so the question can be put.
//!
//! **What is observable is whether the image participated.** Ask one question
//! twice, with the same seed, over two images that differ in the plainest way
//! two images can. If the answers differ, the pixels reached the model. If they
//! are identical, they did not — the projector is missing, unwired, or being
//! ignored — and every answer this artifact gives about an image is a text-only
//! answer wearing a picture's clothes.
//!
//! That is mechanical, needs no rater, and is what D42 asks of a probe: a wrong
//! answer here corrupts a measurement directly. A vision model benchmarked with
//! its projector silently absent is a model being measured at a different task
//! than the one it is named for, which is §3.8's misconfiguration in the place
//! it is hardest to notice — the output still reads like an answer.
//!
//! **What the probe does not do.** It does not say the triangle is a triangle.
//! It counts whether two different pictures produced two different answers, and
//! reports what each said so a reader can see for themselves. Whether the model
//! is *right* about either is the graded task B-110 will hold.

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};

pub mod png;

/// How wide and tall the probe's images are.
///
/// A square, and large enough that a projector's tiler has something to work
/// with rather than one patch of flat colour. Fixed rather than derived: every
/// number below is a coordinate in this square, and a size that moved would
/// move the shapes with it.
const SIDE: u32 = 448;

/// The ground both images share.
///
/// The same in both, so that what differs between them is the shape and its
/// colour and nothing else.
const GROUND: [u8; 3] = [250, 249, 247];

/// One image: a filled red triangle.
const RED: [u8; 3] = [200, 60, 45];
/// The other: a filled blue circle.
const BLUE: [u8; 3] = [40, 90, 190];

/// The question both images are asked.
///
/// Short, and about the two properties a picture of one shape has. It asks for
/// one sentence because a turn that runs to the budget tells us about the
/// budget rather than about the picture (F106).
pub const QUESTION: &str =
    "What shape is in this image, and what colour is it? Answer in one sentence.";

/// Whether a point is inside the probe's triangle.
///
/// Three edge functions, each a cross product. No division anywhere: a
/// coordinate divided is a coordinate rounded, and a shape whose edge depends
/// on a rounding is a shape that differs between two machines.
fn in_triangle(x: i64, y: i64) -> bool {
    // Apex at the top middle, base across the lower third.
    const APEX: (i64, i64) = (224, 90);
    const LEFT: (i64, i64) = (88, 340);
    const RIGHT: (i64, i64) = (360, 340);
    let edge = |a: (i64, i64), b: (i64, i64)| (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
    let one = edge(APEX, RIGHT);
    let two = edge(RIGHT, LEFT);
    let three = edge(LEFT, APEX);
    (one >= 0 && two >= 0 && three >= 0) || (one <= 0 && two <= 0 && three <= 0)
}

/// Whether a point is inside the probe's circle.
fn in_circle(x: i64, y: i64) -> bool {
    const CENTRE: (i64, i64) = (224, 224);
    const RADIUS_SQUARED: i64 = 112 * 112;
    let (dx, dy) = (x - CENTRE.0, y - CENTRE.1);
    dx * dx + dy * dy <= RADIUS_SQUARED
}

/// The bytes of one of the probe's two images.
///
/// Computed, so that two machines running this probe put the same pixels in
/// front of the same model (§3.4).
#[must_use]
pub fn image(shape: Shape) -> Vec<u8> {
    image_of(shape, SIDE)
}

/// The same picture at another side: the shape's geometry is the probe's,
/// scaled to the side asked for, so that a picture at every size is the
/// same picture (D52, B-503).
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

/// Which of the two pictures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A red triangle.
    Triangle,
    /// A blue circle.
    Circle,
}

impl Shape {
    /// What the shape is, for the report — never for grading the answer.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Triangle => "a red triangle",
            Self::Circle => "a blue circle",
        }
    }
}

/// What the vision probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sees {
    /// What the artifact's own metadata says about a projector (A21).
    pub declared: Declared,
    /// Whether two different pictures produced two different answers.
    ///
    /// The whole of the mechanical question. `false` does not mean the model
    /// cannot see: it means the image did not reach it here, under these
    /// conditions, which is a fact about this setup and is the one that spoils
    /// a measurement.
    pub answers_differ: bool,
    /// What it said about the triangle.
    pub about_triangle: String,
    /// What it said about the circle.
    pub about_circle: String,
}

/// What the artifact claims, read and never believed (A21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The architecture its metadata names.
    pub architecture: String,
    /// Whether a projector was found for it, and where.
    pub projector: Option<String>,
}

/// The method.
pub const VISION: Method = Method {
    name: "vision",
    asks: "one question about each of two pictures that differ in the plainest way two pictures \
           can, with the seed held still, and reports whether the two answers differ — which is \
           whether the pixels reached the model at all",
    decides: "whether MCF may put an image to this artifact and have it participate — and \
              nothing about how well it sees, which is a judgement a graded task makes (D42, \
              §XIII)",
};

/// How a caller puts one picture to the model: image bytes and a question in,
/// what it said and what that spent out.
pub type Look<'a> = &'a mut dyn FnMut(&[u8], &str) -> Option<(String, usize)>;

/// What stands where an engine's name would be when there is no tool to ask
/// with.
pub const NO_TOOL: &str = "no provisioned engine here takes an image";

/// The result when nothing in this build can put the question.
///
/// A separate entry point so that *MCF could not ask* is produced by the probe
/// that owns the method rather than assembled by a caller — and so that the
/// conditions carry the same method name as every other outcome of it. A
/// modality reported absent because MCF could not reach it would be reporting
/// MCF's own reach as a property of the model (A21).
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

/// Runs the vision probe.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason
/// (D42's third state).
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
            // Trimmed before comparing: two answers that differ only in
            // trailing whitespace are the same answer, and calling them
            // different would report vision where there is none.
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
