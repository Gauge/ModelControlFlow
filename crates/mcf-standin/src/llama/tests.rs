#![allow(clippy::float_cmp)]

use super::{ARCHITECTURE, Cache, load};
use crate::gguf;

struct Builder {
    metadata: Vec<(String, u32, Vec<u8>)>,
    tensors: Vec<(String, Vec<u64>, Vec<f32>)>,
}

impl Builder {
    fn new(
        blocks: usize,
        embedding: usize,
        heads: usize,
        feed_forward: usize,
        vocabulary: usize,
    ) -> Self {
        let mut builder = Self {
            metadata: Vec::new(),
            tensors: Vec::new(),
        };
        builder = builder
            .text("general.architecture", ARCHITECTURE)
            .integer("llama.block_count", blocks)
            .integer("llama.embedding_length", embedding)
            .integer("llama.attention.head_count", heads)
            .integer("llama.feed_forward_length", feed_forward)
            .integer("llama.context_length", 32)
            .vocabulary(vocabulary);
        builder
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        let mut bytes = u64::try_from(value.len())
            .unwrap_or(0)
            .to_le_bytes()
            .to_vec();
        bytes.extend_from_slice(value.as_bytes());
        self.metadata.push((key.to_owned(), 8, bytes));
        self
    }

    fn integer(mut self, key: &str, value: usize) -> Self {
        let value = u32::try_from(value).unwrap_or(0);
        self.metadata
            .push((key.to_owned(), 5, value.to_le_bytes().to_vec()));
        self
    }

    fn float(mut self, key: &str, value: f32) -> Self {
        self.metadata
            .push((key.to_owned(), 6, value.to_bits().to_le_bytes().to_vec()));
        self
    }

    fn vocabulary(mut self, count: usize) -> Self {
        let mut bytes = 8_u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&u64::try_from(count).unwrap_or(0).to_le_bytes());
        for index in 0..count {
            let token = format!("t{index}");
            bytes.extend_from_slice(&u64::try_from(token.len()).unwrap_or(0).to_le_bytes());
            bytes.extend_from_slice(token.as_bytes());
        }
        self.metadata
            .push(("tokenizer.ggml.tokens".to_owned(), 9, bytes));
        self
    }

    fn tensor(mut self, name: &str, dimensions: &[u64], values: Vec<f32>) -> Self {
        self.tensors
            .push((name.to_owned(), dimensions.to_vec(), values));
        self
    }

    fn build(&self) -> Vec<u8> {
        let mut header = b"GGUF".to_vec();
        header.extend_from_slice(&3_u32.to_le_bytes());
        header.extend_from_slice(&u64::try_from(self.tensors.len()).unwrap_or(0).to_le_bytes());
        header.extend_from_slice(
            &u64::try_from(self.metadata.len())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        for (key, kind, value) in &self.metadata {
            header.extend_from_slice(&u64::try_from(key.len()).unwrap_or(0).to_le_bytes());
            header.extend_from_slice(key.as_bytes());
            header.extend_from_slice(&kind.to_le_bytes());
            header.extend_from_slice(value);
        }
        let mut offset = 0_u64;
        for (name, dimensions, values) in &self.tensors {
            header.extend_from_slice(&u64::try_from(name.len()).unwrap_or(0).to_le_bytes());
            header.extend_from_slice(name.as_bytes());
            header.extend_from_slice(&u32::try_from(dimensions.len()).unwrap_or(0).to_le_bytes());
            for dimension in dimensions {
                header.extend_from_slice(&dimension.to_le_bytes());
            }
            header.extend_from_slice(&0_u32.to_le_bytes());
            header.extend_from_slice(&offset.to_le_bytes());
            offset = offset.saturating_add(u64::try_from(values.len() * 4).unwrap_or(0));
        }
        let padding = (32 - (header.len() % 32)) % 32;
        header.extend(std::iter::repeat_n(0_u8, padding));
        for (_, _, values) in &self.tensors {
            for value in values {
                header.extend_from_slice(&value.to_le_bytes());
            }
        }
        header
    }
}

fn transparent_block(
    builder: Builder,
    block: usize,
    embedding: usize,
    feed_forward: usize,
) -> Builder {
    builder
        .tensor(
            &format!("blk.{block}.attn_norm.weight"),
            &[u64::try_from(embedding).unwrap_or(0)],
            vec![1.0; embedding],
        )
        .tensor(
            &format!("blk.{block}.attn_q.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(embedding).unwrap_or(0),
            ],
            vec![0.0; embedding * embedding],
        )
        .tensor(
            &format!("blk.{block}.attn_k.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(embedding).unwrap_or(0),
            ],
            vec![0.0; embedding * embedding],
        )
        .tensor(
            &format!("blk.{block}.attn_v.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(embedding).unwrap_or(0),
            ],
            vec![0.0; embedding * embedding],
        )
        .tensor(
            &format!("blk.{block}.attn_output.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(embedding).unwrap_or(0),
            ],
            vec![0.0; embedding * embedding],
        )
        .tensor(
            &format!("blk.{block}.ffn_norm.weight"),
            &[u64::try_from(embedding).unwrap_or(0)],
            vec![1.0; embedding],
        )
        .tensor(
            &format!("blk.{block}.ffn_gate.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(feed_forward).unwrap_or(0),
            ],
            vec![0.0; embedding * feed_forward],
        )
        .tensor(
            &format!("blk.{block}.ffn_up.weight"),
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(feed_forward).unwrap_or(0),
            ],
            vec![0.0; embedding * feed_forward],
        )
        .tensor(
            &format!("blk.{block}.ffn_down.weight"),
            &[
                u64::try_from(feed_forward).unwrap_or(0),
                u64::try_from(embedding).unwrap_or(0),
            ],
            vec![0.0; embedding * feed_forward],
        )
}

pub(crate) fn one_hot_model(vocabulary: usize, embedding: usize) -> Vec<u8> {
    let mut table = vec![0.0_f32; vocabulary * embedding];
    for token in 0..vocabulary {
        if let Some(slot) = table.get_mut(token * embedding + (token % embedding)) {
            *slot = 1.0;
        }
    }
    let builder = Builder::new(1, embedding, 1, 4, vocabulary)
        .float("llama.attention.layer_norm_rms_epsilon", 1e-5)
        .tensor(
            "token_embd.weight",
            &[
                u64::try_from(embedding).unwrap_or(0),
                u64::try_from(vocabulary).unwrap_or(0),
            ],
            table,
        )
        .tensor(
            "output_norm.weight",
            &[u64::try_from(embedding).unwrap_or(0)],
            vec![1.0; embedding],
        );
    transparent_block(builder, 0, embedding, 4).build()
}

#[test]
fn a_model_loads_with_the_shape_its_file_states() {
    let bytes = one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("the file is well formed");
    let model = load(&file, &bytes).expect("the model loads");

    assert_eq!(model.shape.blocks, 1);
    assert_eq!(model.shape.embedding, 4);
    assert_eq!(model.shape.heads, 1);
    assert_eq!(
        model.shape.key_value_heads, 1,
        "an unstated kv count is the head count"
    );
    assert_eq!(model.shape.vocabulary, 4);
    assert_eq!(model.shape.head_dimension(), 4);
    assert!(
        model.output_is_tied(),
        "there is no output.weight in this file"
    );
}

#[test]
fn a_first_token_comes_out_and_it_is_the_one_the_wiring_implies() {
    let bytes = one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let model = load(&file, &bytes).expect("loads");

    for token in 0..4 {
        let mut cache = Cache::for_model(&model.shape);
        let logits = model.forward(token, 0, &mut cache).expect("it runs");
        assert_eq!(logits.len(), 4, "one logit per token in the vocabulary");
        assert_eq!(
            crate::ops::argmax(&logits),
            Some(token),
            "token {token} produced logits {logits:?}"
        );
    }
}

#[test]
fn the_same_input_produces_the_same_logits() {
    let bytes = one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let model = load(&file, &bytes).expect("loads");

    let mut first = Cache::for_model(&model.shape);
    let mut second = Cache::for_model(&model.shape);
    let one = model.forward(2, 0, &mut first).expect("runs");
    let other = model.forward(2, 0, &mut second).expect("runs");
    assert_eq!(one, other);
}

#[test]
fn the_cache_remembers_every_token() {
    let bytes = one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let model = load(&file, &bytes).expect("loads");

    let mut cache = Cache::for_model(&model.shape);
    assert_eq!(cache.length(), 0);
    for (position, token) in [1_usize, 2, 3].into_iter().enumerate() {
        let logits = model.forward(token, position, &mut cache).expect("runs");
        assert_eq!(logits.len(), 4);
        assert_eq!(cache.length(), position + 1);
    }
}

#[test]
fn a_token_outside_the_vocabulary_is_refused() {
    let bytes = one_hot_model(4, 4);
    let file = gguf::parse(&bytes).expect("well formed");
    let model = load(&file, &bytes).expect("loads");
    let mut cache = Cache::for_model(&model.shape);
    let failure = model
        .forward(9, 0, &mut cache)
        .expect_err("9 is not a token");
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::ArtifactFormatMalformed
    );
}

#[test]
fn a_model_that_does_not_state_its_shape_is_refused() {
    let mut builder = Builder::new(1, 4, 1, 4, 4);
    builder
        .metadata
        .retain(|(key, _, _)| key != "llama.attention.head_count");
    let bytes = builder.build();
    let file = gguf::parse(&bytes).expect("the file itself is well formed");
    let failure = load(&file, &bytes).expect_err("the model cannot be run");
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::ArtifactProvenanceIncomplete
    );
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("head_count")),
        "the refusal does not say what was missing"
    );
}

#[test]
fn another_architecture_is_refused_by_name() {
    let bytes = Builder::new(1, 4, 1, 4, 4)
        .text("general.architecture", "mamba")
        .build();
    let Ok(file) = gguf::parse(&bytes) else {
        let mut builder = Builder::new(1, 4, 1, 4, 4);
        builder
            .metadata
            .retain(|(key, _, _)| key != "general.architecture");
        let bytes = builder.text("general.architecture", "mamba").build();
        let file = gguf::parse(&bytes).expect("well formed");
        let failure = load(&file, &bytes).expect_err("mamba is not implemented");
        assert_eq!(
            failure.category(),
            mcf_core::failure::Category::ArtifactFormatUnsupported
        );
        return;
    };
    let failure = load(&file, &bytes).expect_err("mamba is not implemented");
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::ArtifactFormatUnsupported
    );
}
