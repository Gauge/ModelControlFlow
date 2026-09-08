use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, filler, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "energy";

const PRODUCE: usize = 384;

const READ: usize = 2048;

const IDLE_MS: u64 = 2000;

const EVERY_MS: u64 = 20;

const ASK: &str = "Describe, at length and in plain prose, how a small town's market day goes \
from the first stall set up before dawn to the last one packed away.";

type Sample = (std::time::Instant, i64);

fn sampling() -> (Arc<AtomicBool>, std::thread::JoinHandle<Vec<Sample>>) {
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&stop);
    let handle = std::thread::spawn(move || {
        let mut samples = Vec::new();
        while !seen.load(Ordering::Relaxed) {
            if let Some(power) = crate::engines::card_sensors().power_uw {
                samples.push((std::time::Instant::now(), power));
            }
            std::thread::sleep(std::time::Duration::from_millis(EVERY_MS));
        }
        samples
    });
    (stop, handle)
}

#[must_use]
pub fn integrated(samples: &[Sample]) -> (u64, u64) {
    let mut microjoules: u128 = 0;
    for pair in samples.windows(2) {
        let [(before, low), (after, high)] = pair else {
            continue;
        };
        let dt_ns = after.duration_since(*before).as_nanos();
        let ends = u128::try_from(low.saturating_add(*high).max(0)).unwrap_or(0);
        microjoules = microjoules.saturating_add(
            ends.saturating_mul(dt_ns)
                .saturating_div(2)
                .saturating_div(1_000_000_000),
        );
    }
    let span = match (samples.first(), samples.last()) {
        (Some((first, _)), Some((last, _))) => {
            u64::try_from(last.duration_since(*first).as_nanos()).unwrap_or(u64::MAX)
        }
        _ => 0,
    };
    (u64::try_from(microjoules).unwrap_or(u64::MAX), span)
}

fn under_sampler<T>(work: impl FnOnce() -> T) -> (T, u64, u64, usize) {
    let (stop, handle) = sampling();
    let out = work();
    stop.store(true, Ordering::Relaxed);
    let samples = handle.join().unwrap_or_default();
    let (microjoules, span) = integrated(&samples);
    (out, microjoules, span, samples.len())
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: idle, a read, a generation, each a set of rows"
)]
pub fn measure(site: &Site<'_>) -> Found {
    if crate::engines::card_sensors().power_uw.is_none() {
        return Found::could_not_tell(
            "this card publishes no power reading, so no energy can be measured here",
        );
    }
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let ((), idle_energy, idle_ns, idle_samples) = under_sampler(|| {
        std::thread::sleep(std::time::Duration::from_millis(IDLE_MS));
    });
    let idle_uw = idle_energy
        .saturating_mul(1_000_000_000)
        .checked_div(idle_ns)
        .unwrap_or(0);
    let mut rows = vec![
        Reading::new(
            &[("phase", Value::text("idle"))],
            "power",
            i64::try_from(idle_uw).unwrap_or(i64::MAX),
            "microwatts",
        ),
        Reading::new(
            &[("phase", Value::text("idle"))],
            "samples",
            as_integer(idle_samples),
            "count",
        ),
    ];
    if site.asker_gone() {
        return Found::could_not_tell(crate::served::CLIENT_LEFT);
    }
    let read_ids = filler(READ);
    let (read, read_uj, read_ns, read_samples) = under_sampler(|| {
        engine.complete(
            Prompt::Identifiers(&read_ids),
            1,
            Draw::greedy(0),
            false,
            site.timed(),
        )
    });
    if let Err(failure) = read {
        return Found::could_not_tell(failure.detail());
    }
    let read_tokens = u64::try_from(read_ids.len()).unwrap_or(1).max(1);
    if site.asker_gone() {
        return Found::could_not_tell(crate::served::CLIENT_LEFT);
    }
    let ids = match framed_ids(&engine, ASK) {
        Ok(ids) => ids,
        Err(why) => return Found::could_not_tell(&why),
    };
    let (made, made_uj, made_ns, made_samples) = under_sampler(|| {
        engine.complete(
            Prompt::Identifiers(&ids),
            PRODUCE,
            Draw::greedy(0),
            false,
            site.timed(),
        )
    });
    let made = match made {
        Ok(completed) => completed,
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let made_tokens = u64::try_from(made.predicted).unwrap_or(0).max(1);
    let above_idle = |uj: u64, ns: u64| {
        uj.saturating_sub(
            idle_uw
                .saturating_mul(ns)
                .checked_div(1_000_000_000)
                .unwrap_or(0),
        )
    };
    let mut lines = vec![format!(
        "  the card's power read every {EVERY_MS} ms and summed over the time: idle for \
         {IDLE_MS} ms, a {read_tokens}-token prompt read, a {PRODUCE}-token generation"
    )];
    lines.push(format!("  idle          {} W", watts(idle_uw)));
    for (phase, uj, ns, samples, tokens, produced) in [
        ("read", read_uj, read_ns, read_samples, read_tokens, 1_u64),
        (
            "generate",
            made_uj,
            made_ns,
            made_samples,
            made_tokens,
            made_tokens,
        ),
    ] {
        let dims = [("phase", Value::text(phase))];
        let per_token = uj.checked_div(tokens).unwrap_or(0);
        let above = above_idle(uj, ns).checked_div(tokens).unwrap_or(0);
        let mean_uw = uj
            .saturating_mul(1_000_000_000)
            .checked_div(ns)
            .unwrap_or(0);
        rows.push(Reading::new(
            &dims,
            "energy",
            i64::try_from(uj).unwrap_or(i64::MAX),
            "microjoules",
        ));
        rows.push(Reading::new(
            &dims,
            "ns",
            i64::try_from(ns).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(
            &dims,
            "tokens",
            i64::try_from(tokens).unwrap_or(i64::MAX),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "produced",
            i64::try_from(produced).unwrap_or(i64::MAX),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "per_token",
            i64::try_from(per_token).unwrap_or(i64::MAX),
            "microjoules",
        ));
        rows.push(Reading::new(
            &dims,
            "per_token_above_idle",
            i64::try_from(above).unwrap_or(i64::MAX),
            "microjoules",
        ));
        rows.push(Reading::new(&dims, "samples", as_integer(samples), "count"));
        lines.push(format!(
            "  {phase:<12}  {} mJ a token ({} above idle) over {tokens} token(s); {} W while it \
             ran",
            millijoules(per_token),
            millijoules(above),
            watts(mean_uw)
        ));
    }
    Found {
        lines,
        fields: vec![
            (
                "idle_uw",
                Value::Integer(i64::try_from(idle_uw).unwrap_or(i64::MAX)),
            ),
            (
                "read_tokens",
                Value::Integer(i64::try_from(read_tokens).unwrap_or(i64::MAX)),
            ),
            (
                "read_uj",
                Value::Integer(i64::try_from(read_uj).unwrap_or(i64::MAX)),
            ),
            (
                "generated",
                Value::Integer(i64::try_from(made_tokens).unwrap_or(i64::MAX)),
            ),
            (
                "generate_uj",
                Value::Integer(i64::try_from(made_uj).unwrap_or(i64::MAX)),
            ),
            (
                "read_uj_per_token",
                Value::Integer(
                    i64::try_from(read_uj.checked_div(read_tokens).unwrap_or(0))
                        .unwrap_or(i64::MAX),
                ),
            ),
            (
                "generate_uj_per_token",
                Value::Integer(
                    i64::try_from(made_uj.checked_div(made_tokens).unwrap_or(0))
                        .unwrap_or(i64::MAX),
                ),
            ),
        ],
        rows,
    }
}

fn watts(uw: u64) -> String {
    super::milli_said(uw.saturating_div(1000))
}

fn millijoules(uj: u64) -> String {
    super::milli_said(uj)
}

#[cfg(test)]
mod tests {
    use super::integrated;

    #[test]
    fn the_trapezoid_sums_power_over_time() {
        let start = std::time::Instant::now();
        let at = |ms: u64| start + std::time::Duration::from_millis(ms);
        let (uj, span) = integrated(&[(at(0), 100_000_000), (at(1000), 100_000_000)]);
        assert_eq!(uj, 100_000_000);
        assert_eq!(span, 1_000_000_000);
        let (uj, _) = integrated(&[(at(0), 0), (at(1000), 100_000_000)]);
        assert_eq!(uj, 50_000_000);
        assert_eq!(integrated(&[]), (0, 0));
    }
}
