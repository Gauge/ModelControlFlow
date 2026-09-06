//! Load time by offload: from the server started to its first token, at
//! each share of the layers on the card (B-555, D55, B-491, D11).
//!
//! The offload curve says what each layer count runs at; nothing said
//! what each takes to become ready, and a model that answers in three
//! seconds after a forty-second load is a different tool from one ready
//! in five. Each layer count is a server started cold from the daemon's
//! point of view — the page cache is whatever it is, and the cold-start
//! measurement is the one that controls it — timed to the first token
//! of a one-token ask.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, as_ms, filler, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "load-time";

/// The shares of the layers put on the card, in hundredths.
pub const SHARES: [u32; 5] = [0, 25, 50, 75, 100];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each share started, timed, a row set each"
)]
pub fn measure(site: &Site<'_>) -> Found {
    if !site.has_card {
        return Found::could_not_tell(
            "no card is here to put layers on, so there is one load time and the cold-start \
             measurement takes it",
        );
    }
    let Some(all) = super::offload::layers_of(site.model) else {
        return Found::could_not_tell("the file's header does not say how many layers it has");
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  a server started at each share of {all} layer(s) on the card and timed to its first \
         token; the page cache as it is"
    )];
    let prompt = filler(16);
    for (at, share) in SHARES.into_iter().enumerate() {
        site.progress(at, SHARES.len(), &format!("{share}% of the layers"));
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        #[expect(clippy::integer_division, reason = "a share of the layers, floored")]
        let layers = if share == 100 {
            // Every layer, and the output past the blocks, as the offload
            // curve counts it.
            all.saturating_add(1)
        } else {
            all.saturating_mul(share) / 100
        };
        let (started, start_ns) = timed(|| {
            site.server(&Startup {
                gpu_layers: layers,
                projector: None,
                ..site.startup()
            })
        });
        let engine = match started {
            Ok(engine) => engine,
            Err(why) => {
                lines.push(format!(
                    "  {share:>3}% ({layers:>3} layer(s))   not started: {why}"
                ));
                continue;
            }
        };
        let (done, first_ns) = timed(|| {
            engine.complete(
                Prompt::Identifiers(&prompt),
                1,
                Draw::greedy(0),
                false,
                site.timed(),
            )
        });
        if let Err(failure) = done {
            lines.push(format!(
                "  {share:>3}% ({layers:>3} layer(s))   started in {} ms, no first token: {}",
                as_ms(start_ns),
                failure.detail()
            ));
            continue;
        }
        let dims = [
            ("share", Value::Integer(i64::from(share))),
            ("layers", Value::Integer(i64::from(layers))),
        ];
        rows.push(Reading::new(
            &dims,
            "start_ns",
            i64::try_from(start_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "first_token_ns",
            i64::try_from(first_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "ready_ns",
            i64::try_from(start_ns.saturating_add(first_ns)).unwrap_or(i64::MAX),
            "ns",
        ));
        if let Some(bytes) = engine.resident_bytes() {
            rows.push(Reading::new(
                &dims,
                "resident",
                i64::try_from(bytes).unwrap_or(i64::MAX),
                "bytes",
            ));
        }
        if let Some(bytes) = crate::engines::card_memory_used() {
            rows.push(Reading::new(
                &dims,
                "card_used",
                i64::try_from(bytes).unwrap_or(i64::MAX),
                "bytes",
            ));
        }
        lines.push(format!(
            "  {share:>3}% ({layers:>3} layer(s))   started in {} ms, first token {} ms after: \
             ready in {} ms",
            as_ms(start_ns),
            as_ms(first_ns),
            as_ms(start_ns.saturating_add(first_ns))
        ));
        drop(engine);
    }
    if rows.is_empty() {
        return Found::could_not_tell("no share of the layers started");
    }
    let (fastest, slowest) = rows
        .iter()
        .filter(|row| row.metric == "ready_ns")
        .map(|row| row.value)
        .fold((i64::MAX, 0), |(low, high), value| {
            (low.min(value), high.max(value))
        });
    Found {
        lines,
        fields: vec![
            ("layers", Value::Integer(i64::from(all))),
            ("shares", Value::Integer(as_integer(SHARES.len()))),
            ("fastest_ready_ns", Value::Integer(fastest)),
            ("slowest_ready_ns", Value::Integer(slowest)),
        ],
        rows,
    }
}
