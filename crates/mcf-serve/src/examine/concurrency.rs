//! Concurrency: how a server's throughput scales with simultaneous
//! requests (B-500, D52).
//!
//! A hosted server answers more than one caller, and what each of them
//! gets is a figure nobody had: the aggregate rises with the count while
//! each request's own rate falls, and where those cross is a fact about
//! this model on this machine. One server with eight slots, and one,
//! two, four and eight pinned generations at once.

use mcf_record::json::Value;

use super::{
    Found, Reading, Site, as_integer, as_ms, filler, median, per_second, ppm, timed, whole,
};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "concurrency";

/// How many slots the server is started with.
const SLOTS: u32 = 8;

/// The counts of simultaneous requests tried.
const COUNTS: [usize; 4] = [1, 2, 4, 8];

/// How deep each request's prompt is.
const DEPTH: usize = 128;

/// How many tokens each request produces, pinned.
const PRODUCE: usize = 32;

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        parallel: Some(SLOTS),
        context: u64::from(SLOTS) * 1024,
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let prompt = filler(DEPTH);
    let ask = || {
        engine.complete(
            Prompt::Identifiers(&prompt),
            PRODUCE,
            Draw::greedy(0),
            true,
            site.timed(),
        )
    };
    if let Err(failure) = ask() {
        return Found::could_not_tell(failure.detail());
    }
    let mut lines = vec![format!(
        "  {SLOTS} slots; each request a prompt of {DEPTH} identifiers and {PRODUCE} tokens pinned"
    )];
    let mut rows = Vec::new();
    let mut readings = Vec::new();
    let mut at_one: Option<u64> = None;
    let mut at_eight: Option<u64> = None;
    for (at, count) in COUNTS.into_iter().enumerate() {
        site.progress(at, COUNTS.len(), &format!("{count} at once"));
        if site.asker_gone() {
            break;
        }
        let (mut own, refused, wall) = at_once(count, &ask);
        for (request, ns) in own.iter().enumerate() {
            readings.push(Reading::new(
                &[
                    ("count", Value::Integer(as_integer(count))),
                    ("request", Value::Integer(as_integer(request))),
                ],
                "request_ns",
                i64::try_from(*ns).unwrap_or(i64::MAX),
                "ns",
            ));
        }
        readings.push(Reading::new(
            &[("count", Value::Integer(as_integer(count)))],
            "wall_ns",
            i64::try_from(wall).unwrap_or(i64::MAX),
            "ns",
        ));
        let Some(middle) = median(&mut own) else {
            lines.push(format!(
                "  {count} at once   not measured: {}",
                refused.unwrap_or_else(|| "no request finished".to_owned())
            ));
            continue;
        };
        let aggregate = per_second((count * PRODUCE) as u64, wall);
        let each_rate = per_second(PRODUCE as u64, middle);
        if count == 1 {
            at_one = Some(aggregate);
        }
        if count == SLOTS as usize {
            at_eight = Some(aggregate);
        }
        lines.push(format!(
            "  {count} at once   {aggregate:>6} tokens a second in all   {each_rate:>6} each   \
             {:>8} ms the median request{}",
            as_ms(middle),
            refused.map_or_else(String::new, |why| format!("   ({why})"))
        ));
        rows.push(Value::map([
            ("count", Value::Integer(as_integer(count))),
            ("wall_ns", whole(wall)),
            ("median_request_ns", whole(middle)),
            ("aggregate_tokens_per_second", whole(aggregate)),
            ("each_tokens_per_second", whole(each_rate)),
        ]));
    }
    if rows.is_empty() {
        return Found::could_not_tell("no count could be measured");
    }
    let scaling = match (at_one, at_eight) {
        (Some(one), Some(eight)) => ppm(eight, one),
        _ => 0,
    };
    if scaling > 0 {
        lines.push(format!(
            "  eight at once give {} of the rate at one, in all",
            super::per_cent(scaling)
        ));
    }
    Found {
        lines,
        fields: vec![
            ("slots", Value::Integer(i64::from(SLOTS))),
            ("depth", Value::Integer(as_integer(DEPTH))),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("counts", Value::List(rows)),
            ("scaling_ppm", Value::Integer(scaling)),
        ],
        rows: readings,
    }
}

/// `count` requests at once: each one's own time where it produced what
/// it was pinned to, why one did not, and the wall time of them all.
fn at_once(
    count: usize,
    ask: &(dyn Fn() -> Result<crate::served::Completed, mcf_core::Failure> + Sync),
) -> (Vec<u64>, Option<String>, u64) {
    let (each, wall) = timed(|| {
        std::thread::scope(|scope| {
            let hands: Vec<_> = (0..count).map(|_| scope.spawn(|| timed(ask))).collect();
            hands
                .into_iter()
                .map(|hand| hand.join().unwrap_or_else(|_| (Err(joined()), 0)))
                .collect::<Vec<_>>()
        })
    });
    let mut own: Vec<u64> = Vec::new();
    let mut refused: Option<String> = None;
    for (done, ns) in each {
        match done {
            Ok(completed) if completed.predicted == PRODUCE => own.push(ns),
            Ok(completed) => {
                refused = Some(format!(
                    "a request produced {} of {PRODUCE} tokens",
                    completed.predicted
                ));
            }
            Err(failure) => refused = Some(failure.detail().to_owned()),
        }
    }
    (own, refused, wall)
}

fn joined() -> mcf_core::Failure {
    mcf_core::Failure::new(
        mcf_core::failure::Category::EngineExitMidstream,
        mcf_core::failure::Attribution::Machine,
        mcf_core::failure::Disposition::Aborted,
        mcf_core::failure::Subsystem::new("mcf-serve::examine"),
        "a request's thread ended without answering",
    )
}
