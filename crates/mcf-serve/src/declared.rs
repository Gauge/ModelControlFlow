//! What a model file declares that the engine does not start on its own
//! (B-456, D43, §3.15).
//!
//! **A file can carry more than the engine loads.** Two of those are here.
//! A draft head — extra layers trained to guess the next token or two, which
//! a file declares as `nextn_predict_layers` — sits in the weights and is
//! not read into memory at all unless the engine is told to use it; the
//! layers are on the disk and the model runs without them, which is a model
//! hosted without a feature its own file declares. A rope scaling stretches
//! the positions a model was trained on over a longer conversation; a file
//! that declares one is followed by the engine, and a file that declares
//! none can still be stretched by somebody who asks for it and accepts what
//! it costs in faithfulness.
//!
//! **MCF reads and reports; it does not switch either on.** Starting a draft
//! head because the file mentions one would be MCF choosing speed on
//! somebody's behalf and changing what the tokens are drawn from; stretching
//! a rope because a longer window was asked for would be MCF answering a
//! question about quality that nobody measured. Both are offered as switches,
//! named in the account whether they were asked for or not, and a run under
//! one is not a run under the other (D43, §3.15).

use std::path::Path;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;
use mcf_standin::gguf::{Model, Value as Held};

/// How much of a model file's front is read to find its header.
///
/// A header sits at the start, and a model is gigabytes nobody needs in
/// memory to answer what it declares. The second cap is for a file whose
/// vocabulary alone is larger than the first (B-072).
const FRONT: [u64; 2] = [4 << 20, 64 << 20];

/// The header of a model file, read from a bounded prefix.
///
/// `None` where the file cannot be opened or does not parse as far as the
/// second cap — which is a file MCF cannot read rather than a file that
/// declares nothing, and every caller here treats it as the former by
/// saying nothing about it (A7).
#[must_use]
pub fn header(path: &Path) -> Option<Model> {
    use std::io::Read as _;
    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in FRONT {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if let Ok(file) = mcf_standin::gguf::parse(&prefix) {
            return Some(file);
        }
        if take >= held {
            return None;
        }
    }
    None
}

/// What a file says about itself that bears on how the engine is started.
///
/// Every field is the file's own statement, read and not checked (A21).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Declared {
    /// How many draft-head layers the file carries, where it declares any.
    pub draft_head: Option<u64>,
    /// The rope scaling the file declares, where it declares one.
    pub rope: Option<Rope>,
    /// How long a conversation the file says it was made for.
    pub context: Option<u64>,
}

/// A rope scaling as a file declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rope {
    /// What the file calls it: `yarn`, `linear`, and whatever a later file
    /// calls something else — passed on rather than translated (A21).
    pub kind: String,
    /// By how much, as the file writes it.
    pub factor: Option<String>,
    /// The window it was trained on, before the scaling.
    pub trained: Option<u64>,
}

impl Declared {
    /// What this file declares, read from its front.
    ///
    /// A file that cannot be read declares nothing here, which is the
    /// absence of a reading and not a reading of absence: nothing downstream
    /// says *this model has no draft head* on the strength of it.
    #[must_use]
    pub fn of(model: &Path) -> Self {
        header(model)
            .map(|file| Self::in_header(&file))
            .unwrap_or_default()
    }

    /// The same, from a header already read.
    #[must_use]
    pub fn in_header(file: &Model) -> Self {
        let Some(architecture) = file.architecture() else {
            return Self::default();
        };
        let under = |suffix: &str| file.get(&format!("{architecture}.{suffix}"));
        let number = |suffix: &str| {
            under(suffix)
                .and_then(Held::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        Self {
            // Zero is a file that says it has none, which is what every file
            // without the key says by not having it: both are `None` here,
            // because a draft head of no layers is not one.
            draft_head: number("nextn_predict_layers").filter(|layers| *layers > 0),
            rope: under("rope.scaling.type")
                .and_then(Held::as_text)
                .map(|kind| Rope {
                    kind: kind.to_owned(),
                    factor: under("rope.scaling.factor").map(written),
                    trained: number("rope.scaling.original_context_length"),
                }),
            context: number("context_length"),
        }
    }

    /// What the file declares that the engine was not started with, in
    /// words, or nothing where there is nothing to say.
    ///
    /// **This is the sentence the row exists for.** A draft head the engine
    /// left in the file is a capability the model has and this run did not,
    /// and a person reading an account of the run has no other way to learn
    /// it — the model answers perfectly well without it, only slower than it
    /// could have.
    #[must_use]
    pub fn not_started(&self, started: Started) -> Option<String> {
        let layers = self.draft_head.filter(|_| !started.draft_head)?;
        let plural = if layers == 1 { "" } else { "s" };
        Some(format!(
            "a draft head of {layers} layer{plural}, which the engine leaves in the file unless \
             it is asked for"
        ))
    }

    /// Read back from what an account carried, so that a reader of the
    /// account and a reader of the file describe the model the same way.
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let number = |held: Option<&Value>| {
            held.and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        let rope = value.get("rope_scaling");
        Self {
            draft_head: number(value.get("draft_head")),
            rope: rope
                .and_then(|rope| rope.get("kind"))
                .and_then(Value::as_text)
                .map(|kind| Rope {
                    kind: kind.to_owned(),
                    factor: rope
                        .and_then(|rope| rope.get("factor"))
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    trained: number(rope.and_then(|rope| rope.get("trained_context"))),
                }),
            context: number(value.get("context")),
        }
    }

    /// As the record carries it.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            (
                "draft_head",
                self.draft_head
                    .map_or(Value::Null, |layers| whole(u128::from(layers))),
            ),
            (
                "rope_scaling",
                self.rope.as_ref().map_or(Value::Null, |rope| {
                    Value::map([
                        ("kind", Value::text(rope.kind.clone())),
                        (
                            "factor",
                            rope.factor
                                .as_ref()
                                .map_or(Value::Null, |factor| Value::text(factor.clone())),
                        ),
                        (
                            "trained_context",
                            rope.trained
                                .map_or(Value::Null, |trained| whole(u128::from(trained))),
                        ),
                    ])
                }),
            ),
            (
                "context",
                self.context
                    .map_or(Value::Null, |context| whole(u128::from(context))),
            ),
        ])
    }
}

/// What the engine is asked to start beyond the plain load.
///
/// Every field is *what was asked*, and the default is nothing asked —
/// which is what MCF sends unless somebody said otherwise, so that no run
/// carries a condition nobody chose (D43).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Started {
    /// Start the draft head the file declares.
    pub draft_head: bool,
    /// Scale the positions this way, rather than however the file's own
    /// declaration leaves the engine.
    pub rope: Option<Scaling>,
    /// By how much, where the asker said. Whole numbers only: every factor
    /// a file has been seen to declare is one, and a factor MCF rounded
    /// would be a condition MCF changed.
    pub factor: Option<u32>,
}

/// How positions are scaled, in the engine's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaling {
    /// None, which is the file's own scaling turned off.
    Off,
    /// Stretched evenly.
    Linear,
    /// Stretched by the scheme that keeps the short range intact.
    Yarn,
}

impl Scaling {
    /// As the engine's switch spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "none",
            Self::Linear => "linear",
            Self::Yarn => "yarn",
        }
    }

    /// From the word somebody wrote, or nothing where it is not one of the
    /// three the engine takes.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "none" => Some(Self::Off),
            "linear" => Some(Self::Linear),
            "yarn" => Some(Self::Yarn),
            _ => None,
        }
    }
}

impl Started {
    /// Whether anything at all was asked for.
    #[must_use]
    pub const fn asks_anything(&self) -> bool {
        self.draft_head || self.rope.is_some() || self.factor.is_some()
    }

    /// The switches the engine is started with.
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.draft_head {
            // The draft head lives in the model's own file, so the engine is
            // told which kind of speculation to run and nothing else: no
            // second model, no sidecar. Told this way it reads the layers in
            // with the weights, which it otherwise skips.
            out.push("--spec-type".to_owned());
            out.push("draft-mtp".to_owned());
        }
        if let Some(rope) = self.rope {
            out.push("--rope-scaling".to_owned());
            out.push(rope.as_str().to_owned());
        }
        if let Some(factor) = self.factor {
            out.push("--rope-scale".to_owned());
            out.push(factor.to_string());
        }
        out
    }

    /// What was asked, in words, for an account and a report.
    #[must_use]
    pub fn said(&self) -> String {
        let mut said = Vec::new();
        if self.draft_head {
            said.push("the draft head".to_owned());
        }
        if let Some(rope) = self.rope {
            let by = self
                .factor
                .map_or_else(String::new, |factor| format!(" ×{factor}"));
            said.push(format!("rope scaling {}{by}", rope.as_str()));
        }
        if said.is_empty() {
            "nothing beyond the plain load".to_owned()
        } else {
            said.join(", ")
        }
    }

    /// As the record and the control plane carry it.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("draft_head", Value::Bool(self.draft_head)),
            (
                "rope_scaling",
                self.rope
                    .map_or(Value::Null, |rope| Value::text(rope.as_str())),
            ),
            (
                "rope_scale",
                self.factor
                    .map_or(Value::Null, |factor| Value::Integer(i64::from(factor))),
            ),
        ])
    }

    /// Read back from what the control plane carried.
    ///
    /// Anything absent is not asked for: a client that said nothing has
    /// asked for the plain load, which is the only reading that keeps a
    /// switch off until somebody turns it on.
    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        Self {
            draft_head: matches!(value.get("draft_head"), Some(Value::Bool(true))),
            rope: value
                .get("rope_scaling")
                .and_then(Value::as_text)
                .and_then(Scaling::from_word),
            factor: value
                .get("rope_scale")
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok()),
        }
    }

    /// Whether these switches can be asked of this file at all.
    ///
    /// **Refused here rather than by the engine.** A draft head asked of a
    /// file that has none starts an engine that either ignores the switch or
    /// dies with a sentence about tensors; a scaling with no factor anywhere
    /// starts an engine that scales by one, which is a condition in the
    /// account and no change to the model. Both are things MCF can see
    /// before it spends the load (A2).
    ///
    /// # Errors
    ///
    /// `config.unsatisfiable` for a draft head the file does not have, and
    /// `config.invalid` for a scaling that would stretch nothing.
    pub fn against(&self, declared: &Declared) -> Result<(), Failure> {
        if self.draft_head && declared.draft_head.is_none() {
            return Err(Failure::new(
                Category::ConfigUnsatisfiable,
                Attribution::Artifact,
                Disposition::Refused,
                Subsystem::new("mcf-serve::declared"),
                "this model's file declares no draft head, so there is none to start: a file \
                 that carries one says how many layers it holds, and the engine reads them in \
                 only when it is asked to",
            ));
        }
        if self.factor.is_some() && self.rope.is_none() {
            return Err(refused(
                "a rope scaling factor was asked for with no scaling to apply it to: the factor \
                 says by how much, and the scaling says in what manner",
            ));
        }
        let stretches = matches!(self.rope, Some(Scaling::Linear | Scaling::Yarn));
        let factor_somewhere = self.factor.is_some()
            || declared
                .rope
                .as_ref()
                .is_some_and(|rope| rope.factor.is_some());
        if stretches && !factor_somewhere {
            return Err(refused(
                "this model's file declares no scaling factor, so a scaling asked for without \
                 one would stretch the positions by one and change nothing: --rope-scale <n> \
                 says by how much",
            ));
        }
        Ok(())
    }
}

/// A refusal the asker can act on.
fn refused(why: &'static str) -> Failure {
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::declared"),
        why,
    )
}

/// A count the record can hold, or nothing where it is larger than the
/// record's own integers — which is a figure MCF will not silently shrink.
fn whole(held: u128) -> Value {
    i64::try_from(held).map_or(Value::Null, Value::Integer)
}

/// A declared value as the file writes it.
///
/// A scaling factor is written in the file as a number with a fractional
/// part it never uses — `32` arrives as `32.0` — so it is carried as the
/// text of itself rather than converted into anything MCF computes with.
fn written(value: &Held) -> String {
    match value {
        Held::Integer(number) => number.to_string(),
        Held::Text(text) => text.clone(),
        Held::Float(held) => format!("{held}"),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests;
