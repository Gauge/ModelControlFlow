//! Prefill saturation: prompt-reading tokens a second across batch sizes
//! (B-493, D52).
//!
//! **The batch was a constant, and a constant is a condition nobody
//! measured.** The engine reads a prompt so many tokens at a time; past
//! some size, reading more at once buys nothing, and below it every
//! prompt is slower than it need be. Four sizes, the same prompt, the
//! same depth, and the reading says where it stops getting faster.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, as_ms, filler, median, per_second, timed, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "prefill-saturation";

/// How deep the prompt is, in identifiers.
const DEPTH: usize = 1024;

/// The batch sizes tried.
const BATCHES: [u32; 4] = [64, 256, 1024, 2048];

/// How many timings each size takes; the median is read.
const REPEATS: usize = 3;

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let mut lines = vec![format!(
        "  a prompt of {DEPTH} identifiers read {REPEATS} times at each batch, the median kept"
    )];
    let mut rows = Vec::new();
    let mut readings = Vec::new();
    let mut rates: Vec<(u32, u64)> = Vec::new();
    for (at, batch) in BATCHES.into_iter().enumerate() {
        site.progress(at, BATCHES.len(), &format!("batch {batch}"));
        if site.asker_gone() {
            break;
        }
        match at_batch(site, batch) {
            Ok(mut samples) => {
                for (repeat, sample) in samples.iter().enumerate() {
                    readings.push(Reading::new(
                        &[
                            ("batch", Value::Integer(i64::from(batch))),
                            ("repeat", Value::Integer(as_integer(repeat))),
                        ],
                        "read_ns",
                        i64::try_from(*sample).unwrap_or(i64::MAX),
                        "ns",
                    ));
                }
                let Some(ns) = median(&mut samples) else {
                    continue;
                };
                let rate = per_second(DEPTH as u64, ns);
                lines.push(format!(
                    "  batch {batch:>5}   {:>8} ms   {rate:>8} tokens a second",
                    as_ms(ns)
                ));
                rates.push((batch, rate));
                rows.push(Value::map([
                    ("batch", Value::Integer(i64::from(batch))),
                    ("ns", whole(ns)),
                    ("tokens_per_second", whole(rate)),
                    ("measured", Value::Bool(true)),
                ]));
            }
            Err(why) => {
                lines.push(format!("  batch {batch:>5}   not measured: {why}"));
                rows.push(Value::map([
                    ("batch", Value::Integer(i64::from(batch))),
                    ("measured", Value::Bool(false)),
                    ("why", Value::text(why)),
                ]));
            }
        }
    }
    let Some(&(fastest, best)) = rates.iter().max_by_key(|(_, rate)| *rate) else {
        return Found::could_not_tell("no batch size could be measured");
    };
    // The smallest batch within a tenth of the fastest: where reading
    // more at once stops buying anything a person would notice.
    let enough = rates
        .iter()
        .find(|(_, rate)| rate.saturating_mul(10) >= best.saturating_mul(9))
        .map_or(fastest, |(batch, _)| *batch);
    lines.push(format!(
        "  fastest at batch {fastest}; batch {enough} is within a tenth of it, and reading more \
         at once past that buys under a tenth"
    ));
    Found {
        lines,
        fields: vec![
            ("depth", Value::Integer(as_integer(DEPTH))),
            ("batches", Value::List(rows)),
            ("fastest_batch", Value::Integer(i64::from(fastest))),
            ("enough_batch", Value::Integer(i64::from(enough))),
        ],
        rows: readings,
    }
}

/// Every timing of reading the prompt at one batch size, raw (D16).
fn at_batch(site: &Site<'_>, batch: u32) -> Result<Vec<u64>, String> {
    let engine = site.server(&Startup {
        batch: Some(batch),
        ubatch: Some(batch),
        context: 2048,
        projector: None,
        ..site.startup()
    })?;
    let prompt = filler(DEPTH);
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let read = || {
        engine.complete(
            Prompt::Identifiers(&prompt),
            1,
            Draw::greedy(0),
            true,
            site.timed(),
        )
    };
    let _warm = read().map_err(said)?;
    let mut samples = Vec::with_capacity(REPEATS);
    for _ in 0..REPEATS {
        let (done, ns) = timed(read);
        let _read = done.map_err(said)?;
        samples.push(ns);
    }
    if samples.is_empty() {
        return Err("no timing was taken".to_owned());
    }
    Ok(samples)
}
