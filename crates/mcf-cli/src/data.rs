//! `mcf data`: a model's readings as a table (D54, B-515).
//!
//! **The rows are what a person compares models by.** Every diagnostic
//! writes its findings as readings — one row a figure, with the
//! dimensions it was taken under, in one schema — and this is the
//! command that hands them over as a table any tool can load:
//! comma-separated by default, one row a line, the dimensions as columns,
//! newest run first; or JSON lines, one row an object, where a program is
//! the reader. Nothing is summarized on the way (D16): a median is the
//! reader's arithmetic over the rows.

use mcf_record::json::Value;
use mcf_record::readings::{Reading, csv_cell, dims_of, rows_of};
use mcf_serve::control::Request;

use crate::Response;
use crate::hosting::ask;
use crate::run::{ambiguous, resolve};

/// The columns every row has before its dimensions.
const FIXED: [&str; 3] = ["method", "taken_at", "engine"];

/// The columns every row has after them.
const FIGURE: [&str; 3] = ["metric", "value", "unit"];

/// Writes a model's readings as a table.
pub(crate) fn run(model: &str, method: Option<&str>, as_json: bool) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };
    let answered = match ask(&Request::Readings {
        model: path.display().to_string(),
        method: method.map(str::to_owned),
    }) {
        Ok(body) => body,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let runs = answered.get("runs").and_then(Value::as_list).unwrap_or(&[]);
    if runs.is_empty() {
        return Response {
            text: format!(
                "no readings of {} yet{}; `mcf examine` and `mcf probe` write them",
                path.display(),
                method.map_or_else(String::new, |method| format!(" under {method}"))
            ),
            served: true,
        };
    }
    Response {
        text: if as_json {
            as_json_lines(runs)
        } else {
            as_csv(runs)
        },
        served: true,
    }
}

/// What a run's rows share: the method, when, and the engine.
fn shared(run: &Value) -> (String, String, String) {
    let text = |key: &str| {
        run.get(key)
            .and_then(Value::as_text)
            .map(str::to_owned)
            .unwrap_or_default()
    };
    // The moment alone: the record writes the local offset in words after
    // it, which is not a cell a table can sort.
    let at = text("at");
    let at = at.split(' ').next().unwrap_or_default().to_owned();
    (text("method"), at, text("engine"))
}

/// Every run's rows as one table with the union of their dimensions.
fn as_csv(runs: &[Value]) -> String {
    let all: Vec<(String, String, String, Vec<Reading>)> = runs
        .iter()
        .map(|run| {
            let (method, at, engine) = shared(run);
            (method, at, engine, rows_of(run))
        })
        .collect();
    let every: Vec<Reading> = all
        .iter()
        .flat_map(|(_, _, _, rows)| rows.clone())
        .collect();
    let dims = dims_of(&every);
    let mut out = String::new();
    let mut head: Vec<String> = FIXED.iter().map(|held| (*held).to_owned()).collect();
    head.extend(dims.iter().cloned());
    head.extend(FIGURE.iter().map(|held| (*held).to_owned()));
    out.push_str(&head.join(","));
    out.push('\n');
    for (method, at, engine, rows) in &all {
        for row in rows {
            let mut cells = vec![csv_cell(method), csv_cell(at), csv_cell(engine)];
            cells.extend(dims.iter().map(|dim| csv_cell(&row.dim(dim))));
            cells.push(csv_cell(&row.metric));
            cells.push(row.value.to_string());
            cells.push(csv_cell(&row.unit));
            out.push_str(&cells.join(","));
            out.push('\n');
        }
    }
    out.trim_end().to_owned()
}

/// Every row as one JSON object a line, with the run's method, time and
/// engine on each so a line stands on its own.
fn as_json_lines(runs: &[Value]) -> String {
    let mut lines = Vec::new();
    for run in runs {
        let (method, at, engine) = shared(run);
        for row in rows_of(run) {
            let mut object = row.to_value();
            if let Value::Map(fields) = &mut object {
                let _m = fields.insert("method".to_owned(), Value::text(method.clone()));
                let _a = fields.insert("taken_at".to_owned(), Value::text(at.clone()));
                let _e = fields.insert("engine".to_owned(), Value::text(engine.clone()));
            }
            lines.push(object.to_line());
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use mcf_record::json::Value;
    use mcf_record::readings::{Reading, run_body};

    /// The table has the fixed columns, the union of the runs' dimensions,
    /// and the figure; a run's rows carry the run's method and time.
    #[test]
    fn the_table_is_one_row_a_reading() {
        let mut first = run_body(
            "/m",
            "prefill-saturation",
            "provisioned",
            vec![],
            &[Reading::new(
                &[("batch", Value::Integer(64))],
                "ns",
                405_000_000,
                "ns",
            )],
        );
        if let Value::Map(fields) = &mut first {
            let _at = fields.insert(
                "at".to_owned(),
                Value::text("2026-09-05T15:31:04Z (local offset +00:00)"),
            );
        }
        let second = run_body(
            "/m",
            "retrieval-by-depth",
            "provisioned",
            vec![],
            &[Reading::new(
                &[
                    ("depth", Value::Integer(1024)),
                    ("placement", Value::Integer(50)),
                ],
                "found",
                1,
                "bool",
            )],
        );
        let csv = super::as_csv(&[first.clone(), second.clone()]);
        let mut lines = csv.lines();
        assert_eq!(
            lines.next(),
            Some("method,taken_at,engine,batch,depth,placement,metric,value,unit")
        );
        assert_eq!(
            lines.next(),
            Some("prefill-saturation,2026-09-05T15:31:04Z,provisioned,64,,,ns,405000000,ns")
        );
        assert_eq!(
            lines.next(),
            Some("retrieval-by-depth,,provisioned,,1024,50,found,1,bool")
        );
        let json = super::as_json_lines(&[first]);
        assert!(json.contains("\"method\":\"prefill-saturation\""), "{json}");
        assert!(json.contains("\"value\":405000000"), "{json}");
    }
}
