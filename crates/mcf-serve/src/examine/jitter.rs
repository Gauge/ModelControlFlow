//! Streaming jitter: every gap between one piece of a long generation
//! arriving and the next, the longest stall, and where the gaps fall
//! (B-532, D55, D16).
//!
//! Tokens a second is a mean, and a mean hides a stall. A reader
//! watching text arrive feels the longest gap, not the average one.
//! This streams one long generation from the hosted server and notes
//! the moment each piece arrives; every gap is a row, and the record
//! keeps the longest, the shortest and how many gaps ran past two,
//! five and ten times the shortest.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, median};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "jitter";

/// How many tokens the generation produces, at most.
const PRODUCE: usize = 512;

/// What is generated from.
const ASK: &str = "Tell, in plain prose and at length, the story of a lighthouse keeper's year, \
season by season, with no headings and no lists.";

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: one generation, a gap a row, the stalls counted"
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
    let began = std::time::Instant::now();
    let mut arrivals: Vec<(std::time::Instant, usize)> = Vec::with_capacity(PRODUCE);
    let mut arriving = |at: usize, _piece: &str| {
        arrivals.push((std::time::Instant::now(), at));
    };
    let completed = match engine.complete_while(
        Prompt::Identifiers(&ids),
        PRODUCE,
        Draw::greedy(0),
        false,
        site.timed(),
        &mut arriving,
    ) {
        Ok(completed) => completed,
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let whole_ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
    if arrivals.len() < 2 {
        return Found::could_not_tell(
            "the server sent the answer in fewer than two pieces, so there are no gaps to time",
        );
    }
    let first_ns = arrivals.first().map_or(0, |(at, _)| {
        u64::try_from(at.duration_since(began).as_nanos()).unwrap_or(u64::MAX)
    });
    let mut gaps: Vec<u64> = arrivals
        .windows(2)
        .map(|pair| match pair {
            [(before, _), (after, _)] => {
                u64::try_from(after.duration_since(*before).as_nanos()).unwrap_or(u64::MAX)
            }
            _ => 0,
        })
        .collect();
    let mut rows = Vec::with_capacity(gaps.len().saturating_add(8));
    for (which, (gap, (_, at))) in gaps.iter().zip(arrivals.iter().skip(1)).enumerate() {
        let dims = [("gap", Value::Integer(as_integer(which)))];
        rows.push(Reading::new(
            &dims,
            "ns",
            i64::try_from(*gap).unwrap_or(i64::MAX),
            "ns",
        ));
        rows.push(Reading::new(&dims, "token", as_integer(*at), "tokens"));
    }
    let longest = gaps.iter().copied().max().unwrap_or(0);
    let shortest = gaps.iter().copied().min().unwrap_or(0);
    let longest_at = gaps
        .iter()
        .position(|gap| *gap == longest)
        .and_then(|which| arrivals.get(which.saturating_add(1)))
        .map_or(0, |(_, at)| *at);
    let typical = median(&mut gaps).unwrap_or(0);
    let past = |times: u64| {
        gaps.iter()
            .filter(|gap| **gap > typical.saturating_mul(times))
            .count()
    };
    let (past_two, past_five, past_ten) = (past(2), past(5), past(10));
    let summary = [
        ("first_ns", first_ns),
        ("whole_ns", whole_ns),
        ("longest_ns", longest),
        ("shortest_ns", shortest),
        ("typical_ns", typical),
    ];
    for (metric, value) in summary {
        rows.push(Reading::new(
            &[],
            metric,
            i64::try_from(value).unwrap_or(i64::MAX),
            "ns",
        ));
    }
    rows.push(Reading::new(&[], "gaps", as_integer(gaps.len()), "count"));
    rows.push(Reading::new(
        &[],
        "past_twice",
        as_integer(past_two),
        "count",
    ));
    rows.push(Reading::new(
        &[],
        "past_five_times",
        as_integer(past_five),
        "count",
    ));
    rows.push(Reading::new(
        &[],
        "past_ten_times",
        as_integer(past_ten),
        "count",
    ));
    rows.push(Reading::new(
        &[],
        "longest_at",
        as_integer(longest_at),
        "tokens",
    ));
    let ms = |ns: u64| super::as_ms(ns);
    Found {
        lines: vec![
            format!(
                "  one generation of {} token(s) streamed in {} piece(s); the first piece after \
                 {} ms, the whole in {} ms",
                completed.predicted,
                arrivals.len(),
                ms(first_ns),
                ms(whole_ns)
            ),
            format!(
                "  gaps: typical {} ms, shortest {} ms, longest {} ms at token {longest_at}",
                ms(typical),
                ms(shortest),
                ms(longest)
            ),
            format!(
                "  {past_two} gap(s) past twice the typical, {past_five} past five times, \
                 {past_ten} past ten times, of {}",
                gaps.len()
            ),
        ],
        fields: vec![
            ("produced", Value::Integer(as_integer(completed.predicted))),
            ("pieces", Value::Integer(as_integer(arrivals.len()))),
            (
                "first_ns",
                Value::Integer(i64::try_from(first_ns).unwrap_or(i64::MAX)),
            ),
            (
                "whole_ns",
                Value::Integer(i64::try_from(whole_ns).unwrap_or(i64::MAX)),
            ),
            (
                "typical_ns",
                Value::Integer(i64::try_from(typical).unwrap_or(i64::MAX)),
            ),
            (
                "longest_ns",
                Value::Integer(i64::try_from(longest).unwrap_or(i64::MAX)),
            ),
            ("longest_at", Value::Integer(as_integer(longest_at))),
            ("past_twice", Value::Integer(as_integer(past_two))),
            ("past_five_times", Value::Integer(as_integer(past_five))),
            ("past_ten_times", Value::Integer(as_integer(past_ten))),
        ],
        rows,
    }
}
