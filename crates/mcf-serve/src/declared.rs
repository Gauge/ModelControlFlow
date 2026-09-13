use std::path::Path;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;
use mcf_standin::gguf::{Model, Value as Held};

const FRONT: [u64; 2] = [4 << 20, 64 << 20];

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Declared {
    pub draft_head: Option<u64>,
    pub rope: Option<Rope>,
    pub context: Option<u64>,
    pub architecture: Option<String>,
    pub thinking: crate::thinking::Thinking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rope {
    pub kind: String,
    pub factor: Option<String>,
    pub trained: Option<u64>,
}

impl Declared {
    #[must_use]
    pub fn of(model: &Path) -> Self {
        header(model)
            .map(|file| Self::in_header(&file))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn in_header(file: &Model) -> Self {
        let thinking = crate::thinking::Thinking::of(file);
        let Some(architecture) = file.architecture() else {
            return Self {
                thinking,
                ..Self::default()
            };
        };
        let under = |suffix: &str| file.get(&format!("{architecture}.{suffix}"));
        let number = |suffix: &str| {
            under(suffix)
                .and_then(Held::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        Self {
            architecture: Some(architecture.to_owned()),
            draft_head: number("nextn_predict_layers").filter(|layers| *layers > 0),
            rope: under("rope.scaling.type")
                .and_then(Held::as_text)
                .map(|kind| Rope {
                    kind: kind.to_owned(),
                    factor: under("rope.scaling.factor").map(written),
                    trained: number("rope.scaling.original_context_length"),
                }),
            context: number("context_length"),
            thinking,
        }
    }

    #[must_use]
    pub fn not_started(&self, started: &Started) -> Option<String> {
        let layers = self.draft_head.filter(|_| !started.draft_head)?;
        let plural = if layers == 1 { "" } else { "s" };
        Some(format!(
            "a draft head of {layers} layer{plural}, which the engine leaves in the file unless \
             it is asked for"
        ))
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let number = |held: Option<&Value>| {
            held.and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        let rope = value.get("rope_scaling");
        Self {
            architecture: value
                .get("architecture")
                .and_then(Value::as_text)
                .map(str::to_owned),
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
            thinking: crate::thinking::Thinking::from_value(value.get("thinking_levels")),
        }
    }

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
            ("thinking_levels", self.thinking.to_value()),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Started {
    pub draft_head: bool,
    pub rope: Option<Scaling>,
    pub factor: Option<u32>,
    pub window: Option<u64>,
    pub drafted: Option<u32>,
    pub trained: Option<u64>,
    pub lift: Option<u64>,
    pub architecture: Option<String>,
    pub thinking: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaling {
    Off,
    Linear,
    Yarn,
}

impl Scaling {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "none",
            Self::Linear => "linear",
            Self::Yarn => "yarn",
        }
    }

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
    #[must_use]
    pub const fn asks_anything(&self) -> bool {
        self.draft_head
            || self.rope.is_some()
            || self.factor.is_some()
            || self.drafted.is_some()
            || self.trained.is_some()
            || self.lift.is_some()
            || self.thinking.is_some()
    }

    #[must_use]
    pub fn same_switches(&self, other: &Self) -> bool {
        self.draft_head == other.draft_head
            && self.rope == other.rope
            && self.factor == other.factor
            && self.drafted == other.drafted
            && self.trained == other.trained
            && self.lift == other.lift
            && self.thinking == other.thinking
    }

    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.draft_head {
            out.push("--spec-type".to_owned());
            out.push("draft-mtp".to_owned());
        }
        if let Some(drafted) = self.drafted {
            out.push("--spec-draft-n-max".to_owned());
            out.push(drafted.to_string());
        }
        if let Some(rope) = self.rope {
            out.push("--rope-scaling".to_owned());
            out.push(rope.as_str().to_owned());
        }
        if let Some(factor) = self.factor {
            out.push("--rope-scale".to_owned());
            out.push(factor.to_string());
        }
        if let Some(trained) = self.trained {
            out.push("--yarn-orig-ctx".to_owned());
            out.push(trained.to_string());
        }
        if let (Some(lift), Some(architecture)) = (self.lift, self.architecture.as_ref()) {
            out.push("--override-kv".to_owned());
            out.push(format!("{architecture}.context_length=int:{lift}"));
        }
        if let Some(thinking) = self.thinking {
            out.push("--reasoning-budget".to_owned());
            out.push(thinking.to_string());
        }
        out
    }

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
        if let Some(window) = self.window {
            said.push(format!("a window of {window} tokens"));
        }
        if let Some(drafted) = self.drafted {
            said.push(format!("{drafted} tokens drafted at a time"));
        }
        if let Some(trained) = self.trained {
            said.push(format!("scaled from the {trained} it was trained on"));
        }
        if let Some(lift) = self.lift {
            said.push(format!("its declared ceiling lifted to {lift}"));
        }
        if let Some(thinking) = self.thinking {
            said.push(if thinking == 0 {
                "thinking cut off at once".to_owned()
            } else {
                format!("thinking capped at {thinking} tokens")
            });
        }
        if said.is_empty() {
            "nothing beyond the plain load".to_owned()
        } else {
            said.join(", ")
        }
    }

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
            (
                "drafted",
                self.drafted
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held))),
            ),
            (
                "trained",
                self.trained
                    .map_or(Value::Null, |held| whole(u128::from(held))),
            ),
            (
                "lift",
                self.lift
                    .map_or(Value::Null, |held| whole(u128::from(held))),
            ),
            (
                "architecture",
                self.architecture
                    .as_ref()
                    .map_or(Value::Null, |held| Value::text(held.clone())),
            ),
            (
                "thinking",
                self.thinking
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held))),
            ),
            (
                "window",
                self.window.map_or(Value::Null, |window| {
                    Value::Integer(i64::try_from(window).unwrap_or(i64::MAX))
                }),
            ),
        ])
    }

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
            window: value
                .get("window")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            drafted: value
                .get("drafted")
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok()),
            trained: value
                .get("trained")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            lift: value
                .get("lift")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            architecture: value
                .get("architecture")
                .and_then(Value::as_text)
                .map(str::to_owned),
            thinking: value
                .get("thinking")
                .and_then(Value::as_integer)
                .and_then(|held| u32::try_from(held).ok()),
        }
    }

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

fn refused(why: &'static str) -> Failure {
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::declared"),
        why,
    )
}

fn whole(held: u128) -> Value {
    i64::try_from(held).map_or(Value::Null, Value::Integer)
}

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
