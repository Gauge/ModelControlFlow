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
}
