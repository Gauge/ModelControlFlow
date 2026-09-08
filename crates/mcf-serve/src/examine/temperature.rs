use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::{Draw, Truncation};
use crate::served::{Prompt, Startup};
use mcf_core::configuration::Thousandths;

pub const NAME: &str = "temperature";

pub const TEMPERATURES: [u32; 5] = [0, 300, 700, 1000, 1500];

pub const DRAWS: usize = 6;

const BUDGET: usize = 60;

pub const QUESTIONS: &[(&str, &str, i64)] = &[
    (
        "twelve-times-twelve",
        "What is 12 times 12? Answer with the number only.",
        144,
    ),
    (
        "hours-in-a-week",
        "How many hours are there in a week? Answer with the number only.",
        168,
    ),
    (
        "fifty-minus-seventeen",
        "What is 50 minus 17? Answer with the number only.",
        33,
    ),
];

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each question at each temperature, each draw a row"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} question(s), {DRAWS} draw(s) each at temperatures {}; the answer read as the \
         paraphrase measurement reads it",
        QUESTIONS.len(),
        TEMPERATURES
            .iter()
            .map(|t| super::milli_said(u64::from(*t)))
            .collect::<Vec<String>>()
            .join(", ")
    )];
    let mut by_temperature: Vec<(usize, usize)> = vec![(0, 0); TEMPERATURES.len()];
    for (at, (name, asks, answer)) in QUESTIONS.iter().enumerate() {
        site.progress(at, QUESTIONS.len(), name);
        let ids = match framed_ids(&engine, asks) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let mut said = Vec::with_capacity(TEMPERATURES.len());
        for (at, temperature) in TEMPERATURES.iter().enumerate() {
            let mut answers: Vec<Option<i64>> = Vec::with_capacity(DRAWS);
            for draw in 0..DRAWS {
                if site.asker_gone() {
                    return Found::could_not_tell(crate::served::CLIENT_LEFT);
                }
                let seed = u64::try_from(draw).unwrap_or(0);
                let drawn = if *temperature == 0 {
                    Draw::greedy(seed)
                } else {
                    Draw {
                        seed,
                        temperature: Thousandths(*temperature),
                        truncation: Truncation::OFF,
                    }
                };
                let completed = match engine.complete(
                    Prompt::Identifiers(&ids),
                    BUDGET,
                    drawn,
                    false,
                    site.timed(),
                ) {
                    Ok(completed) => completed,
                    Err(failure) => return Found::could_not_tell(failure.detail()),
                };
                let read = answer_in(&completed.text);
                answers.push(read);
                let dims = [
                    ("question", Value::text(*name)),
                    ("temperature", Value::Integer(i64::from(*temperature))),
                    ("draw", Value::Integer(as_integer(draw))),
                ];
                rows.push(Reading::new(
                    &dims,
                    "read",
                    i64::from(read.is_some()),
                    "bool",
                ));
                if let Some(read) = read {
                    rows.push(Reading::new(&dims, "answer", read, "count"));
                }
                rows.push(Reading::new(
                    &dims,
                    "right",
                    i64::from(read == Some(*answer)),
                    "bool",
                ));
                rows.push(Reading::new(
                    &dims,
                    "tokens",
                    as_integer(completed.predicted),
                    "tokens",
                ));
            }
            let distinct = answers
                .iter()
                .flatten()
                .collect::<std::collections::BTreeSet<&i64>>()
                .len();
            let right = answers
                .iter()
                .filter(|held| **held == Some(*answer))
                .count();
            if let Some((all_distinct, all_right)) = by_temperature.get_mut(at) {
                *all_distinct = all_distinct.saturating_add(distinct);
                *all_right = all_right.saturating_add(right);
            }
            let dims = [
                ("question", Value::text(*name)),
                ("temperature", Value::Integer(i64::from(*temperature))),
            ];
            rows.push(Reading::new(
                &dims,
                "distinct",
                as_integer(distinct),
                "count",
            ));
            rows.push(Reading::new(&dims, "right", as_integer(right), "count"));
            rows.push(Reading::new(&dims, "draws", as_integer(DRAWS), "count"));
            said.push(format!("{distinct}/{right}"));
        }
        lines.push(format!(
            "  {name:<24} distinct/right at each: {}",
            said.join("   ")
        ));
    }
    let mut fields = vec![
        ("questions", Value::Integer(as_integer(QUESTIONS.len()))),
        ("draws", Value::Integer(as_integer(DRAWS))),
    ];
    let mut summary = Vec::with_capacity(TEMPERATURES.len());
    for (temperature, (distinct, right)) in TEMPERATURES.iter().zip(&by_temperature) {
        summary.push(format!(
            "at {} {distinct} distinct, {right} right",
            super::milli_said(u64::from(*temperature))
        ));
        rows.push(Reading::new(
            &[("temperature", Value::Integer(i64::from(*temperature)))],
            "right",
            as_integer(*right),
            "count",
        ));
    }
    lines.push(format!("  over the questions: {}", summary.join("; ")));
    if let (Some((_, coldest)), Some((_, hottest))) =
        (by_temperature.first(), by_temperature.last())
    {
        fields.push(("right_at_coldest", Value::Integer(as_integer(*coldest))));
        fields.push(("right_at_hottest", Value::Integer(as_integer(*hottest))));
    }
    fields.push((
        "asked_each",
        Value::Integer(as_integer(QUESTIONS.len().saturating_mul(DRAWS))),
    ));
    Found {
        lines,
        fields,
        rows,
    }
}
