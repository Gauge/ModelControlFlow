//! What MCF was told to do differently, and on whose evidence (B-059, D43).
//!
//! **This is not where a probe writes.** D42 is that a probe reports a
//! measurement and configures nothing by itself, and D43 is that MCF never
//! reconfigures under a user. Between the two there has to be an *act*: a
//! person reads what a probe observed and says *do that*. This module holds
//! what the act wrote down.
//!
//! **Why a file rather than a default in the code.** A default in the code is
//! MCF's opinion and applies to every model; this is a fact about one model,
//! learned by asking it, on a machine, through an engine. Those are different
//! kinds of thing and confusing them is how a measurement becomes a measurement
//! of MCF's guess (§3.8).
//!
//! **Everything here answers *why this value*.** The probe that found it, the
//! moment, the build, and the conditions the trial ran under travel with the
//! value and are printed wherever it is used. A derived configuration that
//! could not say where it came from would be indistinguishable from a default
//! somebody typed, which is the whole of what B-059 is about.
//!
//! **It is keyed by the model's path, digested.** A path is what a person
//! names and holds slashes; a digest of it is a filename. Two different files
//! at the same path over time are the same key, which is correct here: the
//! configuration is about *the model MCF runs when you say this*, and a file
//! swapped underneath is a divergence the probe would find on the next run.

use std::path::{Path, PathBuf};

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::{self, Value};
use mcf_standin::tokenizer::Piece;

/// How MCF was told to address one model, and on what evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    /// The addressing's name, as the probe reported it.
    pub name: String,
    /// What goes before the person's text.
    pub before: Vec<Piece>,
    /// What goes after it.
    pub after: Vec<Piece>,
    /// Which probe found this.
    pub probe: String,
    /// When it was applied.
    pub at: String,
    /// The build that applied it.
    pub build: String,
    /// The conditions the probe ran under, as it stated them.
    pub conditions: String,
}

impl Addressing {
    /// The record's shape.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let pieces = |pieces: &[Piece]| {
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
        };
        Value::map([
            ("name", Value::text(self.name.clone())),
            ("before", pieces(&self.before)),
            ("after", pieces(&self.after)),
            ("probe", Value::text(self.probe.clone())),
            ("at", Value::text(self.at.clone())),
            ("build", Value::text(self.build.clone())),
            ("conditions", Value::text(self.conditions.clone())),
        ])
    }

    /// Reads one back.
    ///
    /// Every field is required. A configuration missing its provenance is not
    /// a configuration with a gap in it — it is a value nobody can account
    /// for, and MCF would rather have none than one it cannot explain (A7,
    /// B-059).
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        let pieces = |key: &str| -> Option<Vec<Piece>> {
            match value.get(key) {
                Some(Value::List(items)) => items
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
        };
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

    /// One line saying where this came from, for an account or a report.
    ///
    /// The build is named by its version rather than in full. B-059 asks that
    /// a derived value answer *why this value*, and the answer is the probe,
    /// the moment and the engine; the compiler's version and target are in the
    /// file and in the journal entry, where somebody chasing a difference
    /// between two machines will look for them and nobody else has to read
    /// them on every line (A1 keeps them, legibility puts them there).
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

/// What has changed since a configuration was applied.
///
/// **Two kinds of thing, deliberately not mixed.** That the *conditions* have
/// changed is a fact, checkable with no trials: the configuration says which
/// engine and which build it was taken through, and MCF knows which are in
/// force now. That the *answer* would now differ is not a fact until somebody
/// asks the model again — and reporting the first as though it were the second
/// would be a declaration wearing a measurement's clothes, which is what A21
/// exists to stop.
///
/// So a changed condition says *this may no longer hold, and here is what to
/// run*; only a re-probe says *it does not hold*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Since {
    /// Nothing has changed that MCF can see without asking.
    ConditionsHold,
    /// The configuration was taken under conditions that are no longer in
    /// force, naming each one that moved.
    ConditionsMoved(Vec<Moved>),
}

/// One condition that is not what it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    /// What the condition is called.
    pub what: &'static str,
    /// What it was when the configuration was applied.
    pub was: String,
    /// What it is now.
    pub now: String,
}

/// Whether the conditions a configuration was taken under still hold.
///
/// The engine is compared on the name a probe recorded, and the build on its
/// version. Neither comparison proves the answer would differ — a model
/// addressed one way through two engines usually wants the same addressing,
/// and F39 measured exactly that. What it establishes is that the *evidence*
/// was gathered somewhere else, which is the thing a person needs to know
/// before deciding whether to trust it (D43, §3.4).
#[must_use]
pub fn since(addressing: &Addressing, engine_now: &str, build_now: &str) -> Since {
    let mut moved = Vec::new();
    let head = |said: &str| said.split(',').next().unwrap_or(said).trim().to_owned();
    // The whole build identity, not a version parsed out of it. A different
    // compiler or a different target is a different build, and inventing a
    // parse to decide which differences count would be MCF deciding where it
    // has no evidence (§3.15). What is *shown* is shortened; what is
    // *compared* is everything.
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

/// Where one model's derived configuration lives.
#[must_use]
pub fn path_for(mcf_home: &Path, model: &Path) -> PathBuf {
    let key = mcf_core::digest::sha256(model.display().to_string().as_bytes());
    mcf_home
        .join("configured")
        .join(format!("{}.json", key.hex()))
}

/// What MCF was told about this model, if anything.
///
/// A file that cannot be read or cannot be understood returns `None` rather
/// than an error: nothing was configured, which is the same answer as no file
/// at all from the point of view of what MCF does next. Where it *matters* —
/// the act of writing one — the failure is loud.
#[must_use]
pub fn read(mcf_home: &Path, model: &Path) -> Option<Addressing> {
    let held = std::fs::read_to_string(path_for(mcf_home, model)).ok()?;
    let value = json::parse(&held).ok()?;
    Addressing::from_value(value.get("addressing")?)
}

/// Writes what somebody decided.
///
/// # Errors
///
/// `record.unwritable` if the file cannot be written, which is loud because
/// a configuration somebody asked for and did not get is worse than one they
/// never asked for (A2).
pub fn write(mcf_home: &Path, model: &Path, addressing: &Addressing) -> Result<PathBuf, Failure> {
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
    // The model's own path is written beside the digest that names the file,
    // because a directory of digests nobody can read is a directory nobody can
    // audit (A1).
    let held = Value::map([
        ("model", Value::text(model.display().to_string())),
        ("addressing", addressing.to_value()),
    ]);
    std::fs::write(&path, format!("{}\n", held.to_line()))
        .map_err(|error| unwritable("the configuration could not be written", &error))?;
    Ok(path)
}

/// Forgets what was decided, and says whether there had been anything.
///
/// # Errors
///
/// `record.unwritable` if the file is there and will not go.
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
