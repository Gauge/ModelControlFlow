use crate::bpe::Split;
use crate::ops::{Activation, Rotation};

pub const FAMILIES: &[&str] = &["llama", "qwen3", "gemma3"];

#[must_use]
pub fn rotation(family: &str) -> Rotation {
    match family {
        "qwen2" | "qwen3" | "qwen3moe" | "gemma" | "gemma2" | "gemma3" | "phi2" | "phi3"
        | "stablelm" | "gptneox" | "olmo" | "starcoder2" | "cohere" => Rotation::Halved,
        _ => Rotation::Interleaved,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Habits {
    pub rotation: Rotation,
    pub activation: Activation,
    pub scales_the_embedding: bool,
}

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
        "seed-coder" => Some(Split::ModernOneDigitSymbolsAlone),
        "gpt-4o" | "llama4" | "kanana2" | "talkie" => Some(Split::CasePartitionedThreeDigits),
        _ => None,
    }
}

pub const PRE_TOKENIZERS: &[&str] = &[
    "gpt-2",
    "smollm",
    "llama-bpe",
    "qwen2",
    "seed-coder",
    "gpt-4o",
];

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
