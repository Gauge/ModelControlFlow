use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::{Piece, Tokens};

use super::Trial;

pub const QUESTION: &str = "A shelf holds 3 boxes. Each box holds 7 items. \
                            How many items are on the shelf?";

#[must_use]
pub fn closing_form(marker: &str) -> Option<String> {
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

fn spelled_as_a_byte(token: &str) -> bool {
    token.len() == 6
        && token.starts_with("<0x")
        && token.ends_with('>')
        && token
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit()))
}

#[must_use]
pub fn unpairable(file: &gguf::Model, tokens: &Tokens) -> Vec<String> {
    candidates(file, tokens)
        .into_iter()
        .filter(|marker| closing_form(marker).is_none_or(|closing| !tokens.has_token(&closing)))
        .collect()
}

fn candidates(file: &gguf::Model, tokens: &Tokens) -> Vec<String> {
    let template = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
        .unwrap_or_default();
    let mut seen: Vec<String> = super::markers_in(template)
        .into_iter()
        .filter(|marker| tokens.has_token(marker))
        .collect();
    for identifier in 0..tokens.len() {
        if let Some(token) = tokens.token(identifier)
            && (token.starts_with('<') || token.starts_with('['))
            && !spelled_as_a_byte(token)
            && !seen.iter().any(|held| held == token)
        {
            seen.push(token.to_owned());
        }
    }
    seen.retain(|marker| closing_form(marker).is_some());
    seen
}

#[must_use]
pub fn pairs(file: &gguf::Model, tokens: &Tokens) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for marker in candidates(file, tokens) {
        let Some(closing) = closing_form(&marker) else {
            continue;
        };
        if tokens.has_token(&closing) && !found.iter().any(|(held, _)| *held == marker) {
            found.push((marker, closing));
        }
    }
    found
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Under {
    pub addressing: super::Addressing,
    pub opened_by_turn: Option<String>,
    pub changed: bool,
}

#[must_use]
pub fn under(addressing: &super::Addressing, pairs: &[(String, String)]) -> Under {
    let last = match addressing.pieces_after.last() {
        Some(Piece::Marker(last)) => last.as_str(),
        _ => {
            return Under {
                addressing: addressing.clone(),
                opened_by_turn: None,
                changed: false,
            };
        }
    };
    if let Some((opener, _)) = pairs.iter().find(|(opener, _)| opener == last) {
        return Under {
            addressing: addressing.clone(),
            opened_by_turn: Some(opener.clone()),
            changed: false,
        };
    }
    let Some((opener, _)) = pairs.iter().find(|(_, closer)| closer == last) else {
        return Under {
            addressing: addressing.clone(),
            opened_by_turn: None,
            changed: false,
        };
    };
    let mut turned = addressing.clone();
    turned.pieces_after.pop();
    turned.pieces_after.push(Piece::Marker(opener.clone()));
    turned.name = format!(
        "{}, {} opened by the turn",
        addressing.name,
        super::trim(opener)
    );
    Under {
        addressing: turned,
        opened_by_turn: Some(opener.clone()),
        changed: true,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spends {
    pub available: Vec<String>,
    pub unpairable: Vec<String>,
    pub used: Option<String>,
    pub opened: usize,
    pub closed: usize,
    pub trials: usize,
    pub longest_inside: usize,
    pub longest_turn: usize,
    pub before_in_longest: Option<usize>,
    pub budget: usize,
}

pub const THINKING: Method = Method {
    name: "thinking",
    asks: "one question with a short answer and a reason to work before giving it, and counts \
           the turns that open one of this file's own paired markers, the turns that close it, \
           and the most tokens any of them spent inside",
    decides: "what budget a turn needs before its answer begins — and nothing about whether \
              what happened inside the marker was reasoning, which is a judgement a graded \
              task makes",
};

fn nothing_to_be_inside(could_not_pair: &[String]) -> String {
    if could_not_pair.is_empty() {
        "this file holds no marker of any kind that a turn could be inside. A turn here \
         begins its answer at its first token, and that is a fact about the file rather \
         than about this model's habits (A7)"
            .to_owned()
    } else {
        format!(
            "this file holds no marker MCF could pair — but {} that it could not, {} among \
             them. A turn inside one of those is a turn this probe does not measure, so \
             what is reported here is MCF's reach and not this model's habits (A7, A21)",
            could_not_pair.len(),
            could_not_pair.first().map_or("", String::as_str)
        )
    }
}

#[must_use]
pub fn thinking(
    model: &Path,
    bytes: &[u8],
    trials: usize,
    budget: usize,
    engine: &str,
    opened_by_turn: Option<&str>,
    ask: &mut dyn FnMut(usize) -> (Trial, String),
) -> Probed<Spends> {
    let conditions = super::conditions(&THINKING, model, engine);
    let Ok(file) = gguf::parse(bytes) else {
        return Probed::inconclusive(
            THINKING,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let Ok(tokens) = Tokens::read(&file) else {
        return Probed::inconclusive(
            THINKING,
            "the file lists no tokens, so what it holds as a marker is not something MCF can \
             say",
            0,
            0,
            conditions,
        );
    };
    let available = pairs(&file, &tokens);
    let could_not_pair = unpairable(&file, &tokens);
    if available.is_empty() {
        let because = nothing_to_be_inside(&could_not_pair);
        return Probed::inconclusive(THINKING, &because, 0, 0, conditions);
    }

    let mut used: Option<String> = None;
    let mut opened = 0_usize;
    let mut closed = 0_usize;
    let mut longest_inside = 0_usize;
    let mut longest_turn = 0_usize;
    let mut before_in_longest: Option<usize> = None;
    let mut spent = 0_usize;

    for _ in 0..trials {
        let (trial, said) = ask(budget);
        if let Trial::Stopped { after, before } = trial {
            spent = spent.saturating_add(after);
            if after > longest_turn {
                longest_turn = after;
                before_in_longest = before;
            }
        } else {
            spent = spent.saturating_add(budget);
        }
        let by_turn = available
            .iter()
            .find(|(marker, _)| opened_by_turn == Some(marker.as_str()));
        let Some((marker, closing)) = by_turn.or_else(|| {
            available
                .iter()
                .find(|(marker, _)| said.contains(marker.as_str()))
        }) else {
            continue;
        };
        opened = opened.saturating_add(1);
        if used.is_none() {
            used = Some(marker.clone());
        }
        let after_open = if by_turn.is_some() {
            said.as_str()
        } else {
            let Some((_, rest)) = said.split_once(marker.as_str()) else {
                continue;
            };
            rest
        };
        let Some((inside, _)) = after_open.split_once(closing.as_str()) else {
            continue;
        };
        closed = closed.saturating_add(1);
        longest_inside = longest_inside.max(inside.split_whitespace().count());
    }

    Probed {
        method: THINKING,
        outcome: Outcome::Observed(Spends {
            available: available.into_iter().map(|(marker, _)| marker).collect(),
            unpairable: could_not_pair,
            used,
            opened,
            closed,
            trials,
            longest_inside,
            longest_turn,
            before_in_longest,
            budget,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

#[cfg(test)]
mod tests;
