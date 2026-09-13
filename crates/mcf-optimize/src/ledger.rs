use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{Value, parse};

use crate::dial::{Dial, Step};
use crate::reading::{Ending, Reading};

const WHERE: Subsystem = Subsystem::new("mcf-optimize::ledger");

pub const CORPUS: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Under {
    pub model: String,
    pub model_bytes: u64,
    pub engine: String,
    pub commit: String,
    pub context: u64,
    pub batch: u32,
    pub ubatch: u32,
    pub cache: String,
    pub flash_attention: bool,
    pub draft_head: bool,
    pub draft_depth: Option<u32>,
    pub thinking_budget: Option<u32>,
    pub temperature: Option<u32>,
    pub top_p: Option<u32>,
    pub top_k: Option<u32>,
    pub corpus: u32,
}

impl Under {
    #[must_use]
    pub fn without(&self, dial: Dial) -> Self {
        let mut held = self.clone();
        match dial {
            Dial::ThinkingBudget => held.thinking_budget = None,
            Dial::Temperature => held.temperature = None,
            Dial::TopP => held.top_p = None,
            Dial::TopK => held.top_k = None,
            Dial::MicroBatch => held.ubatch = 0,
            Dial::DraftDepth => held.draft_depth = None,
        }
        held
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        let count =
            |held: Option<u32>| held.map_or(Value::Null, |held| Value::Integer(held.into()));
        Value::map([
            ("model", Value::text(self.model.clone())),
            ("model_bytes", whole(self.model_bytes)),
            ("engine", Value::text(self.engine.clone())),
            ("commit", Value::text(self.commit.clone())),
            ("context", whole(self.context)),
            ("batch", Value::Integer(self.batch.into())),
            ("ubatch", Value::Integer(self.ubatch.into())),
            ("cache", Value::text(self.cache.clone())),
            ("flash_attention", Value::Bool(self.flash_attention)),
            ("draft_head", Value::Bool(self.draft_head)),
            ("draft_depth", count(self.draft_depth)),
            ("thinking_budget", count(self.thinking_budget)),
            ("temperature", count(self.temperature)),
            ("top_p", count(self.top_p)),
            ("top_k", count(self.top_k)),
            ("corpus", Value::Integer(self.corpus.into())),
        ])
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned()
        };
        let big = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(0)
        };
        let small = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok())
        };
        let yes = |key: &str| matches!(value.get(key), Some(Value::Bool(true)));
        Self {
            model: text("model"),
            model_bytes: big("model_bytes"),
            engine: text("engine"),
            commit: text("commit"),
            context: big("context"),
            batch: small("batch").unwrap_or(0),
            ubatch: small("ubatch").unwrap_or(0),
            cache: text("cache"),
            flash_attention: yes("flash_attention"),
            draft_head: yes("draft_head"),
            draft_depth: small("draft_depth"),
            thinking_budget: small("thinking_budget"),
            temperature: small("temperature"),
            top_p: small("top_p"),
            top_k: small("top_k"),
            corpus: small("corpus").unwrap_or(0),
        }
    }

    #[must_use]
    pub fn said(&self) -> String {
        format!(
            "{} on {} {} — {} ctx, ubatch {}, {} cache",
            file_name(&self.model),
            self.engine,
            short(&self.commit),
            self.context,
            self.ubatch,
            self.cache
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At {
    pub dial: Dial,
    pub step: Step,
    pub set: usize,
    pub repeat: u8,
}

impl At {
    #[must_use]
    pub fn to_value(&self) -> Value {
        let (scale, held) = match self.step {
            Step::Whole(held) => ("whole", held),
            Step::Thousandths(held) => ("thousandths", held),
        };
        Value::map([
            ("dial", Value::text(self.dial.label())),
            ("scale", Value::text(scale)),
            ("step", Value::Integer(held.into())),
            ("set", whole(self.set as u64)),
            ("repeat", Value::Integer(self.repeat.into())),
        ])
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let label = value.get("dial").and_then(Value::as_text)?;
        let dial = Dial::ALL.into_iter().find(|held| held.label() == label)?;
        let held = value
            .get("step")
            .and_then(Value::as_integer)
            .and_then(|held| u32::try_from(held).ok())?;
        let step = match value.get("scale").and_then(Value::as_text) {
            Some("thousandths") => Step::Thousandths(held),
            _ => Step::Whole(held),
        };
        Some(Self {
            dial,
            step,
            set: value
                .get("set")
                .and_then(Value::as_integer)
                .and_then(|held| usize::try_from(held).ok())
                .unwrap_or(0),
            repeat: value
                .get("repeat")
                .and_then(Value::as_integer)
                .and_then(|held| u8::try_from(held).ok())
                .unwrap_or(1),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub recorded: String,
    pub under: Under,
    pub at: At,
    pub reading: Reading,
}

impl Row {
    #[must_use]
    pub fn to_line(&self) -> String {
        Value::map([
            ("recorded", Value::text(self.recorded.clone())),
            ("under", self.under.to_value()),
            ("at", self.at.to_value()),
            ("found", found(&self.reading)),
        ])
        .to_line()
    }

    #[must_use]
    pub fn from_line(line: &str) -> Option<Self> {
        let value = parse(line).ok()?;
        let at = At::from_value(value.get("at")?)?;
        let found = value.get("found")?;
        let count = |key: &str| {
            found
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(0)
        };
        let big = |key: &str| {
            found
                .get(key)
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(0)
        };
        let ending = match found.get("ending").and_then(Value::as_text) {
            Some("looped") => Ending::Looped,
            Some("filled the budget") => Ending::Filled,
            Some("failed") => Ending::Failed,
            _ => Ending::Answered,
        };
        let per_task = match found.get("per_task") {
            Some(Value::List(listed)) => listed
                .iter()
                .map(|one| {
                    (
                        one.get("task")
                            .and_then(Value::as_text)
                            .unwrap_or_default()
                            .to_owned(),
                        matches!(one.get("passed"), Some(Value::Bool(true))),
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        Some(Self {
            recorded: value
                .get("recorded")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned(),
            under: Under::from_value(value.get("under")?),
            at,
            reading: Reading {
                dial: at.dial,
                step: at.step,
                set: at.set,
                repeat: at.repeat,
                passed: count("passed"),
                of: count("of"),
                produced: big("produced"),
                milliseconds: big("milliseconds"),
                ending,
                per_task,
            },
        })
    }
}

fn found(reading: &Reading) -> Value {
    Value::map([
        ("passed", Value::Integer(reading.passed.into())),
        ("of", Value::Integer(reading.of.into())),
        ("produced", whole(reading.produced)),
        ("milliseconds", whole(reading.milliseconds)),
        ("ending", Value::text(reading.ending.label())),
        (
            "per_task",
            Value::List(
                reading
                    .per_task
                    .iter()
                    .map(|(name, passed)| {
                        Value::map([
                            ("task", Value::text(name.clone())),
                            ("passed", Value::Bool(*passed)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

#[derive(Debug, Clone, Default)]
pub struct Ledger {
    path: PathBuf,
    rows: Vec<Row>,
    unreadable: usize,
}

impl Ledger {
    #[must_use]
    pub fn beside(home: &Path) -> PathBuf {
        home.join("optimize").join("readings.jsonl")
    }

    pub fn open(path: &Path) -> Result<Self> {
        let mut rows = Vec::new();
        let mut unreadable: usize = 0;
        match std::fs::File::open(path) {
            Ok(file) => {
                for line in BufReader::new(file).lines() {
                    let Ok(line) = line else { break };
                    if line.trim().is_empty() {
                        continue;
                    }
                    match Row::from_line(&line) {
                        Some(row) => rows.push(row),
                        None => unreadable = unreadable.saturating_add(1),
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Failure::new(
                    Category::ArtifactUnreadable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "the record of what has already been measured could not be read, and \
                     starting without it would repeat work already done",
                )
                .with_context("path", path.display().to_string())
                .with_context("reason", error.to_string()));
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
            rows,
            unreadable,
        })
    }

    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    #[must_use]
    pub const fn unreadable(&self) -> usize {
        self.unreadable
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn already(&self, under: &Under, at: &At) -> Option<&Row> {
        let wanted = under.without(at.dial);
        self.rows.iter().find(|row| {
            row.at == *at
                && row.under.without(at.dial) == wanted
                && row.under.corpus == under.corpus
        })
    }

    #[must_use]
    pub fn against(&self, under: &Under, dial: Dial) -> Vec<&Row> {
        let wanted = under.without(dial);
        self.rows
            .iter()
            .filter(|row| {
                row.at.dial == dial
                    && row.under.without(dial) == wanted
                    && row.under.corpus == under.corpus
            })
            .collect()
    }

    pub fn record(
        &mut self,
        under: &Under,
        at: At,
        reading: &Reading,
        recorded: &str,
    ) -> Result<()> {
        let row = Row {
            recorded: recorded.to_owned(),
            under: under.clone(),
            at,
            reading: reading.clone(),
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| written(&self.path, &error))?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| written(&self.path, &error))?;
        writeln!(file, "{}", row.to_line()).map_err(|error| written(&self.path, &error))?;
        file.flush().map_err(|error| written(&self.path, &error))?;
        file.sync_data()
            .map_err(|error| written(&self.path, &error))?;
        self.rows.push(row);
        Ok(())
    }
}

fn written(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::ResourceDiskReadonly,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "a measurement that is not written down is one that has to be taken again, so a \
         reading MCF cannot record is a reading it refuses to take credit for",
    )
    .with_context("path", path.display().to_string())
    .with_context("reason", error.to_string())
}

fn whole(held: u64) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn short(commit: &str) -> &str {
    commit.get(..9).unwrap_or(commit)
}

#[cfg(test)]
mod tests;
