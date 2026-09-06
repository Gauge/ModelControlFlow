//! Determinism: whether a figure on this model is comparable with itself
//! (B-496, D52, §3.12).
//!
//! The embedding probe asks for the same vector twice; nothing asked the
//! same of a generation. The same prompt, the same seed, greedy, five
//! times on one server, then once each under another thread count and
//! another batch size: how many runs match the first token for token,
//! and where the first that does not parts from it. A divergence here is
//! arithmetic summing in a different order, and it is the floor under
//! every comparison taken on this engine.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer};
use crate::generation::Draw;
use crate::served::{Prompt, Served, Startup};

/// The measurement's name.
pub const NAME: &str = "determinism";

/// How many runs on the first server.
const REPEATS: usize = 5;

/// How many tokens each run produces, at most.
const PRODUCE: usize = 64;

/// The batch the other server is started with.
const OTHER_BATCH: u32 = 256;

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let (prompt, runs) = match repeated(site) {
        Ok(runs) => runs,
        Err(why) => return Found::could_not_tell(&why),
    };
    let Some(first) = runs.first().cloned() else {
        return Found::could_not_tell("no run produced anything");
    };
    let identical = runs.iter().filter(|run| **run == first).count();
    let first_divergence = runs.iter().find_map(|run| divergence_at(&first, run));
    // How many different sequences the repeats produced, and whether the
    // runs after the first agree among themselves: a first request that
    // stands alone while every later one agrees is the engine settling
    // after a start, which is a different fact from runs that scatter.
    let distinct = distinct_of(&runs);
    let later_agree = runs.get(1..).is_some_and(|later| distinct_of(later) <= 1);
    let cores = std::thread::available_parallelism().map_or(2, std::num::NonZeroUsize::get);
    #[expect(
        clippy::integer_division,
        reason = "half the cores, floored, at least one"
    )]
    let other_threads = u32::try_from((cores / 2).max(1)).unwrap_or(1);
    let under_threads = another(
        site,
        &prompt,
        &first,
        &Startup {
            threads: Some(other_threads),
            projector: None,
            ..site.startup()
        },
    );
    let under_batch = another(
        site,
        &prompt,
        &first,
        &Startup {
            batch: Some(OTHER_BATCH),
            ubatch: Some(OTHER_BATCH),
            projector: None,
            ..site.startup()
        },
    );
    let (threads_same, threads_at) = as_fields(&under_threads);
    let (batch_same, batch_at) = as_fields(&under_batch);
    let mut rows = Vec::new();
    for (run, words) in runs.iter().enumerate() {
        let dims = [("run", Value::Integer(as_integer(run)))];
        rows.push(Reading::new(
            &dims,
            "produced",
            as_integer(words.len()),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "identical_to_first",
            i64::from(*words == first),
            "bool",
        ));
        if let Some(at) = divergence_at(&first, words) {
            rows.push(Reading::new(
                &dims,
                "divergence_at",
                as_integer(at),
                "tokens",
            ));
        }
    }
    for (setting, held) in [("threads", &under_threads), ("batch", &under_batch)] {
        let dims = [("setting", Value::text(setting))];
        if let Ok(parted) = held {
            rows.push(Reading::new(
                &dims,
                "identical_to_first",
                i64::from(parted.is_none()),
                "bool",
            ));
            if let Some(at) = parted {
                rows.push(Reading::new(
                    &dims,
                    "divergence_at",
                    as_integer(*at),
                    "tokens",
                ));
            }
        }
    }
    Found {
        lines: vec![
            format!(
                "  \"{}\", greedy from seed 0, up to {PRODUCE} tokens; the first run produced {}",
                crate::crosscheck::PROMPT,
                first.len()
            ),
            format!(
                "  {identical} of {REPEATS} run(s) on one server identical to the first{}",
                first_divergence.map_or_else(String::new, |at| format!(
                    "; the first that differed parted at token {at}"
                ))
            ),
            format!(
                "  {distinct} distinct sequence(s) in all{}",
                if identical < REPEATS && later_agree {
                    "; the runs after the first agree with each other, so the first request \
                     after a start stands alone"
                } else if identical < REPEATS {
                    "; the runs after the first differ among themselves"
                } else {
                    ""
                }
            ),
            format!(
                "  under {other_threads} thread(s)   {}",
                said(&under_threads)
            ),
            format!("  under batch {OTHER_BATCH}     {}", said(&under_batch)),
        ],
        fields: vec![
            ("prompt", Value::text(crate::crosscheck::PROMPT.to_owned())),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("produced", Value::Integer(as_integer(first.len()))),
            ("repeats", Value::Integer(as_integer(REPEATS))),
            ("identical", Value::Integer(as_integer(identical))),
            ("distinct", Value::Integer(as_integer(distinct))),
            ("later_agree", Value::Bool(later_agree)),
            (
                "first_divergence",
                first_divergence.map_or(Value::Null, |at| Value::Integer(as_integer(at))),
            ),
            ("other_threads", Value::Integer(i64::from(other_threads))),
            ("other_threads_identical", threads_same),
            ("other_threads_divergence", threads_at),
            ("other_batch", Value::Integer(i64::from(OTHER_BATCH))),
            ("other_batch_identical", batch_same),
            ("other_batch_divergence", batch_at),
        ],
        rows,
    }
}

/// A run under another setting, in words.
fn said(held: &Result<Option<usize>, String>) -> String {
    match held {
        Ok(None) => "identical".to_owned(),
        Ok(Some(at)) => format!("diverged at token {at}"),
        Err(why) => format!("not measured: {why}"),
    }
}

/// The same, as the record's two fields: whether it was identical, and
/// where it parted.
fn as_fields(held: &Result<Option<usize>, String>) -> (Value, Value) {
    match held {
        Ok(None) => (Value::Bool(true), Value::Null),
        Ok(Some(at)) => (Value::Bool(false), Value::Integer(as_integer(*at))),
        Err(_) => (Value::Null, Value::Null),
    }
}

/// The prompt's identifiers and the repeats on one server.
fn repeated(site: &Site<'_>) -> Result<(Vec<usize>, Vec<Vec<usize>>), String> {
    let engine = site.server(&Startup {
        projector: None,
        ..site.startup()
    })?;
    let prompt: Vec<usize> = engine
        .tokenize(crate::crosscheck::PROMPT, true, false)
        .map_err(|failure| failure.detail().to_owned())?
        .into_iter()
        .map(|token| token.id)
        .collect();
    let mut runs: Vec<Vec<usize>> = Vec::with_capacity(REPEATS);
    for run in 0..REPEATS {
        site.progress(run, REPEATS, "run");
        if site.asker_gone() {
            return Err(crate::served::CLIENT_LEFT.to_owned());
        }
        runs.push(one_run(&engine, &prompt, site)?);
    }
    Ok((prompt, runs))
}

fn one_run(engine: &Served, prompt: &[usize], site: &Site<'_>) -> Result<Vec<usize>, String> {
    engine
        .complete(
            Prompt::Identifiers(prompt),
            PRODUCE,
            Draw::greedy(0),
            false,
            site.waiting,
        )
        .map(|completed| completed.words().to_vec())
        .map_err(|failure| failure.detail().to_owned())
}

/// One run on a server started another way, against the first run:
/// `None` where they match, else where they part.
fn another(
    site: &Site<'_>,
    prompt: &[usize],
    first: &[usize],
    startup: &Startup,
) -> Result<Option<usize>, String> {
    if site.asker_gone() {
        return Err(crate::served::CLIENT_LEFT.to_owned());
    }
    let engine = site.server(startup)?;
    let words = one_run(&engine, prompt, site)?;
    Ok(divergence_at(first, &words))
}

/// How many different sequences a set of runs holds.
pub(crate) fn distinct_of(runs: &[Vec<usize>]) -> usize {
    runs.iter()
        .collect::<std::collections::BTreeSet<&Vec<usize>>>()
        .len()
}

/// Where two runs part: the first position whose tokens differ, or the
/// shorter one's length where one is a prefix of the other; `None` where
/// they are the same.
pub(crate) fn divergence_at(first: &[usize], other: &[usize]) -> Option<usize> {
    if first == other {
        return None;
    }
    Some(
        first
            .iter()
            .zip(other)
            .position(|(left, right)| left != right)
            .unwrap_or_else(|| first.len().min(other.len())),
    )
}
