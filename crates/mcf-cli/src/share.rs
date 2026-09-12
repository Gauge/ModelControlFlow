use std::path::{Path, PathBuf};

use mcf_core::contribution::{Comparison, Contribution, TERMS, Workload};
use mcf_core::measurement::PartsPerMillion;
use mcf_core::trial::Arm;
use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryKind};
use mcf_record::json::Value;

use crate::Response;

fn publishable(arm: &str) -> Arm {
    let name = arm
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or("");
    if name.is_empty() {
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

fn write(held: &Contribution, to: &Path) -> Result<usize, String> {
    if held.rows().is_empty() {
        return Ok(0);
    }
    if let Some(parent) = to.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return Err(format!("{} could not be made: {error}", parent.display()));
    }
    match std::fs::write(to, format!("{held}\n")) {
        Ok(()) => Ok(held.rows().len()),
        Err(error) => Err(format!("{} could not be written: {error}", to.display())),
    }
}

fn destination(into: Option<&str>) -> PathBuf {
    into.map_or_else(|| PathBuf::from("contribution.mcf"), PathBuf::from)
}

fn read_comparison(entry: &Entry) -> Option<Comparison> {
    let body = entry.body();
    let outcome = body.get("outcome")?;
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
        workload: workload_of(body),
    })
}

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
    lines.push("  and sending it is your act and not MCF's.".to_owned());
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
