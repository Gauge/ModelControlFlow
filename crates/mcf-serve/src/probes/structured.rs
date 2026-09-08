use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::tokenizer::Piece;

use super::{Addressing, Trial};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub kind: Kind,
    pub means: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Number,
    Boolean,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }

    #[must_use]
    pub const fn matches(self, value: &Value) -> bool {
        matches!(
            (self, value),
            (Self::Text, Value::Text(_))
                | (Self::Number, Value::Integer(_))
                | (Self::Boolean, Value::Bool(_))
        )
    }
}

pub const SENTENCE: &str = "The cat sat on the mat.";

pub const SHAPE: [Field; 3] = [
    Field {
        name: "sentence",
        kind: Kind::Text,
        means: "the sentence itself, copied",
    },
    Field {
        name: "words",
        kind: Kind::Number,
        means: "how many words it has",
    },
    Field {
        name: "is_question",
        kind: Kind::Boolean,
        means: "whether it is a question",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    Conformed { extra: Vec<String> },
    Departed { because: String },
    NoObject,
    Unfinished,
    CouldNotTell { because: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framing {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structured {
    pub conformed: Vec<(String, usize)>,
    pub departed: Vec<(String, usize)>,
    pub no_object: Vec<(String, usize)>,
    pub unfinished: Vec<(String, usize)>,
    pub with_extra: usize,
    pub reasons: Vec<String>,
    pub of: usize,
    pub best: Option<String>,
}

impl Structured {
    #[must_use]
    pub fn conforming(&self) -> usize {
        self.conformed
            .iter()
            .map(|(_, count)| *count)
            .fold(0, usize::saturating_add)
    }
}

pub type Ask<'a> = &'a mut dyn FnMut(&[Piece], usize) -> (Trial, String);

pub const STRUCTURED_OUTPUT: Method = Method {
    name: "structured-output",
    asks: "asks for one JSON object with three named fields of three different kinds, about a \
           sentence put in front of the model so that no world knowledge is needed, through \
           each of three ways of describing a shape, and counts the trials whose output parses \
           and carries every field of the kind asked for",
    decides: "whether MCF may ask this model for a shape and expect one — and nothing else: \
              whether the values are *right* is a capability a laboratory grades, not a shape a \
              parser reads (D42, §XIII)",
};

pub fn structured_output(
    model: &Path,
    bytes: &[u8],
    addressing: Option<&Addressing>,
    trials: usize,
    budget: usize,
    engine: &str,
    generate: Ask<'_>,
) -> Probed<Structured> {
    let conditions = super::conditions(&STRUCTURED_OUTPUT, model, engine);
    if gguf::parse(bytes).is_err() {
        return Probed::inconclusive(
            STRUCTURED_OUTPUT,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    }

    let mut conformed = Vec::new();
    let mut departed = Vec::new();
    let mut no_object = Vec::new();
    let mut unfinished = Vec::new();
    let mut reasons = Vec::new();
    let mut with_extra = 0_usize;
    let mut spent = 0_usize;
    let mut undecidable = Vec::new();

    for framing in framings() {
        let mut tally = Tally::default();
        for _ in 0..trials {
            let wrapped = Addressing::wrapped(addressing, &framing.text);
            let (trial, said) = generate(&wrapped, budget);
            spent = spent.saturating_add(budget);
            tally.add(
                read(&said, &trial),
                &framing.name,
                &mut reasons,
                &mut undecidable,
            );
        }
        with_extra = with_extra.saturating_add(tally.with_extra);
        conformed.push((framing.name.clone(), tally.conformed));
        departed.push((framing.name.clone(), tally.departed));
        no_object.push((framing.name.clone(), tally.no_object));
        unfinished.push((framing.name.clone(), tally.unfinished));
    }

    let decided: usize = conformed
        .iter()
        .chain(departed.iter())
        .chain(no_object.iter())
        .chain(unfinished.iter())
        .map(|(_, count)| *count)
        .fold(0, usize::saturating_add);
    if decided == 0 {
        let because = undecidable
            .first()
            .cloned()
            .unwrap_or_else(|| "no framing could be put to this model".to_owned());
        return Probed::inconclusive(STRUCTURED_OUTPUT, &because, trials, spent, conditions);
    }

    let best = conformed
        .iter()
        .filter(|(_, count)| *count > 0)
        .max_by_key(|(_, count)| *count)
        .map(|(name, _)| name.clone());

    Probed {
        method: STRUCTURED_OUTPUT,
        outcome: Outcome::Observed(Structured {
            conformed,
            departed,
            no_object,
            unfinished,
            with_extra,
            reasons,
            of: trials,
            best,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

#[derive(Debug, Default)]
struct Tally {
    conformed: usize,
    departed: usize,
    no_object: usize,
    unfinished: usize,
    with_extra: usize,
}

impl Tally {
    fn add(
        &mut self,
        attempt: Attempt,
        framing: &str,
        reasons: &mut Vec<String>,
        undecidable: &mut Vec<String>,
    ) {
        match attempt {
            Attempt::Conformed { extra } => {
                self.conformed = self.conformed.saturating_add(1);
                if !extra.is_empty() {
                    self.with_extra = self.with_extra.saturating_add(1);
                    reasons.push(format!(
                        "{framing}: conformed and added {}",
                        extra.join(", ")
                    ));
                }
            }
            Attempt::Departed { because } => {
                self.departed = self.departed.saturating_add(1);
                reasons.push(format!("{framing}: {because}"));
            }
            Attempt::NoObject => self.no_object = self.no_object.saturating_add(1),
            Attempt::Unfinished => self.unfinished = self.unfinished.saturating_add(1),
            Attempt::CouldNotTell { because } => undecidable.push(format!("{framing}: {because}")),
        }
    }
}

fn framings() -> Vec<Framing> {
    let described = SHAPE
        .iter()
        .map(|field| {
            format!(
                "\"{}\" ({}): {}",
                field.name,
                field.kind.as_str(),
                field.means
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let schema = SHAPE
        .iter()
        .map(|field| format!("\"{}\": \"{}\"", field.name, field.kind.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let example = "{\"sentence\": \"A dog barked loudly.\", \"words\": 4, \"is_question\": false}";

    vec![
        Framing {
            name: "described in words".to_owned(),
            text: format!(
                "Reply with only a JSON object about the sentence below, with these fields: \
                 {described}.\n\nSentence: {SENTENCE}"
            ),
        },
        Framing {
            name: "a schema".to_owned(),
            text: format!(
                "Reply with only a JSON object matching this schema:\n{{{schema}}}\n\nIt is \
                 about this sentence: {SENTENCE}"
            ),
        },
        Framing {
            name: "an example filled in".to_owned(),
            text: format!(
                "Here is the format, filled in for a different sentence:\n{example}\n\nReply \
                 with only the same object for this sentence: {SENTENCE}"
            ),
        },
    ]
}

fn read(said: &str, trial: &Trial) -> Attempt {
    if let Trial::CouldNotTell(because) = trial {
        return Attempt::CouldNotTell {
            because: because.clone(),
        };
    }
    let Some(candidate) = bare(said) else {
        return match trial {
            Trial::RanOut => Attempt::Unfinished,
            Trial::Stopped { .. } | Trial::CouldNotTell(_) => Attempt::NoObject,
        };
    };
    let parsed = match mcf_record::json::parse(&candidate) {
        Ok(value) => value,
        Err(error) => {
            return Attempt::Departed {
                because: format!("what came out did not parse as JSON ({error}): {candidate:?}"),
            };
        }
    };
    let Value::Map(fields) = &parsed else {
        return Attempt::Departed {
            because: format!("what came out is not an object: {candidate:?}"),
        };
    };
    for field in SHAPE {
        let Some(value) = fields.get(field.name) else {
            return Attempt::Departed {
                because: format!("no \"{}\" field: {candidate:?}", field.name),
            };
        };
        if !field.kind.matches(value) {
            return Attempt::Departed {
                because: format!(
                    "\"{}\" is {}, and a {} was asked for: {candidate:?}",
                    field.name,
                    value.to_line(),
                    field.kind.as_str()
                ),
            };
        }
    }
    let extra = fields
        .keys()
        .filter(|key| !SHAPE.iter().any(|field| field.name == key.as_str()))
        .cloned()
        .collect();
    Attempt::Conformed { extra }
}

fn bare(said: &str) -> Option<String> {
    super::tools::first_object(said)
}

#[cfg(test)]
mod tests;
