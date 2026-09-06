//! A diagnostic's findings as readings: one row a figure, in one schema
//! for every diagnostic (D54, B-511).
//!
//! **One shape, so that two models can be set side by side.** Each
//! diagnostic wrote what it chose — a map of counts, a map with lists in
//! it, a median with its samples thrown away — and a person comparing two
//! models had five shapes to read. A reading is `dims`, `metric`, `value`,
//! `unit`: the dimensions it was taken under, what was measured, the whole
//! number measured, and its unit. A run is one record entry of a model, a
//! method, an engine, the conditions the run shared, and its rows.
//!
//! **Raw, never summarized** (D16). The row is the sample: a repeat, a
//! position, a trial, a placement. A median is arithmetic over rows at the
//! moment of asking, and nothing here writes a mean — the crate has none
//! to write.
//!
//! **Whole numbers only** (A6). A value is an `i64` in its unit —
//! nanoseconds, bytes, tokens, millibits, parts per million, a count, or
//! `bool` written as one or nought — so a record can be ordered and no
//! float reaches it.

use std::collections::BTreeMap;

use crate::json::Value;

/// One figure a diagnostic read, with the dimensions it was read under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The dimensions: a depth, a batch, a repeat, a position, a placement,
    /// a name — each a whole number, a name or a yes-or-no.
    pub dims: BTreeMap<String, Value>,
    /// What was measured, as the method names it.
    pub metric: String,
    /// The figure, whole, in its unit.
    pub value: i64,
    /// The unit: `ns`, `bytes`, `tokens`, `millibits`, `ppm`, `count`,
    /// `bool`, or another the method states.
    pub unit: String,
}

impl Reading {
    /// A reading under these dimensions.
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

    /// The row as the record writes it.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("dims", Value::Map(self.dims.clone())),
            ("metric", Value::text(self.metric.clone())),
            ("value", Value::Integer(self.value)),
            ("unit", Value::text(self.unit.clone())),
        ])
    }

    /// A row read back; `None` where the value is not a reading.
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

    /// One dimension as text, for a table: a number as digits, a name as
    /// itself, a yes-or-no as `1` or `0`, and nothing where it is absent.
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

/// A run's entry body: the model, the method, the engine, the conditions
/// shared by every row, and the rows.
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

/// The rows of a run's body, read back; empty where it holds none.
#[must_use]
pub fn rows_of(body: &Value) -> Vec<Reading> {
    body.get("rows")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(Reading::from_value)
        .collect()
}

/// The dimension names a set of rows uses, in order, for a table's
/// columns.
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

/// A table cell as comma-separated text: quoted where it holds a comma, a
/// quote or a line break, the quotes inside doubled.
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

    use super::{Reading, csv_cell, dims_of, rows_of, run_body};
    use crate::json::Value;

    /// A reading round-trips through the record's shape, dimensions and all.
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

    /// A run's body carries its rows, and the dimensions are read off them
    /// in the order they first appear.
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

    /// A cell with a comma or a quote is quoted, and one without is bare.
    #[test]
    fn a_cell_is_quoted_only_where_it_must_be() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
