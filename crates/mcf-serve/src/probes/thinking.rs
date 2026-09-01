//! How much of a turn happens before the answer does (B-421, D42, §X, A21, F106).
//!
//! **The question is not whether the model reasons.** *Reasoning* is a
//! judgement about what came out, and grading it needs a rater — which is why
//! reasoning modes was declined as a modality and why the entry said the
//! observable half deserved a row of its own.
//!
//! **What is observable is where the tokens went.** Several families open a
//! marker of their own, say a great deal inside it, close it, and only then
//! answer. Whether that is *thinking* is not MCF's to say. That the turn spent
//! a hundred and eighty tokens before its first word of answer is a fact, it is
//! countable, and it decides what budget a question needs.
//!
//! **Why it earns a place under D42's test.** A wrong answer here corrupts a
//! measurement directly, and did: F106 read ten trials as *produced no object*
//! when every one of them was still going when the budget ran out — MCF
//! interrupting the model rather than the model declining. A default budget of
//! thirty-two tokens put to a model that spends a hundred inside a marker
//! measures the budget, and every figure taken through it is about MCF's
//! impatience rather than the model.
//!
//! **The markers come from the model's own file** (A21, F79). MCF keeps no
//! table of which family uses which spelling — one would be out of date the
//! week it was written. A marker is a candidate here when the model's own
//! vocabulary holds it as a real token *and* holds its closing form, which is
//! a property of the file rather than a name MCF recognises. Which of them the
//! model actually uses, the model says by using it.

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

use super::Trial;

/// The question every trial asks.
///
/// Something with a short answer and a reason to work before giving it, so a
/// model that thinks before answering has cause to. Arithmetic rather than
/// world knowledge, so the answer does not depend on what the model was
/// trained on (§X).
pub const QUESTION: &str = "A shelf holds 3 boxes. Each box holds 7 items. \
                            How many items are on the shelf?";

/// The closing form of a marker, where it has one.
///
/// `<think>` closes as `</think>` and `[THINK]` as `[/THINK]`: the slash goes
/// after the bracket. Anything else is not a pair, and a marker with no closing
/// form is not one a turn can be *inside*.
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

/// Whether a token is spelled the way every byte-fallback token is spelled.
///
/// `<0xNN>`, which is a byte and not a marker. The tokenizer has the same
/// judgement for the same reason; it is repeated here rather than shared
/// because what counts as a marker is this probe's question.
fn spelled_as_a_byte(token: &str) -> bool {
    token.len() == 6
        && token.starts_with("<0x")
        && token.ends_with('>')
        && token
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Every marker this file holds that MCF could not pair.
///
/// **Said rather than passed over.** Some families delimit a region without a
/// closing form of the shape this probe knows — a channel marker opened by
/// `<|channel|>` and ended by a different token entirely, rather than by
/// `</|channel|>`. A turn inside one of those is a turn this probe does not
/// measure, and a report that mentioned only what it *could* pair would let a
/// reader conclude the model spends nothing before its answer when MCF simply
/// did not look (A7).
///
/// Counted rather than interpreted: which of them delimits thinking is a fact
/// about a family, and a table of families is the thing F79 refuses to keep.
#[must_use]
pub fn unpairable(file: &gguf::Model, vocabulary: &Vocabulary) -> Vec<String> {
    candidates(file, vocabulary)
        .into_iter()
        .filter(|marker| {
            closing_form(marker).is_none_or(|closing| !vocabulary.has_token(&closing))
        })
        .collect()
}

/// Every marker-shaped token this file holds, from its template and its
/// vocabulary.
fn candidates(file: &gguf::Model, vocabulary: &Vocabulary) -> Vec<String> {
    let template = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
        .unwrap_or_default();
    let mut seen: Vec<String> = super::markers_in(template)
        .into_iter()
        .filter(|marker| vocabulary.has_token(marker))
        .collect();
    for identifier in 0..vocabulary.len() {
        if let Some(token) = vocabulary.token(identifier)
            && (token.starts_with('<') || token.starts_with('['))
            // A byte-fallback token is spelled like a marker and is not one:
            // `<0x41>` is how a vocabulary writes the letter A when it can
            // spell nothing else. Counting all 256 of them as markers a turn
            // could be inside made the count meaningless — which is worse than
            // no count, because it reads as a finding.
            && !spelled_as_a_byte(token)
            && !seen.iter().any(|held| held == token)
        {
            seen.push(token.to_owned());
        }
    }
    // A closing form is not an opener: counting `</think>` as a marker a turn
    // could be inside would double every family that has one.
    seen.retain(|marker| closing_form(marker).is_some());
    seen
}

/// Every marker this file holds as a real token and can close.
///
/// Read from the vocabulary rather than from a list: a spelling that is not a
/// token is text, and text cannot open anything (D46, F26).
#[must_use]
pub fn pairs(file: &gguf::Model, vocabulary: &Vocabulary) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for marker in candidates(file, vocabulary) {
        let Some(closing) = closing_form(&marker) else {
            continue;
        };
        if vocabulary.has_token(&closing) && !found.iter().any(|(held, _)| *held == marker) {
            found.push((marker, closing));
        }
    }
    found
}

/// What the thinking probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spends {
    /// Every pair the file holds, whether the model used it or not.
    pub available: Vec<String>,
    /// Markers the file holds that MCF could not pair, and therefore could not
    /// measure the inside of. Named so that *nothing was found* is told apart
    /// from *nothing was looked for* (A7).
    pub unpairable: Vec<String>,
    /// The pair the model actually opened, where it opened one.
    pub used: Option<String>,
    /// How many trials opened it.
    pub opened: usize,
    /// How many of those also closed it — a turn still inside its marker when
    /// the budget ran out is the shape F106 caught, and is counted apart.
    pub closed: usize,
    /// How many trials there were.
    pub trials: usize,
    /// The longest run of tokens spent inside the marker, across the trials
    /// that closed it.
    ///
    /// The longest rather than the mean: a budget has to cover the worst turn
    /// seen, and an average budget is one that truncates half the turns (A4).
    pub longest_inside: usize,
    /// The budget each trial was given, which is what any of this is relative
    /// to.
    pub budget: usize,
}

/// The method.
pub const THINKING: Method = Method {
    name: "thinking",
    asks: "one question with a short answer and a reason to work before giving it, and counts \
           the turns that open one of this file's own paired markers, the turns that close it, \
           and the most tokens any of them spent inside",
    decides: "what budget a turn needs before its answer begins — and nothing about whether \
              what happened inside the marker was reasoning, which is a judgement a graded \
              task makes",
};

/// Runs the thinking probe.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason
/// (D42's third state).
#[must_use]
pub fn thinking(
    model: &Path,
    bytes: &[u8],
    trials: usize,
    budget: usize,
    engine: &str,
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
    let Ok(vocabulary) = Vocabulary::read(&file) else {
        return Probed::inconclusive(
            THINKING,
            "the vocabulary could not be read, so what this file holds as a marker is not \
             something MCF can say",
            0,
            0,
            conditions,
        );
    };
    let available = pairs(&file, &vocabulary);
    let could_not_pair = unpairable(&file, &vocabulary);
    if available.is_empty() {
        // **And it says how many it could not pair.** Withholding that here
        // was the defect this whole `unpairable` list exists to prevent, kept
        // in the one place it matters most: a file whose markers are all
        // unpairable reads as a file with no markers, and a reader concludes
        // the model spends nothing before its answer when MCF simply could not
        // look. A channel-format family lands exactly here (A7).
        let because = if could_not_pair.is_empty() {
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
        };
        return Probed::inconclusive(THINKING, &because, 0, 0, conditions);
    }

    let mut used: Option<String> = None;
    let mut opened = 0_usize;
    let mut closed = 0_usize;
    let mut longest_inside = 0_usize;
    let mut spent = 0_usize;

    for _ in 0..trials {
        let (trial, said) = ask(budget);
        if let Trial::Stopped { after } = trial {
            spent = spent.saturating_add(after);
        } else {
            spent = spent.saturating_add(budget);
        }
        // The first pair this turn opened, if any. Whichever the model used is
        // the one this file's turns are inside; MCF does not pick for it.
        let Some((marker, closing)) = available
            .iter()
            .find(|(marker, _)| said.contains(marker.as_str()))
        else {
            continue;
        };
        opened = opened.saturating_add(1);
        if used.is_none() {
            used = Some(marker.clone());
        }
        let Some(after_open) = said.split_once(marker.as_str()).map(|(_, rest)| rest) else {
            continue;
        };
        let Some((inside, _)) = after_open.split_once(closing.as_str()) else {
            // Opened and never closed: the turn was still inside its marker
            // when the budget ran out, which is exactly F106's shape and is a
            // fact about the budget rather than about the model.
            continue;
        };
        closed = closed.saturating_add(1);
        // Words rather than identifiers: what came back is text, and a token
        // count MCF invented would be a figure nobody measured (A7).
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
            budget,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

#[cfg(test)]
mod tests;
