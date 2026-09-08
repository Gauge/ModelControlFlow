use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;

use crate::served::Served;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Turn {
    pub thinking: Option<bool>,
    pub effort: Option<String>,
    pub system: Option<String>,
}

impl Turn {
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

    #[must_use]
    pub fn asks_anything(&self) -> bool {
        self.thinking.is_some() || self.effort.is_some() || self.system.is_some()
    }

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

pub const PLACE: &str = "MCFPROMPTPLACEd41c7e";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub before: String,
    pub after: String,
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

const NO_SUCH_EFFORT: &str = "MCFNOSUCHEFFORTd41c7e";

fn unread(switch: &str, turn: &Turn) -> Failure {
    refused(&format!(
        "the model's template renders the same with {switch} as without it, alongside the rest \
         of what was asked, so {switch} changes nothing in this turn"
    ))
    .with_context("switch", switch.to_owned())
    .with_context("asked", turn.said())
}

fn worded(switch: String, as_unsaid: bool) -> String {
    if as_unsaid {
        format!("{switch}, as the template renders it unsaid")
    } else {
        switch
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenedBy {
    Turn,
    Model,
}

impl OpenedBy {
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Turn => "the turn",
            Self::Model => "the model",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeforeTheAnswer {
    pub inside: String,
    pub opened_by: OpenedBy,
    pub tokens: usize,
    pub closed: bool,
    pub text: String,
    pub answer: String,
}

impl BeforeTheAnswer {
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

struct Pair {
    opener: String,
    opener_id: usize,
    closer_id: usize,
}

fn pairs_of(engine: &Served, template: &str) -> Vec<Pair> {
    let mut pairs = Vec::new();
    let mut seen = Vec::new();
    for marker in markers_in(template) {
        if seen.contains(&marker) {
            continue;
        }
        seen.push(marker.clone());
        let Some(closing) = closing_form(&marker) else {
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

const ANSWER_PLACE: &str = "MCFANSWERPLACEd41c7e";

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
    for marker in markers_in(before) {
        at.extend(before.match_indices(&marker).map(|(found, _)| found));
    }
    at.sort_unstable();
    at.dedup();
    let second_to_last = at.len().checked_sub(2).and_then(|back| at.get(back))?;
    before.get(*second_to_last..).map(str::to_owned)
}

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
    let opened_at = produced.iter().position(|token| token == first);
    let Some(opened_at) = opened_at else {
        return Ok(None);
    };
    let closed_at = last_run(produced, &ids);
    let spent = closed_at.map_or(produced.len(), |at| at.saturating_add(ids.len()));
    let (thought, rest) = produced.split_at(spent.min(produced.len()));
    Ok(Some(BeforeTheAnswer {
        inside: what_it_opened(engine, produced, opened_at, &ids)?,
        opened_by: OpenedBy::Model,
        tokens: spent,
        closed: closed_at.is_some(),
        text: engine.detokenize(thought)?,
        answer: engine.detokenize(rest)?,
    }))
}

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

pub(crate) fn markers_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let characters: Vec<char> = text.chars().collect();
    let mut at = 0;
    while at < characters.len() {
        let opener = characters.get(at).copied();
        let closer = match opener {
            Some('<') => '>',
            Some('[') => ']',
            _ => {
                at = at.saturating_add(1);
                continue;
            }
        };
        let mut end = at.saturating_add(1);
        while end < characters.len() && characters.get(end).copied() != Some(closer) {
            end = end.saturating_add(1);
        }
        if end < characters.len() {
            found.push(
                characters
                    .get(at..=end)
                    .unwrap_or_default()
                    .iter()
                    .collect(),
            );
            at = end.saturating_add(1);
        } else {
            at = at.saturating_add(1);
        }
    }
    found
}

fn closing_form(marker: &str) -> Option<String> {
    let mut characters = marker.chars();
    let opener = characters.next()?;
    let closer = match opener {
        '<' => '>',
        '[' => ']',
        _ => return None,
    };
    if !marker.ends_with(closer) {
        return None;
    }
    let inner = marker.get(opener.len_utf8()..marker.len() - closer.len_utf8())?;
    if inner.is_empty() || inner.starts_with('/') {
        return None;
    }
    Some(format!("{opener}/{inner}{closer}"))
}
