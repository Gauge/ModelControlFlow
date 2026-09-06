//! Soak: two hundred requests through one server, one after another —
//! which failed, what each took, and the server's resident bytes at the
//! start and the end (B-535, D55, B-232).
//!
//! A server that answers once is not a server that answers all evening.
//! Two hundred short requests, each a row with its own clock, show a
//! server that slows, a request that fails, or memory that grows from
//! the first request to the last; and none of that shows in a figure
//! taken from one request.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "soak";

/// How many requests.
pub const REQUESTS: usize = 200;

/// How many tokens each produces, at most.
const PRODUCE: usize = 32;

/// The prompts cycled through, so the cache is not one prompt's.
const ASKS: [&str; 4] = [
    "Name a colour and say one thing about it.",
    "Give a short sentence about the weather.",
    "Say what a clock is for, briefly.",
    "Describe a chair in one sentence.",
];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: every request a row, the resident bytes at each end"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut prompts = Vec::with_capacity(ASKS.len());
    for ask in ASKS {
        match framed_ids(&engine, ask) {
            Ok(ids) => prompts.push(ids),
            Err(why) => return Found::could_not_tell(&why),
        }
    }
    let resident_start = engine.resident_bytes();
    let mut rows = Vec::with_capacity(REQUESTS.saturating_mul(3).saturating_add(4));
    let (mut failed, mut produced_all) = (0_usize, 0_u64);
    let mut by_hundred: Vec<Vec<u64>> = Vec::new();
    let mut slowest: Option<(u64, usize)> = None;
    for request in 0..REQUESTS {
        if request % 10 == 0 {
            site.progress(request, REQUESTS, "request");
        }
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let Some(ids) = prompts.get(request % ASKS.len()) else {
            break;
        };
        let (done, ns) = timed(|| {
            engine.complete(
                Prompt::Identifiers(ids),
                PRODUCE,
                Draw::greedy(u64::try_from(request).unwrap_or(0)),
                false,
                site.timed(),
            )
        });
        let dims = [("request", Value::Integer(as_integer(request)))];
        rows.push(Reading::new(
            &dims,
            "ns",
            i64::try_from(ns).unwrap_or(i64::MAX),
            "ns",
        ));
        #[expect(clippy::integer_division, reason = "which hundred a request falls in")]
        let hundred = request / 100;
        while by_hundred.len() <= hundred {
            by_hundred.push(Vec::new());
        }
        if let Some(held) = by_hundred.get_mut(hundred) {
            held.push(ns);
        }
        if slowest.is_none_or(|(held, _)| ns > held) {
            slowest = Some((ns, request));
        }
        if let Ok(completed) = done {
            produced_all =
                produced_all.saturating_add(u64::try_from(completed.predicted).unwrap_or(0));
            rows.push(Reading::new(&dims, "ok", 1, "bool"));
            rows.push(Reading::new(
                &dims,
                "produced",
                as_integer(completed.predicted),
                "tokens",
            ));
        } else {
            failed = failed.saturating_add(1);
            rows.push(Reading::new(&dims, "ok", 0, "bool"));
        }
    }
    let resident_end = engine.resident_bytes();
    for (when, held) in [("start", resident_start), ("end", resident_end)] {
        if let Some(bytes) = held {
            rows.push(Reading::new(
                &[("when", Value::text(when))],
                "resident",
                i64::try_from(bytes).unwrap_or(i64::MAX),
                "bytes",
            ));
        }
    }
    let mut lines = vec![format!(
        "  {REQUESTS} request(s) one after another through one server, {} prompt(s) in turn, \
         up to {PRODUCE} tokens each, greedy",
        ASKS.len()
    )];
    for (which, held) in by_hundred.iter_mut().enumerate() {
        let (low, high) = (
            held.iter().copied().min().unwrap_or(0),
            held.iter().copied().max().unwrap_or(0),
        );
        let typical = super::median(held).unwrap_or(0);
        lines.push(format!(
            "  hundred {}   typical {} ms   fastest {} ms   slowest {} ms",
            which.saturating_add(1),
            super::as_ms(typical),
            super::as_ms(low),
            super::as_ms(high)
        ));
    }
    lines.push(format!(
        "  {failed} failed; {produced_all} token(s) produced in all{}",
        slowest.map_or(String::new(), |(ns, at)| format!(
            "; the slowest was request {at} at {} ms",
            super::as_ms(ns)
        ))
    ));
    lines.push(match (resident_start, resident_end) {
        (Some(start), Some(end)) => format!(
            "  resident {} at the start, {} at the end",
            super::gigabytes(start),
            super::gigabytes(end)
        ),
        _ => "  resident bytes could not be read".to_owned(),
    });
    Found {
        lines,
        fields: vec![
            ("requests", Value::Integer(as_integer(REQUESTS))),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("failed", Value::Integer(as_integer(failed))),
            (
                "produced",
                Value::Integer(i64::try_from(produced_all).unwrap_or(i64::MAX)),
            ),
            (
                "slowest_ns",
                slowest.map_or(Value::Null, |(ns, _)| {
                    Value::Integer(i64::try_from(ns).unwrap_or(i64::MAX))
                }),
            ),
            (
                "slowest_at",
                slowest.map_or(Value::Null, |(_, at)| Value::Integer(as_integer(at))),
            ),
            (
                "resident_start",
                resident_start.map_or(Value::Null, |bytes| {
                    Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                }),
            ),
            (
                "resident_end",
                resident_end.map_or(Value::Null, |bytes| {
                    Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                }),
            ),
        ],
        rows,
    }
}
