//! A turn the engine renders from the model's own template, and what the
//! model spends before it answers (D47, B-450, B-451).
//!
//! **Why the engine and not MCF.** A model's template is a small program, and
//! MCF does not run it (D46). The engine that answers the turn runs it for
//! every chat request it serves, and it will say what it would send when
//! asked — so the switches the template offers, thinking on or off and how
//! hard to reason, reach the model in the template's own words rather than
//! in a spelling MCF guessed at. What comes back is the engine's rendering
//! (A4), and the account says it was.
//!
//! **What a person's words never become.** The rendered frame is read with
//! its markers taken as markers: it is the template's text, from the model's
//! own file. The person's prompt goes in between and is read without that,
//! so `<|im_start|>` typed into a prompt stays seven characters of text and
//! never opens a turn (F26, F161).
//!
//! **What comes before the answer is counted, not judged.** A model that
//! opens a marker of its own, says a great deal inside it and closes it
//! before its first word of answer has spent tokens a budget has to cover
//! and a reader wants set apart (B-421, F106). Which marker is the model's to
//! say: the candidates come from its template, survive only where its own
//! engine reads each as one token, and the one that appears is the one
//! counted.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;

use crate::served::Served;

/// How a person asked the turn to be framed.
///
/// Every field is *what was said*: `None` is the switch left alone, which is
/// not the same as either position of it (D43, §3.15).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Turn {
    /// Thinking on or off, where the person said; the template's own default
    /// where they did not.
    pub thinking: Option<bool>,
    /// How hard to reason, in the template's own vocabulary (`low`,
    /// `medium`, `high`, …), where the person said.
    pub effort: Option<String>,
    /// A system turn, where the person wrote one.
    pub system: Option<String>,
}

impl Turn {
    /// The record's shape.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("thinking", self.thinking.map_or(Value::Null, Value::Bool)),
            (
                "effort",
                self.effort
                    .as_ref()
                    .map_or(Value::Null, |effort| Value::text(effort.clone())),
            ),
            (
                "system",
                self.system
                    .as_ref()
                    .map_or(Value::Null, |system| Value::text(system.clone())),
            ),
        ])
    }

    /// A turn as a record holds it; `None` where the value is not a map.
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let Value::Map(fields) = value else {
            return None;
        };
        let text = |key: &str| fields.get(key).and_then(Value::as_text).map(str::to_owned);
        Some(Self {
            thinking: match fields.get("thinking") {
                Some(Value::Bool(on)) => Some(*on),
                _ => None,
            },
            effort: text("effort"),
            system: text("system"),
        })
    }

    /// Whether anything at all was asked.
    #[must_use]
    pub fn asks_anything(&self) -> bool {
        self.thinking.is_some() || self.effort.is_some() || self.system.is_some()
    }

    /// The switches as the template names them, for the engine.
    fn switches(&self) -> Value {
        let mut switches = Vec::new();
        if let Some(on) = self.thinking {
            switches.push(("enable_thinking", Value::Bool(on)));
        }
        if let Some(effort) = &self.effort {
            switches.push(("reasoning_effort", Value::text(effort.clone())));
        }
        Value::map(switches)
    }

    /// The conversation the template is asked to render, with the prompt's
    /// place held by [`PLACE`].
    fn messages(&self) -> Value {
        let turn = |role: &str, content: &str| {
            Value::map([
                ("role", Value::text(role)),
                ("content", Value::text(content)),
            ])
        };
        let mut messages = Vec::new();
        if let Some(system) = &self.system {
            messages.push(turn("system", system));
        }
        messages.push(turn("user", PLACE));
        Value::List(messages)
    }

    /// What was asked, in words, for the account.
    #[must_use]
    pub fn said(&self) -> String {
        let mut said = Vec::new();
        if let Some(on) = self.thinking {
            said.push(format!("thinking {}", if on { "on" } else { "off" }));
        }
        if let Some(effort) = &self.effort {
            said.push(format!("reasoning effort {effort}"));
        }
        if self.system.is_some() {
            said.push("a system turn".to_owned());
        }
        if said.is_empty() {
            "nothing switched".to_owned()
        } else {
            said.join(", ")
        }
    }
}

/// What stands in for the prompt while the template is rendered.
///
/// Plain letters and digits, so that no template trims, escapes or
/// re-spells it, and unlikely enough that a template of its own accord
/// writes it nowhere. A rendering in which it appears other than once is
/// refused rather than guessed at.
pub const PLACE: &str = "MCFPROMPTPLACEd41c7e";

/// A turn as the engine rendered it, with the prompt's place cut out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The template's text before the person's words.
    pub before: String,
    /// The template's text after them, up to where the model writes.
    pub after: String,
    /// What was asked of the template, in words.
    pub asked: String,
}

fn refused(why: &str) -> Failure {
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::turn"),
        why,
    )
}

/// The turn as the engine renders it for this request.
///
/// **A switch the template does not have is refused, not passed over.** The
/// rendering with the switches is compared to the rendering without them; a
/// template that renders the same either way has no such switch, and a
/// person who asked for thinking off would otherwise be told they had it
/// (A2, A7).
///
/// # Errors
///
/// The template raised on what it was given, in its own words; it rendered
/// the prompt's place other than once; or the switches asked for changed
/// nothing.
pub fn frame(engine: &Served, turn: &Turn) -> Result<Frame, Failure> {
    let rendered = engine.render(turn.messages(), turn.switches())?;
    if turn.thinking.is_some() || turn.effort.is_some() {
        let unswitched = engine.render(turn.messages(), Value::map::<&str>([]))?;
        if unswitched == rendered {
            return Err(refused(
                "the model's template renders the same with these switches as without \
                 them, so the model has no such switch",
            )
            .with_context("asked", turn.said()));
        }
    }
    let mut places = rendered.match_indices(PLACE);
    let (Some((at, _)), None) = (places.next(), places.next()) else {
        return Err(
            refused("the model's template did not render the prompt's place exactly once")
                .with_context("rendered", rendered.chars().take(400).collect::<String>()),
        );
    };
    Ok(Frame {
        before: rendered.get(..at).unwrap_or_default().to_owned(),
        after: rendered
            .get(at.saturating_add(PLACE.len())..)
            .unwrap_or_default()
            .to_owned(),
        asked: turn.said(),
    })
}

/// Who opened the marker the model's turn began inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenedBy {
    /// The rendered turn ended with the opener, so the model began inside it.
    Turn,
    /// The model's first token was the opener.
    Model,
}

impl OpenedBy {
    /// The account's word.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Turn => "the turn",
            Self::Model => "the model",
        }
    }
}

/// What a turn spent inside a marker before its first word of answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeforeTheAnswer {
    /// The marker it was inside.
    pub inside: String,
    /// Who opened it.
    pub opened_by: OpenedBy,
    /// How many of the produced tokens were spent there, the closer
    /// included where it came.
    pub tokens: usize,
    /// Whether the marker closed before the generation ended. A turn that
    /// did not close was cut by the budget, and every token of it is
    /// before an answer that never came (F106).
    pub closed: bool,
    /// The spent tokens as text, spelled by the engine.
    pub text: String,
    /// What followed, spelled by the engine: the answer.
    pub answer: String,
}

impl BeforeTheAnswer {
    /// The account's shape.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("inside", Value::text(self.inside.clone())),
            ("opened_by", Value::text(self.opened_by.said())),
            (
                "tokens",
                Value::Integer(i64::try_from(self.tokens).unwrap_or(i64::MAX)),
            ),
            ("closed", Value::Bool(self.closed)),
            (
                "text_bytes",
                Value::Integer(i64::try_from(self.text.len()).unwrap_or(i64::MAX)),
            ),
        ])
    }
}

/// A marker pair the model's own engine reads as two tokens.
struct Pair {
    opener: String,
    opener_id: usize,
    closer_id: usize,
}

/// Every pair the template names that the engine holds as tokens.
fn pairs_of(engine: &Served, template: &str) -> Vec<Pair> {
    let mut pairs = Vec::new();
    let mut seen = Vec::new();
    for marker in crate::probes::markers_in(template) {
        if seen.contains(&marker) {
            continue;
        }
        seen.push(marker.clone());
        let Some(closing) = crate::probes::thinking::closing_form(&marker) else {
            continue;
        };
        let one = |text: &str| match engine.tokenize(text, false, true) {
            Ok(read) if read.len() == 1 => read.first().map(|token| token.id),
            _ => None,
        };
        if let (Some(opener_id), Some(closer_id)) = (one(&marker), one(&closing)) {
            pairs.push(Pair {
                opener: marker,
                opener_id,
                closer_id,
            });
        }
    }
    pairs
}

/// What the model spent before its answer, or `None` where its turn began
/// with an answer.
///
/// `tail` is the rendered turn's text after the person's words, where there
/// was one: a turn that ends with an opener has put the model inside it.
///
/// # Errors
///
/// The engine would not spell the tokens.
pub fn before_the_answer(
    engine: &Served,
    template: &str,
    tail: Option<&str>,
    produced: &[usize],
) -> Result<Option<BeforeTheAnswer>, Failure> {
    let pairs = pairs_of(engine, template);
    let opened_by_turn = tail.and_then(|tail| {
        let tail = tail.trim_end();
        pairs.iter().find(|pair| tail.ends_with(&pair.opener))
    });
    let (pair, opened_by, from) = match (opened_by_turn, produced.first()) {
        (Some(pair), _) => (pair, OpenedBy::Turn, 0),
        (None, Some(first)) => match pairs.iter().find(|pair| pair.opener_id == *first) {
            Some(pair) => (pair, OpenedBy::Model, 1),
            None => return Ok(None),
        },
        (None, None) => return Ok(None),
    };
    let closed_at = produced
        .iter()
        .skip(from)
        .position(|token| *token == pair.closer_id)
        .map(|at| at.saturating_add(from));
    let spent = closed_at.map_or(produced.len(), |at| at.saturating_add(1));
    let (thought, rest) = produced.split_at(spent.min(produced.len()));
    Ok(Some(BeforeTheAnswer {
        inside: pair.opener.clone(),
        opened_by,
        tokens: spent,
        closed: closed_at.is_some(),
        text: engine.detokenize(thought)?,
        answer: engine.detokenize(rest)?,
    }))
}

#[cfg(test)]
mod tests;
