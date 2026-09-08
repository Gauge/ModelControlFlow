use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "degeneration";

pub const PRODUCE: usize = 1024;

pub const ASK: &str = "Write a long story about a lighthouse keeper who finds a message in a \
bottle. Keep going for as long as you can.";

const RUN: usize = 4;

const BLOCK: usize = 32;

#[must_use]
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
    let completed = match engine.complete(
        Prompt::Identifiers(&ids),
        PRODUCE,
        Draw::greedy(0),
        false,
        site.waiting,
    ) {
        Ok(completed) => completed,
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let words = completed.words();
    if words.is_empty() {
        return Found::could_not_tell("the model produced nothing");
    }
    let hundreds = repeated_by_hundred(words);
    let loop_at = loop_onset(words);
    let mut rows = vec![Reading::new(
        &[],
        "produced",
        as_integer(words.len()),
        "tokens",
    )];
    for (hundred, ppm) in hundreds.iter().enumerate() {
        rows.push(Reading::new(
            &[("hundred", Value::Integer(as_integer(hundred)))],
            "repeated_runs_ppm",
            *ppm,
            "ppm",
        ));
    }
    if let Some(at) = loop_at {
        rows.push(Reading::new(&[], "loop_at", as_integer(at), "tokens"));
    }
    let mut lines = vec![format!(
        "  {} token(s) produced of {PRODUCE} allowed, greedy; {}",
        words.len(),
        match completed.stop {
            crate::served::Stop::Eos => "the model ended its turn".to_owned(),
            crate::served::Stop::Limit => "cut at the budget".to_owned(),
            ref other => format!("stopped: {other:?}"),
        }
    )];
    let shares: Vec<String> = hundreds.iter().map(|ppm| super::per_cent(*ppm)).collect();
    lines.push(format!(
        "  repeated {RUN}-token runs by hundred: {}",
        shares.join("  ")
    ));
    lines.push(loop_at.map_or_else(
        || format!("  no block of {BLOCK} tokens repeated an earlier one exactly"),
        |at| format!("  a block of {BLOCK} tokens first repeats an earlier one at token {at}"),
    ));
    Found {
        lines,
        fields: vec![
            ("ask", Value::text(ASK.to_owned())),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("produced", Value::Integer(as_integer(words.len()))),
            (
                "stop",
                Value::text(format!("{:?}", completed.stop).to_ascii_lowercase()),
            ),
            (
                "repeated_ppm_by_hundred",
                Value::List(hundreds.into_iter().map(Value::Integer).collect()),
            ),
            (
                "loop_at",
                loop_at.map_or(Value::Null, |at| Value::Integer(as_integer(at))),
            ),
        ],
        rows,
    }
}

pub(crate) fn repeated_by_hundred(words: &[usize]) -> Vec<i64> {
    words
        .chunks(100)
        .map(|hundred| {
            let runs: Vec<&[usize]> = hundred.windows(RUN).collect();
            if runs.is_empty() {
                return 0;
            }
            let mut seen: std::collections::BTreeSet<&[usize]> = std::collections::BTreeSet::new();
            let repeated = runs.iter().filter(|run| !seen.insert(run)).count();
            super::ppm(repeated as u64, runs.len() as u64)
        })
        .collect()
}

pub(crate) fn loop_onset(words: &[usize]) -> Option<usize> {
    let mut seen: std::collections::BTreeSet<&[usize]> = std::collections::BTreeSet::new();
    words
        .windows(BLOCK)
        .position(|block| !seen.insert(block))
        .map(|at| at + BLOCK)
}
