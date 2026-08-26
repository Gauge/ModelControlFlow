//! Models built to be run (B-028, D31, B19).
//!
//! **Why the laboratory owns this.** A test of the whole path — a command, a
//! file, an engine, tokens, text — needs a model file that really is one, and
//! the only models MCF can run are the ones its own reader accepts. Fetching a
//! real one would put a network in the gating tier (B19) and a model in the
//! repository (B28); building one here puts neither.
//!
//! **It is degenerate on purpose.** The model below has four tokens, four
//! dimensions and one block, and its embedding table is one-hot — so the logits
//! for a token are that token's own row, and greedy decoding repeats whatever
//! it is given. That makes its answer something a person can state in advance,
//! which is the only way a test of arithmetic this size says anything the
//! arithmetic did not (A19).
//!
//! **It is not a stand-in for a real model and cannot be measured.** B65
//! forbids a speed from MCF's own engine, and this is smaller than the smallest
//! real thing besides: what it establishes is that the pieces fit together, and
//! nothing about what a model is like.

/// What the fixture's vocabulary says.
///
/// Space-prefixed the way a unigram vocabulary writes them, so that decoding
/// produces text with the spaces where a reader expects them.
pub const TOKENS: [&str; 4] = ["<s>", "\u{2581}yes", "\u{2581}no", "\u{2581}maybe"];

/// The width of everything: four tokens, four dimensions, one head, one block.
const WIDTH: usize = 4;

/// A model MCF's own engine will run.
///
/// One block, a one-hot embedding table and identity-shaped weights: asked for
/// `yes`, greedy decoding answers ` yes` for as many tokens as it is given.
#[must_use]
pub fn a_model_that_runs() -> Vec<u8> {
    let metadata = vec![
        text("general.architecture", "llama"),
        text("tokenizer.ggml.model", "llama"),
        integer("llama.block_count", 1),
        integer("llama.embedding_length", WIDTH),
        integer("llama.attention.head_count", 1),
        integer("llama.feed_forward_length", WIDTH),
        integer("llama.context_length", 16),
        integer("tokenizer.ggml.bos_token_id", 0),
        token_list("tokenizer.ggml.tokens", &TOKENS),
        score_list("tokenizer.ggml.scores", &[0.0, -1.0, -1.0, -1.0]),
    ];

    let mut table = vec![0.0_f32; WIDTH * WIDTH];
    for token in 0..WIDTH {
        if let Some(slot) = table.get_mut(token * WIDTH + token) {
            *slot = 1.0;
        }
    }

    let square = vec![0.0_f32; WIDTH * WIDTH];
    let ones = vec![1.0_f32; WIDTH];
    let mut tensors: Vec<(String, Vec<u64>, Vec<f32>)> = vec![
        ("token_embd.weight".to_owned(), vec![4, 4], table),
        ("output_norm.weight".to_owned(), vec![4], ones.clone()),
    ];
    for (name, dimensions, values) in [
        ("attn_norm.weight", vec![4_u64], ones.clone()),
        ("attn_q.weight", vec![4, 4], square.clone()),
        ("attn_k.weight", vec![4, 4], square.clone()),
        ("attn_v.weight", vec![4, 4], square.clone()),
        ("attn_output.weight", vec![4, 4], square.clone()),
        ("ffn_norm.weight", vec![4], ones),
        ("ffn_gate.weight", vec![4, 4], square.clone()),
        ("ffn_up.weight", vec![4, 4], square.clone()),
        ("ffn_down.weight", vec![4, 4], square),
    ] {
        tensors.push((format!("blk.0.{name}"), dimensions, values));
    }

    write(&metadata, &tensors)
}

fn text(key: &str, value: &str) -> (String, u32, Vec<u8>) {
    let mut bytes = length(value.len()).to_vec();
    bytes.extend_from_slice(value.as_bytes());
    (key.to_owned(), 8, bytes)
}

fn integer(key: &str, value: usize) -> (String, u32, Vec<u8>) {
    let value = u32::try_from(value).unwrap_or(0);
    (key.to_owned(), 5, value.to_le_bytes().to_vec())
}

fn token_list(key: &str, values: &[&str]) -> (String, u32, Vec<u8>) {
    let mut bytes = 8_u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&length(values.len()));
    for value in values {
        bytes.extend_from_slice(&length(value.len()));
        bytes.extend_from_slice(value.as_bytes());
    }
    (key.to_owned(), 9, bytes)
}

fn score_list(key: &str, values: &[f32]) -> (String, u32, Vec<u8>) {
    let mut bytes = 6_u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&length(values.len()));
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    (key.to_owned(), 9, bytes)
}

fn length(value: usize) -> [u8; 8] {
    u64::try_from(value).unwrap_or(0).to_le_bytes()
}

/// Writes a GGUF: the header, the metadata, the tensor directory, the padding
/// the format's alignment asks for, and the tensor data.
fn write(metadata: &[(String, u32, Vec<u8>)], tensors: &[(String, Vec<u64>, Vec<f32>)]) -> Vec<u8> {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(tensors.len()));
    out.extend_from_slice(&length(metadata.len()));
    for (key, kind, value) in metadata {
        out.extend_from_slice(&length(key.len()));
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(value);
    }
    let mut offset = 0_u64;
    for (name, dimensions, values) in tensors {
        out.extend_from_slice(&length(name.len()));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&u32::try_from(dimensions.len()).unwrap_or(0).to_le_bytes());
        for dimension in dimensions {
            out.extend_from_slice(&dimension.to_le_bytes());
        }
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset = offset.saturating_add(u64::try_from(values.len().saturating_mul(4)).unwrap_or(0));
    }
    let padding = (32 - (out.len() % 32)) % 32;
    out.extend(std::iter::repeat_n(0_u8, padding));
    for (_, _, values) in tensors {
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests;
