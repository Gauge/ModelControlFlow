//! `mcf show <id>`: a statement, expanded into what it rests on (B-252, B55,
//! §3.28, §3.15).
//!
//! **B55's rule.** *Generalization happens in the rendering only: the record
//! keeps the precise measurement, and any generalized statement expands on
//! demand into the measurements, conditions and spread behind it.* Its
//! violation is *"41.2 vs 38.4 tok/s" offered as a recommendation*, which is
//! false precision inviting action on a difference inside the noise — and the
//! other half of the same failure is a generalization with no way back to the
//! numbers.
//!
//! `mcf log` prints one line an event: *the left arm is quicker by 26.1%*.
//! That is a generalization, and it is honest only if a reader can get from it
//! to every trial, every condition and every unanswered question in one step
//! **without leaving the interface** (A22: the headless surface is the
//! complete one). This is that step.
//!
//! **`--full` was not it.** `mcf log --full` prints the record's own JSON,
//! which is what a script reads and what `mcf export` sends — machine-readable
//! first, as §3.3 ranks it. Second is not omitted: a human asking *what is
//! that number made of* was being handed nine hundred characters of one line.
//!
//! **What is expanded is what the record holds**, which is everything: a
//! comparison keeps every pair's two raw durations, which arm ran first, what
//! each drew, both arms' full condition floors including the questions nobody
//! could answer, and what the run reused. Nothing here is recomputed and
//! nothing here is summarized — the summary is the line in `mcf log`, and this
//! is underneath it.

use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId};
use mcf_record::json::Value;

use crate::Response;

/// Expands one recorded entry into its evidence.
pub(crate) fn run(wanted: &str) -> Response {
    let Some(path) = mcf_record::journal::default_path() else {
        return Response {
            text: "mcf: there is no record to read — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let index = match Index::over(&path, &index::default_path(&path)) {
        Ok(index) => index,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the record could not be read", &failure),
                served: false,
            };
        }
    };

    // Every entry, newest first, until the identifier matches. The index knows
    // kinds and offsets and not identifiers, so this reads rather than seeks —
    // which is the honest cost of asking for one entry by name, and is stated
    // here rather than hidden behind a second index nobody asked for (B15).
    let mut looked = 0_usize;
    for located in index.latest(None, index.count_matching(None)) {
        let Ok(entry) = index.read(&located) else {
            continue;
        };
        looked = looked.saturating_add(1);
        if entry.id().map(EntryId::as_str) == Some(wanted) {
            return Response {
                text: expand(&entry),
                served: true,
            };
        }
    }
    Response {
        text: format!(
            "mcf: no entry called {wanted} in {}\n  {looked} entries were read; `mcf log` lists \
             what is there, and an identifier is the first field of each line",
            path.display()
        ),
        served: false,
    }
}

/// One entry, in full and legibly.
fn expand(entry: &Entry) -> String {
    let body = entry.body();
    let mut lines = vec![
        entry
            .id()
            .map_or("(unidentified)", EntryId::as_str)
            .to_owned(),
        format!("  kind      {}", entry.kind()),
        format!("  at        {}", entry.recorded_at()),
        String::new(),
        format!("  {}", crate::log::summarize(entry)),
        String::new(),
    ];
    lines.push("── what that rests on ───────────────────────────────────────".to_owned());
    lines.extend(unfolded(body, 2));
    lines.push(String::new());
    lines
        .push("  Nothing above is recomputed and nothing is summarized: the summary is".to_owned());
    lines.push("  the line in `mcf log`, and this is what is underneath it (B55).".to_owned());
    lines.join("\n")
}

/// A recorded value, one field a line, nested by indentation.
///
/// A list of maps — which is what a comparison's pairs are — is numbered, so
/// that *the seventh pair* is a thing a reader can point at. `null` is printed
/// rather than skipped: a question asked and unanswered is not the same as a
/// question nobody asked, and dropping the nulls would erase the difference
/// (A7).
fn unfolded(value: &Value, depth: usize) -> Vec<String> {
    let pad = " ".repeat(depth);
    match value {
        Value::Map(fields) => fields
            .iter()
            .flat_map(|(name, held)| match held {
                Value::Map(_) | Value::List(_) => {
                    let mut out = vec![format!("{pad}{name}")];
                    out.extend(unfolded(held, depth.saturating_add(2)));
                    out
                }
                _ => vec![format!("{pad}{name:<24}{}", scalar(held))],
            })
            .collect(),
        Value::List(held) => held
            .iter()
            .enumerate()
            .flat_map(|(at, one)| match one {
                Value::Map(_) | Value::List(_) => {
                    let mut out = vec![format!("{pad}#{at}")];
                    out.extend(unfolded(one, depth.saturating_add(2)));
                    out
                }
                _ => vec![format!("{pad}#{at:<23}{}", scalar(one))],
            })
            .collect(),
        _ => vec![format!("{pad}{}", scalar(value))],
    }
}

/// A scalar as a reader wants it, with `null` said rather than blank.
fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "— not answered".to_owned(),
        Value::Text(held) => held.clone(),
        other => other.to_line(),
    }
}

#[cfg(test)]
mod tests;
