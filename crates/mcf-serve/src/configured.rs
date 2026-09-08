use std::path::{Path, PathBuf};

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::{self, Value};
use mcf_standin::tokenizer::Piece;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    pub name: String,
    pub before: Vec<Piece>,
    pub after: Vec<Piece>,
    pub probe: String,
    pub at: String,
    pub build: String,
    pub conditions: String,
}

impl Addressing {
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("name", Value::text(self.name.clone())),
            ("before", pieces_to_value(&self.before)),
            ("after", pieces_to_value(&self.after)),
            ("probe", Value::text(self.probe.clone())),
            ("at", Value::text(self.at.clone())),
            ("build", Value::text(self.build.clone())),
            ("conditions", Value::text(self.conditions.clone())),
        ])
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        let pieces = |key: &str| value.get(key).and_then(pieces_from_value);
        Some(Self {
            name: text("name")?,
            before: pieces("before")?,
            after: pieces("after")?,
            probe: text("probe")?,
            at: text("at")?,
            build: text("build")?,
            conditions: text("conditions")?,
        })
    }

    #[must_use]
    pub fn provenance(&self) -> String {
        let build = self
            .build
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{} — set by the {} probe at {}, through {} ({build})",
            self.name,
            self.probe,
            self.at.split('.').next().unwrap_or(&self.at),
            self.conditions
                .split(',')
                .next()
                .unwrap_or(&self.conditions),
        )
    }
}

#[must_use]
pub fn pieces_to_value(pieces: &[Piece]) -> Value {
    Value::List(
        pieces
            .iter()
            .map(|piece| match piece {
                Piece::Marker(marker) => Value::map([
                    ("kind", Value::text("marker")),
                    ("value", Value::text(marker.clone())),
                ]),
                Piece::Text(text) => Value::map([
                    ("kind", Value::text("text")),
                    ("value", Value::text(text.clone())),
                ]),
            })
            .collect(),
    )
}

#[must_use]
pub fn pieces_from_value(value: &Value) -> Option<Vec<Piece>> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|item| {
                let held = item.get("value").and_then(Value::as_text)?.to_owned();
                match item.get("kind").and_then(Value::as_text) {
                    Some("marker") => Some(Piece::Marker(held)),
                    Some("text") => Some(Piece::Text(held)),
                    _ => None,
                }
            })
            .collect(),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Since {
    ConditionsHold,
    ConditionsMoved(Vec<Moved>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub what: &'static str,
    pub was: String,
    pub now: String,
}

#[must_use]
pub fn since(addressing: &Addressing, engine_now: &str, build_now: &str) -> Since {
    let mut moved = Vec::new();
    let head = |said: &str| said.split(',').next().unwrap_or(said).trim().to_owned();
    let version = str::to_owned;
    if head(&addressing.conditions) != head(engine_now) {
        moved.push(Moved {
            what: "engine",
            was: head(&addressing.conditions),
            now: head(engine_now),
        });
    }
    if version(&addressing.build) != version(build_now) {
        let shown = |said: &str| {
            said.split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        };
        moved.push(Moved {
            what: "build",
            was: shown(&addressing.build),
            now: shown(build_now),
        });
    }
    if moved.is_empty() {
        Since::ConditionsHold
    } else {
        Since::ConditionsMoved(moved)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub tokens: usize,
    pub before: Option<usize>,
    pub probe: String,
    pub at: String,
    pub build: String,
    pub conditions: String,
}

impl Budget {
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut value = Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(self.tokens).unwrap_or(i64::MAX)),
            ),
            ("probe", Value::text(self.probe.clone())),
            ("at", Value::text(self.at.clone())),
            ("build", Value::text(self.build.clone())),
            ("conditions", Value::text(self.conditions.clone())),
        ]);
        if let (Some(before), Value::Map(map)) = (self.before, &mut value) {
            map.insert(
                "before".to_owned(),
                Value::Integer(i64::try_from(before).unwrap_or(i64::MAX)),
            );
        }
        value
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        Some(Self {
            tokens: value
                .get("tokens")
                .and_then(Value::as_integer)
                .and_then(|found| usize::try_from(found).ok())?,
            before: value
                .get("before")
                .and_then(Value::as_integer)
                .and_then(|found| usize::try_from(found).ok()),
            probe: text("probe")?,
            at: text("at")?,
            build: text("build")?,
            conditions: text("conditions")?,
        })
    }

    #[must_use]
    pub fn provenance(&self) -> String {
        format!(
            "{} tokens{} — set by the {} probe at {}, through {}",
            self.tokens,
            match self.before {
                Some(before) =>
                    format!(", of which up to {before} were spent thinking before the answer"),
                None => String::new(),
            },
            self.probe,
            self.at.split('.').next().unwrap_or(&self.at),
            self.conditions
                .split(',')
                .next()
                .unwrap_or(&self.conditions),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Derived {
    pub addressing: Option<Addressing>,
    pub budget: Option<Budget>,
}

impl Derived {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.addressing.is_none() && self.budget.is_none()
    }
}

#[must_use]
pub fn path_for(mcf_home: &Path, model: &Path) -> PathBuf {
    let key = mcf_core::digest::sha256(model.display().to_string().as_bytes());
    mcf_home
        .join("configured")
        .join(format!("{}.json", key.hex()))
}

#[must_use]
pub fn read(mcf_home: &Path, model: &Path) -> Option<Addressing> {
    read_derived(mcf_home, model).addressing
}

#[must_use]
pub fn read_derived(mcf_home: &Path, model: &Path) -> Derived {
    let Ok(held) = std::fs::read_to_string(path_for(mcf_home, model)) else {
        return Derived::default();
    };
    let Ok(value) = json::parse(&held) else {
        return Derived::default();
    };
    Derived {
        addressing: value.get("addressing").and_then(Addressing::from_value),
        budget: value.get("budget").and_then(Budget::from_value),
    }
}

pub fn write(mcf_home: &Path, model: &Path, addressing: &Addressing) -> Result<PathBuf, Failure> {
    let mut held = read_derived(mcf_home, model);
    held.addressing = Some(addressing.clone());
    write_derived(mcf_home, model, &held)
}

pub fn write_budget(mcf_home: &Path, model: &Path, budget: &Budget) -> Result<PathBuf, Failure> {
    let mut held = read_derived(mcf_home, model);
    held.budget = Some(budget.clone());
    write_derived(mcf_home, model, &held)
}

pub fn write_derived(mcf_home: &Path, model: &Path, derived: &Derived) -> Result<PathBuf, Failure> {
    let path = path_for(mcf_home, model);
    let unwritable = |what: &str, error: &std::io::Error| {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-serve::configured"),
            what,
        )
        .with_context("path", path.display().to_string())
        .with_context("error", error.to_string())
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| unwritable("the configuration directory could not be made", &error))?;
    }
    let mut fields = vec![("model", Value::text(model.display().to_string()))];
    if let Some(addressing) = &derived.addressing {
        fields.push(("addressing", addressing.to_value()));
    }
    if let Some(budget) = &derived.budget {
        fields.push(("budget", budget.to_value()));
    }
    let held = Value::map(fields);
    std::fs::write(&path, format!("{}\n", held.to_line()))
        .map_err(|error| unwritable("the configuration could not be written", &error))?;
    Ok(path)
}

pub fn forget(mcf_home: &Path, model: &Path) -> Result<bool, Failure> {
    let path = path_for(mcf_home, model);
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path)
        .map_err(|error| {
            Failure::new(
                Category::RecordUnwritable,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::configured"),
                "the configuration is there and could not be removed",
            )
            .with_context("path", path.display().to_string())
            .with_context("error", error.to_string())
        })
        .map(|()| true)
}

#[cfg(test)]
mod tests;
