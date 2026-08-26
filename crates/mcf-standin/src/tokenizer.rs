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
    /// Which tokens the file marks as raw bytes rather than pieces.
    ///
    /// GGUF carries a type per token, and byte tokens are type 6. A vocabulary
    /// that does not carry the array falls back to the spelling — `<0x41>` —
    /// which is what a byte token looks like in every vocabulary that has one.
    byte_tokens: Vec<bool>,
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

        // GGUF's own token types: 6 is BYTE. Where the array is absent the
        // spelling decides, which is the same answer by another route.
        let byte_tokens: Vec<bool> = match file
            .get("tokenizer.ggml.token_type")
            .and_then(Value::as_list)
        {
            Some(types) => tokens
                .iter()
                .enumerate()
                .map(|(identifier, token)| {
                    matches!(
                        types.get(identifier).and_then(Value::as_integer),
                        Some(BYTE_TOKEN)
                    ) || spelled_as_a_byte(token)
                })
                .collect(),
            None => tokens
                .iter()
                .map(|token| spelled_as_a_byte(token))
                .collect(),
        };

        let mut by_token = BTreeMap::new();
        for (identifier, token) in tokens.iter().enumerate() {
            // First wins. A vocabulary that spells one token twice is a file
            // MCF did not write, and the lower identifier is what a
            // segmentation should prefer for reproducibility.
            by_token.entry(token.clone()).or_insert(identifier);
        }

        Ok(Self {
            byte_tokens,
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

        // Every character is a symbol, in a chain that merges rather than a
        // lattice that is searched. See the note on this function for why the
        // difference is the whole of it.
        let mut symbols: Vec<Symbol> = Vec::new();
        for (position, character) in prepared.char_indices() {
            let previous = i64::try_from(symbols.len()).unwrap_or(0) - 1;
            symbols.push(Symbol {
                at: position,
                length: character.len_utf8(),
                previous,
                next: i64::try_from(symbols.len()).unwrap_or(0) + 1,
            });
        }
        if let Some(last) = symbols.last_mut() {
            last.next = -1;
        }

        // Every adjacent pair the vocabulary has a token for, best first.
        let mut queue: Vec<Bigram> = Vec::new();
        for index in 1..symbols.len() {
            let right = i64::try_from(index).unwrap_or(0);
            self.offer(&prepared, &symbols, right - 1, right, &mut queue);
        }

        while let Some(best) = take_best(&mut queue) {
            let (Some(left), Some(right)) = (
                symbols
                    .get(usize::try_from(best.left).unwrap_or(0))
                    .copied(),
                symbols
                    .get(usize::try_from(best.right).unwrap_or(0))
                    .copied(),
            ) else {
                continue;
            };
            // A symbol that was already merged into another is gone, and a pair
            // whose size no longer matches is a pair that has changed under
            // this entry — both are skipped rather than reconciled.
            if left.length == 0 || right.length == 0 || left.length + right.length != best.size {
                continue;
            }

            let merged_next = right.next;
            if let Some(slot) = symbols.get_mut(usize::try_from(best.left).unwrap_or(0)) {
                slot.length += right.length;
                slot.next = merged_next;
            }
            if let Some(slot) = symbols.get_mut(usize::try_from(best.right).unwrap_or(0)) {
                slot.length = 0;
            }
            if merged_next >= 0
                && let Some(slot) = symbols.get_mut(usize::try_from(merged_next).unwrap_or(0))
            {
                slot.previous = best.left;
            }

            let previous = symbols
                .get(usize::try_from(best.left).unwrap_or(0))
                .map_or(-1, |symbol| symbol.previous);
            self.offer(&prepared, &symbols, previous, best.left, &mut queue);
            self.offer(&prepared, &symbols, best.left, merged_next, &mut queue);
        }

        let mut identifiers = Vec::new();
        let mut walk = 0_i64;
        while walk >= 0 {
            let Some(symbol) = symbols.get(usize::try_from(walk).unwrap_or(0)).copied() else {
                break;
            };
            let piece = prepared
                .get(symbol.at..symbol.at.saturating_add(symbol.length))
                .unwrap_or("");
            match self.by_token.get(piece).copied() {
                Some(identifier) => identifiers.push(identifier),
                // A symbol that never became a token is emitted as the bytes it
                // is made of — every byte, in order. This is the only place
                // byte tokens are used at all: they are what a vocabulary says
                // when it has nothing to say, not a candidate competing on
                // score (F19).
                None => {
                    for byte in piece.as_bytes() {
                        let spelled = format!("<0x{byte:02X}>");
                        match self.by_token.get(&spelled).copied() {
                            Some(identifier) => identifiers.push(identifier),
                            None => {
                                return Err(malformed(
                                    "the vocabulary cannot represent this text, and it has no \
                                     byte fallback",
                                    piece,
                                ));
                            }
                        }
                    }
                }
            }
            walk = symbol.next;
        }

        if with_beginning && let Some(beginning) = self.beginning {
            identifiers.insert(0, beginning);
        }
        Ok(identifiers)
    }

    /// Offers the pair of symbols at `left` and `right` to the queue, if the
    /// vocabulary has a token for what they spell together.
    fn offer(
        &self,
        text: &str,
        symbols: &[Symbol],
        left: i64,
        right: i64,
        queue: &mut Vec<Bigram>,
    ) {
        if left < 0 || right < 0 {
            return;
        }
        let (Some(first), Some(second)) = (
            symbols.get(usize::try_from(left).unwrap_or(0)),
            symbols.get(usize::try_from(right).unwrap_or(0)),
        ) else {
            return;
        };
        let size = first.length.saturating_add(second.length);
        let Some(piece) = text.get(first.at..first.at.saturating_add(size)) else {
            return;
        };
        let Some(identifier) = self.by_token.get(piece).copied() else {
            return;
        };
        queue.push(Bigram {
            left,
            right,
            score: self.scores.get(identifier).copied().unwrap_or(0.0),
            size,
        });
    }

    /// Whether an identifier names a raw byte rather than a piece.
    #[must_use]
    fn is_byte(&self, identifier: usize) -> bool {
        self.byte_tokens.get(identifier).copied().unwrap_or(false)
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
        // Consecutive byte tokens are one character between them, more often
        // than not: a character outside the vocabulary is two to four bytes in
        // UTF-8, and decoding them one at a time turns every one into a
        // replacement mark (F19). They are gathered and decoded together.
        let mut bytes: Vec<u8> = Vec::new();
        let flush = |bytes: &mut Vec<u8>, out: &mut String| {
            if bytes.is_empty() {
                return;
            }
            match core::str::from_utf8(bytes) {
                Ok(text) => out.push_str(text),
                // Bytes that are not a character are what they are: rendering
                // them as replacement marks says how many there were, which is
                // more than dropping them says (A1).
                Err(_) => {
                    for _ in bytes.iter() {
                        out.push('\u{FFFD}');
                    }
                }
            }
            bytes.clear();
        };

        for identifier in identifiers {
            match self.token(*identifier) {
                Some(token) if self.is_byte(*identifier) => {
                    if let Some(byte) = byte_of(token) {
                        bytes.push(byte);
                        continue;
                    }
                    flush(&mut bytes, &mut out);
                    out.push_str(&undo(token));
                }
                Some(token) => {
                    flush(&mut bytes, &mut out);
                    out.push_str(&undo(token));
                }
                // The write cannot fail: the target is a `String`.
                None => {
                    flush(&mut bytes, &mut out);
                    let _written = write!(out, "<id {identifier}>");
                }
            }
        }
        flush(&mut bytes, &mut out);
        out
    }
}

/// The byte a `<0x41>`-shaped token stands for.
fn byte_of(token: &str) -> Option<u8> {
    let hex = token.strip_prefix("<0x")?.strip_suffix('>')?;
    u8::from_str_radix(hex, 16).ok()
}

/// The type GGUF gives a token that stands for one raw byte.
const BYTE_TOKEN: i64 = 6;

/// Whether a token is spelled the way every byte token is spelled.
fn spelled_as_a_byte(token: &str) -> bool {
    token.len() == 6
        && token.starts_with("<0x")
        && token.ends_with('>')
        && token
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// One run of characters in the text being tokenized.
///
/// A doubly-linked chain by index rather than by pointer: merging is *removing*
/// a link, and an index survives the vector growing under it.
#[derive(Debug, Clone, Copy)]
struct Symbol {
    /// Where it starts in the prepared text.
    at: usize,
    /// How many bytes it covers. Zero means it was merged into its neighbour
    /// and is no longer in the chain.
    length: usize,
    previous: i64,
    next: i64,
}

/// A pair of adjacent symbols the vocabulary has a token for.
#[derive(Debug, Clone, Copy)]
struct Bigram {
    left: i64,
    right: i64,
    score: f32,
    /// What the pair spelled when it was offered. A pair whose symbols have
    /// changed size since is stale, and this is how that is noticed.
    size: usize,
}

/// The best pending merge: highest score, and the leftmost of equals.
///
/// A linear scan rather than a heap. The queue is short, the engine is written
/// to be read (D38), and the ordering is the part that has to be right: two
/// implementations that break ties differently produce different tokens and
/// therefore different answers from the same model.
fn take_best(queue: &mut Vec<Bigram>) -> Option<Bigram> {
    let mut best = 0;
    for (index, bigram) in queue.iter().enumerate() {
        let current = queue.get(best)?;
        // `float_cmp`: the comparison is between two scores read from the same
        // file, and a tolerance would make which merge happens first depend on
        // arithmetic MCF did not do.
        #[allow(clippy::float_cmp)]
        let ties = bigram.score == current.score;
        if bigram.score > current.score || (ties && bigram.left < current.left) {
            best = index;
        }
    }
    if queue.is_empty() {
        None
    } else {
        Some(queue.swap_remove(best))
    }
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
