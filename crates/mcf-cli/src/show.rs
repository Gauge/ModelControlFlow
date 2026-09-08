use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId};
use mcf_record::json::Value;

use crate::Response;

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
    lines.push("  the line in `mcf log`, and this is what is underneath it.".to_owned());
    lines.join("\n")
}

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

fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "— not answered".to_owned(),
        Value::Text(held) => held.clone(),
        other => other.to_line(),
    }
}

#[cfg(test)]
mod tests;
