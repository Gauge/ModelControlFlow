//! `mcf share`: what would leave this machine, shown row by row before
//! anything does (B-160, A24, §3.20, §6.27, D21).
//!
//! **A24 is why this command reads the way it does.** *Nothing leaves this
//! machine except by an explicit act, taken per share, that shows the user the
//! rows that leave rather than a description of them, and that states plainly
//! that the act cannot be undone.* Each clause of that sentence is a thing this
//! surface has to do, and the one most easily lost is the middle: a count of
//! rows is a description, and a person deciding about something irreversible is
//! entitled to read what it says. So every row is printed, in full, and
//! `Contribution` has no rendering that omits them.
//!
//! **Producing is not sending, and MCF cannot send.** There is no destination
//! in this build, no address to configure and no code that opens one —
//! `Gated::Publication` says so, and `the_five_gates.rs` holds the absence
//! against the tree. This command writes a file the operator named. Nothing
//! leaves, and the report says so rather than leaving it to be assumed.
//!
//! **What is refused is said, with the reason.** A row from the operator's own
//! workload cannot travel because nobody else has that workload (B42); an
//! absolute without its full condition floor cannot travel because a bare
//! number from a stranger's machine is uninterpretable (B54). Both refusals
//! come from the row rather than from a judgement made here, and both are
//! printed — a contribution that silently dropped what it could not carry
//! would leave the operator believing they had shared something they had not
//! (A1, A4).

use std::path::{Path, PathBuf};

use mcf_core::contribution::{Comparison, Contribution, TERMS, Workload};
use mcf_core::measurement::PartsPerMillion;
use mcf_core::trial::Arm;
use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryKind};
use mcf_record::json::Value;

use crate::Response;

/// Builds a contribution from what the record holds and shows it.
/// The name of an arm, as a row may carry it.
///
/// **A path is not an outcome.** The record names an arm by the file it ran,
/// which is right for a local record and wrong for a row that leaves: the terms
/// say what travels is *scores, classifications, conditions and effect sizes*,
/// and that no file leaves. A path is none of those, and it carries the
/// operator's user name and the shape of their disk with it —
/// `/home/somebody/.local/share/mcf/models/…` was what a contribution actually
/// held before this existed.
///
/// What travels is the file's own name, which is the model's identity and the
/// only part anybody else can use.
fn publishable(arm: &str) -> Arm {
    // The last part that is actually a name. Falling back to the whole string
    // would have let `/` through unchanged, which is a separator and not a
    // name — found by asking what happens to the degenerate cases.
    let name = arm
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("");
    if name.is_empty() {
        // An arm that names no file names nothing anybody else can use, and
        // saying so is better than passing a separator along (A7).
        return Arm::new("unnamed");
    }
    Arm::new(name)
}

pub(crate) fn run(into: Option<&str>) -> Response {
    let Some(journal) = mcf_record::journal::default_path() else {
        return Response {
            text: "mcf: there is no record to share from — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let index = match Index::over(&journal, &index::default_path(&journal)) {
        Ok(index) => index,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the record could not be read", &failure),
                served: false,
            };
        }
    };

    let wanted = Some(EntryKind::Comparison);
    let mut held = Contribution::empty();
    let mut refused: Vec<String> = Vec::new();
    let mut unreadable = 0_usize;
    for located in index.latest(wanted, index.count_matching(wanted)) {
        let Ok(entry) = index.read(&located) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        match read_comparison(&entry) {
            Some(compared) => match held.clone().and_comparison(compared) {
                Ok(more) => held = more,
                Err(why) => refused.push(format!(
                    "  {} — {why}",
                    entry.id().map_or_else(
                        || "an unidentified row".to_owned(),
                        |id| id.as_str().to_owned()
                    )
                )),
            },
            // A comparison whose outcome has no size is not a row: there is
            // nothing to contribute about it, and saying so beats inventing a
            // number (A9, A7).
            None => refused.push(format!(
                "  {} — no established size to contribute: the arms did not separate, or the \
                 comparison was confounded, or the size was not established at the resolution \
                 asked about (A8, F92)",
                entry.id().map_or_else(
                    || "an unidentified row".to_owned(),
                    |id| id.as_str().to_owned()
                )
            )),
        }
    }

    let to = destination(into);
    render(&held, &refused, unreadable, &to, write(&held, &to))
}

/// Writes the file, and says what happened.
fn write(held: &Contribution, to: &Path) -> Result<usize, String> {
    if held.rows().is_empty() {
        return Ok(0);
    }
    if let Some(parent) = to.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return Err(format!("{} could not be made: {error}", parent.display()));
    }
    // The rows as they are shown, so that what the operator read and what the
    // file holds are the same text. A file whose contents differ from the
    // confirmation is a confirmation of something else (A24).
    match std::fs::write(to, format!("{held}\n")) {
        Ok(()) => Ok(held.rows().len()),
        Err(error) => Err(format!("{} could not be written: {error}", to.display())),
    }
}

/// Where the file goes.
fn destination(into: Option<&str>) -> PathBuf {
    into.map_or_else(|| PathBuf::from("contribution.mcf"), PathBuf::from)
}

/// One comparison from the record, where it is one.
///
/// Reads the record rather than recomputing: the row that would travel is the
/// row that was recorded, and a second computation here would be a second
/// opinion about what happened (D20, C1).
fn read_comparison(entry: &Entry) -> Option<Comparison> {
    let body = entry.body();
    let outcome = body.get("outcome")?;
    // Only an established size travels. `same`, `not_comparable`, `ordered`
    // and `not_yet` are all real results and none of them is a size (A9, A8).
    if outcome.get("kind").and_then(Value::as_text)? != "differ" {
        return None;
    }
    let effect = PartsPerMillion(
        u64::try_from(outcome.get("difference").and_then(Value::as_integer)?).ok()?,
    );
    let pairs = usize::try_from(outcome.get("pairs").and_then(Value::as_integer)?).ok()?;
    let left_quicker = outcome.get("quicker").and_then(Value::as_text)? == "left";
    let left = publishable(body.get("left")?.get("arm").and_then(Value::as_text)?);
    let right = publishable(body.get("right")?.get("arm").and_then(Value::as_text)?);
    Some(Comparison {
        left,
        right,
        pairs,
        effect,
        left_quicker,
        conditions: mcf_record::encode::conditions_from(body.get("left")?.get("conditions")?)?,
        // The record does not yet carry where a benchmark's workload came from,
        // and a share that assumed *declared* would be assuming the one thing
        // B42 says decides whether a row may travel. Until the row says, this
        // reads the safe way: the operator's own, which refuses (A7, B-203).
        workload: workload_of(body),
    })
}

/// Where the workload came from, as the row says — and the operator's own where
/// the row does not say.
fn workload_of(body: &Value) -> Workload {
    match body
        .get("method")
        .and_then(|method| method.get("workload"))
        .and_then(Value::as_text)
    {
        Some("declared") => Workload::Declared,
        _ => Workload::Custom,
    }
}

/// What the operator reads before deciding.
fn render(
    held: &Contribution,
    refused: &[String],
    unreadable: usize,
    to: &Path,
    written: Result<usize, String>,
) -> Response {
    let mut lines =
        vec!["── what would leave this machine ────────────────────────────".to_owned()];
    if held.rows().is_empty() {
        lines.push("  nothing: no row in this record can travel".to_owned());
    } else {
        // Every row, in full. A24: the rows themselves, never a description of
        // them.
        for line in held.to_string().lines() {
            lines.push(format!("  {line}"));
        }
    }
    lines.push(String::new());

    if !refused.is_empty() {
        lines.push(format!(
            "── what cannot travel, and why ({} row(s)) ───────────────────",
            refused.len()
        ));
        lines.extend(refused.iter().cloned());
        lines.push(String::new());
    }
    if unreadable > 0 {
        lines.push(format!(
            "  {unreadable} recorded row(s) could not be read, and are neither included nor \
             counted above (A4)"
        ));
        lines.push(String::new());
    }

    lines.push("── the terms ────────────────────────────────────────────────".to_owned());
    for sentence in TERMS.split(". ") {
        lines.push(format!("  {}", sentence.trim()));
    }
    lines.push(String::new());
    lines.push("  NOTHING HAS LEFT THIS MACHINE. MCF has no destination, no address to".to_owned());
    lines.push("  configure and no path that opens one: this wrote a file you named,".to_owned());
    lines.push("  and sending it is your act and not MCF's (A24, §3.20).".to_owned());
    lines.push(String::new());

    match written {
        Ok(0) => lines.push("  no file was written: there was nothing to write".to_owned()),
        Ok(rows) => lines.push(format!("  wrote {} — {rows} row(s)", to.display())),
        Err(why) => {
            lines.push(format!("  NOT WRITTEN — {why}"));
            return Response {
                text: lines.join("\n"),
                served: false,
            };
        }
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}

#[cfg(test)]
mod tests;
