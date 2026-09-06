//! Grammar cost: what constraining the answer to a shape does to the
//! shape and the bill (B-504, D52, B-054).
//!
//! The structured probe asks whether a model produces JSON when asked
//! for it. The served path can also *make* it: the engine takes a
//! schema and samples nothing outside it. Here the same question is put
//! both ways over the same seeds — the rate of valid JSON with the keys
//! asked for, the tokens and the time — so that a person can read what
//! the constraint buys and costs on this model.

use mcf_record::json::Value;

use super::{Found, Site, as_integer, as_ms, framed_ids, timed, whole};
use crate::generation::{Draw, Truncation};
use crate::served::{Extras, Prompt, Startup};
use mcf_core::configuration::Thousandths;

/// The measurement's name.
pub const NAME: &str = "grammar-cost";

/// How many trials each way.
pub const TRIALS: usize = 5;

/// The temperature, in thousandths: warm enough that the trials differ.
const TEMPERATURE: u32 = 700;

/// How many tokens an answer may take.
const BUDGET: usize = 120;

/// What is asked.
pub const ASK: &str = "Give an imaginary person's name, age and city as JSON with the keys name \
(text), age (whole number) and city (text). Answer with the JSON only.";

/// The keys the answer must carry.
const KEYS: [&str; 3] = ["name", "age", "city"];

/// The schema the constrained trials are held to.
fn schema() -> Value {
    Value::map([
        ("type", Value::text("object")),
        (
            "properties",
            Value::map([
                ("name", Value::map([("type", Value::text("string"))])),
                ("age", Value::map([("type", Value::text("integer"))])),
                ("city", Value::map([("type", Value::text("string"))])),
            ]),
        ),
        (
            "required",
            Value::List(
                KEYS.iter()
                    .map(|key| Value::text((*key).to_owned()))
                    .collect(),
            ),
        ),
    ])
}

/// What one way of asking came to.
#[derive(Debug, Default)]
struct Tally {
    valid: usize,
    tokens: u64,
    ns: u64,
}

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let ids = match framed_ids(&engine, ASK) {
        Ok(ids) => ids,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut free = Tally::default();
    let mut constrained = Tally::default();
    for seed in 0..TRIALS as u64 {
        for (tally, extras) in [
            (&mut free, Extras::default()),
            (
                &mut constrained,
                Extras {
                    json_schema: Some(schema()),
                    ..Extras::default()
                },
            ),
        ] {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let draw = Draw {
                seed,
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
            if conforms(&completed.text) {
                tally.valid = tally.valid.saturating_add(1);
            }
            tally.tokens = tally
                .tokens
                .saturating_add(u64::try_from(completed.predicted).unwrap_or(u64::MAX));
            tally.ns = tally.ns.saturating_add(ns);
        }
    }
    found(&free, &constrained)
}

/// A tally's tokens and time over the trials it was summed across.
#[expect(
    clippy::integer_division,
    reason = "a total over the trials it was summed across"
)]
fn per(tally: &Tally) -> (u64, u64) {
    let trials = u64::try_from(TRIALS).unwrap_or(1).max(1);
    (tally.tokens / trials, tally.ns / trials)
}

/// The two tallies as the finding.
fn found(free: &Tally, constrained: &Tally) -> Found {
    let (free_tokens, free_ns) = per(free);
    let (held_tokens, held_ns) = per(constrained);
    Found {
        lines: vec![
            format!(
                "  \"{ASK}\" at temperature 0.7, seeds 0 to {}, each way",
                TRIALS - 1
            ),
            format!(
                "  free          valid JSON with the keys in {} of {TRIALS}   {free_tokens:>4} \
                 token(s) a trial   {:>8} ms",
                free.valid,
                as_ms(free_ns)
            ),
            format!(
                "  under schema  valid JSON with the keys in {} of {TRIALS}   {held_tokens:>4} \
                 token(s) a trial   {:>8} ms",
                constrained.valid,
                as_ms(held_ns)
            ),
        ],
        fields: vec![
            ("ask", Value::text(ASK.to_owned())),
            ("trials", Value::Integer(as_integer(TRIALS))),
            (
                "temperature_thousandths",
                Value::Integer(i64::from(TEMPERATURE)),
            ),
            ("free_valid", Value::Integer(as_integer(free.valid))),
            ("free_tokens_per_trial", whole(free_tokens)),
            ("free_ns_per_trial", whole(free_ns)),
            (
                "constrained_valid",
                Value::Integer(as_integer(constrained.valid)),
            ),
            ("constrained_tokens_per_trial", whole(held_tokens)),
            ("constrained_ns_per_trial", whole(held_ns)),
        ],
    }
}

/// Whether a text carries a JSON object with the keys asked for, the age
/// a whole number — read by a parser, never for sense.
pub(crate) fn conforms(text: &str) -> bool {
    let (Some(open), Some(close)) = (text.find('{'), text.rfind('}')) else {
        return false;
    };
    let Some(object) = text.get(open..=close) else {
        return false;
    };
    let Ok(Value::Map(fields)) = mcf_record::json::parse(object) else {
        return false;
    };
    KEYS.iter().all(|key| fields.contains_key(*key))
        && matches!(fields.get("name"), Some(Value::Text(_)))
        && matches!(fields.get("city"), Some(Value::Text(_)))
        && matches!(fields.get("age"), Some(Value::Integer(_)))
}
