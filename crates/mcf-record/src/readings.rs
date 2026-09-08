use std::collections::BTreeMap;

use crate::json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub dims: BTreeMap<String, Value>,
    pub metric: String,
    pub value: i64,
    pub unit: String,
}

impl Reading {
    #[must_use]
    pub fn new(dims: &[(&str, Value)], metric: &str, value: i64, unit: &str) -> Self {
        Self {
            dims: dims
                .iter()
                .map(|(key, held)| ((*key).to_owned(), held.clone()))
                .collect(),
            metric: metric.to_owned(),
            value,
            unit: unit.to_owned(),
        }
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("dims", Value::Map(self.dims.clone())),
            ("metric", Value::text(self.metric.clone())),
            ("value", Value::Integer(self.value)),
            ("unit", Value::text(self.unit.clone())),
        ])
    }

    #[must_use]
    pub fn from_value(held: &Value) -> Option<Self> {
        let dims = match held.get("dims") {
            Some(Value::Map(dims)) => dims.clone(),
            Some(Value::Null) | None => BTreeMap::new(),
            Some(_) => return None,
        };
        Some(Self {
            dims,
            metric: held.get("metric")?.as_text()?.to_owned(),
            value: held.get("value")?.as_integer()?,
            unit: held.get("unit")?.as_text()?.to_owned(),
        })
    }

    #[must_use]
    pub fn dim(&self, key: &str) -> String {
        match self.dims.get(key) {
            Some(Value::Integer(held)) => held.to_string(),
            Some(Value::Text(held)) => held.clone(),
            Some(Value::Bool(held)) => u8::from(*held).to_string(),
            Some(other) => other.to_line(),
            None => String::new(),
        }
    }
}

#[must_use]
pub fn run_body(
    model: &str,
    method: &str,
    engine: &str,
    conditions: Vec<(&str, Value)>,
    rows: &[Reading],
) -> Value {
    Value::map([
        ("model", Value::text(model.to_owned())),
        ("method", Value::text(method.to_owned())),
        ("engine", Value::text(engine.to_owned())),
        ("conditions", Value::map(conditions)),
        (
            "rows",
            Value::List(rows.iter().map(Reading::to_value).collect()),
        ),
    ])
}

#[must_use]
#[allow(clippy::too_many_arguments, reason = "one part's fields, each named")]
pub fn part_body(
    model: &str,
    method: &str,
    engine: &str,
    conditions: Vec<(&str, Value)>,
    rows: &[Reading],
    run: &str,
    part: u64,
    ended: Option<&str>,
) -> Value {
    let mut body = run_body(model, method, engine, conditions, rows);
    if let Value::Map(fields) = &mut body {
        let _run = fields.insert("run".to_owned(), Value::text(run.to_owned()));
        let _part = fields.insert(
            "part".to_owned(),
            Value::Integer(i64::try_from(part).unwrap_or(i64::MAX)),
        );
        if let Some(ended) = ended {
            let _ended = fields.insert("ended".to_owned(), Value::text(ended.to_owned()));
        }
    }
    body
}

#[must_use]
pub fn merged(bodies: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut place: BTreeMap<String, usize> = BTreeMap::new();
    for body in bodies {
        let Some(run) = body.get("run").and_then(Value::as_text).map(str::to_owned) else {
            out.push(body);
            continue;
        };
        if let Some(at) = place.get(&run).copied() {
            let more = rows_of(&body);
            let engine = body
                .get("engine")
                .and_then(Value::as_text)
                .map(str::to_owned);
            let ended = body
                .get("ended")
                .and_then(Value::as_text)
                .map(str::to_owned);
            if let Some(Value::Map(fields)) = out.get_mut(at) {
                if let Some(Value::List(rows)) = fields.get_mut("rows") {
                    rows.extend(more.iter().map(Reading::to_value));
                }
                if let Some(engine) = engine.filter(|held| !held.is_empty()) {
                    let _engine = fields.insert("engine".to_owned(), Value::text(engine));
                }
                if let Some(ended) = ended {
                    let _ended = fields.insert("ended".to_owned(), Value::text(ended));
                }
                let _parts = fields.remove("part");
            }
            continue;
        }
        let _at = place.insert(run, out.len());
        let mut first = body;
        if let Value::Map(fields) = &mut first {
            let _part = fields.remove("part");
        }
        out.push(first);
    }
    out
}

#[must_use]
pub fn ended_of(body: &Value) -> Option<String> {
    body.get("ended")
        .and_then(Value::as_text)
        .map(str::to_owned)
}

#[must_use]
pub fn in_parts(body: &Value) -> bool {
    body.get("run").and_then(Value::as_text).is_some()
}

#[must_use]
pub fn rows_of(body: &Value) -> Vec<Reading> {
    body.get("rows")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(Reading::from_value)
        .collect()
}

#[must_use]
pub fn dims_of(rows: &[Reading]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for row in rows {
        for key in row.dims.keys() {
            if !seen.iter().any(|held| held == key) {
                seen.push(key.clone());
            }
        }
    }
    seen
}

#[must_use]
pub fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use super::{
        Reading, csv_cell, dims_of, ended_of, in_parts, merged, part_body, rows_of, run_body,
    };
    use crate::json::Value;

    #[test]
    fn parts_recorded_as_taken_read_back_as_one_run() {
        let row = |n: i64| {
            Reading::new(
                &[("attempt", Value::Integer(n))],
                "tokens",
                n * 10,
                "tokens",
            )
        };
        let conditions = || vec![("retries", Value::Integer(3))];
        let whole = run_body("m", "editing", "e", conditions(), &[row(9)]);
        let opened = part_body("m", "challenges-easy", "", conditions(), &[], "r1", 0, None);
        let first = part_body(
            "m",
            "challenges-easy",
            "llama",
            vec![],
            &[row(1)],
            "r1",
            1,
            None,
        );
        let second = part_body(
            "m",
            "challenges-easy",
            "llama",
            vec![],
            &[row(2)],
            "r1",
            2,
            None,
        );
        let closed = part_body(
            "m",
            "challenges-easy",
            "llama",
            vec![],
            &[],
            "r1",
            3,
            Some("finished"),
        );
        let cut = part_body("m", "challenges-hard", "", conditions(), &[], "r2", 0, None);
        let cut_row = part_body(
            "m",
            "challenges-hard",
            "llama",
            vec![],
            &[row(5)],
            "r2",
            1,
            None,
        );
        let runs = merged(vec![
            whole.clone(),
            opened,
            first,
            second,
            closed,
            cut,
            cut_row,
        ]);
        assert_eq!(runs.len(), 3, "{runs:?}");
        assert_eq!(runs[0], whole, "a whole run is as it was");
        assert!(!in_parts(&whole));
        let easy = &runs[1];
        assert!(in_parts(easy));
        assert_eq!(
            easy.get("method").and_then(Value::as_text),
            Some("challenges-easy")
        );
        assert_eq!(
            easy.get("engine").and_then(Value::as_text),
            Some("llama"),
            "the last engine named"
        );
        assert_eq!(
            easy.get("conditions").and_then(|c| c.get("retries")),
            Some(&Value::Integer(3)),
            "the opening part's conditions"
        );
        assert_eq!(
            rows_of(easy).iter().map(|r| r.value).collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert_eq!(ended_of(easy).as_deref(), Some("finished"));
        assert!(easy.get("part").is_none());
        let hard = &runs[2];
        assert_eq!(rows_of(hard).len(), 1);
        assert_eq!(ended_of(hard), None, "no part said how it ended");
    }

    #[test]
    fn a_reading_round_trips() {
        let row = Reading::new(
            &[
                ("depth", Value::Integer(1024)),
                ("repeat", Value::Integer(2)),
            ],
            "ns_per_token",
            8_200_000,
            "ns",
        );
        let back = Reading::from_value(&row.to_value()).expect("a reading reads back");
        assert_eq!(back, row);
        assert_eq!(back.dim("depth"), "1024");
        assert_eq!(back.dim("nothing"), "");
        assert_eq!(
            Reading::new(&[("found", Value::Bool(true))], "x", 1, "bool").dim("found"),
            "1"
        );
        assert_eq!(Reading::from_value(&Value::text("not a row")), None);
    }

    #[test]
    fn a_run_carries_its_rows() {
        let rows = vec![
            Reading::new(&[("batch", Value::Integer(64))], "ns", 1, "ns"),
            Reading::new(
                &[("batch", Value::Integer(64)), ("repeat", Value::Integer(1))],
                "ns",
                2,
                "ns",
            ),
        ];
        let body = run_body(
            "/m",
            "prefill-saturation",
            "e",
            vec![("depth", Value::Integer(1024))],
            &rows,
        );
        assert_eq!(rows_of(&body), rows);
        assert_eq!(
            dims_of(&rows),
            vec!["batch".to_owned(), "repeat".to_owned()]
        );
        assert_eq!(
            body.get("conditions").and_then(|c| c.get("depth")),
            Some(&Value::Integer(1024))
        );
    }

    #[test]
    fn a_cell_is_quoted_only_where_it_must_be() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
