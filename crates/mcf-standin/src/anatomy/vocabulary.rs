use super::grouped;
use crate::gguf::{Model, Value};

const BYTE_LEVEL_SPACE: char = '\u{120}';
const PIECE_SPACE: char = crate::tokenizer::SPACE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Normal,
    Unknown,
    Control,
    UserDefined,
    Unused,
    Byte,
    Other(i64),
}

impl Kind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub what: &'static str,
    pub identifier: i64,
    pub spelled: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub bytes: u64,
    pub markers: Option<Vec<String>>,
    pub mentions: Vec<&'static str>,
}

impl Template {
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

pub const NO_TEMPLATE: &str = "none in the file — a chat turn has no framing the file states";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vocabulary {
    pub tokens: u64,
    pub model: Option<String>,
    pub pre: Option<String>,
    pub merges: Option<u64>,
    pub kinds: Option<Vec<(Kind, u64)>>,
    pub named: Vec<Named>,
    pub adds_beginning: Option<bool>,
    pub longest: Option<(String, u64)>,
    pub word_starts: u64,
    pub digit_tokens: (u64, u64),
    pub template: Option<Template>,
}

impl Vocabulary {
    #[must_use]
    pub fn segmentation(&self) -> String {
        match (self.model.as_deref(), self.pre.as_deref()) {
            (Some(model), Some(pre)) => format!("{model}, pre-tokenised as {pre}"),
            (Some(model), None) => model.to_owned(),
            (None, _) => "the file does not say".to_owned(),
        }
    }

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

    #[must_use]
    pub const fn beginning_said(&self) -> &'static str {
        match self.adds_beginning {
            Some(true) => "yes, the file says so",
            Some(false) => "no, the file says so",
            None => "the file does not say; the convention for this segmentation applies",
        }
    }

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

const NAMED: [(&str, &str); 7] = [
    ("bos_token_id", "beginning of text"),
    ("eos_token_id", "end of text"),
    ("eot_token_id", "end of turn"),
    ("eom_token_id", "end of message"),
    ("padding_token_id", "padding"),
    ("unknown_token_id", "unknown"),
    ("seperator_token_id", "separator"),
];

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
