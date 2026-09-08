use mcf_core::measurement::{Isolation, PartsPerMillion};
use mcf_core::time::{ClockKind, Measurable};
use mcf_record::encode;
use mcf_record::json::Value;

use super::compare::{
    Comparison, Difference, Discipline, Finding, MachineHeld, Method, Side, Strength, UnderTest,
};
use super::enough::Verdict;

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
        ("method", self::method(method)),
        ("machine", machine.map_or(Value::Null, held_machine)),
        ("reuse", Value::text(held.reuse().condition())),
        (
            "cut_short",
            held.cut_short().map_or(Value::Null, Value::text),
        ),
        ("pairs", Value::List(pairs(held))),
    ])
}

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
        (
            "workload",
            Value::text(match held.workload {
                mcf_core::contribution::Workload::Declared => "declared",
                mcf_core::contribution::Workload::Custom => "custom",
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

fn discipline(held: &Discipline) -> Value {
    match held {
        Discipline::Timing { seed, tokens } => Value::map([
            ("kind", Value::text("timing")),
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

fn arm(held: &UnderTest) -> Value {
    Value::map([
        ("arm", Value::text(held.arm().as_str())),
        ("conditions", encode::conditions(held.conditions())),
    ])
}

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
            (
                "quicker",
                Value::text(if *left_quicker { "left" } else { "right" }),
            ),
            ("resolution", Value::Null),
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
            ("kind", Value::text("apart")),
            (
                "quicker",
                Value::text(if *left_quicker { "left" } else { "right" }),
            ),
            ("resolution", Value::Null),
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
            ("kind", Value::text("same")),
            ("quicker", Value::Null),
            ("resolution", parts_per_million(*resolving)),
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
            ("kind", Value::text("unmeasurable")),
            ("quicker", Value::Null),
            ("by_ppm", Value::Null),
        ]),
    }
}

const fn side(held: Side) -> &'static str {
    match held {
        Side::Left => "left",
        Side::Right => "right",
    }
}

fn parts_per_million(held: PartsPerMillion) -> Value {
    Value::Integer(i64::try_from(held.0).unwrap_or(i64::MAX))
}

fn nanos(held: u64) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

fn count(held: usize) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests;
