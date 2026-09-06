//! The cache's precision: the same generation with the engine's
//! key-value cache held at sixteen, eight and four bits — tokens a
//! second, resident bytes, and whether the output agrees with the
//! sixteen-bit one and where it parts (B-534, D55, B-491).
//!
//! A quantized cache is the usual way to fit a longer window on a card,
//! and it is a change to the arithmetic every token is drawn from. Each
//! precision is a server started with the cache told to hold keys and
//! values that way; the generation is the same prompt, greedy, so what
//! differs is the cache.

use mcf_record::json::Value;

use super::determinism::divergence_at;
use super::{Found, Reading, Site, as_integer, framed_ids, per_second, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "cache-precision";

/// The cache types tried, as the engine names them, with the bits each
/// holds a value in.
pub const PRECISIONS: [(&str, u32); 3] = [("f16", 16), ("q8_0", 8), ("q4_0", 4)];

/// How many tokens the generation produces, at most.
const PRODUCE: usize = 384;

/// How long the prompt read before it is, in tokens: enough that the
/// cache holds something.
const PROMPT: usize = 1024;

/// What is generated from, after the filler.
const ASK: &str = "Now write, in plain prose, a description of a harbour at dawn.";

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each precision started, generated, compared"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  one prompt of about {PROMPT} tokens then {PRODUCE} produced, greedy, with the \
         key-value cache at each precision; compared with the 16-bit run token for token"
    )];
    let mut reference: Option<Vec<usize>> = None;
    let mut fields = vec![
        ("produce", Value::Integer(as_integer(PRODUCE))),
        ("prompt", Value::Integer(as_integer(PROMPT))),
    ];
    for (cache, bits) in PRECISIONS {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let engine = match site.server(&Startup {
            projector: None,
            cache: Some(cache),
            ..site.startup()
        }) {
            Ok(engine) => engine,
            Err(why) => {
                lines.push(format!("  {cache:<6} not started: {why}"));
                rows.push(Reading::new(
                    &[("cache", Value::text(cache))],
                    "started",
                    0,
                    "bool",
                ));
                continue;
            }
        };
        let mut ids = match framed_ids(&engine, &prompt_text()) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        ids.truncate(ids.len());
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
        let resident = engine.resident_bytes();
        let card = crate::engines::card_memory_used();
        let words = completed.words().to_vec();
        let parted = reference
            .as_ref()
            .and_then(|held| divergence_at(held, &words));
        let identical = reference.as_ref().is_some_and(|held| *held == words);
        if reference.is_none() {
            reference = Some(words.clone());
        }
        let dims = [("cache", Value::text(cache))];
        rows.push(Reading::new(&dims, "started", 1, "bool"));
        rows.push(Reading::new(&dims, "bits", i64::from(bits), "bits"));
        rows.push(Reading::new(
            &dims,
            "prompt_tokens",
            as_integer(ids.len()),
            "tokens",
        ));
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
        if let Some(bytes) = resident {
            rows.push(Reading::new(
                &dims,
                "resident",
                i64::try_from(bytes).unwrap_or(i64::MAX),
                "bytes",
            ));
        }
        if let Some(bytes) = card {
            rows.push(Reading::new(
                &dims,
                "card_used",
                i64::try_from(bytes).unwrap_or(i64::MAX),
                "bytes",
            ));
        }
        if cache != "f16" {
            rows.push(Reading::new(
                &dims,
                "identical",
                i64::from(identical),
                "bool",
            ));
            if let Some(at) = parted {
                rows.push(Reading::new(
                    &dims,
                    "divergence_at",
                    as_integer(at),
                    "tokens",
                ));
            }
        }
        lines.push(format!(
            "  {cache:<6} {rate} tokens/s{}{}{}",
            resident.map_or(String::new(), |bytes| format!(
                "   {} resident",
                super::gigabytes(bytes)
            )),
            card.map_or(String::new(), |bytes| format!(
                "   {} on the card",
                super::gigabytes(bytes)
            )),
            if cache == "f16" {
                String::new()
            } else if identical {
                "   identical to the 16-bit run".to_owned()
            } else {
                parted.map_or_else(
                    || "   not compared".to_owned(),
                    |at| format!("   parts from the 16-bit run at token {at}"),
                )
            }
        ));
        fields.push((
            match cache {
                "f16" => "f16_per_second",
                "q8_0" => "q8_per_second",
                _ => "q4_per_second",
            },
            Value::Integer(i64::try_from(rate).unwrap_or(i64::MAX)),
        ));
        if cache != "f16" {
            fields.push((
                if cache == "q8_0" {
                    "q8_divergence"
                } else {
                    "q4_divergence"
                },
                parted.map_or(Value::Null, |at| Value::Integer(as_integer(at))),
            ));
        }
    }
    Found {
        lines,
        fields,
        rows,
    }
}

/// The prompt: filler to about `PROMPT` tokens, then the ask.
fn prompt_text() -> String {
    let filler = super::retrieval::FILLER;
    let mut text = String::new();
    // About six characters a token: enough repeats to reach the length.
    while text.len() < PROMPT.saturating_mul(4) {
        text.push_str(filler);
    }
    text.push_str("\n\n");
    text.push_str(ASK);
    text
}
