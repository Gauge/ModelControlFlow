//! Memory as predicted: what the engine holds against what MCF said it
//! would (B-495, D52, B-058).
//!
//! MCF says whether a model fits by arithmetic on the header — the
//! weights, an allowance over them, and a cache the header's widths give
//! a token — and a person reads *fits here* off that. Here the engine is
//! started at several windows, made to touch every weight, and asked
//! what it holds: its resident bytes from the kernel, and what the card
//! gained while it was up. A difference is a divergence between what
//! was declared and what was observed, and it is reported as one.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, filler, gigabytes, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "memory-as-predicted";

/// The windows tried, in tokens; each is capped at the trained context.
const WINDOWS: [u64; 3] = [2048, 8192, 32768];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let Some(bytes) = crate::probes::run::read_prefix(site.model) else {
        return Found::could_not_tell("the file could not be read as a model");
    };
    let Ok(file) = mcf_standin::gguf::parse(&bytes) else {
        return Found::could_not_tell("the file could not be read as a model");
    };
    let Ok(weights) = mcf_hub::store::bytes_of_the_whole(site.model) else {
        return Found::could_not_tell("the file's size could not be read");
    };
    let per_token = crate::engines::cache_bytes_per_token(&file);
    let trained = file.architecture().and_then(|architecture| {
        file.get(&format!("{architecture}.context_length"))
            .and_then(mcf_standin::gguf::Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
    });
    let mut lines = vec![format!(
        "  weights {}; MCF allows {} over them and {} a token of window",
        gigabytes(weights),
        gigabytes(crate::engines::overhead_for(weights)),
        per_token.map_or_else(
            || "an unknown amount".to_owned(),
            |held| format!("{held} bytes")
        )
    )];
    let mut rows = Vec::new();
    let mut readings = Vec::new();
    let mut measured = 0_usize;
    let mut worst: i64 = 0;
    for window in WINDOWS {
        if trained.is_some_and(|trained| window > trained) || site.asker_gone() {
            continue;
        }
        let predicted = weights
            .saturating_add(crate::engines::overhead_for(weights))
            .saturating_add(per_token.unwrap_or(0).saturating_mul(window));
        match at_window(site, window) {
            Ok((resident, card)) => {
                measured = measured.saturating_add(1);
                let at = [("window", whole(window))];
                let bytes = |held: u64| i64::try_from(held).unwrap_or(i64::MAX);
                readings.push(Reading::new(
                    &at,
                    "predicted_bytes",
                    bytes(predicted),
                    "bytes",
                ));
                readings.push(Reading::new(
                    &at,
                    "resident_bytes",
                    bytes(resident),
                    "bytes",
                ));
                if let Some(card) = card {
                    readings.push(Reading::new(&at, "card_bytes", bytes(card), "bytes"));
                }
                let observed = resident.saturating_add(card.unwrap_or(0));
                let signed = i64::try_from(observed).unwrap_or(i64::MAX)
                    - i64::try_from(predicted).unwrap_or(i64::MAX);
                let off =
                    super::ppm(signed.unsigned_abs(), predicted) * if signed < 0 { -1 } else { 1 };
                if off.abs() > worst.abs() {
                    worst = off;
                }
                lines.push(format!(
                    "  window {window:>6}   predicted {:>8}   observed {:>8} (resident {}{})   \
                     {} {} the prediction",
                    gigabytes(predicted),
                    gigabytes(observed),
                    gigabytes(resident),
                    card.map_or_else(String::new, |card| format!(", card +{}", gigabytes(card))),
                    super::per_cent(off.abs()),
                    if signed < 0 { "under" } else { "over" }
                ));
                rows.push(Value::map([
                    ("window", whole(window)),
                    ("predicted_bytes", whole(predicted)),
                    ("resident_bytes", whole(resident)),
                    ("card_bytes", card.map_or(Value::Null, whole)),
                    ("observed_bytes", whole(observed)),
                    ("ppm_of_prediction", Value::Integer(off)),
                    ("measured", Value::Bool(true)),
                ]));
            }
            Err(why) => {
                lines.push(format!("  window {window:>6}   not measured: {why}"));
                rows.push(Value::map([
                    ("window", whole(window)),
                    ("predicted_bytes", whole(predicted)),
                    ("measured", Value::Bool(false)),
                    ("why", Value::text(why)),
                ]));
            }
        }
    }
    if measured == 0 {
        return Found::could_not_tell("no window could be measured");
    }
    lines.push(
        "  observed is the engine's peak resident memory plus what the card gained while it \
         was up, where the card was read; a prediction the engine outgrows is what *fits* is \
         wrong about"
            .to_owned(),
    );
    Found {
        lines,
        fields: vec![
            ("weights_bytes", whole(weights)),
            (
                "cache_bytes_per_token",
                per_token.map_or(Value::Null, whole),
            ),
            ("windows", Value::List(rows)),
            ("measured", Value::Integer(as_integer(measured))),
            ("worst_ppm", Value::Integer(worst)),
        ],
        rows: readings,
    }
}

/// The engine's peak resident bytes and what the card gained, at one window.
fn at_window(site: &Site<'_>, window: u64) -> Result<(u64, Option<u64>), String> {
    let before = crate::engines::card_memory_used();
    let engine = site.server(&Startup {
        context: window,
        ..site.startup()
    })?;
    let prompt = filler(64);
    // A short generation touches every weight; a server that has only
    // loaded has mapped the file and read little of it.
    let _touched = engine
        .complete(
            Prompt::Identifiers(&prompt),
            8,
            Draw::greedy(0),
            true,
            site.waiting,
        )
        .map_err(|failure| failure.detail().to_owned())?;
    let resident = engine
        .peak_resident_bytes()
        .ok_or_else(|| "the kernel did not say what the engine holds".to_owned())?;
    let card = match (before, crate::engines::card_memory_used()) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    Ok((resident, card))
}
