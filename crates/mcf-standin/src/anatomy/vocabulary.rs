//! The vocabulary, counted from the header's own token list.
//!
//! A vocabulary decides what a model can say cheaply and what it must spell
//! out: how many tokens it has, how many of them start a word, how long a run
//! of digits it can write as one piece, which tokens are the model's own
//! markers rather than text. Everything here is read from `tokenizer.ggml.*`
//! and counted; nothing is tokenised, and nothing is said about the model's
//! fluency in anything (§3.15, DEC-002).
//!
//! **The named tokens are looked up, not trusted.** A header names its
//! end-of-text token by number, and the number is only meaningful if the list
//! has that many entries. Where it does not, the row says so, because an
//! engine that reads that number will index past the list (A2).
//!
//! **What is deliberately not counted.** A byte-level vocabulary stores its
//! tokens in a printable alphabet where a byte above 127 is spelled as some
//! other character, so "how many tokens are non-ASCII" cannot be read off the
//! spellings without the alphabet, and a count that would be wrong for the
//! commonest vocabulary kind is not offered for any (A7).

use super::grouped;
use crate::gguf::{Model, Value};

/// The mark a byte-level vocabulary spells a leading space with.
const BYTE_LEVEL_SPACE: char = '\u{120}';
/// The mark `SentencePiece` spells a leading space with.
const PIECE_SPACE: char = crate::tokenizer::SPACE;

/// What the file marks a token as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Ordinary text.
    Normal,
    /// The stand-in for text nothing covers.
    Unknown,
    /// A marker the model owns — a turn boundary, an end of text.
    Control,
    /// Text the file wants matched literally before segmentation.
    UserDefined,
    /// Reserved and unused.
    Unused,
    /// One raw byte.
    Byte,
    /// A number this reader does not know.
    Other(i64),
}

impl Kind {
    /// GGUF's numbering.
    #[must_use]
    pub const fn of(number: i64) -> Self {
        match number {
            1 => Self::Normal,
            2 => Self::Unknown,
            3 => Self::Control,
            4 => Self::UserDefined,
            5 => Self::Unused,
            6 => Self::Byte,
            other => Self::Other(other),
        }
    }

    /// What to call it.
    #[must_use]
    pub fn as_str(self) -> String {
        match self {
            Self::Normal => "text".to_owned(),
            Self::Unknown => "unknown".to_owned(),
            Self::Control => "control".to_owned(),
            Self::UserDefined => "user-defined".to_owned(),
            Self::Unused => "unused".to_owned(),
            Self::Byte => "byte".to_owned(),
            Self::Other(number) => format!("type {number}"),
        }
    }
}

/// A token the header names by number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// What the header calls it.
    pub what: &'static str,
    /// The number the header gives.
    pub identifier: i64,
    /// How the list spells that number, where the list reaches it.
    pub spelled: Option<String>,
}

/// The chat template the file carries, read for what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// Its length.
    pub bytes: u64,
    /// The control tokens whose spelling occurs in it — the markers a turn
    /// is framed with. `None` where the file does not type its tokens.
    pub markers: Option<Vec<String>>,
    /// The template's variables and tags this reader looks for, found.
    pub mentions: Vec<&'static str>,
}

impl Template {
    /// Why no marker is shown, where none is — the file does not type its
    /// tokens, or none of its control tokens is spelled in the template.
    /// `None` where [`Self::markers`] has some to show.
    ///
    /// Here rather than on a surface so that the window and the command line
    /// say it one way (B-072).
    #[must_use]
    pub fn no_markers(&self) -> Option<&'static str> {
        match &self.markers {
            None => Some(
                "not read: the file does not type its tokens, so a marker cannot be told from \
                 text",
            ),
            Some(markers) if markers.is_empty() => {
                Some("none of the control tokens appear in it by spelling")
            }
            Some(_) => None,
        }
    }
}

/// What is said of a file that carries no chat template.
pub const NO_TEMPLATE: &str = "none in the file — a chat turn has no framing the file states";

/// The vocabulary, counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    /// How many tokens the list holds.
    pub tokens: u64,
    /// `tokenizer.ggml.model`.
    pub model: Option<String>,
    /// `tokenizer.ggml.pre`, the pre-tokenisation the file asks for.
    pub pre: Option<String>,
    /// How many merges a byte-pair vocabulary lists.
    pub merges: Option<u64>,
    /// Each kind's count, where the file types its tokens.
    pub kinds: Option<Vec<(Kind, u64)>>,
    /// The tokens the header names by number.
    pub named: Vec<Named>,
    /// Whether the file says a beginning token belongs in front.
    pub adds_beginning: Option<bool>,
    /// The longest token, and its length in bytes.
    pub longest: Option<(String, u64)>,
    /// How many tokens carry the word-start mark.
    pub word_starts: u64,
    /// How many tokens are a run of digits, and the longest such run.
    pub digit_tokens: (u64, u64),
    /// The chat template, where the file carries one.
    pub template: Option<Template>,
}

impl Vocabulary {
    /// The segmentation the file names, in the words every surface uses.
    #[must_use]
    pub fn segmentation(&self) -> String {
        match (self.model.as_deref(), self.pre.as_deref()) {
            (Some(model), Some(pre)) => format!("{model}, pre-tokenised as {pre}"),
            (Some(model), None) => model.to_owned(),
            (None, _) => "the file does not say".to_owned(),
        }
    }

    /// How a number is written, read off the digit runs the list holds.
    #[must_use]
    pub fn digits_said(&self) -> String {
        match self.digit_tokens {
            (0, _) => "none: every digit is spelled some other way".to_owned(),
            (count, 1) => format!(
                "{} tokens of one digit each — a number is written one digit at a time",
                grouped(count)
            ),
            (count, longest) => format!(
                "{} tokens, the longest {longest} digits — a number is written in pieces of up \
                 to that many",
                grouped(count)
            ),
        }
    }

    /// Whether a beginning token is put in front, as the file says or does
    /// not.
    #[must_use]
    pub const fn beginning_said(&self) -> &'static str {
        match self.adds_beginning {
            Some(true) => "yes, the file says so",
            Some(false) => "no, the file says so",
            None => "the file does not say; the convention for this segmentation applies",
        }
    }

    /// What is wrong where a named token is beyond the list: the header names
    /// a number the list does not reach, and an engine reading it indexes
    /// past the list (A2). `None` where the list spells it.
    #[must_use]
    pub fn beyond(&self, named: &Named) -> Option<String> {
        named.spelled.is_none().then(|| {
            format!(
                "BEYOND THE LIST — the header names token {} and the list holds {}; an engine \
                 reading that number indexes past the list",
                named.identifier, self.tokens
            )
        })
    }
}

/// The header's named tokens, and what to call them.
const NAMED: [(&str, &str); 7] = [
    ("bos_token_id", "beginning of text"),
    ("eos_token_id", "end of text"),
    ("eot_token_id", "end of turn"),
    ("eom_token_id", "end of message"),
    ("padding_token_id", "padding"),
    ("unknown_token_id", "unknown"),
    ("seperator_token_id", "separator"),
];

/// Template variables and tags worth knowing a template uses.
///
/// `bos_token` and `eos_token` are there because a template that frames a
/// turn with those variables rather than a spelled marker shows no control
/// token in its text, and *none appear* would then be read as *none used*.
const MENTIONS: [&str; 8] = [
    "tools",
    "system",
    "enable_thinking",
    "reasoning",
    "<think>",
    "add_generation_prompt",
    "bos_token",
    "eos_token",
];

/// Counts the vocabulary a file carries.
#[must_use]
pub fn of(model: &Model) -> Vocabulary {
    let tokens: Vec<&str> = model
        .get("tokenizer.ggml.tokens")
        .and_then(Value::as_list)
        .map(|held| held.iter().filter_map(Value::as_text).collect())
        .unwrap_or_default();
    let text = |key: &str| {
        model
            .get(&format!("tokenizer.ggml.{key}"))
            .and_then(Value::as_text)
            .map(str::to_owned)
    };
    let kinds = model
        .get("tokenizer.ggml.token_type")
        .and_then(Value::as_list)
        .map(kinds_of);
    let space = if text("model").as_deref() == Some("gpt2") {
        BYTE_LEVEL_SPACE
    } else {
        PIECE_SPACE
    };
    let mut longest: Option<(String, u64)> = None;
    let mut word_starts: u64 = 0;
    let mut digit_tokens = (0_u64, 0_u64);
    for token in &tokens {
        let length = u64::try_from(token.len()).unwrap_or(u64::MAX);
        if longest.as_ref().is_none_or(|(_, held)| length > *held) {
            longest = Some(((*token).to_owned(), length));
        }
        let bare = token.strip_prefix(space).map_or(*token, |rest| {
            word_starts = word_starts.saturating_add(1);
            rest
        });
        if !bare.is_empty() && bare.bytes().all(|byte| byte.is_ascii_digit()) {
            let run = u64::try_from(bare.len()).unwrap_or(u64::MAX);
            digit_tokens = (digit_tokens.0.saturating_add(1), digit_tokens.1.max(run));
        }
    }
    Vocabulary {
        tokens: u64::try_from(tokens.len()).unwrap_or(u64::MAX),
        model: text("model"),
        pre: text("pre"),
        merges: model
            .get("tokenizer.ggml.merges")
            .and_then(Value::as_list)
            .map(|held| u64::try_from(held.len()).unwrap_or(u64::MAX)),
        kinds,
        named: named_of(model, &tokens),
        adds_beginning: model
            .get("tokenizer.ggml.add_bos_token")
            .and_then(Value::as_bool),
        longest,
        word_starts,
        digit_tokens,
        template: model
            .get("tokenizer.chat_template")
            .and_then(Value::as_text)
            .map(|template| template_of(template, model, &tokens)),
    }
}

/// Each kind's count, in the order the kinds are numbered.
fn kinds_of(types: &[Value]) -> Vec<(Kind, u64)> {
    let mut counted: Vec<(Kind, u64)> = Vec::new();
    for kind in types.iter().filter_map(Value::as_integer).map(Kind::of) {
        match counted.iter_mut().find(|(held, _)| *held == kind) {
            Some((_, count)) => *count = count.saturating_add(1),
            None => counted.push((kind, 1)),
        }
    }
    counted.sort_by_key(|(kind, _)| *kind);
    counted
}

/// The tokens the header names by number, spelled from the list.
fn named_of(model: &Model, tokens: &[&str]) -> Vec<Named> {
    NAMED
        .iter()
        .filter_map(|(key, what)| {
            let identifier = model
                .get(&format!("tokenizer.ggml.{key}"))
                .and_then(Value::as_integer)?;
            let spelled = usize::try_from(identifier)
                .ok()
                .and_then(|index| tokens.get(index))
                .map(|held| (*held).to_owned());
            Some(Named {
                what,
                identifier,
                spelled,
            })
        })
        .collect()
}

/// What a template names, read from its text.
fn template_of(template: &str, model: &Model, tokens: &[&str]) -> Template {
    let markers = model
        .get("tokenizer.ggml.token_type")
        .and_then(Value::as_list)
        .map(|types| {
            tokens
                .iter()
                .zip(types)
                .filter(|(_, kind)| kind.as_integer().map(Kind::of) == Some(Kind::Control))
                .filter(|(token, _)| template.contains(*token))
                .map(|(token, _)| (*token).to_owned())
                .collect()
        });
    Template {
        bytes: u64::try_from(template.len()).unwrap_or(u64::MAX),
        markers,
        mentions: MENTIONS
            .iter()
            .copied()
            .filter(|name| template.contains(name))
            .collect(),
    }
}
