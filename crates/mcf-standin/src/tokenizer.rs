use std::collections::BTreeMap;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::bpe::{self, Ranks, Split};
use crate::gguf::{Model as File, Value};
use crate::wordpiece;

const WHERE: Subsystem = Subsystem::new("mcf-standin::tokenizer");

pub const SPACE: char = '\u{2581}';

#[derive(Debug, Clone)]
pub enum Scheme {
    Unigram,
    Pairs {
        ranks: Ranks,
        split: Split,
    },
    WordPieces {
        unknown: usize,
        lowercase: bool,
        strip_accents: bool,
        separator: Option<usize>,
    },
}

impl Scheme {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unigram => "llama (unigram, with byte fallback)",
            Self::Pairs { .. } => "gpt2 (byte-level byte-pair)",
            Self::WordPieces { .. } => "bert (word pieces, greedy longest match)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Marker(String),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct Tokens {
    spelled: Vec<String>,
    by_token: BTreeMap<String, usize>,
    pub beginning: Option<usize>,
    pub ending: Option<usize>,
}

impl Tokens {
    pub fn read(file: &File) -> Result<Self> {
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
        let mut by_token = BTreeMap::new();
        for (identifier, token) in tokens.iter().enumerate() {
            by_token.entry(token.clone()).or_insert(identifier);
        }
        Ok(Self {
            spelled: tokens,
            by_token,
            beginning: identifier(file, "tokenizer.ggml.bos_token_id"),
            ending: identifier(file, "tokenizer.ggml.eos_token_id"),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.spelled.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spelled.is_empty()
    }

    #[must_use]
    pub fn token(&self, identifier: usize) -> Option<&str> {
        self.spelled.get(identifier).map(String::as_str)
    }

    #[must_use]
    pub fn has_token(&self, spelled: &str) -> bool {
        self.by_token.contains_key(spelled)
    }

    #[must_use]
    pub fn identifier(&self, spelled: &str) -> Option<usize> {
        self.by_token.get(spelled).copied()
    }
}

#[derive(Debug, Clone)]
pub struct Vocabulary {
    scheme: Scheme,
    specials: Vec<(String, usize)>,
    space_prefix: bool,
    add_beginning: Option<bool>,
    tokens: Vec<String>,
    scores: Vec<f32>,
    byte_tokens: Vec<bool>,
    by_token: BTreeMap<String, usize>,
    pub beginning: Option<usize>,
    pub ending: Option<usize>,
}

impl Vocabulary {
    #[allow(
        clippy::too_many_lines,
        reason = "reading a vocabulary is one sequence of readings from one file, and each is \
                  guarded by what its absence means; splitting it would put the guards in one \
                  place and what they guard in another"
    )]
    pub fn read(file: &File) -> Result<Self> {
        let kind = file
            .get("tokenizer.ggml.model")
            .and_then(Value::as_text)
            .unwrap_or("unstated");
        let scheme = match kind {
            "llama" => Scheme::Unigram,
            "gpt2" => Scheme::Pairs {
                ranks: Ranks::read(&strings(file, "tokenizer.ggml.merges")),
                split: pre_tokenizer(file)?,
            },
            "bert" => {
                let lowercase = file
                    .get("tokenizer.ggml.normalizer.lowercase")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                Scheme::WordPieces {
                    unknown: identifier(file, "tokenizer.ggml.unknown_token_id")
                        .ok_or_else(|| missing("tokenizer.ggml.unknown_token_id"))?,
                    lowercase,
                    strip_accents: file
                        .get("tokenizer.ggml.normalizer.strip_accents")
                        .and_then(Value::as_bool)
                        .unwrap_or(lowercase),
                    separator: identifier(file, "tokenizer.ggml.seperator_token_id")
                        .or_else(|| identifier(file, "tokenizer.ggml.eos_token_id")),
                }
            }
            _ => {
                return Err(Failure::new(
                    Category::EngineUnavailable,
                    Attribution::Mcf,
                    Disposition::Refused,
                    WHERE,
                    "the stand-in engine implements two tokenizers, and this file carries a third",
                )
                .with_context("carried", kind.to_owned())
                .with_context(
                    "implemented",
                    "llama (unigram, with byte fallback), gpt2 (byte-level byte-pair)",
                ));
            }
        };
        if let Scheme::Pairs { ranks, .. } = &scheme
            && ranks.is_empty()
        {
            return Err(missing("tokenizer.ggml.merges"));
        }

        let Tokens {
            spelled: tokens,
            by_token,
            beginning,
            ending,
        } = Tokens::read(file)?;

        let scores: Vec<f32> = match file.get("tokenizer.ggml.scores").and_then(Value::as_list) {
            Some(values) => values
                .iter()
                .map(|value| match value {
                    Value::Float(score) => narrow(*score),
                    _ => 0.0,
                })
                .collect(),
            None => vec![0.0; tokens.len()],
        };
        if scores.len() != tokens.len() {
            return Err(malformed(
                "the vocabulary has a different number of scores than tokens",
                &format!("{} tokens, {} scores", tokens.len(), scores.len()),
            ));
        }

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

        let mut specials: Vec<(String, usize)> = match file
            .get("tokenizer.ggml.token_type")
            .and_then(Value::as_list)
        {
            Some(types) => tokens
                .iter()
                .enumerate()
                .filter(|(identifier, _)| {
                    matches!(
                        types.get(*identifier).and_then(Value::as_integer),
                        Some(USER_DEFINED_TOKEN)
                    )
                })
                .map(|(identifier, token)| (token.clone(), identifier))
                .collect(),
            None => Vec::new(),
        };
        specials.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.1.cmp(&right.1))
        });

        Ok(Self {
            scheme,
            specials,
            space_prefix: file
                .get("tokenizer.ggml.add_space_prefix")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            add_beginning: file
                .get("tokenizer.ggml.add_bos_token")
                .and_then(Value::as_bool),
            byte_tokens,
            tokens,
            scores,
            by_token,
            beginning,
            ending,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    #[must_use]
    pub fn token(&self, identifier: usize) -> Option<&str> {
        self.tokens.get(identifier).map(String::as_str)
    }

    #[must_use]
    pub fn scheme(&self) -> &Scheme {
        &self.scheme
    }

    #[must_use]
    pub fn adds_a_space_prefix(&self) -> bool {
        self.space_prefix && matches!(self.scheme, Scheme::Unigram)
    }

    #[must_use]
    pub fn addressed(&self, pieces: &[Piece]) -> Option<Vec<usize>> {
        let mut identifiers = Vec::new();
        if self.add_beginning.unwrap_or(true)
            && let Some(beginning) = self.beginning
        {
            identifiers.push(beginning);
        }
        for piece in pieces {
            match piece {
                Piece::Marker(marker) => identifiers.push(self.by_token.get(marker).copied()?),
                Piece::Text(text) => identifiers.extend(self.encode(text, false).ok()?),
            }
        }
        Some(identifiers)
    }

    #[must_use]
    pub fn has_token(&self, spelled: &str) -> bool {
        self.by_token.contains_key(spelled)
    }

    pub fn encode(&self, text: &str, with_beginning: bool) -> Result<Vec<usize>> {
        let mut identifiers = Vec::new();
        let mut at = 0;
        while at <= text.len() {
            let Some(rest) = text.get(at..) else { break };
            let Some((offset, length, identifier)) = self.next_special(rest) else {
                if !rest.is_empty() || at == 0 {
                    identifiers.extend(self.segment(rest)?);
                }
                break;
            };
            if let Some(before) = rest.get(..offset)
                && !before.is_empty()
            {
                identifiers.extend(self.segment(before)?);
            }
            identifiers.push(identifier);
            at = at.saturating_add(offset).saturating_add(length);
        }

        if with_beginning
            && self.add_beginning.unwrap_or(true)
            && let Some(beginning) = self.beginning
        {
            identifiers.insert(0, beginning);
        }
        if with_beginning
            && let Scheme::WordPieces {
                separator: Some(separator),
                ..
            } = &self.scheme
        {
            identifiers.push(*separator);
        }
        Ok(identifiers)
    }

    fn word_pieces(
        &self,
        text: &str,
        unknown: usize,
        lowercase: bool,
        strip_accents: bool,
    ) -> Vec<usize> {
        let mut identifiers = Vec::new();
        for word in wordpiece::words(text, lowercase, strip_accents) {
            let spelled = format!("{SPACE}{word}");
            let before = identifiers.len();
            let mut at = 0;
            while at < spelled.len() {
                let mut matched = None;
                for end in (at + 1..=spelled.len()).rev() {
                    let Some(piece) = spelled.get(at..end) else {
                        continue;
                    };
                    if let Some(identifier) = self.by_token.get(piece).copied() {
                        matched = Some((identifier, end));
                        break;
                    }
                }
                let Some((identifier, end)) = matched else {
                    identifiers.truncate(before);
                    identifiers.push(unknown);
                    break;
                };
                identifiers.push(identifier);
                at = end;
            }
            if identifiers.len() == before {
                identifiers.push(unknown);
            }
        }
        identifiers
    }

    fn next_special(&self, text: &str) -> Option<(usize, usize, usize)> {
        let mut best: Option<(usize, usize, usize)> = None;
        for (token, identifier) in &self.specials {
            if token.is_empty() {
                continue;
            }
            if let Some(offset) = text.find(token.as_str()) {
                let candidate = (offset, token.len(), *identifier);
                if best.is_none_or(|(previous, _, _)| offset < previous) {
                    best = Some(candidate);
                }
            }
        }
        best
    }

    fn segment(&self, text: &str) -> Result<Vec<usize>> {
        match &self.scheme {
            Scheme::Unigram => self.unigram(text),
            Scheme::Pairs { ranks, split } => self.pairs(text, ranks, *split),
            Scheme::WordPieces {
                unknown,
                lowercase,
                strip_accents,
                ..
            } => Ok(self.word_pieces(text, *unknown, *lowercase, *strip_accents)),
        }
    }

    fn pairs(&self, text: &str, ranks: &Ranks, split: Split) -> Result<Vec<usize>> {
        let mut identifiers = Vec::new();
        for piece in bpe::pieces(text, split) {
            for merged in bpe::merge(&bpe::spell(piece), ranks) {
                match self.by_token.get(&merged).copied() {
                    Some(identifier) => identifiers.push(identifier),
                    None => {
                        for character in merged.chars() {
                            let single = character.to_string();
                            match self.by_token.get(&single).copied() {
                                Some(identifier) => identifiers.push(identifier),
                                None => {
                                    return Err(malformed(
                                        "the vocabulary cannot represent this text: neither the \
                                         merged piece nor its characters are tokens",
                                        &merged,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(identifiers)
    }

    fn unigram(&self, text: &str) -> Result<Vec<usize>> {
        let prepared = prepare(text, self.space_prefix);

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
        Ok(identifiers)
    }

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

    #[must_use]
    pub fn is_byte(&self, identifier: usize) -> bool {
        self.byte_tokens.get(identifier).copied().unwrap_or(false)
    }

    #[must_use]
    pub fn bytes_of(&self, identifier: usize) -> Option<Vec<u8>> {
        let token = self.token(identifier)?;
        Some(match self.scheme {
            Scheme::Pairs { .. } => token
                .chars()
                .flat_map(|character| match bpe::byte_of(character) {
                    Some(byte) => vec![byte],
                    None => character.to_string().into_bytes(),
                })
                .collect(),
            Scheme::Unigram | Scheme::WordPieces { .. } => {
                match byte_of(token).filter(|_byte| self.is_byte(identifier)) {
                    Some(byte) => vec![byte],
                    None => token.replace(SPACE, " ").into_bytes(),
                }
            }
        })
    }

    #[must_use]
    pub fn decode(&self, identifiers: &[usize]) -> String {
        match self.scheme {
            Scheme::Unigram | Scheme::WordPieces { .. } => self.decode_unigram(identifiers),
            Scheme::Pairs { .. } => self.decode_pairs(identifiers),
        }
    }

    fn decode_pairs(&self, identifiers: &[usize]) -> String {
        use std::fmt::Write as _;

        let mut bytes: Vec<u8> = Vec::new();
        let mut unknown: Vec<usize> = Vec::new();
        for identifier in identifiers {
            match self.token(*identifier) {
                Some(token) => {
                    for character in token.chars() {
                        match bpe::byte_of(character) {
                            Some(byte) => bytes.push(byte),
                            None => bytes.extend_from_slice(character.to_string().as_bytes()),
                        }
                    }
                }
                None => unknown.push(*identifier),
            }
        }
        let mut out = String::from_utf8_lossy(&bytes).into_owned();
        for identifier in unknown {
            let _written = write!(out, "<id {identifier}>");
        }
        out
    }

    fn decode_unigram(&self, identifiers: &[usize]) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        let mut bytes: Vec<u8> = Vec::new();
        let flush = |bytes: &mut Vec<u8>, out: &mut String| {
            if bytes.is_empty() {
                return;
            }
            match core::str::from_utf8(bytes) {
                Ok(text) => out.push_str(text),
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

fn byte_of(token: &str) -> Option<u8> {
    let hex = token.strip_prefix("<0x")?.strip_suffix('>')?;
    u8::from_str_radix(hex, 16).ok()
}

const BYTE_TOKEN: i64 = 6;

const USER_DEFINED_TOKEN: i64 = 4;

fn strings(file: &File, key: &str) -> Vec<String> {
    file.get(key)
        .and_then(Value::as_list)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_text().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn pre_tokenizer(file: &File) -> Result<Split> {
    let named = file
        .get("tokenizer.ggml.pre")
        .and_then(Value::as_text)
        .unwrap_or("(the file does not say)");
    crate::architecture::pre_tokenizer(named).ok_or_else(|| {
        Failure::new(
            Category::EngineUnavailable,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "this vocabulary asks for a pre-tokenizer the stand-in engine does not implement",
        )
        .with_context("asked for", named.to_owned())
        .with_context(
            "implemented",
            crate::architecture::PRE_TOKENIZERS.join(", "),
        )
    })
}

fn spelled_as_a_byte(token: &str) -> bool {
    token.len() == 6
        && token.starts_with("<0x")
        && token.ends_with('>')
        && token
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit()))
}

#[derive(Debug, Clone, Copy)]
struct Symbol {
    at: usize,
    length: usize,
    previous: i64,
    next: i64,
}

#[derive(Debug, Clone, Copy)]
struct Bigram {
    left: i64,
    right: i64,
    score: f32,
    size: usize,
}

fn take_best(queue: &mut Vec<Bigram>) -> Option<Bigram> {
    let mut best = 0;
    for (index, bigram) in queue.iter().enumerate() {
        let current = queue.get(best)?;
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

fn prepare(text: &str, space_prefix: bool) -> String {
    let mut out = if space_prefix {
        String::from(SPACE)
    } else {
        String::new()
    };
    for character in text.chars() {
        if character == ' ' {
            out.push(SPACE);
        } else {
            out.push(character);
        }
    }
    out
}

fn undo(token: &str) -> String {
    if let Some(hex) = token
        .strip_prefix("<0x")
        .and_then(|rest| rest.strip_suffix('>'))
        && let Ok(byte) = u8::from_str_radix(hex, 16)
    {
        return String::from_utf8_lossy(&[byte]).into_owned();
    }
    token.replace(SPACE, " ")
}

fn identifier(file: &File, key: &str) -> Option<usize> {
    file.get(key)
        .and_then(Value::as_integer)
        .and_then(|value| usize::try_from(value).ok())
}

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
