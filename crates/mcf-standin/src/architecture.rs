//! Everything MCF knows that a model file states as a *name* (B28, B-018, F20).
//!
//! **Why these tables are in one file, and only in this one.** B28 forbids a
//! code path that behaves differently because an artifact is the reference
//! model, and B29 says why: a path that recognizes one artifact has stopped
//! measuring. Two quite different things wear the same words. Recognizing *an
//! artifact* — a publisher, a repository, a file name, a digest — is the thing
//! B28 forbids and MCF does nowhere. Reading *a field the file itself states*
//! — `general.architecture`, `tokenizer.ggml.pre` — is what the format is for,
//! and every engine that runs GGUF does it.
//!
//! Keeping the second in one file is what makes the first checkable. The
//! neutrality check exempts this module and asserts three things about it that
//! together mean it cannot be hiding the first: no publisher is named, no
//! artifact is named, and every table here is **total** — each one has a
//! default arm, so an unlisted name is a decision MCF already made rather than
//! a model that falls through.
//!
//! **Nothing here decides whether a model runs.** These tables answer questions
//! the format leaves open for models MCF is going to try either way. What a
//! model *has* — a tensor, a metadata key — is always read from the file
//! (§3.18); only what no file states is read from a name.

use crate::bpe::Split;
use crate::ops::Rotation;

/// The families the stand-in engine's structure is written for.
///
/// A family here is a claim that the block structure matches: normalization,
/// attention, feed-forward, in that arrangement. What varies *within* the
/// structure — a stated head width, a per-head normalization — is read from
/// the file rather than from this list.
pub const FAMILIES: &[&str] = &[
    // The architecture the format was designed around, and the one every
    // structural decision here was read from.
    "llama",
    // The same structure with two additions the file itself declares: a head
    // width that is not the embedding divided by the heads, and a
    // normalization of each query and key head before the rotation. Both are
    // read from what the file carries — a file of this family without
    // `attn_q_norm` would simply not be normalized (§3.18).
    "qwen3",
];

/// Which rotary convention a family was trained with.
///
/// **A table, because there is nowhere else to read it from.** GGUF states the
/// rotation's base and the head's width and never states which two components
/// of a head turn together, so every engine that runs these files carries this
/// table under some name. MCF's is written out so that adding a family means
/// deciding this on purpose.
///
/// The default is llama's, and a family that wants the other is named —
/// because being wrong here does not fail. It produces English (F20).
#[must_use]
pub fn rotation(family: &str) -> Rotation {
    match family {
        "qwen2" | "qwen3" | "qwen3moe" | "gemma" | "gemma2" | "gemma3" | "phi2" | "phi3"
        | "stablelm" | "gptneox" | "olmo" | "starcoder2" | "cohere" => Rotation::Halved,
        _ => Rotation::Interleaved,
    }
}

/// Which pre-tokenizer a byte-pair vocabulary is asking for.
///
/// `None` is the honest answer for a name this crate does not implement: the
/// caller turns it into a refusal that says which name it was. Substituting
/// another is not a small inaccuracy — it changes where the text is cut, and
/// therefore which merges can apply, and therefore the identifiers, silently
/// (A7, A19).
///
/// Unlike [`rotation`], this table is total in the other direction: everything
/// it does not name is refused rather than defaulted, because here MCF has a
/// choice between saying *no* and being quietly wrong.
#[must_use]
pub fn pre_tokenizer(named: &str) -> Option<Split> {
    match named {
        // What the format meant before the field existed.
        "gpt-2" | "gpt2" | "default" => Some(Split::Gpt2),
        "llama-bpe" | "qwen2" | "deepseek-llm" | "smaug-bpe" => Some(Split::Modern),
        _ => None,
    }
}

/// The pre-tokenizer names this crate answers to, for a refusal that lists
/// them and for tests that need one without spelling it.
pub const PRE_TOKENIZERS: &[&str] = &[
    "gpt-2",
    "default",
    "llama-bpe",
    "qwen2",
    "deepseek-llm",
    "smaug-bpe",
];

/// A pre-tokenizer name of each shape, for callers that need one and should not
/// be spelling a family into their own source.
#[must_use]
pub fn a_pre_tokenizer(split: Split) -> &'static str {
    match split {
        Split::Gpt2 => "gpt-2",
        Split::Modern => "llama-bpe",
    }
}

#[cfg(test)]
mod tests;
