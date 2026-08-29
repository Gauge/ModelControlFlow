//! What a comparison leaves behind (B-086, A9, B56, D16).
//!
//! **A9 in one sentence:** *"no measurable difference" and "does not fit here"
//! are findings, not failures.* A comparison that separated its arms, one that
//! established they are the same to a stated resolution, and one that refused
//! a delta because more than one variable differed are three outcomes of the
//! same event. They are written to one place, in one shape, under one kind —
//! [`EntryKind::Comparison`] — because a null result filed somewhere else is a
//! null result nobody will find beside the positive ones, and *what has been
//! compared on this machine* stops being a question the register can answer.
//!
//! **The distribution is written, not only the verdict.** B56 keeps the trials
//! and derives the summary; D16 keeps raw trials always. A comparison whose
//! paired differences were thrown away is a question nobody can re-ask, and
//! the verdict is the one part of it a later reader is most likely to want to
//! recompute — the stopping condition's own rule has already changed once
//! (F55).
//!
//! **A simulated timing cannot be written** (A11, B-082). The encoder is
//! bounded by [`Measurable`], which [`Monotonic`] implements and [`Simulated`]
//! does not, so `record::comparison` of a laboratory comparison is a compile
//! error rather than a review comment. D9 gives the laboratory a clock to
//! reproduce *behaviour* deterministically; a performance figure taken from it
//! would be MCF reporting how fast its own arithmetic is.
//!
//! [`Measurable`]: mcf_core::time::Measurable
//! [`Monotonic`]: mcf_core::time::Monotonic
//! [`Simulated`]: mcf_core::time::Simulated
//!
//! **The encoder lives here rather than in `mcf-record`.** `mcf-bench` depends
//! on `mcf-record` and the reverse edge does not exist, so the crate that owns
//! the type owns the shape it is written in. That is also where the knowledge
//! is: what a comparison must not lose is a question about comparisons.
//!
//! [`EntryKind::Comparison`]: mcf_record::journal::EntryKind::Comparison
//!
//! **Not an instrument:** it encodes what other modules measured.

use mcf_core::measurement::{Isolation, PartsPerMillion};
use mcf_core::time::{ClockKind, Measurable};
use mcf_record::encode;
use mcf_record::json::Value;

use super::compare::{
    Comparison, Difference, Discipline, Finding, MachineHeld, Method, Side, Strength, UnderTest,
};
use super::enough::Verdict;

/// Everything one comparison established, and everything it established it on.
///
/// The verdict, the isolation, how the arms were brought together, the
/// conditions of both arms, and every pair — both durations, which ran first,
/// and the difference. Nothing is summarized away.
#[must_use]
pub fn comparison<K: ClockKind + Measurable>(
    held: &Comparison<K>,
    finding: &Finding,
    method: &Method,
    machine: Option<&MachineHeld>,
) -> Value {
    let (left, right) = held.arms();
    let (left_first, right_first) = held.order_balance();
    Value::map([
        ("left", arm(left)),
        ("right", arm(right)),
        ("outcome", outcome(finding)),
        ("isolation", isolation(finding.isolation())),
        (
            "declared_confound",
            finding.declared().map_or(Value::Null, Value::text),
        ),
        ("strength", strength(&held.strength())),
        (
            "order",
            Value::map([
                ("left_first", count(left_first)),
                ("right_first", count(right_first)),
            ]),
        ),
        ("discipline", discipline(held.discipline())),
        // What the *question* was, which the conditions do not say. §II asks
        // that somebody else be able to repeat this, and a floor full of
        // hardware does not tell them what to run (PR2, B30, B-211).
        ("method", self::method(method)),
        // What the machine was doing either side of the run (B-217, §3.8). A
        // level that moved is a condition of the result; it carries no verdict,
        // because what movement is too much is DEC-007's open band.
        ("machine", machine.map_or(Value::Null, held_machine)),
        // §6.13, B-081: what the run reused. A mixed run is not one
        // measurement, and this is where that stops being invisible.
        ("reuse", Value::text(held.reuse().condition())),
        // A4, B-087: a run that was interrupted keeps every pair it completed
        // and says what was lost. `null` is *it finished*, which is a
        // different fact from an interruption nobody recorded.
        (
            "cut_short",
            held.cut_short().map_or(Value::Null, Value::text),
        ),
        ("pairs", Value::List(pairs(held))),
    ])
}

/// How much the machine's own load moved across the run (B-217).
fn held_machine(held: &MachineHeld) -> Value {
    let fraction = |value: Option<u64>| {
        value.map_or(Value::Null, |held| {
            Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
        })
    };
    Value::map([
        (
            "competing_before_thousandths",
            Value::Integer(i64::try_from(held.before).unwrap_or(i64::MAX)),
        ),
        (
            "competing_after_thousandths",
            Value::Integer(i64::try_from(held.after).unwrap_or(i64::MAX)),
        ),
        ("steady_before_ppm", fraction(held.steady_before)),
        ("steady_after_ppm", fraction(held.steady_after)),
        ("moved_ppm", fraction(held.moved())),
    ])
}

/// What the comparison was asked to do (PR2, B30, B-211).
///
/// **The prompt is a condition and it is also content** (A6 and A25 at once,
/// F105). A8 needs to be able to say that two comparisons were not asked the
/// same thing, and A25 says what a person typed does not live in the record.
/// Both are absolute, and they are reconciled the way `Content::length_bytes`
/// already suggests: what the record keeps is a *measurement about* the prompt
/// — its length and its digest — which is enough to refuse a confound and is
/// not the text. The text goes to the content store under this entry's
/// identifier, where `mcf bundle` discloses it deliberately.
///
/// The digest identifies; it does not conceal. Anyone who guesses `Once upon a
/// time` can confirm it, and the record does not claim otherwise — it claims
/// not to *hold* the prompt, which is what makes an export of it carry none.
fn method(held: &Method) -> Value {
    Value::map([
        (
            "prompt_bytes",
            Value::Integer(i64::try_from(held.prompt.len()).unwrap_or(i64::MAX)),
        ),
        (
            "prompt_digest",
            Value::text({
                let mut digest = mcf_core::digest::Sha256::new();
                digest.update(held.prompt.as_bytes());
                digest.finish().hex()
            }),
        ),
        ("resolving_ppm", parts_per_million(held.resolving)),
        ("ceiling", count(held.ceiling)),
        (
            "engine_asked",
            held.engine
                .as_ref()
                .map_or(Value::Null, |named| Value::text(named.clone())),
        ),
        ("cold", Value::Bool(held.cold)),
    ])
}

/// Which discipline the trials were taken under (B61, D19, B-290).
///
/// The seed set travels here because D19 makes it a condition and A8 makes a
/// condition something a later comparison checks: two measurements on
/// different sets are not comparable, and the record is where that is found
/// out.
fn discipline(held: &Discipline) -> Value {
    match held {
        Discipline::Timing { seed, tokens } => Value::map([
            ("kind", Value::text("timing")),
            // Text, for the reason `mcf_record::encode::draw` gives: a seed is
            // a `u64` and the record's integers are `i64`.
            ("seed", Value::text(seed.to_string())),
            ("tokens_pinned", Value::Integer(i64::from(*tokens))),
            ("seed_set", Value::text(held.seed_set())),
        ]),
        Discipline::Behaviour { seeds } => Value::map([
            ("kind", Value::text("behaviour")),
            ("seed", Value::Null),
            ("tokens_pinned", Value::Null),
            ("seed_set", Value::text(seeds.identifier())),
        ]),
    }
}

/// One arm: what it is called, and what it was measured under.
fn arm(held: &UnderTest) -> Value {
    Value::map([
        ("arm", Value::text(held.arm().as_str())),
        ("conditions", encode::conditions(held.conditions())),
    ])
}

/// What the comparison concluded.
///
/// Four outcomes and no failure among them. `not_comparable` is the confounded
/// case, which has no delta by construction (A8) — it is written as the
/// outcome it is rather than omitted, because *these were compared and the
/// comparison said nothing* is exactly the fact a later reader needs in order
/// not to run it again the same way.
fn outcome(finding: &Finding) -> Value {
    match finding.verdict() {
        None => Value::map([
            ("kind", Value::text("not_comparable")),
            ("quicker", Value::Null),
            ("resolution", Value::Null),
            ("difference", Value::Null),
            ("by_chance", Value::Null),
            ("pairs", Value::Null),
        ]),
        Some(Verdict::Differ {
            by,
            left_quicker,
            by_chance,
            after,
        }) => Value::map([
            ("kind", Value::text("differ")),
            // A size without a direction is not a comparison (F67).
            (
                "quicker",
                Value::text(if *left_quicker { "left" } else { "right" }),
            ),
            ("resolution", Value::Null),
            // **A range, and the coverage it was computed at** (F92). The old
            // `difference` was a point estimate with nothing qualifying it;
            // the three fields together are what a later reader needs to know
            // how much the size is worth. The name is kept for the low bound
            // so that a record written before this can still be read: what
            // was one number is now the bottom of the range it always was.
            ("difference", parts_per_million(by.low)),
            ("difference_high", parts_per_million(by.high)),
            ("coverage", parts_per_million(by.coverage)),
            ("by_chance", parts_per_million(*by_chance)),
            ("pairs", count(*after)),
        ]),
        Some(Verdict::Ordered {
            left_quicker,
            by,
            resolving,
            by_chance,
            after,
        }) => Value::map([
            // A kind of its own, because it is the marking: a reader
            // filtering the record for results whose size is established must
            // be able to exclude these without knowing anything else (F92).
            ("kind", Value::text("ordered")),
            (
                "quicker",
                Value::text(if *left_quicker { "left" } else { "right" }),
            ),
            ("resolution", parts_per_million(*resolving)),
            ("difference", parts_per_million(by.low)),
            ("difference_high", parts_per_million(by.high)),
            ("coverage", parts_per_million(by.coverage)),
            ("by_chance", parts_per_million(*by_chance)),
            ("pairs", count(*after)),
        ]),
        Some(Verdict::Apart {
            by,
            left_quicker,
            by_chance,
            after,
        }) => Value::map([
            // Its own kind, so that a reader can exclude sizes that carry no
            // interval without knowing why they do not (F92, B53).
            ("kind", Value::text("apart")),
            (
                "quicker",
                Value::text(if *left_quicker { "left" } else { "right" }),
            ),
            ("resolution", Value::Null),
            // The same three fields the paired row carries (B-388). The kind
            // is what tells a reader these arms were never paired; the
            // interval itself is built to the same standard by different
            // mathematics, and a reader comparing the two needs both stated
            // the same way.
            ("difference", parts_per_million(by.low)),
            ("difference_high", parts_per_million(by.high)),
            ("coverage", parts_per_million(by.coverage)),
            ("by_chance", parts_per_million(*by_chance)),
            ("pairs", count(*after)),
        ]),
        Some(Verdict::Same {
            resolving,
            by,
            after,
        }) => Value::map([
            // Named for what it is. A9: this is a finding, and a reader
            // filtering for results must not have to know that `same` is one.
            ("kind", Value::text("same")),
            ("quicker", Value::Null),
            ("resolution", parts_per_million(*resolving)),
            // The measured difference, which is smaller than the resolution
            // and is not nothing. A reader who later cares about a smaller
            // resolution needs it, and A1 forbids dropping it.
            ("difference", parts_per_million(*by)),
            ("by_chance", Value::Null),
            ("pairs", count(*after)),
        ]),
        Some(Verdict::NotYet { so_far }) => Value::map([
            ("kind", Value::text("not_yet")),
            ("quicker", Value::Null),
            ("resolution", Value::Null),
            ("difference", Value::Null),
            ("by_chance", Value::Null),
            ("pairs", count(*so_far)),
        ]),
    }
}

/// What separated the arms' configurations.
fn isolation(held: &Isolation) -> Value {
    let (kind, unread) = match held {
        Isolation::SameConfiguration => ("same_configuration", Vec::new()),
        Isolation::Isolated { .. } => ("isolated", Vec::new()),
        Isolation::Confounded { .. } => ("confounded", Vec::new()),
        Isolation::Undetermined { unread, .. } => ("undetermined", unread.clone()),
    };
    Value::map([
        ("kind", Value::text(kind)),
        (
            "differ",
            Value::List(
                held.differing()
                    .iter()
                    .map(|held| Value::text(*held))
                    .collect(),
            ),
        ),
        (
            "unread",
            Value::List(unread.into_iter().map(Value::text).collect()),
        ),
    ])
}

/// How the arms were brought together (§3.27).
fn strength(held: &Strength) -> Value {
    match held {
        Strength::Paired(session) => Value::map([
            ("kind", Value::text("paired")),
            ("session", Value::text(session.as_str())),
            ("left_session", Value::Null),
            ("right_session", Value::Null),
        ]),
        Strength::Assembled { left, right } => Value::map([
            ("kind", Value::text("assembled")),
            ("session", Value::Null),
            ("left_session", Value::text(left.as_str())),
            ("right_session", Value::text(right.as_str())),
        ]),
    }
}

/// Every pair, in interleaving order.
///
/// Empty for a comparison assembled from separate sessions, which has none —
/// and the emptiness is the record of that, not a gap in it.
fn pairs<K: ClockKind + Measurable>(held: &Comparison<K>) -> Vec<Value> {
    held.pairs()
        .iter()
        .map(|pair| {
            let (left_at, right_at) = pair.positions();
            Value::map([
                ("left_ns", Value::Integer(nanos(pair.left().as_nanos()))),
                ("right_ns", Value::Integer(nanos(pair.right().as_nanos()))),
                ("left_position", Value::Integer(i64::from(left_at.0))),
                ("right_position", Value::Integer(i64::from(right_at.0))),
                ("first", Value::text(side(pair.first()))),
                ("left_found", Value::text(pair.warmth().0.to_string())),
                ("right_found", Value::text(pair.warmth().1.to_string())),
                ("drew", encode::draw(pair.drew())),
                ("difference", difference(pair.difference())),
            ])
        })
        .collect()
}

/// One pair's difference, as the signed thing it is.
fn difference(held: Difference) -> Value {
    match held {
        Difference::Quicker { side: which, by } => Value::map([
            ("kind", Value::text("quicker")),
            ("quicker", Value::text(side(which))),
            ("by_ppm", parts_per_million(by)),
        ]),
        Difference::Level => Value::map([
            ("kind", Value::text("level")),
            ("quicker", Value::Null),
            ("by_ppm", Value::Null),
        ]),
        Difference::Unmeasurable => Value::map([
            // A7: the quicker arm took no measurable time, so there is no
            // ratio. Written as what it is rather than as a zero.
            ("kind", Value::text("unmeasurable")),
            ("quicker", Value::Null),
            ("by_ppm", Value::Null),
        ]),
    }
}

/// Which side, as the record writes it.
const fn side(held: Side) -> &'static str {
    match held {
        Side::Left => "left",
        Side::Right => "right",
    }
}

/// A ratio, saturating rather than wrapping into a negative number.
fn parts_per_million(held: PartsPerMillion) -> Value {
    Value::Integer(i64::try_from(held.0).unwrap_or(i64::MAX))
}

/// A duration, saturating for the same reason.
fn nanos(held: u64) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

/// A count, saturating for the same reason.
fn count(held: usize) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests;
