//! Turning text into the identifiers a model reads, and back.
//!
//! The vocabulary is in the model file — the tokens, their scores, their types
//! — so this is a reader for that metadata and the segmentation algorithm it
//! implies. Without it a caller has to supply token identifiers, which is a
//! fine way to test a forward pass and no way to run a model.
//!
//! **One algorithm is implemented and the other is named.** GGUF's llama models
//! carry a *unigram* vocabulary with a score per token, segmented by choosing
//! the split whose scores sum highest — which is what this does. Models
//! carrying byte-pair *merges* instead are refused as `engine.unavailable`
//! naming the scheme, which is D31's third state for an artifact: it does not
//! run, and MCF says which component was missing. A tokenizer that guessed at
//! the other algorithm would produce identifiers that are valid, plausible and
//! not what the model was trained on.
//!
//! **Byte fallback is part of the algorithm, not a rescue.** A unigram
//! vocabulary includes single-byte tokens spelled `<0xNN>` precisely so that
//! text outside the vocabulary is representable; using them is how the
//! segmentation stays total. Where a model's vocabulary lacks them, text it
//! cannot represent is a named refusal rather than a dropped character (A1: the
//! bytes are not lost silently).
//!
//! **Spaces are a character.** `SentencePiece` rewrites a space as `▁` and marks
//! the start of the text with one, and a decoder undoes it. That convention is
//! stated here and asserted in the tests because getting it wrong produces a
//! model that reads fluent text as gibberish.

use std::collections::BTreeMap;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::gguf::{Model as File, Value};

const WHERE: Subsystem = Subsystem::new("mcf-standin::tokenizer");

/// The character `SentencePiece` writes a space as.
pub const SPACE: char = '\u{2581}';

/// A vocabulary, as a model file carries it.
#[derive(Debug, Clone)]
pub struct Vocabulary {
    tokens: Vec<String>,
    scores: Vec<f32>,
    by_token: BTreeMap<String, usize>,
    /// The identifier of the beginning-of-text token, where the file names one.
    pub beginning: Option<usize>,
    /// The identifier of the end-of-text token, where the file names one.
    pub ending: Option<usize>,
}

impl Vocabulary {
    /// Reads the vocabulary out of a model file.
    ///
    /// # Errors
    ///
    /// `engine.unavailable` when the file carries a tokenizer this crate does
    /// not implement, naming which; `artifact.provenance.incomplete` when it
    /// carries none at all; `artifact.format.malformed` when the tokens and
    /// their scores disagree about how many there are.
    pub fn read(file: &File) -> Result<Self> {
        let kind = file
            .get("tokenizer.ggml.model")
            .and_then(Value::as_text)
            .unwrap_or("unstated");
        // "llama" is what a unigram vocabulary is called in GGUF, for the model
        // family it came from rather than for the algorithm.
        if kind != "llama" {
            return Err(Failure::new(
                Category::EngineUnavailable,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the stand-in engine implements one tokenizer, and this file carries another",
            )
            .with_context("carried", kind.to_owned())
            .with_context("implemented", "llama (unigram, with byte fallback)"));
        }

        let tokens: Vec<String> = file
            .get("tokenizer.ggml.tokens")
            .and_then(Value::as_list)
            .ok_or_else(|| missing("tokenizer.ggml.tokens"))?
            .iter()
            .filter_map(|value| value.as_text().map(str::to_owned))
            .collect();
        if tokens.is_empty() {
            return Err(missing("tokenizer.ggml.tokens"));
        }

        let scores: Vec<f32> = match file.get("tokenizer.ggml.scores").and_then(Value::as_list) {
            Some(values) => values
                .iter()
                .map(|value| match value {
                    Value::Float(score) => narrow(*score),
                    _ => 0.0,
                })
                .collect(),
            // A vocabulary with no scores is one where every token is equally
            // preferred, which makes the segmentation "fewest tokens" rather
            // than "highest score". That is a real algorithm and a different
            // one, so it is stated here rather than silently substituted.
            None => vec![0.0; tokens.len()],
        };
        if scores.len() != tokens.len() {
            return Err(malformed(
                "the vocabulary has a different number of scores than tokens",
                &format!("{} tokens, {} scores", tokens.len(), scores.len()),
            ));
        }

        let mut by_token = BTreeMap::new();
        for (identifier, token) in tokens.iter().enumerate() {
            // First wins. A vocabulary that spells one token twice is a file
            // MCF did not write, and the lower identifier is what a
            // segmentation should prefer for reproducibility.
            by_token.entry(token.clone()).or_insert(identifier);
        }

        Ok(Self {
            tokens,
            scores,
            by_token,
            beginning: identifier(file, "tokenizer.ggml.bos_token_id"),
            ending: identifier(file, "tokenizer.ggml.eos_token_id"),
        })
    }

    /// How many tokens it has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Whether it has none, which no real vocabulary does.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// The token an identifier names.
    #[must_use]
    pub fn token(&self, identifier: usize) -> Option<&str> {
        self.tokens.get(identifier).map(String::as_str)
    }

    /// Segments text into identifiers, by the unigram algorithm.
    ///
    /// The segmentation maximizes the sum of the chosen tokens' scores over
    /// every way of splitting the text, computed by a forward pass over the
    /// characters — the standard dynamic program, written out. A tie goes to
    /// the split that reaches the position with fewer tokens, and then to the
    /// lower identifier, so the result is a function of the vocabulary alone.
    ///
    /// # Errors
    ///
    /// `artifact.format.malformed` when a byte of the text has no
    /// representation at all: no token spells it and the vocabulary has no
    /// byte fallback. Dropping it would be losing information silently (A1).
    pub fn encode(&self, text: &str, with_beginning: bool) -> Result<Vec<usize>> {
        let prepared = prepare(text);
        let characters: Vec<char> = prepared.chars().collect();
        let count = characters.len();

        // best[i] is the best way to have reached character i.
        let mut best: Vec<Option<Reached>> = vec![None; count + 1];
        if let Some(slot) = best.get_mut(0) {
            *slot = Some(Reached {
                score: 0.0,
                tokens: 0,
                from: 0,
                identifier: usize::MAX,
            });
        }

        for at in 0..count {
            let Some(Some(reached)) = best.get(at).copied() else {
                continue;
            };
            for end in (at + 1)..=count {
                let piece: String = characters.get(at..end).unwrap_or(&[]).iter().collect();
                let Some(identifier) = self.by_token.get(&piece).copied() else {
                    continue;
                };
                let score = reached.score + self.scores.get(identifier).copied().unwrap_or(0.0);
                let candidate = Reached {
                    score,
                    tokens: reached.tokens.saturating_add(1),
                    from: at,
                    identifier,
                };
                if let Some(slot) = best.get_mut(end) {
                    *slot = Some(better(*slot, candidate));
                }
            }

            // Byte fallback for the single character at this position, which is
            // what makes the segmentation total.
            let Some(character) = characters.get(at).copied() else {
                continue;
            };
            let mut buffer = [0_u8; 4];
            let encoded = character.encode_utf8(&mut buffer);
            let end = at + 1;
            for byte in encoded.as_bytes() {
                let spelled = format!("<0x{byte:02X}>");
                let Some(identifier) = self.by_token.get(&spelled).copied() else {
                    continue;
                };
                let score = reached.score + self.scores.get(identifier).copied().unwrap_or(0.0);
                let candidate = Reached {
                    score,
                    tokens: reached.tokens.saturating_add(1),
                    from: at,
                    identifier,
                };
                if let Some(slot) = best.get_mut(end) {
                    *slot = Some(better(*slot, candidate));
                }
            }
        }

        let Some(Some(_)) = best.get(count) else {
            return Err(malformed(
                "the vocabulary cannot represent this text, and it has no byte fallback",
                &prepared,
            ));
        };

        // Walk the chain back, which is where the tokens actually come from.
        let mut identifiers = Vec::new();
        let mut at = count;
        while at > 0 {
            let Some(Some(reached)) = best.get(at).copied() else {
                break;
            };
            identifiers.push(reached.identifier);
            at = reached.from;
        }
        identifiers.reverse();

        if with_beginning && let Some(beginning) = self.beginning {
            identifiers.insert(0, beginning);
        }
        Ok(identifiers)
    }

    /// Turns identifiers back into text.
    ///
    /// Unknown identifiers are rendered as `<id N>` rather than dropped: a
    /// decoder that silently omitted them would make a model that produced
    /// nonsense look like a model that produced nothing (A1).
    #[must_use]
    pub fn decode(&self, identifiers: &[usize]) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        for identifier in identifiers {
            match self.token(*identifier) {
                Some(token) => out.push_str(&undo(token)),
                // The write cannot fail: the target is a `String`.
                None => {
                    let _written = write!(out, "<id {identifier}>");
                }
            }
        }
        out
    }
}

/// The best way found so far of reaching a position.
#[derive(Debug, Clone, Copy)]
struct Reached {
    score: f32,
    tokens: usize,
    from: usize,
    identifier: usize,
}

/// Which of two ways of reaching a position to keep.
///
/// Higher score, then fewer tokens, then the lower identifier. The tie-breaks
/// are what make a segmentation a function of the vocabulary rather than of the
/// order the candidates happened to be considered in (§3.12).
fn better(existing: Option<Reached>, candidate: Reached) -> Reached {
    let Some(existing) = existing else {
        return candidate;
    };
    // The scores are compared exactly and deliberately: two segmentations whose
    // scores differ by one bit are two different segmentations, and a tolerance
    // here would make which one is chosen depend on how the sum was
    // accumulated. `float_cmp` is right about arithmetic in general and wrong
    // about a comparison whose purpose is to be exact.
    #[allow(clippy::float_cmp)]
    let tied = candidate.score == existing.score;
    let keep_candidate = candidate.score > existing.score
        || (tied
            && (candidate.tokens < existing.tokens
                || (candidate.tokens == existing.tokens
                    && candidate.identifier < existing.identifier)));
    if keep_candidate { candidate } else { existing }
}

/// `SentencePiece`'s preparation: a leading space, and every space written as
/// `▁`.
fn prepare(text: &str) -> String {
    let mut out = String::from(SPACE);
    for character in text.chars() {
        if character == ' ' {
            out.push(SPACE);
        } else {
            out.push(character);
        }
    }
    out
}

/// Undoing it, and turning a byte-fallback token back into its byte.
fn undo(token: &str) -> String {
    if let Some(hex) = token
        .strip_prefix("<0x")
        .and_then(|rest| rest.strip_suffix('>'))
        && let Ok(byte) = u8::from_str_radix(hex, 16)
    {
        // A single byte, which may be part of a multi-byte character. Rendering
        // it as a lone `char` is wrong for anything outside ASCII; this is the
        // honest approximation and the tests say so.
        return String::from_utf8_lossy(&[byte]).into_owned();
    }
    token.replace(SPACE, " ")
}

fn identifier(file: &File, key: &str) -> Option<usize> {
    file.get(key)
        .and_then(Value::as_integer)
        .and_then(|value| usize::try_from(value).ok())
}

/// See the note on `llama::narrow`: the arithmetic here is `f32`, and a score
/// carried at higher precision than the numbers it is compared with is
/// precision nobody can use.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

fn missing(what: &str) -> Failure {
    Failure::new(
        Category::ArtifactProvenanceIncomplete,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the model file carries no vocabulary",
    )
    .with_context("wanted", what.to_owned())
}

fn malformed(detail: &str, found: &str) -> Failure {
    Failure::new(
        Category::ArtifactFormatMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("found", found.to_owned())
}

#[cfg(test)]
mod tests;
