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
///
/// **Every intermediate piece is here, and that is the point.** A real
/// vocabulary is built by merging: `▁yes` is reached from `▁` and `y` through
/// `▁y` and `▁ye`, and a vocabulary holding only the whole word cannot be
/// tokenized at all by the algorithm the models actually use. This fixture held
/// only whole words, which is why it ran happily while the tokenizer used an
/// algorithm no model is tokenized by ([findings.md](../../../doc/findings.md)
/// F19). A fixture that cannot be wrong the way a real file is wrong is a
/// fixture that hides defects (D26's habit, applied to a vocabulary).
pub const TOKENS: [&str; 20] = [
    // The special token, and the space every piece is prefixed with.
    "<s>",
    "\u{2581}",
    // The letters the three words are built from.
    "y",
    "e",
    "s",
    "n",
    "o",
    "m",
    "a",
    "b",
    // And the ladder up to each whole word, which is how a merge reaches one.
    "\u{2581}y",
    "\u{2581}ye",
    "\u{2581}yes",
    "\u{2581}n",
    "\u{2581}no",
    "\u{2581}m",
    "\u{2581}ma",
    "\u{2581}may",
    "\u{2581}mayb",
    "\u{2581}maybe",
];

/// The three whole words, for a caller that wants to name one.
pub const WORDS: [&str; 3] = ["\u{2581}yes", "\u{2581}no", "\u{2581}maybe"];

/// The width of everything: one dimension per token, one head, one block.
const WIDTH: usize = TOKENS.len();

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
        // Longer pieces score better than the ones they are built from, so a
        // merge that can reach a whole word does — which is the ordering a real
        // vocabulary has and the reason a merge algorithm reaches it.
        score_list("tokenizer.ggml.scores", &scores()),
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
        ("token_embd.weight".to_owned(), vec![wide(), wide()], table),
        ("output_norm.weight".to_owned(), vec![wide()], ones.clone()),
    ];
    for (name, dimensions, values) in [
        ("attn_norm.weight", vec![wide()], ones.clone()),
        ("attn_q.weight", vec![wide(), wide()], square.clone()),
        ("attn_k.weight", vec![wide(), wide()], square.clone()),
        ("attn_v.weight", vec![wide(), wide()], square.clone()),
        ("attn_output.weight", vec![wide(), wide()], square.clone()),
        ("ffn_norm.weight", vec![wide()], ones),
        ("ffn_gate.weight", vec![wide(), wide()], square.clone()),
        ("ffn_up.weight", vec![wide(), wide()], square.clone()),
        ("ffn_down.weight", vec![wide(), wide()], square),
    ] {
        tensors.push((format!("blk.0.{name}"), dimensions, values));
    }

    write(&metadata, &tensors)
}

/// A model whose weights are dense, seeded and nothing like an identity
/// (B-366).
///
/// **Why the degenerate fixture cannot answer B-366's question.**
/// [`a_model_that_runs`] is one-hot and its projections are zeros, so almost
/// every sum in its forward pass is a sum of zeros — and a sum of zeros is the
/// same number in any order. A partition of that model's work agrees with the
/// serial path whether or not the partition is sound, which is a test that
/// cannot fail. This model's every weight is non-zero, of mixed magnitude and
/// mixed sign, which is the condition under which floating-point addition
/// notices the order it was performed in.
///
/// **It is not a model of anything and its output means nothing.** What it is
/// for is the one property that does not need the output to mean anything: the
/// same input produces the same bytes however the work was divided. It has
/// several blocks and a width that no thread count divides evenly, because a
/// partition that is only ever exercised on a multiple of itself is a partition
/// whose remainder nobody has run.
///
/// `seed` fixes every weight, so two calls produce identical bytes and a
/// failure can be reproduced from the number in the test.
///
/// It is about twelve megabytes of weights, built in memory. That is the price
/// of a model wide enough to be partitioned, and it is paid once per call.
#[must_use]
pub fn a_model_with_dense_weights(seed: u64) -> Vec<u8> {
    // **Wide enough that the engine actually partitions it.** MCF hands a
    // product only as many workers as its size earns (F99), so a narrow model
    // would run every product serially and a test comparing thread counts on it
    // would compare the serial path with itself — a test that cannot fail.
    // 323 × 323 is a hundred thousand elements, which earns two workers, and
    // the feed-forward products earn four.
    //
    // Both dimensions are deliberately prime, so no thread count divides either
    // evenly and every partition has a short chunk at the end — which is where
    // an off-by-one in a partition lives.
    const DENSE_WIDTH: usize = 323;
    const BLOCKS: usize = 3;
    const INNER: usize = 769;
    let vocabulary = TOKENS.len();

    let metadata = vec![
        text("general.architecture", "llama"),
        text("tokenizer.ggml.model", "llama"),
        integer("llama.block_count", BLOCKS),
        integer("llama.embedding_length", DENSE_WIDTH),
        integer("llama.attention.head_count", 1),
        integer("llama.feed_forward_length", INNER),
        integer("llama.context_length", 16),
        integer("tokenizer.ggml.bos_token_id", 0),
        token_list("tokenizer.ggml.tokens", &TOKENS),
        score_list("tokenizer.ggml.scores", &scores()),
    ];

    let mut noise = Noise::seeded(seed);
    let wide = u64::try_from(DENSE_WIDTH).unwrap_or(0);
    let inner = u64::try_from(INNER).unwrap_or(0);
    let tall = u64::try_from(vocabulary).unwrap_or(0);
    let mut tensors: Vec<(String, Vec<u64>, Vec<f32>)> = vec![
        (
            "token_embd.weight".to_owned(),
            vec![wide, tall],
            noise.values(vocabulary * DENSE_WIDTH),
        ),
        (
            "output_norm.weight".to_owned(),
            vec![wide],
            noise.values(DENSE_WIDTH),
        ),
    ];
    for block in 0..BLOCKS {
        for (name, dimensions, count) in [
            ("attn_norm.weight", vec![wide], DENSE_WIDTH),
            ("attn_q.weight", vec![wide, wide], DENSE_WIDTH * DENSE_WIDTH),
            ("attn_k.weight", vec![wide, wide], DENSE_WIDTH * DENSE_WIDTH),
            ("attn_v.weight", vec![wide, wide], DENSE_WIDTH * DENSE_WIDTH),
            (
                "attn_output.weight",
                vec![wide, wide],
                DENSE_WIDTH * DENSE_WIDTH,
            ),
            ("ffn_norm.weight", vec![wide], DENSE_WIDTH),
            ("ffn_gate.weight", vec![wide, inner], INNER * DENSE_WIDTH),
            ("ffn_up.weight", vec![wide, inner], INNER * DENSE_WIDTH),
            ("ffn_down.weight", vec![inner, wide], DENSE_WIDTH * INNER),
        ] {
            tensors.push((
                format!("blk.{block}.{name}"),
                dimensions,
                noise.values(count),
            ));
        }
    }

    write(&metadata, &tensors)
}

/// Weights from a seed, so that a run is reproducible from a number.
///
/// A multiplicative congruential generator, which is enough for weights whose
/// only requirement is that they be dense and of mixed magnitude. It is not a
/// source of randomness anything is measured against — where a seed decides a
/// *result*, MCF uses [`mcf_standin::sample::Rng`], and where it decides
/// coverage the laboratory says so.
struct Noise(u64);

impl Noise {
    const fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// The next weight: mixed sign, and magnitudes spanning three orders so
    /// that adding them in a different order gives a different answer.
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let bits = u32::try_from(self.0 >> 40).unwrap_or(0);
        // A value in [-1, 1), scaled by a power of ten chosen from the same
        // stream: a matrix of uniform magnitude is a matrix whose sum order
        // barely matters.
        let half = u16::try_from(bits & 0xFFFF).unwrap_or(0);
        let unit = (f32::from(half) - 32_768.0) / 32_768.0;
        let decade = match (bits >> 16) % 3 {
            0 => 0.01,
            1 => 1.0,
            _ => 100.0,
        };
        unit * decade
    }

    fn values(&mut self, count: usize) -> Vec<f32> {
        (0..count).map(|_| self.next()).collect()
    }
}

/// A score per token: worse for a letter than for a piece, and worse for a
/// piece than for the word it builds towards.
fn scores() -> Vec<f32> {
    TOKENS
        .iter()
        .map(|token| match token.chars().count() {
            0 | 1 => -9.0,
            // Longer is better, by one per character. `f32::from` on a byte
            // rather than a cast: the workspace keeps floating point out of
            // shipped code except where a format demands it, and a fixture that
            // writes a model file is one of those places.
            length => -9.0 + f32::from(u8::try_from(length).unwrap_or(0)),
        })
        .collect()
}

/// The fixture's width, as a dimension is written.
fn wide() -> u64 {
    u64::try_from(WIDTH).unwrap_or(0)
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
