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

/// A reasoning effort no template names, to ask whether one reads the
/// switch at all when the asked value renders as the unsaid one does.
const NO_SUCH_EFFORT: &str = "MCFNOSUCHEFFORTd41c7e";

/// A switch that changed nothing, refused by name.
///
/// **Says what was seen, not why.** A template that renders the same with
/// the switch as without it may have no such switch, or may have one that
/// the rest of the turn makes inert — Qwen3.8's reasoning effort is written
/// nowhere once thinking is off — and the rendering cannot tell those apart.
/// Either way the person asked for something the turn does not carry, and
/// saying *no such switch* to the second would be a diagnosis wearing an
/// observation's clothes (A21).
fn unread(switch: &str, turn: &Turn) -> Failure {
    refused(&format!(
        "the model's template renders the same with {switch} as without it, alongside the rest \
         of what was asked, so {switch} changes nothing in this turn"
    ))
    .with_context("switch", switch.to_owned())
    .with_context("asked", turn.said())
}

/// A switch's words for the account, saying where the asked position is the
/// one the template renders unsaid — which is a fact about the template and
/// not a choice MCF made for the person (§3.15).
fn worded(switch: String, as_unsaid: bool) -> String {
    if as_unsaid {
        format!("{switch}, as the template renders it unsaid")
    } else {
        switch
    }
}

/// Whether the template reads each switch asked for, with the switches'
/// words for the account.
///
/// **Each switch is asked on its own, against another position of itself.**
/// A first cut compared the rendering with every switch to the rendering
/// with none, and refused a model whose thinking is on unsaid when a person
/// asked for thinking on: the two renderings were the same because the
/// switch was already there, not because it was absent. So thinking is
/// asked against its other position; a reasoning effort that renders as the
/// unsaid one is asked once more with a word no template names, and a
/// template that raises on the word or renders differently reads the switch;
/// a system turn is asked against its absence, since a template with no
/// place for one drops it in silence.
fn read_switches(engine: &Served, turn: &Turn, rendered: &str) -> Result<Vec<String>, Failure> {
    let render = |other: &Turn| engine.render(other.messages(), other.switches());
    let mut asked = Vec::new();
    if let Some(on) = turn.thinking {
        let flipped = Turn {
            thinking: Some(!on),
            ..turn.clone()
        };
        if render(&flipped)? == rendered {
            return Err(unread("thinking", turn));
        }
        let unsaid = Turn {
            thinking: None,
            ..turn.clone()
        };
        asked.push(worded(
            format!("thinking {}", if on { "on" } else { "off" }),
            render(&unsaid)? == rendered,
        ));
    }
    if let Some(effort) = &turn.effort {
        let unsaid = Turn {
            effort: None,
            ..turn.clone()
        };
        let as_unsaid = render(&unsaid)? == rendered;
        if as_unsaid {
            let unknown = Turn {
                effort: Some(NO_SUCH_EFFORT.to_owned()),
                ..turn.clone()
            };
            match render(&unknown) {
                Ok(other) if other == rendered => return Err(unread("a reasoning effort", turn)),
                // The template raised on the word: it reads the switch.
                Err(failure) if failure.category() == Category::ConfigInvalid => {}
                Ok(_) => {}
                Err(failure) => return Err(failure),
            }
        }
        asked.push(worded(format!("reasoning effort {effort}"), as_unsaid));
    }
    if turn.system.is_some() {
        let without = Turn {
            system: None,
            ..turn.clone()
        };
        if render(&without)? == rendered {
            return Err(unread("a system turn", turn));
        }
        asked.push("a system turn".to_owned());
    }
    Ok(asked)
}

/// The turn as the engine renders it for this request.
///
/// **A switch the template does not have is refused, not passed over.** Each
/// switch asked for is checked against the template by rendering, in
/// [`read_switches`]; a person who asked for thinking off would otherwise be
/// told they had it (A2, A7).
///
/// # Errors
///
/// The template raised on what it was given, in its own words; it rendered
/// the prompt's place other than once; or a switch asked for changed
/// nothing.
pub fn frame(engine: &Served, turn: &Turn) -> Result<Frame, Failure> {
    let rendered = engine.render(turn.messages(), turn.switches())?;
    let asked = read_switches(engine, turn, &rendered)?;
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
        asked: if asked.is_empty() {
            "nothing switched".to_owned()
        } else {
            asked.join(", ")
        },
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
            // No pair of this file's own opened. A family that writes its
            // thought as channels rather than as a marker pair is counted
            // the other way, by the same figure (B-457).
            None => return channelled(engine, produced),
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

/// What stands in for the model's own answer while the template is asked
/// where it puts one.
///
/// The same shape as [`PLACE`] and for the same reason: plain letters and
/// digits, which no template trims, escapes or re-spells.
const ANSWER_PLACE: &str = "MCFANSWERPLACEd41c7e";

/// Where a template opens the model's answer, read from the template's own
/// rendering of one (B-457, D47).
///
/// **Some families do not write a thought inside a marker pair.** They write
/// it in a channel — the model names a channel, opens a message, says its
/// piece, ends it, and turns to another channel for the answer — and there
/// is no closer to find, because nothing was opened in the pair's sense. The
/// place the answer begins is still written down: it is what the template
/// puts in front of an assistant message, and the engine will render one on
/// request at no forward pass.
///
/// What comes back is the tail of that rendering from its second-to-last
/// marker: for a channel family that is the channel marker, the channel's
/// own name and the marker a message opens with — the string the model
/// writes when it turns to its answer. `None` where the rendering carries
/// fewer than two markers before the answer, which is every family that
/// opens an assistant turn with one marker and needs none of this.
fn answer_opener(engine: &Served) -> Option<String> {
    let turn = |role: &str, content: &str| {
        Value::map([
            ("role", Value::text(role)),
            ("content", Value::text(content)),
        ])
    };
    let rendered = engine
        .render(
            Value::List(vec![turn("user", PLACE), turn("assistant", ANSWER_PLACE)]),
            Value::map::<&str>([]),
        )
        .ok()?;
    let before = rendered.split(ANSWER_PLACE).next()?;
    let mut at: Vec<usize> = Vec::new();
    for marker in crate::probes::markers_in(before) {
        at.extend(before.match_indices(&marker).map(|(found, _)| found));
    }
    at.sort_unstable();
    at.dedup();
    let second_to_last = at.len().checked_sub(2).and_then(|back| at.get(back))?;
    before.get(*second_to_last..).map(str::to_owned)
}

/// What a model spent before its answer where its family writes channels
/// rather than a marker pair (B-457).
///
/// Counted as the same figure the pair path counts: every token up to and
/// including the one that opens the answer, the answer being what follows.
/// A turn that never reached that opener spent all of it before an answer
/// that never came, which is what `closed` says.
///
/// `Ok(None)` where this is not a channel family at all — the model wrote no
/// channel marker — because *not counted* is not nought (A7).
fn channelled(engine: &Served, produced: &[usize]) -> Result<Option<BeforeTheAnswer>, Failure> {
    let Some(opener) = answer_opener(engine) else {
        return Ok(None);
    };
    let ids: Vec<usize> = engine
        .tokenize(&opener, false, true)?
        .into_iter()
        .map(|token| token.id)
        .collect();
    let (Some(first), true) = (ids.first(), ids.len() > 1) else {
        return Ok(None);
    };
    // The model wrote a channel of its own, or this is not that kind of turn.
    let opened_at = produced.iter().position(|token| token == first);
    let Some(opened_at) = opened_at else {
        return Ok(None);
    };
    let closed_at = last_run(produced, &ids);
    let spent = closed_at.map_or(produced.len(), |at| at.saturating_add(ids.len()));
    let (thought, rest) = produced.split_at(spent.min(produced.len()));
    Ok(Some(BeforeTheAnswer {
        inside: what_it_opened(engine, produced, opened_at, &ids)?,
        // The turn's tail ended before the channel: the model opened it.
        opened_by: OpenedBy::Model,
        tokens: spent,
        closed: closed_at.is_some(),
        text: engine.detokenize(thought)?,
        answer: engine.detokenize(rest)?,
    }))
}

/// What the model opened, spelled: the channel it named and the marker its
/// message began with, from where it opened it.
fn what_it_opened(
    engine: &Served,
    produced: &[usize],
    opened_at: usize,
    ids: &[usize],
) -> Result<String, Failure> {
    let ends = ids.last();
    let until = produced
        .iter()
        .skip(opened_at)
        .position(|token| Some(token) == ends)
        .map_or(produced.len(), |at| {
            opened_at.saturating_add(at).saturating_add(1)
        });
    engine.detokenize(produced.get(opened_at..until).unwrap_or_default())
}

/// Where a run of identifiers last appears in another, or nothing where it
/// does not appear at all.
fn last_run(held: &[usize], run: &[usize]) -> Option<usize> {
    if run.is_empty() || held.len() < run.len() {
        return None;
    }
    (0..=held.len().saturating_sub(run.len()))
        .rev()
        .find(|at| held.get(*at..at.saturating_add(run.len())) == Some(run))
}

#[cfg(test)]
mod tests;
