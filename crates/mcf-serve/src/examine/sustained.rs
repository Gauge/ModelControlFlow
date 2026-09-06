//! Sustained generation: tokens a second sampled every thirty seconds
//! over five minutes, with the card's temperature, clock and power
//! beside each sample (B-530, D55, D11).
//!
//! A throughput figure is taken in the first seconds after a load, on a
//! cold card at its highest clock. A card that heats and throttles, or a
//! machine that shares the card, gives a different figure at minute
//! four. This keeps the model generating from one prompt, completion
//! after completion, and every thirty seconds writes down what it did
//! since the last sample and what the sensors said at that moment.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "sustained";

/// How long it runs, in seconds.
pub const DURATION_S: u64 = 300;

/// How often a sample is taken, in seconds.
pub const EVERY_S: u64 = 30;

/// How many tokens each completion produces, at most.
const PRODUCE: usize = 256;

/// What is generated from.
const ASK: &str = "Write a long, plain account of a walk along a river from its source to the \
sea, one paragraph after another, with no headings and no lists.";

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: completion after completion, a sample a period"
)]
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
    let sensors_at_start = crate::engines::card_sensors();
    let began = std::time::Instant::now();
    let mut last_sample = began;
    let mut since: u64 = 0;
    let mut sample = 0_usize;
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  one prompt completed over and over for {DURATION_S} s, {PRODUCE} tokens at a time, \
         greedy; a sample every {EVERY_S} s with the card's sensors beside it{}",
        if sensors_at_start.any() {
            ""
        } else {
            " — this card publishes no sensors, so those columns are empty"
        }
    )];
    let mut seed = 0_u64;
    let (mut slowest, mut fastest): (Option<u64>, Option<u64>) = (None, None);
    loop {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let completed = match engine.complete(
            Prompt::Identifiers(&ids),
            PRODUCE,
            Draw::greedy(seed),
            false,
            site.timed(),
        ) {
            Ok(completed) => completed,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        seed = seed.wrapping_add(1);
        since = since.saturating_add(u64::try_from(completed.predicted).unwrap_or(0));
        let now = std::time::Instant::now();
        let period_ns =
            u64::try_from(now.duration_since(last_sample).as_nanos()).unwrap_or(u64::MAX);
        let elapsed_s = now.duration_since(began).as_secs();
        if period_ns >= EVERY_S.saturating_mul(1_000_000_000) || elapsed_s >= DURATION_S {
            let per_second_milli = since
                .saturating_mul(1_000_000_000_000)
                .checked_div(period_ns)
                .unwrap_or(0);
            slowest = Some(slowest.map_or(per_second_milli, |held| held.min(per_second_milli)));
            fastest = Some(fastest.map_or(per_second_milli, |held| held.max(per_second_milli)));
            let sensors = crate::engines::card_sensors();
            let dims = [("sample", Value::Integer(as_integer(sample)))];
            let since_start =
                u64::try_from(now.duration_since(began).as_millis()).unwrap_or(u64::MAX);
            rows.push(Reading::new(
                &dims,
                "elapsed_ms",
                i64::try_from(since_start).unwrap_or(i64::MAX),
                "ms",
            ));
            rows.push(Reading::new(
                &dims,
                "tokens",
                i64::try_from(since).unwrap_or(i64::MAX),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "period_ns",
                i64::try_from(period_ns).unwrap_or(i64::MAX),
                "ns",
            ));
            rows.push(Reading::new(
                &dims,
                "per_second_milli",
                i64::try_from(per_second_milli).unwrap_or(i64::MAX),
                "millitokens/s",
            ));
            for (metric, held, unit) in [
                ("temperature", sensors.temperature_millic, "millicelsius"),
                ("clock", sensors.clock_hz, "hz"),
                ("power", sensors.power_uw, "microwatts"),
            ] {
                if let Some(value) = held {
                    rows.push(Reading::new(&dims, metric, value, unit));
                }
            }
            lines.push(format!(
                "  at {:>4} s   {} tokens/s{}{}{}",
                elapsed_s,
                super::milli_said(per_second_milli),
                sensors
                    .temperature_millic
                    .map_or(String::new(), |t| format!(
                        "   {} °C",
                        t.saturating_div(1000)
                    )),
                sensors.clock_hz.map_or(String::new(), |c| format!(
                    "   {} MHz",
                    c.saturating_div(1_000_000)
                )),
                sensors.power_uw.map_or(String::new(), |p| format!(
                    "   {} W",
                    p.saturating_div(1_000_000)
                )),
            ));
            sample = sample.saturating_add(1);
            since = 0;
            last_sample = now;
        }
        if elapsed_s >= DURATION_S {
            break;
        }
    }
    lines.push(format!(
        "  {sample} sample(s); slowest {} and fastest {} tokens/s",
        super::milli_said(slowest.unwrap_or(0)),
        super::milli_said(fastest.unwrap_or(0))
    ));
    Found {
        lines,
        fields: vec![
            (
                "duration_s",
                Value::Integer(as_integer(usize::try_from(DURATION_S).unwrap_or(0))),
            ),
            (
                "every_s",
                Value::Integer(as_integer(usize::try_from(EVERY_S).unwrap_or(0))),
            ),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("samples", Value::Integer(as_integer(sample))),
            (
                "slowest_milli",
                slowest.map_or(Value::Null, |held| {
                    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
                }),
            ),
            (
                "fastest_milli",
                fastest.map_or(Value::Null, |held| {
                    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
                }),
            ),
            ("sensors", Value::Bool(sensors_at_start.any())),
        ],
        rows,
    }
}
