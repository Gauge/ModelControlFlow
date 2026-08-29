//! Whether a model *emits* the shape it was asked for, or only text about it
//! (B-054, D42, §X, A21, A6).
//!
//! **A characterizing probe** (D42): it describes the model and changes nothing
//! about how MCF addresses it. Its sibling [`super::tools`] asks whether a call
//! comes out; this asks the more general question underneath — asked for output
//! of a stated shape, does the model produce that shape.
//!
//! **What conformance means here, and it needs no judgement.** Four mechanical
//! questions, each answerable by a parser (A19):
//!
//! 1. Did anything object-shaped come out?
//! 2. Does it parse as JSON?
//! 3. Is every requested field there?
//! 4. Is each of them of the requested kind — text, a number, a boolean?
//!
//! **Nothing here reads a value for sense.** Whether the model counted the
//! words correctly is a *capability*, measured by a laboratory against a
//! graded task (§XIII); whether it produced an integer where an integer was
//! asked for is a *shape*, and this probe observes only the second. A probe
//! that graded the answer would be scoring the model, and it would report a
//! model that conforms perfectly and cannot count as though it could not
//! produce JSON.
//!
//! **The task carries no knowledge.** Every field is derivable from a sentence
//! put in front of the model, so a model that does not know a fact is never
//! confounded with one that cannot produce a shape — the same reason
//! [`super::tools`] asks about weather rather than about arithmetic.
//!
//! **A field it added is not a field it got wrong.** Extra keys are recorded
//! and do not make a trial non-conforming: what was asked for is present and
//! of the right kind, which is the question. A caller who needs exactly the
//! requested keys and no others is asking something narrower, and the count of
//! trials that added keys is there for them to read (A1).
//!
//! **The framing is a condition, not a constant.** How a shape is described to
//! a model is MCF's own choice (D46 — MCF does not execute templates), and one
//! chosen framing deciding the answer would make this a measurement of the
//! framing. Three are tried and each is reported with its own count, the way
//! [`super::tools`] reports each offering.

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

use super::{Addressing, Trial};

/// A field the model is asked for, and the kind of thing it must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    /// Its key.
    pub name: &'static str,
    /// What kind of value conforms.
    pub kind: Kind,
    /// How it is described to the model.
    pub means: &'static str,
}

/// The kinds a field can be asked for.
///
/// Three, because three is what it takes to tell *emitted an object with the
/// right keys* from *emitted an object whose values are all strings* — a real
/// and common way for a model to half-conform, and one a single-kind shape
/// could not observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A JSON string.
    Text,
    /// A JSON number, which the record's parser reads as an integer.
    Number,
    /// A JSON `true` or `false`.
    Boolean,
}

impl Kind {
    /// What it is called when the shape is described to the model.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }

    /// Whether a value is of this kind.
    ///
    /// The record's `Value` has no float (A6's arithmetic discipline), so a
    /// number arrives as an integer; a model that answers `6.0` where a number
    /// was asked for is reported as having emitted something that is not a
    /// number **by this reader**, which is stated rather than hidden.
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

/// The sentence the model is asked about, and the shape it is asked for.
///
/// Every field is derivable from the sentence itself: a model that cannot do
/// world knowledge is not thereby a model that cannot produce a shape.
pub const SENTENCE: &str = "The cat sat on the mat.";

/// The shape asked for.
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

/// What one trial produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    /// An object came out with every requested field, each of its kind.
    Conformed {
        /// Keys beyond the ones asked for. Not a failure; a fact (A1).
        extra: Vec<String>,
    },
    /// Something object-shaped came out and did not conform.
    ///
    /// Kept apart from [`Self::NoObject`] because they are different facts: one
    /// tried and got the shape wrong, the other did not try.
    Departed {
        /// Which question it failed, with what came out.
        because: String,
    },
    /// Nothing object-shaped came out and the model finished its turn.
    ///
    /// It answered in prose, or said nothing, and it was *done* — which is
    /// what makes this an observation about the model rather than about the
    /// budget.
    NoObject,
    /// Nothing object-shaped came out and the turn was still going when the
    /// budget ran out.
    ///
    /// **Not [`Self::NoObject`]** (A1, A7, F101's lesson repeated). A model
    /// cut off mid-sentence has not declined to produce a shape; MCF stopped
    /// it. Folding the two together would report *this model does not do
    /// structured output* about a model that was still writing the object —
    /// and the first real model this probe met produced exactly that shape of
    /// answer under two of three framings (F106).
    Unfinished,
    /// The trial itself could not be run or read (D42's third state).
    CouldNotTell {
        /// Why.
        because: String,
    },
}

/// One way of describing a shape to a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framing {
    /// What to call it in a result.
    pub name: String,
    /// The text put to the model.
    pub text: String,
}

/// What the structured-output probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structured {
    /// Every framing, and how many of its trials conformed.
    pub conformed: Vec<(String, usize)>,
    /// Every framing, and how many produced an object that did not conform.
    pub departed: Vec<(String, usize)>,
    /// Every framing, and how many produced nothing object-shaped in a turn
    /// the model itself ended.
    pub no_object: Vec<(String, usize)>,
    /// Every framing, and how many were still going when the budget ran out.
    ///
    /// A count of what MCF interrupted, kept apart from what the model
    /// declined to do (A7).
    pub unfinished: Vec<(String, usize)>,
    /// How many conforming trials also carried keys nobody asked for.
    pub with_extra: usize,
    /// Why the departures departed, in order and in the model's own output, so
    /// a reader sees what happened rather than a count of failures (A1).
    pub reasons: Vec<String>,
    /// How many trials each framing had.
    pub of: usize,
    /// The framing that produced the most conforming trials, if any did.
    pub best: Option<String>,
}

impl Structured {
    /// How many trials conformed, across every framing.
    #[must_use]
    pub fn conforming(&self) -> usize {
        self.conformed
            .iter()
            .map(|(_, count)| *count)
            .fold(0, usize::saturating_add)
    }
}

/// How a caller runs one trial, as in [`super::tools`]: the wrapped turn and a
/// budget in, what the model said and how the turn ended out.
pub type Ask<'a> = &'a mut dyn FnMut(&[usize], usize) -> (Trial, String);

/// The method, written where the result carries it.
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

/// Runs the structured-output probe.
///
/// `generate` takes the wrapped question and a budget and answers with what the
/// model said and how the turn ended, which keeps this independent of which
/// engine ran it — the engine is a condition and the caller states it.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason,
/// which is D42's third state rather than a failure.
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
    let Ok(file) = gguf::parse(bytes) else {
        return Probed::inconclusive(
            STRUCTURED_OUTPUT,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let Ok(vocabulary) = Vocabulary::read(&file) else {
        return Probed::inconclusive(
            STRUCTURED_OUTPUT,
            "the vocabulary could not be read",
            0,
            0,
            conditions,
        );
    };

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
            let Some(wrapped) = wrap(addressing, &vocabulary, &framing.text) else {
                undecidable.push(format!(
                    "{}: the question could not be put through this model's vocabulary",
                    framing.name
                ));
                break;
            };
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

    // Nothing decidable at all is the third state, not a negative: *the model
    // did not produce a shape* and *MCF could not tell* are different facts
    // (D42).
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

/// One framing's trials, counted.
///
/// A struct rather than five locals, because the five counts are one thing —
/// what happened under this framing — and a probe that added a sixth outcome
/// and forgot to thread it through would be reporting a total that does not add
/// up (A4).
#[derive(Debug, Default)]
struct Tally {
    /// Trials whose output carried every field of the kind asked for.
    conformed: usize,
    /// Trials that produced an object which did not conform.
    departed: usize,
    /// Trials that finished the turn with nothing object-shaped.
    no_object: usize,
    /// Trials the budget cut short with nothing object-shaped yet.
    unfinished: usize,
    /// Conforming trials that also carried keys nobody asked for.
    with_extra: usize,
}

impl Tally {
    /// Files one trial, and keeps what it said where a reader will see it.
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

/// The question, wrapped the way the chat-template probe found this model wants
/// to be addressed — or bare, where nothing was found.
fn wrap(
    addressing: Option<&Addressing>,
    vocabulary: &Vocabulary,
    question: &str,
) -> Option<Vec<usize>> {
    match addressing {
        Some(addressing) => addressing.wrap(vocabulary, question),
        None => vocabulary.encode(question, true).ok(),
    }
}

/// The three ways the shape is described, likeliest to work last.
///
/// They differ in *how much of the answer is shown*, which is the axis a
/// framing can be wrong along: a description in words, a schema, and a filled
/// example. A model that conforms only when shown an example has told MCF
/// something true about itself, and one that conforms under all three has told
/// it something stronger.
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

/// Reads one answer: did the shape come out?
fn read(said: &str, trial: &Trial) -> Attempt {
    if let Trial::CouldNotTell(because) = trial {
        return Attempt::CouldNotTell {
            because: because.clone(),
        };
    }
    let Some(candidate) = bare(said) else {
        // Nothing object-shaped — and *why the turn ended* decides which fact
        // that is. A turn the budget cut short says nothing about whether the
        // model would have produced the shape (A7).
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

/// The first balanced `{…}` in an answer, which is what an object looks like in
/// a reply that may carry prose around it.
///
/// The same reader [`super::tools`] uses, because *what an object looks like in
/// a model's output* is one question and two answers to it would eventually
/// disagree (F79).
fn bare(said: &str) -> Option<String> {
    super::tools::first_object(said)
}

#[cfg(test)]
mod tests;
