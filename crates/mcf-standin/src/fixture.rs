pub const TOKENS: [&str; 20] = [
    "<s>",
    "\u{2581}",
    "y",
    "e",
    "s",
    "n",
    "o",
    "m",
    "a",
    "b",
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

pub const WORDS: [&str; 3] = ["\u{2581}yes", "\u{2581}no", "\u{2581}maybe"];

const WIDTH: usize = TOKENS.len();

#[must_use]
pub fn a_model_that_runs() -> Vec<u8> {
    runnable(None)
}

#[must_use]
pub fn a_model_that_ends_its_turn() -> Vec<u8> {
    runnable(Some(0))
}

/// A model whose header says enough for a placement to be planned: the
/// key/value head count the others leave out, which is what sizes a cache.
#[must_use]
pub fn a_model_that_can_be_hosted() -> Vec<u8> {
    built(Some(0), true)
}

fn runnable(ending: Option<usize>) -> Vec<u8> {
    built(ending, false)
}

fn built(ending: Option<usize>, key_value_heads: bool) -> Vec<u8> {
    let mut metadata = vec![
        text("general.architecture", "llama"),
        text("tokenizer.ggml.model", "llama"),
        integer("llama.block_count", 1),
        integer("llama.embedding_length", WIDTH),
        integer("llama.attention.head_count", 1),
        integer("llama.feed_forward_length", WIDTH),
        integer("llama.context_length", 16),
        integer("tokenizer.ggml.bos_token_id", 0),
        token_list("tokenizer.ggml.tokens", &TOKENS),
        score_list("tokenizer.ggml.scores", &scores()),
    ];
    if key_value_heads {
        metadata.push(integer("llama.attention.head_count_kv", 1));
    }
    if let Some(ending) = ending {
        metadata.push(integer("tokenizer.ggml.eos_token_id", ending));
    }

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

#[must_use]
pub fn a_model_with_dense_weights(seed: u64) -> Vec<u8> {
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

struct Noise(u64);

impl Noise {
    const fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let bits = u32::try_from(self.0 >> 40).unwrap_or(0);
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

fn scores() -> Vec<f32> {
    TOKENS
        .iter()
        .map(|token| match token.chars().count() {
            0 | 1 => -9.0,
            length => -9.0 + f32::from(u8::try_from(length).unwrap_or(0)),
        })
        .collect()
}

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
