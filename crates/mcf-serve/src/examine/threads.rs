use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, per_second, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "threads";

const PRODUCE: usize = 64;

const ASK: &str = "Describe a quiet street in the early morning, in plain prose.";

#[must_use]
pub fn counts() -> Vec<u32> {
    let all = std::thread::available_parallelism()
        .map_or(1, |held| u32::try_from(held.get()).unwrap_or(1));
    let mut out: Vec<u32> = [1, 2, 4, 8, 16, 32]
        .into_iter()
        .filter(|count| *count < all)
        .collect();
    out.push(all);
    out
}

#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let mut rows = Vec::new();
    let counts = counts();
    let mut lines = vec![format!(
        "  {PRODUCE} token(s) generated on the processor at each of {} thread count(s), greedy \
         from one prompt",
        counts.len()
    )];
    let mut fastest: Option<(u32, u64)> = None;
    for (at, threads) in counts.iter().enumerate() {
        site.progress(at, counts.len(), &format!("{threads} thread(s)"));
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let engine = match site.server(&Startup {
            gpu_layers: 0,
            threads: Some(*threads),
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
        let milli = produced
            .saturating_mul(1_000_000_000_000)
            .checked_div(ns)
            .unwrap_or(0);
        if fastest.is_none_or(|(_, held)| milli > held) {
            fastest = Some((*threads, milli));
        }
        let dims = [("threads", Value::Integer(i64::from(*threads)))];
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
        rows.push(Reading::new(
            &dims,
            "per_second_milli",
            i64::try_from(milli).unwrap_or(i64::MAX),
            "millitokens/s",
        ));
        lines.push(format!(
            "  {threads:>3} thread(s)   {} tokens/s",
            super::milli_said(milli)
        ));
    }
    let (best_threads, best_milli) = fastest.unwrap_or((0, 0));
    lines.push(format!(
        "  fastest at {best_threads} thread(s), {} tokens/s",
        super::milli_said(best_milli)
    ));
    Found {
        lines,
        fields: vec![
            ("counts", Value::Integer(as_integer(counts.len()))),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("fastest_threads", Value::Integer(i64::from(best_threads))),
            (
                "fastest_per_second_milli",
                Value::Integer(i64::try_from(best_milli).unwrap_or(i64::MAX)),
            ),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_counts_end_with_all_the_machine_has() {
        let counts = super::counts();
        assert!(!counts.is_empty());
        assert!(counts.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
