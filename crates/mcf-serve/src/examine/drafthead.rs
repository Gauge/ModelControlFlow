//! The draft head's gain: the same generation with and without the
//! model's own draft head — tokens a second each way, and whether the
//! outputs agree (B-533, D55, B-456).
//!
//! A file that carries a draft head carries a promise of speed, and an
//! engine told to use it draws several tokens a step and keeps those the
//! model would have drawn. Whether it is faster here, and whether what
//! comes out is the same, are two measurements, and a file that carries
//! no draft head has neither: this says so and measures nothing.

use mcf_record::json::Value;

use super::determinism::divergence_at;
use super::{Found, Reading, Site, as_integer, framed_ids, per_second, timed};
use crate::declared::{Declared, Started};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "draft-head";

/// How many tokens each generation produces, at most.
const PRODUCE: usize = 384;

/// How many generations each way.
const REPEATS: usize = 3;

/// What is generated from.
const ASK: &str = "Explain, in plain prose and at some length, how a bicycle's gears let a rider \
climb a hill and then go fast on the flat.";

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each way started, each generation a row, the agreement beside them"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let Some(layers) = Declared::of(site.model).draft_head else {
        return Found::could_not_tell(
            "this file declares no draft head, so there is nothing to start with or without",
        );
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  the file declares a draft head of {layers} layer(s); {REPEATS} generation(s) of up \
         to {PRODUCE} tokens each way, greedy, from one prompt"
    )];
    let mut outputs: [Vec<Vec<usize>>; 2] = [Vec::new(), Vec::new()];
    let mut rates: [Vec<u64>; 2] = [Vec::new(), Vec::new()];
    for (way, with) in [("without", false), ("with", true)] {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let engine = match site.server(&Startup {
            projector: None,
            started: Started {
                draft_head: with,
                ..Started::default()
            },
            ..site.startup()
        }) {
            Ok(engine) => engine,
            Err(why) => return Found::could_not_tell(&why),
        };
        let ids = match framed_ids(&engine, ASK) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        for repeat in 0..REPEATS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let (done, ns) = timed(|| {
                engine.complete(
                    Prompt::Identifiers(&ids),
                    PRODUCE,
                    Draw::greedy(0),
                    false,
                    site.timed(),
                )
            });
            let completed = match done {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let produced = u64::try_from(completed.predicted).unwrap_or(0);
            let rate = per_second(produced, ns);
            let dims = [
                ("draft_head", Value::Bool(with)),
                ("repeat", Value::Integer(as_integer(repeat))),
            ];
            rows.push(Reading::new(
                &dims,
                "produced",
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
                "per_second",
                i64::try_from(rate).unwrap_or(i64::MAX),
                "tokens/s",
            ));
            if let Some(held) = outputs.get_mut(usize::from(with)) {
                held.push(completed.words().to_vec());
            }
            if let Some(held) = rates.get_mut(usize::from(with)) {
                held.push(rate);
            }
        }
        lines.push(format!(
            "  {way:<8} {} tokens/s over {REPEATS} generation(s)",
            rates
                .get(usize::from(with))
                .map(|held| held
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<String>>()
                    .join(", "))
                .unwrap_or_default()
        ));
    }
    // Agreement: the first generation each way, token for token.
    let (plain, drafted) = (
        outputs.first().and_then(|held| held.first()),
        outputs.get(1).and_then(|held| held.first()),
    );
    let parted = match (plain, drafted) {
        (Some(plain), Some(drafted)) => divergence_at(plain, drafted),
        _ => None,
    };
    let identical = matches!((plain, drafted), (Some(_), Some(_))) && parted.is_none();
    rows.push(Reading::new(&[], "identical", i64::from(identical), "bool"));
    if let Some(at) = parted {
        rows.push(Reading::new(&[], "divergence_at", as_integer(at), "tokens"));
    }
    lines.push(match parted {
        None if identical => "  the outputs are identical token for token".to_owned(),
        None => "  the outputs could not be compared".to_owned(),
        Some(at) => format!("  the outputs part at token {at}"),
    });
    Found {
        lines,
        fields: vec![
            (
                "layers",
                Value::Integer(i64::try_from(layers).unwrap_or(i64::MAX)),
            ),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("repeats", Value::Integer(as_integer(REPEATS))),
            (
                "without_per_second",
                rates
                    .first()
                    .and_then(|held| held.first())
                    .map_or(Value::Null, |rate| {
                        Value::Integer(i64::try_from(*rate).unwrap_or(i64::MAX))
                    }),
            ),
            (
                "with_per_second",
                rates
                    .get(1)
                    .and_then(|held| held.first())
                    .map_or(Value::Null, |rate| {
                        Value::Integer(i64::try_from(*rate).unwrap_or(i64::MAX))
                    }),
            ),
            ("identical", Value::Bool(identical)),
            (
                "divergence_at",
                parted.map_or(Value::Null, |at| Value::Integer(as_integer(at))),
            ),
        ],
        rows,
    }
}
