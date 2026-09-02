//! What a ladder of readings says beyond its rungs.
//!
//! **Two measurements were in the run and not in the answer.** Each rung of
//! the ladder is two timed generations — one token and seventeen — and the
//! per-token cost is their difference. The shorter of the pair is a
//! measurement in its own right: the time to a first token, which is the
//! model loading, the prompt being read, and one token produced. Across the
//! rungs the load is the same and the prompt is not, so the first-token time
//! at the deepest rung less the shallowest, over the tokens between, is what
//! reading a token of prompt costs. Both were taken on every run and thrown
//! away, while the screen listed *prompt reading speed* and *start-up* as
//! tests that never ran (A7, §3.15).
//!
//! **Both runs of a pair are pinned, and the pin is proven.** The difference
//! is divided by sixteen because sixteen tokens separate the two runs, which
//! is only so if the engine produced one and seventeen — a model whose end of
//! text came fifth would have produced five, and a cost read off that pair
//! would be a cost of nothing in particular (F117). So the request tells the
//! engine to run past its end of text, and the daemon reads the count back
//! from the account rather than assuming it: a pair that did not produce what
//! it pinned is not a sample, and a rung with none says so (B-396, A21).
//!
//! **Derived here, once** — in the daemon, from the readings it took — so that
//! the command line, the window and the record hold one figure, not three
//! arithmetics that agree today (B-072).
//!
//! **The start-up figure is warm, and says so.** The daemon loads the model
//! for every request (DEC-018), but the file is in the page cache after the
//! first, and nothing here evicts it; the figure is the time to a first
//! token with the file already in memory, which is what a second request
//! costs and not what the first one after a reboot does. A cold disk is a
//! different measurement, not taken here.
//!
//! **The memory a rung costs is read off the engine's process** (B-424). The
//! kernel keeps each process's high-water mark of resident memory, and the
//! engine that served a rung is asked for it after the rung. The window an
//! engine runs in is sized for the request, and the cache is allocated for
//! the window, so between two rungs that ran in different windows the growth
//! in the high-water mark over the tokens between is what a token of window
//! costs — measured, against the figure the header plans. What the two say a
//! machine's free memory can hold is then the same arithmetic the planner
//! does, on the measured figure instead of the declared one (A20, A21). What
//! is not observed is said: the mark is the process's own memory, and a
//! device's memory is not in it.
//!
//! **The fall-off is read off the rungs and set against the header** (B-400).
//! The cost a token at the deepest rung less the shallowest, over the tokens
//! between, is what every token of depth adds to a token; the header's cache a
//! token of depth over that slope is the rate the engine re-read the cache
//! at. A header that describes a fixed state instead of a growing cache is
//! said to be outside that arithmetic rather than predicted wrongly.
//!
//! **And predicted from the machine** (B-427). Before the engine is up the
//! daemon reads a working set the size the cache takes at each rung, with
//! every core, and the rate at the deepest rung's size is the one the
//! prediction divides the header's bytes by — the deepest is the rung whose
//! cache is furthest from fitting a faster level of the memory (F121). The
//! predicted slope stands beside the measured one with what the engine
//! reached of it, and each attending block's read a position is set against
//! the 2 KiB from which a read streams rather than waits, which is where the
//! corpus divided into a regime the header predicts and one it does not.
//! A prediction is a prediction (A20): it is never in the figure's place.
//!
//! **The slope sits in a bracket the rungs' spread draws** (F155). Each rung
//! says how far its samples spread, and the slope read between the two
//! extremes each way is where the measured slope could be. The method
//! measures sixteen tokens of generation against a prefill that grows with
//! depth, and at depth the prefill's jitter is larger than the generation,
//! so a deep rung can spread wider than the slope it is read off — then no
//! slope is resolved, and the line says so rather than a figure (A9, A7).
//!
//! **The sentences travel with the figures.** What a surface prints of each
//! figure is composed here too, so that the window's row and the console's
//! line cannot drift into two readings of one object (B-072).

use mcf_record::json::Value;
use mcf_standin::anatomy::grouped;

/// A count, as the wire carries one.
fn count(held: u64) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

/// A reading's depth and first-token time, where it measured one.
fn first_token_of(reading: &Value) -> Option<(u64, u64)> {
    if reading.get("measured") != Some(&Value::Bool(true)) {
        return None;
    }
    let depth = reading.get("depth")?.as_integer()?;
    let ns = reading.get("first_token_ns")?.as_integer()?;
    Some((u64::try_from(depth).ok()?, u64::try_from(ns).ok()?))
}

/// The nanoseconds a token of prompt costs to read, from the first-token
/// times at the shallowest and deepest measured rungs.
///
/// Not measured — and said so — where fewer than two rungs measured, or
/// where the deeper rung's first token came no later than the shallower's,
/// which is a difference that is not a cost (A7, A9).
#[must_use]
pub fn prompt_reading(readings: &[Value], as_milliseconds: fn(u64) -> String) -> Value {
    let rungs: Vec<(u64, u64)> = readings.iter().filter_map(first_token_of).collect();
    let (Some(shallow), Some(deep)) = (rungs.first(), rungs.last()) else {
        return Value::map([
            ("measured", Value::Bool(false)),
            (
                "why",
                Value::text(
                    "no rung measured a first token, so there is nothing to read a prompt cost off",
                ),
            ),
        ]);
    };
    if rungs.len() < 2 || deep.0 <= shallow.0 {
        return Value::map([
            ("measured", Value::Bool(false)),
            (
                "why",
                Value::text(
                    "one rung measured, and a prompt cost is read between two: the deeper first \
                     token less the shallower, over the tokens between",
                ),
            ),
        ]);
    }
    if deep.1 <= shallow.1 {
        return Value::map([
            ("measured", Value::Bool(false)),
            (
                "why",
                Value::text(format!(
                    "the first token at {} deep came no later than at {} deep, so their \
                     difference is not a cost",
                    deep.0, shallow.0
                )),
            ),
        ]);
    }
    #[expect(
        clippy::integer_division,
        reason = "a difference in nanoseconds over the tokens between two rungs; the \
                  remainder is under a nanosecond a token"
    )]
    let per_token = (deep.1 - shallow.1) / (deep.0 - shallow.0);
    Value::map([
        ("measured", Value::Bool(true)),
        ("ns_per_token", count(per_token)),
        ("ms_per_token", Value::text(as_milliseconds(per_token))),
        (
            "between",
            Value::List(vec![count(shallow.0), count(deep.0)]),
        ),
        (
            "method",
            Value::text(
                "the time to a first token at the deepest rung less the shallowest, over the \
                 tokens between — the load is in both and cancels",
            ),
        ),
    ])
}

/// The time to a first token at the shallowest measured rung: loading the
/// model, reading that many tokens, and producing one.
#[must_use]
pub fn first_token(readings: &[Value], as_milliseconds: fn(u64) -> String) -> Value {
    let Some((depth, ns)) = readings.iter().find_map(first_token_of) else {
        return Value::map([
            ("measured", Value::Bool(false)),
            ("why", Value::text("no rung measured a first token")),
        ]);
    };
    Value::map([
        ("measured", Value::Bool(true)),
        ("depth", count(depth)),
        ("ns", count(ns)),
        ("ms", Value::text(as_milliseconds(ns))),
        (
            "includes",
            Value::text(format!(
                "loading the model, reading {depth} tokens and producing one — with the file \
                 already in the page cache, which is a second request's cost and not the \
                 first's after a reboot"
            )),
        ),
    ])
}

/// What the run knew about memory before it ran: the header's arithmetic
/// and the machine, for the measured figure to be set against.
#[derive(Debug, Clone, Default)]
pub struct Planned {
    /// The cache one token costs by the header's widths, where it names them.
    pub per_token: Option<u64>,
    /// The weights, whole.
    pub weights: Option<u64>,
    /// The memory free on the machine when the run started.
    pub free: Option<u64>,
    /// The context the model was trained for, where the header says.
    pub trained: Option<u64>,
    /// A sliding window the header declares, within which some blocks stop
    /// reading further back — so the cache a token of depth re-reads is at
    /// most the header's figure past it.
    pub sliding_window: Option<u64>,
    /// How many blocks keep the cache, which is how many separate reads a
    /// position is spread over.
    pub attending_blocks: Option<u64>,
    /// What this machine read at, measured at the working set the cache
    /// takes at each depth, before the engine was up (B-427).
    pub bandwidth: Vec<crate::bandwidth::Reading>,
}

/// A reading's depth, the window it ran in and the engine's peak resident
/// memory, where it measured and the engine's process was observed.
fn peak_of(reading: &Value) -> Option<(u64, u64, u64)> {
    if reading.get("measured") != Some(&Value::Bool(true)) {
        return None;
    }
    let held = |key: &str| {
        reading
            .get(key)?
            .as_integer()
            .and_then(|value| u64::try_from(value).ok())
    };
    Some((
        held("depth")?,
        held("window")?,
        held("peak_resident_bytes")?,
    ))
}

/// A figure that was not taken, and why (A7).
fn unmeasured(why: String) -> Value {
    Value::map([("measured", Value::Bool(false)), ("why", Value::text(why))])
}

/// The memory a token of window costs, read off the engine's peak resident
/// memory between the shallowest and deepest rungs that ran in different
/// windows — and the largest context the machine's free memory holds at
/// that cost, by the planner's own arithmetic.
///
/// Not measured, and said so, where no rung's engine was observed, where
/// every rung ran in one window (the cache is sized to the window, so it did
/// not grow between them), or where the deeper rung held no more than the
/// shallower (A7, A9).
#[must_use]
pub fn memory(readings: &[Value], planned: &Planned) -> Value {
    let rungs: Vec<(u64, u64, u64)> = readings.iter().filter_map(peak_of).collect();
    let (Some(shallow), Some(deep)) = (rungs.first(), rungs.last()) else {
        return unmeasured(
            "no rung's engine was observed, so there is no peak to read a cost off".to_owned(),
        );
    };
    if deep.1 <= shallow.1 {
        return unmeasured(format!(
            "every rung ran in one window of {} tokens, and the cache is sized to the window, so \
             it did not grow between them; a rung deeper than half the window would",
            grouped(shallow.1)
        ));
    }
    if deep.2 <= shallow.2 {
        return unmeasured(format!(
            "the engine held no more at a window of {} than at {}, so their difference is not a \
             cost",
            grouped(deep.1),
            grouped(shallow.1)
        ));
    }
    #[expect(
        clippy::integer_division,
        reason = "a difference in bytes over the tokens between two windows; the remainder is \
                  under a byte a token"
    )]
    let per_token = (deep.2 - shallow.2) / (deep.1 - shallow.1);
    let largest = match (planned.weights, planned.free, planned.trained) {
        (Some(weights), Some(free), Some(trained)) => Value::map([
            (
                "measured",
                count(crate::engines::largest_context(
                    weights, per_token, free, trained,
                )),
            ),
            (
                "planned",
                planned.per_token.map_or(Value::Null, |per_token| {
                    count(crate::engines::largest_context(
                        weights, per_token, free, trained,
                    ))
                }),
            ),
            ("weights", count(weights)),
            ("free", count(free)),
            ("trained", count(trained)),
        ]),
        _ => Value::map([
            ("measured", Value::Null),
            ("planned", Value::Null),
            (
                "why",
                Value::text(
                    "the weights, the free memory or the trained context is not known, and the \
                     ceiling is arithmetic on all three",
                ),
            ),
        ]),
    };
    Value::map([
        ("measured", Value::Bool(true)),
        ("per_token_bytes", count(per_token)),
        (
            "between_windows",
            Value::List(vec![count(shallow.1), count(deep.1)]),
        ),
        (
            "at_deepest",
            Value::map([
                ("depth", count(deep.0)),
                ("window", count(deep.1)),
                ("bytes", count(deep.2)),
            ]),
        ),
        (
            "planned_per_token_bytes",
            planned.per_token.map_or(Value::Null, count),
        ),
        ("largest_context", largest),
        (
            "what",
            Value::text(
                "the engine process's peak resident memory, read from the kernel's high-water \
                 mark, over the window each rung ran in; a device's memory is not in it",
            ),
        ),
    ])
}

/// The bracket line, where the rungs said their spread: what the slope sits
/// between, or that the spread reaches it and no slope is resolved.
fn bracket_said(held: &Value) -> Option<String> {
    let bracket = held
        .get("bracket")
        .filter(|value| !matches!(value, Value::Null))?;
    let (low, high) = (
        text(bracket, "low_ms_per_token_per_thousand_deep"),
        text(bracket, "high_ms_per_token_per_thousand_deep"),
    );
    Some(if bracket.get("resolved") == Some(&Value::Bool(true)) {
        format!("between {low} and {high} ms a thousand by the spread at the two rungs")
    } else {
        format!(
            "not resolved: the spread at the two rungs reaches the slope, which could be anything \
             up to {high} ms a thousand — sixteen tokens of generation are measured against a \
             prefill that grows with depth, and at depth the prefill's jitter is the larger (F155)"
        )
    })
}

/// Where the slope sits by the spread at the rungs it was read between.
#[derive(Debug, Clone, Copy)]
struct Bracket {
    /// The shallowest slope the rungs' spread allows, in nanoseconds a token
    /// of depth — zero where the spread reaches the slope.
    low: u64,
    /// The steepest.
    high: u64,
    /// Whether the predicted slope sits inside the bracket, where there is
    /// a prediction.
    prediction_within: Option<bool>,
}

impl Bracket {
    /// The bracket on the wire: both ends as nanoseconds a token of depth
    /// and as milliseconds a thousand deep, whether a slope is resolved at
    /// all, and where the prediction sits.
    fn as_value(self, as_milliseconds: fn(u64) -> String) -> Value {
        let count = |ns: u64| Value::Integer(i64::try_from(ns).unwrap_or(i64::MAX));
        let a_thousand = |ns: u64| Value::text(as_milliseconds(ns.saturating_mul(1_000)));
        Value::map([
            ("low_ns_per_token_of_depth", count(self.low)),
            ("high_ns_per_token_of_depth", count(self.high)),
            ("low_ms_per_token_per_thousand_deep", a_thousand(self.low)),
            ("high_ms_per_token_per_thousand_deep", a_thousand(self.high)),
            ("resolved", Value::Bool(self.low > 0)),
            (
                "prediction_within",
                self.prediction_within.map_or(Value::Null, Value::Bool),
            ),
        ])
    }
}

/// The bracket the slope sits in: each rung's median is taken to sit
/// within half its spread either way, and the slope between the two
/// extremes is read the way the slope itself is. Where the bracket reaches
/// zero the run has not resolved a slope at all — the method measures
/// sixteen tokens of generation against a prefill that grows with depth,
/// and at depth the prefill's jitter is larger than the generation (F155).
/// `None` where a rung did not say its spread, which older records do not.
fn bracket_of(
    readings: &[Value],
    shallow: (u64, u64),
    deep: (u64, u64),
    predicted: Option<u64>,
) -> Option<Bracket> {
    let spread_at = |depth: u64| {
        readings
            .iter()
            .find(|reading| per_token_of(reading).is_some_and(|(at, _)| at == depth))
            .and_then(|reading| reading.get("spread_ns"))
            .and_then(Value::as_integer)
            .and_then(|spread| u64::try_from(spread).ok())
    };
    let (shallow_half, deep_half) = (spread_at(shallow.0)? >> 1_u8, spread_at(deep.0)? >> 1_u8);
    let between = deep.0.checked_sub(shallow.0).filter(|tokens| *tokens > 0)?;
    let low = deep
        .1
        .saturating_sub(deep_half)
        .saturating_sub(shallow.1.saturating_add(shallow_half))
        .checked_div(between)?;
    let high = deep
        .1
        .saturating_add(deep_half)
        .saturating_sub(shallow.1.saturating_sub(shallow_half))
        .checked_div(between)?;
    Some(Bracket {
        low,
        high,
        prediction_within: predicted.map(|slope| (low..=high).contains(&slope)),
    })
}

/// Whether the slope was predicted, and from what — or why not (A7).
fn why_predicted(reread: Option<u64>, measured: bool) -> &'static str {
    match (reread, measured) {
        (Some(0), _) => {
            "unpredicted: the header describes a model that keeps a fixed state rather than a \
             cache that grows with depth, so the arithmetic that predicts a slope from the header \
             does not describe it — what was measured stands alone"
        }
        (Some(_), true) => {
            "predicted: the header's bytes a token of depth over what this machine read a \
             working set that size at, measured before the engine was up (B-427); what the \
             engine reached of it is the predicted slope's share of the measured one"
        }
        (Some(_), false) => {
            "not predicted: a slope predicted from the header divides what a token of depth \
             re-reads by this machine's read bandwidth, and this run did not measure it; the \
             rate above is what such a prediction would have to reproduce (B-400)"
        }
        (None, _) => {
            "not predicted: the header does not name the widths that size the cache, so what a \
             token of depth re-reads is not known and there is nothing to predict from"
        }
    }
}

/// The slope the header and the machine predict, beside the measured one.
struct Prediction {
    /// What this machine read at, at the working set of the deepest rung.
    machine: Option<crate::bandwidth::Reading>,
    /// The header's bytes a token of depth over that rate, in nanoseconds.
    predicted: Option<u64>,
    /// The predicted slope's share of the measured one, in whole percent.
    reached: Option<u64>,
    /// What each attending block reads a position, in bytes.
    per_block: Option<u64>,
}

/// The prediction: the header's bytes over what this machine read at,
/// measured at the working set the cache takes at the deepest rung — the
/// rung whose cache is furthest from fitting a faster level (F121). What the
/// engine reached of it is the predicted slope over the measured, in whole
/// percent — a slope no steeper than predicted is the whole of it.
fn predicted_slope(planned: &Planned, deepest: u64, per_token_of_depth: u64) -> Prediction {
    let reread = planned.per_token.filter(|bytes| *bytes > 0);
    let machine = planned
        .bandwidth
        .iter()
        .find(|reading| reading.at_depth == Some(deepest))
        .or_else(|| planned.bandwidth.last())
        .copied();
    let predicted = reread.zip(machine).and_then(|(bytes, machine)| {
        bytes
            .checked_mul(1_000_000_000)?
            .checked_div(machine.bytes_per_second)
    });
    let reached = predicted.map(|predicted| {
        predicted
            .saturating_mul(100)
            .checked_div(per_token_of_depth)
            .unwrap_or(0)
    });
    let per_block = reread
        .zip(planned.attending_blocks)
        .and_then(|(bytes, blocks)| bytes.checked_div(blocks))
        .filter(|bytes| *bytes > 0);
    Prediction {
        machine,
        predicted,
        reached,
        per_block,
    }
}

/// The contiguous read per block per position from which a read streams
/// rather than waits: 2 KiB, where the corpus divided into a bandwidth-bound
/// regime the header predicts to within 7% and a latency-bound one it does
/// not (F121).
const STREAMS_FROM: u64 = 2048;

/// A reading's depth and per-token cost, where it measured one.
fn per_token_of(reading: &Value) -> Option<(u64, u64)> {
    if reading.get("measured") != Some(&Value::Bool(true)) {
        return None;
    }
    let depth = reading.get("depth")?.as_integer()?;
    let ns = reading.get("ns_per_token")?.as_integer()?;
    Some((u64::try_from(depth).ok()?, u64::try_from(ns).ok()?))
}

/// What a token costs more for every token of depth: the per-token cost at
/// the deepest measured rung less the shallowest, over the tokens between —
/// and, set against it, what the header says a token of depth makes the
/// engine re-read, so that the rate the cache was read at is read off the two
/// (B-400, F121).
///
/// **The slope is measured; nothing here predicts it.** A prediction divides
/// the header's bytes by this machine's read bandwidth, and MCF has not
/// measured that bandwidth, so the figure a prediction would have to
/// reproduce is stated and the prediction is not (A20, A7). Where the header
/// describes a model whose cost does not grow with depth — a fixed recurrent
/// state — the arithmetic does not describe the model, and the slope is
/// reported as unpredicted by it rather than predicted wrongly (F122).
///
/// Not read, and said so, where fewer than two rungs measured or where the
/// deeper rung cost no more a token than the shallower (A9).
#[must_use]
pub fn fall_off(
    readings: &[Value],
    planned: &Planned,
    as_milliseconds: fn(u64) -> String,
) -> Value {
    let rungs: Vec<(u64, u64)> = readings.iter().filter_map(per_token_of).collect();
    let (Some(shallow), Some(deep)) = (rungs.first(), rungs.last()) else {
        return unmeasured(
            "no rung measured a cost a token, so there is no slope to read".to_owned(),
        );
    };
    if rungs.len() < 2 || deep.0 <= shallow.0 {
        return unmeasured(
            "one rung measured, and a fall-off is read between two: the deeper cost a token less \
             the shallower, over the tokens between"
                .to_owned(),
        );
    }
    if deep.1 <= shallow.1 {
        return unmeasured(format!(
            "a token at {} deep cost no more than at {} deep, so there is no fall-off to read a \
             slope off between them; the cost at each rung is in the ladder",
            grouped(deep.0),
            grouped(shallow.0)
        ));
    }
    #[expect(
        clippy::integer_division,
        reason = "a difference in nanoseconds over the tokens between two rungs; the remainder \
                  is under a nanosecond a token"
    )]
    let per_token_of_depth = (deep.1 - shallow.1) / (deep.0 - shallow.0);
    // What the header says a token of depth adds to the read, and the rate
    // that read went at: bytes a token of depth over the nanoseconds a token
    // of depth costs, in bytes a second.
    let reread = planned.per_token;
    let rate = reread.filter(|bytes| *bytes > 0).and_then(|bytes| {
        bytes
            .checked_mul(1_000_000_000)?
            .checked_div(per_token_of_depth)
    });
    let Prediction {
        machine,
        predicted,
        reached,
        per_block,
    } = predicted_slope(planned, deep.0, per_token_of_depth);
    let bracket = bracket_of(readings, *shallow, *deep, predicted);
    let prediction = why_predicted(reread, machine.is_some());
    Value::map([
        ("measured", Value::Bool(true)),
        ("ns_per_token_of_depth", count(per_token_of_depth)),
        (
            "ms_per_token_per_thousand_deep",
            Value::text(as_milliseconds(per_token_of_depth.saturating_mul(1_000))),
        ),
        (
            "between",
            Value::List(vec![count(shallow.0), count(deep.0)]),
        ),
        (
            "reread_bytes_per_token_of_depth",
            reread.map_or(Value::Null, count),
        ),
        (
            "read_rate_bytes_per_second",
            rate.map_or(Value::Null, count),
        ),
        (
            "sliding_window",
            planned.sliding_window.map_or(Value::Null, count),
        ),
        ("per_block_read_bytes", per_block.map_or(Value::Null, count)),
        (
            "predicted_ns_per_token_of_depth",
            predicted.map_or(Value::Null, count),
        ),
        (
            "predicted_ms_per_token_per_thousand_deep",
            predicted.map_or(Value::Null, |ns| {
                Value::text(as_milliseconds(ns.saturating_mul(1_000)))
            }),
        ),
        (
            "machine",
            machine.map_or(Value::Null, |reading| reading.as_value()),
        ),
        ("reached_percent", reached.map_or(Value::Null, count)),
        ("prediction", Value::text(prediction)),
        (
            "bracket",
            bracket.map_or(Value::Null, |bracket| bracket.as_value(as_milliseconds)),
        ),
        (
            "method",
            Value::text(
                "the cost a token at the deepest rung less the shallowest, over the tokens \
                 between; the header's cache a token of depth over that slope is the rate the \
                 engine re-read the cache at",
            ),
        ),
    ])
}

/// The lines a surface prints of the fall-off: the slope, the depths it was
/// read between, what the header says a token of depth re-reads and the rate
/// that went at, and why the slope is not predicted — or why there is none.
#[must_use]
pub fn fall_off_said(held: Option<&Value>) -> Vec<String> {
    let Some(held) = measured_or_said(held) else {
        return not_measured(held);
    };
    let number = |key: &str| {
        held.get(key)
            .and_then(Value::as_integer)
            .and_then(|value| u64::try_from(value).ok())
    };
    let between = held
        .get("between")
        .and_then(Value::as_list)
        .map(|depths| {
            depths
                .iter()
                .filter_map(Value::as_integer)
                .map(|depth| grouped(u64::try_from(depth).unwrap_or(0)))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();
    let mut lines = vec![
        format!(
            "{} ms more a token for every thousand tokens of depth",
            text(held, "ms_per_token_per_thousand_deep")
        ),
        format!("between {between} tokens deep"),
    ];
    lines.extend(bracket_said(held));
    if let Some(rate) = number("read_rate_bytes_per_second") {
        let bound = match number("sliding_window") {
            Some(window) => format!(
                " at most — the header declares a sliding window of {} tokens, past which the \
                 blocks that read within it stop growing",
                grouped(window)
            ),
            None => String::new(),
        };
        lines.push(format!(
            "{} bytes of cache a token of depth by the header's widths, which the engine re-read \
             at {}/s{bound}",
            grouped(number("reread_bytes_per_token_of_depth").unwrap_or(0)),
            gigabytes(rate)
        ));
    }
    if let Some(machine) = held
        .get("machine")
        .and_then(crate::bandwidth::Reading::from_value)
    {
        lines.push(format!(
            "predicted {} ms from those bytes over the {}/s this machine read a working set of \
             {} at, with {} threads — the engine reached {}% of it",
            text(held, "predicted_ms_per_token_per_thousand_deep"),
            gigabytes(machine.bytes_per_second),
            gigabytes(machine.working_set_bytes),
            machine.threads,
            number("reached_percent").unwrap_or(0)
        ));
        if let Some(Value::Bool(within)) = held
            .get("bracket")
            .and_then(|bracket| bracket.get("prediction_within"))
        {
            lines.push(if *within {
                "the prediction sits inside the bracket the rungs' spread puts the slope in"
                    .to_owned()
            } else {
                "the prediction sits outside the bracket the rungs' spread puts the slope in"
                    .to_owned()
            });
        }
    }
    if let Some(per_block) = number("per_block_read_bytes") {
        lines.push(if per_block >= STREAMS_FROM {
            format!(
                "each attending block reads {} bytes a position, over the {} where a read \
                 streams (F121)",
                grouped(per_block),
                grouped(STREAMS_FROM)
            )
        } else {
            format!(
                "each attending block reads {} bytes a position, under the {} where a read \
                 streams, so the read is bound by latency rather than bandwidth and the measured \
                 slope sits under the prediction (F121)",
                grouped(per_block),
                grouped(STREAMS_FROM)
            )
        });
    }
    lines.push(text(held, "prediction").to_owned());
    lines.push(text(held, "method").to_owned());
    lines
}

/// The lines a surface prints of the memory figure: the cost of a token of
/// window against the planned one, what the deepest rung held, the largest
/// context the machine holds at that cost, and what was observed — or why
/// there is none.
#[must_use]
pub fn memory_said(held: Option<&Value>) -> Vec<String> {
    let Some(held) = measured_or_said(held) else {
        return not_measured(held);
    };
    let number = |value: Option<&Value>| {
        value
            .and_then(Value::as_integer)
            .and_then(|value| u64::try_from(value).ok())
    };
    let windows = held
        .get("between_windows")
        .and_then(Value::as_list)
        .map(|windows| {
            windows
                .iter()
                .filter_map(Value::as_integer)
                .map(|window| grouped(u64::try_from(window).unwrap_or(0)))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();
    let planned = number(held.get("planned_per_token_bytes")).map_or_else(
        || "the header does not size a cache".to_owned(),
        |bytes| format!("{} bytes planned from the header", grouped(bytes)),
    );
    let deepest = held.get("at_deepest");
    let largest = held.get("largest_context");
    let ceiling = match number(largest.and_then(|figure| figure.get("measured"))) {
        Some(tokens) => format!(
            "{} tokens is the largest context the free memory holds at that cost — {}",
            grouped(tokens),
            number(largest.and_then(|figure| figure.get("planned"))).map_or_else(
                || "unplanned".to_owned(),
                |planned| format!("{} tokens planned", grouped(planned))
            )
        ),
        None => format!(
            "no ceiling: {}",
            largest
                .and_then(|figure| figure.get("why"))
                .and_then(Value::as_text)
                .unwrap_or("MCF did not say why")
        ),
    };
    vec![
        format!(
            "{} bytes a token of window between windows of {windows}; {planned}",
            grouped(number(held.get("per_token_bytes")).unwrap_or(0))
        ),
        format!(
            "{} held at {} deep, in a window of {}",
            gigabytes(number(deepest.and_then(|at| at.get("bytes"))).unwrap_or(0)),
            grouped(number(deepest.and_then(|at| at.get("depth"))).unwrap_or(0)),
            grouped(number(deepest.and_then(|at| at.get("window"))).unwrap_or(0)),
        ),
        ceiling,
        text(held, "what").to_owned(),
    ]
}

/// A size in gigabytes to two places, exact and without a float.
fn gigabytes(bytes: u64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "exact: a float would round a byte count before printing it"
    )]
    {
        let whole = bytes / 1_000_000_000;
        let hundredths = (bytes % 1_000_000_000) / 10_000_000;
        format!("{whole}.{hundredths:02} GB")
    }
}

/// The lines a surface prints of the prompt-reading figure: the cost, the
/// depths it was read between, and how — or why there is none (A7).
///
/// `None` is a run that never reached its last line, or a record from
/// before the figure was derived, which said nothing and is printed as
/// having said nothing.
#[must_use]
pub fn prompt_reading_said(held: Option<&Value>) -> Vec<String> {
    let Some(held) = measured_or_said(held) else {
        return not_measured(held);
    };
    let between = held
        .get("between")
        .and_then(Value::as_list)
        .map(|depths| {
            depths
                .iter()
                .filter_map(Value::as_integer)
                .map(|depth| grouped(u64::try_from(depth).unwrap_or(0)))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .unwrap_or_default();
    vec![
        format!("{} ms a token of prompt", text(held, "ms_per_token")),
        format!("between {between} tokens deep"),
        text(held, "method").to_owned(),
    ]
}

/// The lines a surface prints of the first-token figure: the time and the
/// depth it was taken at, and what the time includes — or why there is none.
#[must_use]
pub fn first_token_said(held: Option<&Value>) -> Vec<String> {
    let Some(held) = measured_or_said(held) else {
        return not_measured(held);
    };
    let depth = held
        .get("depth")
        .and_then(Value::as_integer)
        .and_then(|depth| u64::try_from(depth).ok())
        .unwrap_or(0);
    vec![
        format!(
            "{} ms to the first token, {} tokens deep",
            text(held, "ms"),
            grouped(depth)
        ),
        text(held, "includes").to_owned(),
    ]
}

/// The figure, where it says it measured.
fn measured_or_said(held: Option<&Value>) -> Option<&Value> {
    held.filter(|figure| figure.get("measured") == Some(&Value::Bool(true)))
}

/// Why there is no figure: the daemon's reason, or that it said nothing.
fn not_measured(held: Option<&Value>) -> Vec<String> {
    let Some(held) = held else {
        return vec!["MCF did not say".to_owned()];
    };
    let why = held
        .get("why")
        .and_then(Value::as_text)
        .unwrap_or("MCF did not say why");
    vec![format!("not measured: {why}")]
}

/// A text the figure carries, or a mark that it does not.
fn text<'a>(held: &'a Value, key: &str) -> &'a str {
    held.get(key).and_then(Value::as_text).unwrap_or("?")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(ns: u64) -> String {
        #[expect(clippy::integer_division, reason = "whole milliseconds, for a test")]
        let whole = ns / 1_000_000;
        whole.to_string()
    }

    fn rung(depth: i64, first_token_ns: Option<i64>) -> Value {
        match first_token_ns {
            Some(ns) => Value::map([
                ("depth", Value::Integer(depth)),
                ("first_token_ns", Value::Integer(ns)),
                ("measured", Value::Bool(true)),
            ]),
            None => Value::map([
                ("depth", Value::Integer(depth)),
                ("measured", Value::Bool(false)),
            ]),
        }
    }

    /// 512 deep in 1 s, 2048 deep in 4 s: 1 536 tokens cost 3 s, which is
    /// about 1.95 ms a token — and the unmeasured rung between is skipped.
    #[test]
    fn a_prompt_cost_is_read_between_the_outer_rungs() {
        let readings = [
            rung(512, Some(1_000_000_000)),
            rung(1024, None),
            rung(2048, Some(4_000_000_000)),
        ];
        let said = prompt_reading(&readings, ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(true)));
        assert_eq!(
            said.get("ns_per_token").and_then(Value::as_integer),
            Some(1_953_125)
        );
        assert_eq!(
            said.get("between").and_then(Value::as_list).map(<[_]>::len),
            Some(2)
        );
        let start = first_token(&readings, ms);
        assert_eq!(start.get("depth").and_then(Value::as_integer), Some(512));
        assert_eq!(start.get("ms").and_then(Value::as_text), Some("1000"));
    }

    /// One rung, or a deeper rung that came no later, is not a cost (A9).
    #[test]
    fn what_cannot_be_read_is_said_not_read() {
        let one = [rung(512, Some(1_000_000_000))];
        let said = prompt_reading(&one, ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(false)));
        assert!(
            said.get("why")
                .and_then(Value::as_text)
                .is_some_and(|why| why.contains("one rung"))
        );
        let backwards = [
            rung(512, Some(4_000_000_000)),
            rung(1024, Some(3_000_000_000)),
        ];
        let said = prompt_reading(&backwards, ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(false)));
        assert!(
            said.get("why")
                .and_then(Value::as_text)
                .is_some_and(|why| why.contains("no later"))
        );
        let none = [rung(512, None)];
        assert_eq!(
            prompt_reading(&none, ms).get("measured"),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            first_token(&none, ms).get("measured"),
            Some(&Value::Bool(false))
        );
    }

    /// The sentences are the figures' own, and a figure that is not there is
    /// said to be missing rather than left blank.
    #[test]
    fn the_sentences_follow_the_figures() {
        let readings = [
            rung(512, Some(1_000_000_000)),
            rung(2048, Some(4_000_000_000)),
        ];
        let reading = prompt_reading(&readings, ms);
        assert_eq!(
            prompt_reading_said(Some(&reading)),
            vec![
                "1 ms a token of prompt".to_owned(),
                "between 512 and 2,048 tokens deep".to_owned(),
                "the time to a first token at the deepest rung less the shallowest, over the \
                 tokens between — the load is in both and cancels"
                    .to_owned(),
            ]
        );
        let start = first_token(&readings, ms);
        assert_eq!(
            first_token_said(Some(&start)).first().map(String::as_str),
            Some("1000 ms to the first token, 512 tokens deep")
        );
        assert_eq!(first_token_said(None), vec!["MCF did not say".to_owned()]);
        let one = [rung(512, Some(1_000_000_000))];
        let lines = prompt_reading_said(Some(&prompt_reading(&one, ms)));
        assert_eq!(lines.len(), 1);
        assert!(lines.iter().all(|line| line.starts_with("not measured: ")));
    }

    fn held(depth: i64, window: i64, peak: Option<i64>) -> Value {
        let mut fields = vec![
            ("depth", Value::Integer(depth)),
            ("window", Value::Integer(window)),
            ("measured", Value::Bool(true)),
        ];
        if let Some(peak) = peak {
            fields.push(("peak_resident_bytes", Value::Integer(peak)));
        }
        Value::map(fields)
    }

    /// Windows of 4,096 and 8,228 with 500 MB between them: about 121 kB a
    /// token of window, read between the outer rungs whatever the rungs
    /// between held — and the ceiling is the planner's arithmetic on it.
    #[test]
    fn a_memory_cost_is_read_between_the_outer_windows() {
        let readings = [
            held(512, 4096, Some(2_000_000_000)),
            held(1024, 4096, Some(2_000_000_000)),
            held(2048, 4132, None),
            held(4096, 8228, Some(2_500_000_000)),
        ];
        let planned = Planned {
            per_token: Some(114_688),
            weights: Some(1_000_000_000),
            free: Some(10_000_000_000),
            trained: Some(32_768),
            ..Planned::default()
        };
        let said = memory(&readings, &planned);
        assert_eq!(said.get("measured"), Some(&Value::Bool(true)));
        assert_eq!(
            said.get("per_token_bytes").and_then(Value::as_integer),
            Some(121_006)
        );
        let ceiling = said.get("largest_context").expect("a ceiling");
        assert_eq!(
            ceiling.get("measured").and_then(Value::as_integer),
            i64::try_from(crate::engines::largest_context(
                1_000_000_000,
                121_006,
                10_000_000_000,
                32_768
            ))
            .ok()
        );
        assert_eq!(
            ceiling.get("planned").and_then(Value::as_integer),
            i64::try_from(crate::engines::largest_context(
                1_000_000_000,
                114_688,
                10_000_000_000,
                32_768
            ))
            .ok()
        );
        let lines = memory_said(Some(&said));
        assert_eq!(
            lines.first().map(String::as_str),
            Some(
                "121,006 bytes a token of window between windows of 4,096 and 8,228; 114,688 \
                 bytes planned from the header"
            )
        );
        assert_eq!(
            lines.get(1).map(String::as_str),
            Some("2.50 GB held at 4,096 deep, in a window of 8,228")
        );
        assert!(
            lines
                .get(2)
                .is_some_and(|line| line.contains("largest context"))
        );
    }

    /// One window, an engine that held no more deeper, or no peak at all is
    /// not a cost (A7, A9) — and without the machine there is no ceiling.
    #[test]
    fn what_memory_cannot_be_read_is_said_not_read() {
        let why = |readings: &[Value]| {
            let said = memory(readings, &Planned::default());
            assert_eq!(said.get("measured"), Some(&Value::Bool(false)));
            said.get("why")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned()
        };
        assert!(why(&[held(512, 4096, None)]).contains("no rung's engine was observed"));
        assert!(
            why(&[
                held(512, 4096, Some(2_000_000_000)),
                held(2048, 4096, Some(2_100_000_000)),
            ])
            .contains("one window of 4,096")
        );
        assert!(
            why(&[
                held(512, 4096, Some(2_000_000_000)),
                held(4096, 8228, Some(2_000_000_000)),
            ])
            .contains("held no more")
        );
        let said = memory(
            &[
                held(512, 4096, Some(2_000_000_000)),
                held(4096, 8228, Some(2_500_000_000)),
            ],
            &Planned::default(),
        );
        assert_eq!(said.get("measured"), Some(&Value::Bool(true)));
        assert!(
            memory_said(Some(&said))
                .get(2)
                .is_some_and(|line| line.starts_with("no ceiling: "))
        );
        assert_eq!(memory_said(None), vec!["MCF did not say".to_owned()]);
    }

    /// A rung's depth and cost a token, for the slope read between rungs.
    fn costing(depth: i64, ns_per_token: i64) -> Value {
        Value::map([
            ("depth", Value::Integer(depth)),
            ("ns_per_token", Value::Integer(ns_per_token)),
            ("measured", Value::Bool(true)),
        ])
    }

    /// What the header planned, with a cache of the given bytes a token.
    fn header_says(per_token: Option<u64>, sliding_window: Option<u64>) -> Planned {
        Planned {
            per_token,
            sliding_window,
            ..Planned::default()
        }
    }

    /// 15 ms a token at 512 deep and 139 ms at 8,192: 124 ms over 7,680
    /// tokens of depth is 16,145 ns a token of depth — and a header that
    /// re-reads 114,688 bytes a token of depth was read at 7.10 GB/s. The
    /// slope is measured and the prediction is declined, with the reason.
    #[test]
    fn a_fall_off_is_read_between_the_outer_rungs_and_set_against_the_header() {
        let readings = [
            costing(512, 15_000_000),
            costing(1024, 17_000_000),
            costing(8192, 139_000_000),
        ];
        let said = fall_off(&readings, &header_says(Some(114_688), None), ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(true)));
        assert_eq!(
            said.get("ns_per_token_of_depth")
                .and_then(Value::as_integer),
            Some(16_145)
        );
        assert_eq!(
            said.get("read_rate_bytes_per_second")
                .and_then(Value::as_integer),
            Some(7_103_623_412)
        );
        assert!(
            said.get("prediction")
                .and_then(Value::as_text)
                .is_some_and(|why| why.starts_with("not predicted") && why.contains("bandwidth"))
        );
        let lines = fall_off_said(Some(&said));
        assert_eq!(
            lines.first().map(String::as_str),
            Some("16 ms more a token for every thousand tokens of depth")
        );
        assert_eq!(
            lines.get(1).map(String::as_str),
            Some("between 512 and 8,192 tokens deep")
        );
        assert_eq!(
            lines.get(2).map(String::as_str),
            Some(
                "114,688 bytes of cache a token of depth by the header's widths, which the \
                 engine re-read at 7.10 GB/s"
            )
        );
    }

    /// A sliding window bounds the re-read, so the rate is *at most*; a
    /// fixed state is not a growing cache, so the slope is unpredicted; a
    /// header that does not size its cache gives nothing to predict from.
    #[test]
    fn what_the_header_cannot_predict_is_said_unpredicted() {
        let readings = [costing(512, 15_000_000), costing(8192, 139_000_000)];
        let windowed = fall_off(&readings, &header_says(Some(114_688), Some(512)), ms);
        assert!(
            fall_off_said(Some(&windowed))
                .get(2)
                .is_some_and(|line| line.contains("at most") && line.contains("512 tokens"))
        );
        let fixed = fall_off(&readings, &header_says(Some(0), None), ms);
        assert_eq!(fixed.get("read_rate_bytes_per_second"), Some(&Value::Null));
        assert!(
            fixed
                .get("prediction")
                .and_then(Value::as_text)
                .is_some_and(|why| why.starts_with("unpredicted"))
        );
        assert_eq!(fall_off_said(Some(&fixed)).len(), 4, "no re-read line");
        assert_eq!(
            fixed.get("predicted_ns_per_token_of_depth"),
            Some(&Value::Null)
        );
        let unsaid = fall_off(&readings, &header_says(None, None), ms);
        assert!(
            unsaid
                .get("prediction")
                .and_then(Value::as_text)
                .is_some_and(|why| why.contains("does not name the widths"))
        );
    }

    /// With the machine measured, the slope is predicted: 114,688 bytes over
    /// 110 GB/s is 1,042 ns a token of depth, and a measured 16,145 ns is
    /// 6% of it reached. The reading at the deepest rung is the one used,
    /// and each block's read is set against where a read streams.
    #[test]
    fn with_the_machine_measured_the_slope_is_predicted() {
        let readings = [costing(512, 15_000_000), costing(8192, 139_000_000)];
        let machine = |depth: u64, bytes_per_second: u64| crate::bandwidth::Reading {
            at_depth: Some(depth),
            working_set_bytes: depth.saturating_mul(114_688),
            threads: 32,
            bytes_per_second,
            passes: 10,
        };
        let planned = Planned {
            per_token: Some(114_688),
            attending_blocks: Some(28),
            bandwidth: vec![
                machine(512, 900_000_000_000),
                machine(8192, 110_000_000_000),
            ],
            ..Planned::default()
        };
        let said = fall_off(&readings, &planned, ms);
        assert_eq!(
            said.get("predicted_ns_per_token_of_depth")
                .and_then(Value::as_integer),
            Some(1_042)
        );
        assert_eq!(
            said.get("reached_percent").and_then(Value::as_integer),
            Some(6)
        );
        assert_eq!(
            said.get("per_block_read_bytes").and_then(Value::as_integer),
            Some(4_096)
        );
        assert!(
            said.get("prediction")
                .and_then(Value::as_text)
                .is_some_and(|why| why.starts_with("predicted"))
        );
        let lines = fall_off_said(Some(&said));
        assert_eq!(
            lines.get(3).map(String::as_str),
            Some(
                "predicted 1 ms from those bytes over the 110.00 GB/s this machine read a \
                 working set of 0.93 GB at, with 32 threads — the engine reached 6% of it"
            )
        );
        assert!(lines.get(4).is_some_and(|line| {
            line.starts_with("each attending block reads 4,096 bytes")
                && line.contains("over the 2,048")
        }));
    }

    /// A rung that said its spread brackets the slope: 512 deep at 17.6 ms
    /// ± 1.4 and 8,192 deep at 164.4 ± 34.4 put the slope between 14.5 and
    /// 23.8 ms a thousand, and a prediction of 1.2 sits outside it. Where
    /// the deep rung's spread reaches the shallow one — 23.2 ± 12.3 against
    /// 17.6 ± 1.4 — nothing is resolved, and the line says why (F155).
    #[test]
    fn the_rungs_spread_brackets_the_slope_or_says_none_is_resolved() {
        let spread = |depth: i64, ns: i64, spread: i64| {
            Value::map([
                ("depth", Value::Integer(depth)),
                ("ns_per_token", Value::Integer(ns)),
                ("spread_ns", Value::Integer(spread)),
                ("measured", Value::Bool(true)),
            ])
        };
        let machine = crate::bandwidth::Reading {
            at_depth: Some(8192),
            working_set_bytes: 939_524_096,
            threads: 32,
            bytes_per_second: 95_274_787_469,
            passes: 7,
        };
        let planned = Planned {
            per_token: Some(114_688),
            bandwidth: vec![machine],
            ..Planned::default()
        };
        let steep = [
            spread(512, 17_585_533, 2_841_000),
            spread(8192, 164_386_598, 68_808_000),
        ];
        let said = fall_off(&steep, &planned, ms);
        let bracket = said.get("bracket").expect("bracketed");
        assert_eq!(bracket.get("resolved"), Some(&Value::Bool(true)));
        assert_eq!(bracket.get("prediction_within"), Some(&Value::Bool(false)));
        assert_eq!(
            bracket
                .get("low_ns_per_token_of_depth")
                .and_then(Value::as_integer),
            Some(14_450)
        );
        let lines = fall_off_said(Some(&said));
        assert_eq!(
            lines.get(2).map(String::as_str),
            Some("between 14 and 23 ms a thousand by the spread at the two rungs")
        );
        assert!(lines.iter().any(|line| {
            line.ends_with("sits outside the bracket the rungs' spread puts the slope in")
        }));
        let flat = [
            spread(512, 17_585_533, 2_841_000),
            spread(4096, 23_201_575, 24_524_000),
        ];
        let said = fall_off(&flat, &planned, ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(true)));
        assert_eq!(
            said.get("bracket")
                .and_then(|bracket| bracket.get("resolved")),
            Some(&Value::Bool(false))
        );
        assert!(
            fall_off_said(Some(&said))
                .get(2)
                .is_some_and(|line| line.starts_with("not resolved:") && line.contains("F155"))
        );
        let unsaid = fall_off(
            &[costing(512, 17_585_533), costing(4096, 23_201_575)],
            &planned,
            ms,
        );
        assert_eq!(unsaid.get("bracket"), Some(&Value::Null));
    }

    /// One rung, or a deeper rung that cost no more, is not a slope (A9).
    #[test]
    fn a_fall_off_that_cannot_be_read_is_said_not_read() {
        let one = [costing(512, 15_000_000)];
        let said = fall_off(&one, &header_says(Some(114_688), None), ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(false)));
        let flat = [costing(512, 15_000_000), costing(8192, 15_000_000)];
        let said = fall_off(&flat, &header_says(Some(114_688), None), ms);
        assert_eq!(said.get("measured"), Some(&Value::Bool(false)));
        assert!(
            said.get("why")
                .and_then(Value::as_text)
                .is_some_and(|why| why.contains("no more than"))
        );
        assert_eq!(fall_off_said(None), vec!["MCF did not say".to_owned()]);
    }
}
