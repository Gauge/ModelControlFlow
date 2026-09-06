//! Schema variety under a grammar: nested objects, arrays, enums and
//! optional fields, each asked for free and under the engine's schema
//! constraint, each answer read by a parser for validity and for the
//! shape (B-539, D55, B-504).
//!
//! The grammar-cost measurement holds one flat schema. Real schemas
//! nest, list, choose among a few words and leave fields out; whether
//! a model produces those shapes on its own, and what the constraint
//! costs in tokens and time on each, are rows a person picks a model
//! for structured output by.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::{Draw, Truncation};
use crate::served::{Extras, Prompt, Startup};
use mcf_core::configuration::Thousandths;

/// The measurement's name.
pub const NAME: &str = "schemas";

/// How many trials a schema each way.
pub const TRIALS: usize = 3;

/// The temperature, in thousandths.
const TEMPERATURE: u32 = 700;

/// How many tokens an answer may take.
const BUDGET: usize = 200;

/// One schema: its name, what is asked, the schema itself, and a check
/// of the shape beyond validity.
#[derive(Debug)]
pub struct Shape {
    /// Its name.
    pub name: &'static str,
    /// What is asked.
    pub asks: &'static str,
    /// The schema, built when asked for.
    pub schema: fn() -> Value,
    /// Whether a parsed value has the shape.
    pub conforms: fn(&Value) -> bool,
}

/// The shapes.
pub const SHAPES: &[Shape] = &[
    Shape {
        name: "nested",
        asks: "Give an imaginary person as JSON with the keys name (text) and address, where \
               address is an object with the keys street (text), city (text) and postcode \
               (text). Answer with the JSON only.",
        schema: || {
            Value::map([
                ("type", Value::text("object")),
                (
                    "properties",
                    Value::map([
                        ("name", Value::map([("type", Value::text("string"))])),
                        (
                            "address",
                            Value::map([
                                ("type", Value::text("object")),
                                (
                                    "properties",
                                    Value::map([
                                        ("street", Value::map([("type", Value::text("string"))])),
                                        ("city", Value::map([("type", Value::text("string"))])),
                                        ("postcode", Value::map([("type", Value::text("string"))])),
                                    ]),
                                ),
                                ("required", texts(&["street", "city", "postcode"])),
                            ]),
                        ),
                    ]),
                ),
                ("required", texts(&["name", "address"])),
            ])
        },
        conforms: |held| {
            matches!(held.get("name"), Some(Value::Text(_)))
                && held.get("address").is_some_and(|address| {
                    ["street", "city", "postcode"]
                        .iter()
                        .all(|key| matches!(address.get(key), Some(Value::Text(_))))
                })
        },
    },
    Shape {
        name: "array",
        asks: "Give three imaginary books as JSON: an object with one key, books, holding an \
               array of exactly three objects, each with the keys title (text) and pages (whole \
               number). Answer with the JSON only.",
        schema: || {
            Value::map([
                ("type", Value::text("object")),
                (
                    "properties",
                    Value::map([(
                        "books",
                        Value::map([
                            ("type", Value::text("array")),
                            ("minItems", Value::Integer(3)),
                            ("maxItems", Value::Integer(3)),
                            (
                                "items",
                                Value::map([
                                    ("type", Value::text("object")),
                                    (
                                        "properties",
                                        Value::map([
                                            (
                                                "title",
                                                Value::map([("type", Value::text("string"))]),
                                            ),
                                            (
                                                "pages",
                                                Value::map([("type", Value::text("integer"))]),
                                            ),
                                        ]),
                                    ),
                                    ("required", texts(&["title", "pages"])),
                                ]),
                            ),
                        ]),
                    )]),
                ),
                ("required", texts(&["books"])),
            ])
        },
        conforms: |held| {
            matches!(held.get("books"), Some(Value::List(books)) if books.len() == 3
            && books.iter().all(|book| {
                matches!(book.get("title"), Some(Value::Text(_)))
                    && matches!(book.get("pages"), Some(Value::Integer(_)))
            }))
        },
    },
    Shape {
        name: "enum",
        asks: "Describe today's imaginary weather as JSON with the keys sky, which must be one \
               of \"clear\", \"cloudy\" or \"rain\", and wind, which must be one of \"calm\", \
               \"breeze\" or \"gale\". Answer with the JSON only.",
        schema: || {
            Value::map([
                ("type", Value::text("object")),
                (
                    "properties",
                    Value::map([
                        (
                            "sky",
                            Value::map([("enum", texts(&["clear", "cloudy", "rain"]))]),
                        ),
                        (
                            "wind",
                            Value::map([("enum", texts(&["calm", "breeze", "gale"]))]),
                        ),
                    ]),
                ),
                ("required", texts(&["sky", "wind"])),
            ])
        },
        conforms: |held| {
            matches!(held.get("sky"), Some(Value::Text(sky)) if ["clear", "cloudy", "rain"].contains(&sky.as_str()))
                && matches!(held.get("wind"), Some(Value::Text(wind)) if ["calm", "breeze", "gale"].contains(&wind.as_str()))
        },
    },
    Shape {
        name: "optional",
        asks: "Give an imaginary product as JSON with the required keys name (text) and price \
               (whole number of cents), and an optional key discount (whole number of cents) \
               which you may leave out. Include no other keys. Answer with the JSON only.",
        schema: || {
            Value::map([
                ("type", Value::text("object")),
                (
                    "properties",
                    Value::map([
                        ("name", Value::map([("type", Value::text("string"))])),
                        ("price", Value::map([("type", Value::text("integer"))])),
                        ("discount", Value::map([("type", Value::text("integer"))])),
                    ]),
                ),
                ("required", texts(&["name", "price"])),
                ("additionalProperties", Value::Bool(false)),
            ])
        },
        conforms: |held| {
            let Value::Map(fields) = held else {
                return false;
            };
            matches!(fields.get("name"), Some(Value::Text(_)))
                && matches!(fields.get("price"), Some(Value::Integer(_)))
                && fields
                    .keys()
                    .all(|key| ["name", "price", "discount"].contains(&key.as_str()))
                && fields
                    .get("discount")
                    .is_none_or(|held| matches!(held, Value::Integer(_)))
        },
    },
];

/// A list of texts as a JSON value.
fn texts(words: &[&str]) -> Value {
    Value::List(words.iter().map(|word| Value::text(*word)).collect())
}

/// The JSON object in an answer, if any.
fn object_in(said: &str) -> Option<Value> {
    let (open, close) = (said.find('{')?, said.rfind('}')?);
    let held = mcf_record::json::parse(said.get(open..=close)?).ok()?;
    matches!(held, Value::Map(_)).then_some(held)
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each shape, each way, each trial a row"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} shape(s), {TRIALS} trial(s) each free and each under the schema, at temperature \
         {TEMPERATURE}; each answer read for valid JSON and for the shape",
        SHAPES.len()
    )];
    let (mut free_shaped, mut constrained_shaped, mut trials) = (0_usize, 0_usize, 0_usize);
    for (at, shape) in SHAPES.iter().enumerate() {
        site.progress(at, SHAPES.len(), shape.name);
        let ids = match framed_ids(&engine, shape.asks) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let mut said = Vec::with_capacity(2);
        for (condition, extras) in [
            ("free", Extras::default()),
            (
                "constrained",
                Extras {
                    json_schema: Some((shape.schema)()),
                    ..Extras::default()
                },
            ),
        ] {
            let mut shaped = 0_usize;
            for seed in 0..TRIALS {
                if site.asker_gone() {
                    return Found::could_not_tell(crate::served::CLIENT_LEFT);
                }
                let draw = Draw {
                    seed: u64::try_from(seed).unwrap_or(0),
                    temperature: Thousandths(TEMPERATURE),
                    truncation: Truncation::OFF,
                };
                let (done, ns) = timed(|| {
                    engine.complete_with(
                        Prompt::Identifiers(&ids),
                        BUDGET,
                        draw,
                        false,
                        &extras,
                        site.timed(),
                    )
                });
                let completed = match done {
                    Ok(completed) => completed,
                    Err(failure) => return Found::could_not_tell(failure.detail()),
                };
                let object = object_in(&completed.text);
                let conforms = object.as_ref().is_some_and(|held| (shape.conforms)(held));
                shaped = shaped.saturating_add(usize::from(conforms));
                trials = trials.saturating_add(1);
                let dims = [
                    ("shape", Value::text(shape.name)),
                    ("condition", Value::text(condition)),
                    ("trial", Value::Integer(as_integer(seed))),
                ];
                rows.push(Reading::new(
                    &dims,
                    "valid",
                    i64::from(object.is_some()),
                    "bool",
                ));
                rows.push(Reading::new(&dims, "shaped", i64::from(conforms), "bool"));
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
            }
            if condition == "free" {
                free_shaped = free_shaped.saturating_add(shaped);
            } else {
                constrained_shaped = constrained_shaped.saturating_add(shaped);
            }
            said.push(format!("{condition} {shaped}/{TRIALS}"));
        }
        lines.push(format!("  {:<10} {}", shape.name, said.join("   ")));
    }
    lines.push(format!(
        "  the shape held in {free_shaped} free and {constrained_shaped} constrained trial(s) of \
         {} each way",
        trials.saturating_div(2)
    ));
    Found {
        lines,
        fields: vec![
            ("shapes", Value::Integer(as_integer(SHAPES.len()))),
            ("trials", Value::Integer(as_integer(TRIALS))),
            ("free_shaped", Value::Integer(as_integer(free_shaped))),
            (
                "constrained_shaped",
                Value::Integer(as_integer(constrained_shaped)),
            ),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{SHAPES, object_in};

    #[test]
    fn each_shape_is_told_from_a_near_miss() {
        let cases = [
            (
                "nested",
                r#"{"name":"A","address":{"street":"S","city":"C","postcode":"P"}}"#,
                r#"{"name":"A","address":"S, C"}"#,
            ),
            (
                "array",
                r#"{"books":[{"title":"a","pages":1},{"title":"b","pages":2},{"title":"c","pages":3}]}"#,
                r#"{"books":[{"title":"a","pages":1}]}"#,
            ),
            (
                "enum",
                r#"{"sky":"rain","wind":"gale"}"#,
                r#"{"sky":"sunny","wind":"gale"}"#,
            ),
            (
                "optional",
                r#"{"name":"n","price":100}"#,
                r#"{"name":"n","price":100,"colour":"red"}"#,
            ),
        ];
        for (name, yes, no) in cases {
            let shape = SHAPES.iter().find(|shape| shape.name == name).unwrap();
            assert!((shape.conforms)(&object_in(yes).unwrap()), "{name} yes");
            assert!(!(shape.conforms)(&object_in(no).unwrap()), "{name} no");
            assert!(matches!((shape.schema)(), mcf_record::json::Value::Map(_)));
        }
        assert!((SHAPES[3].conforms)(
            &object_in(r#"{"name":"n","price":100,"discount":5}"#).unwrap()
        ));
    }
}
