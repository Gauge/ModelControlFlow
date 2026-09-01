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
use crate::ops::{Activation, Rotation};

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
    // The same structure again, with a normalization on the way *out* of each
    // half of the block as well as into it — which the file carries as two more
    // tensors per block — and three habits it does not carry: a gated block
    // that activates with `GELU`, an embedding scaled by the square root of its
    // width, and the other rotary pairing.
    "gemma3",
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

/// What a family does that its file does not say it does.
///
/// **Everything here is unobservable, and that is the entry condition.** A
/// tensor MCF can look for is read from the file (§3.18) — that is how the
/// per-head normalizations and the sandwich normalizations are found, and why
/// a file that carries them gets them whoever published it. What lands in this
/// struct is what no tensor and no metadata key reveals: which activation a
/// gated block was trained with, whether the embedding is scaled on the way in,
/// which two components of a head the rotation turns. Being wrong about any of
/// them produces text rather than an error (F20).
#[derive(Debug, Clone, Copy)]
pub struct Habits {
    /// Which two components of a head the rotary embedding turns together.
    pub rotation: Rotation,
    /// Which activation the gated feed-forward block applies.
    pub activation: Activation,
    /// Whether the embedding is multiplied by the square root of its width on
    /// the way into the first block.
    ///
    /// The gemma family was trained that way and the llama family was not. It
    /// is a single scalar multiply and it changes every number after it.
    pub scales_the_embedding: bool,
}

/// What a family does, by name.
///
/// The default is the llama family's, and every departure is named — because a
/// family MCF has not heard of is better run as the architecture the format was
/// designed around than refused for a habit it may not even have.
#[must_use]
pub fn habits(family: &str) -> Habits {
    Habits {
        rotation: rotation(family),
        activation: match family {
            "gemma" | "gemma2" | "gemma3" | "gemma3n" => Activation::Gelu,
            _ => Activation::Silu,
        },
        scales_the_embedding: matches!(family, "gemma" | "gemma2" | "gemma3" | "gemma3n"),
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
/// **The names are transcribed, not inferred.** MCF's first version of this
/// table grouped `qwen2` with `llama-bpe` because their expressions look alike,
/// and claimed `deepseek-llm` for the same group. Neither was true: qwen2 cuts
/// digits one at a time, and deepseek's pre-tokenizer is six expressions over
/// explicit character ranges and is not implemented here at all (F23). What a
/// name maps to is a fact about somebody else's software, and the only way to
/// know it is to read that software.
///
/// Unlike [`rotation`], this table is total in the other direction: everything
/// it does not name is refused rather than defaulted, because here MCF has a
/// choice between saying *no* and being quietly wrong.
#[must_use]
pub fn pre_tokenizer(named: &str) -> Option<Split> {
    match named {
        "gpt-2" | "phi-2" | "jina-es" | "jina-de" | "jina-v2-es" | "jina-v2-de" | "jina-v1-en"
        | "jina-v2-code" | "roberta-bpe" | "gigachat" | "a.x-4.0" | "mellum" | "modern-bert"
        | "exaone4" | "mpt" | "olmo" | "jais" | "trillion" | "granite-docling" => Some(Split::Gpt2),
        "smollm" | "starcoder" | "refact" | "command-r" | "codeshell" | "exaone" | "minerva-7b"
        | "mellum2" => Some(Split::Gpt2DigitsApart),
        "llama3" | "llama-v3" | "llama-bpe" | "falcon3" | "falcon-h1" | "pixtral" | "midm-2.0"
        | "lfm2" | "jina-v5-nano" | "smaug-bpe" => Some(Split::ModernThreeDigits),
        "qwen2" | "qwen35" | "deepseek-r1-qwen" | "kormo" | "f2llmv2" | "megrez" | "stablelm2"
        | "hunyuan" | "solar-open" => Some(Split::ModernOneDigit),
        // Its symbol run takes nothing after it, where `qwen2`'s takes the
        // newlines: a different cut, so a different expression (F23).
        "seed-coder" => Some(Split::ModernOneDigitSymbolsAlone),
        // Letters cut where their case changes. Grouped with these three in
        // the reference implementation, which is where the grouping comes
        // from rather than from a resemblance.
        "gpt-4o" | "llama4" | "kanana2" | "talkie" => Some(Split::CasePartitionedThreeDigits),
        // `default` is deliberately absent. It is not GPT-2's expression — it
        // is four expressions including one that splits on punctuation — and a
        // file that names it, or names none at all, is refused rather than run
        // through something that resembles it (A7). So are the deepseek
        // families, whose pre-tokenizers are their own.
        _ => None,
    }
}

/// The pre-tokenizer names this crate answers to, for a refusal that lists
/// them and for tests that need one without spelling it.
///
/// One name per expression rather than all of them: a refusal that printed
/// thirty-eight names would be a refusal nobody reads to the end.
pub const PRE_TOKENIZERS: &[&str] = &[
    "gpt-2",
    "smollm",
    "llama-bpe",
    "qwen2",
    "seed-coder",
    "gpt-4o",
];

/// A pre-tokenizer name of each shape, for callers that need one and should not
/// be spelling a family into their own source.
#[must_use]
pub fn a_pre_tokenizer(split: Split) -> &'static str {
    match split {
        Split::Gpt2 => "gpt-2",
        Split::Gpt2DigitsApart => "smollm",
        Split::ModernThreeDigits => "llama-bpe",
        Split::ModernOneDigit => "qwen2",
        Split::ModernOneDigitSymbolsAlone => "seed-coder",
        Split::CasePartitionedThreeDigits => "gpt-4o",
    }
}

#[cfg(test)]
mod tests;
