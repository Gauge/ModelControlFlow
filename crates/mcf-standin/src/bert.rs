//! The bert family: a model that reads a whole text at once and answers with a
//! vector (B-371, DEC-055, D38).
//!
//! **This is the other kind of model.** Everything in [`crate::llama`] produces
//! a next token from the tokens so far: attention is causal, positions arrive
//! one at a time, and a cache carries the past. Nothing here does any of that.
//! A bert model reads every position of the text *at once*, each attending to
//! all of them, and what comes out is not a distribution over next tokens — the
//! model has no output head at all — but one vector per position, pooled into
//! one vector for the text.
//!
//! **The structure, read from the reference** (`llama.cpp`'s `models/bert.cpp`,
//! the same method F23 made standing policy): token embedding plus a learned
//! *position* embedding plus a token-type row, layer-normalized — the classic
//! kind with mean and bias, not RMS — then blocks of non-causal attention and
//! an ungated `GELU` feed-forward, each half normalized *after* its residual
//! (post-norm, where the llama line is pre-norm). Biases everywhere. Then mean
//! pooling, because that is what this file declares (`bert.pooling_type`).
//!
//! **What is observable is read from the file; nothing here is a habit table.**
//! Every difference from llama that matters — the biases, the second embedding,
//! the pooling — is a tensor or a key the file carries. The one exception is
//! the activation, which for this family is `GELU` by the reference's own
//! hard-coding, and is stated here for the same reason.

use std::collections::BTreeMap;

use mcf_core::degradation::Degraded;
use mcf_core::engine::{Behaviour, Run, StandIn};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::gguf::Model as File;
use crate::llama::{count, float, malformed, missing, number, read_tensor};
use crate::ops;
use crate::threads::Threads;

const WHERE: Subsystem = Subsystem::new("mcf-standin::bert");

/// The families this module's structure is written for.
pub const FAMILIES: &[&str] = &["bert"];

/// What the file says the model is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// How many transformer blocks.
    pub blocks: usize,
    /// The width of the residual stream, and of the answer.
    pub embedding: usize,
    /// How many attention heads.
    pub heads: usize,
    /// The inner width of the feed-forward.
    pub feed_forward: usize,
    /// The longest text the file claims, which is also the width of the
    /// position embedding: position `n` has no row to read past it.
    pub context: usize,
    /// How many tokens the vocabulary has.
    pub vocabulary: usize,
    /// How many token types the type embedding carries.
    pub token_types: usize,
}

/// How one vector is made from many, as the file declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pooling {
    /// The mean over every position.
    Mean,
    /// The first position's vector alone — the `[CLS]` convention.
    First,
}

/// A model, loaded and ready to embed.
#[derive(Debug)]
pub struct Loaded {
    /// What the file said it is.
    pub shape: Shape,
    /// How the positions pool into one vector.
    pub pooling: Pooling,
    /// The normalization epsilon the file states.
    epsilon: f32,
    /// How many processors each product may divide its rows across.
    ///
    /// One at load, which is the definition; a caller asks for more with
    /// [`Loaded::across`]. It changes what the work costs and not what it says
    /// (B-366, [`crate::threads`]).
    threads: Threads,
    tensors: BTreeMap<String, Vec<f32>>,
}

/// Reads a bert-family model out of a parsed file.
///
/// # Errors
///
/// `artifact.format.unsupported` for another architecture;
/// `artifact.provenance.incomplete` when the file does not state something the
/// architecture needs — including a pooling type this module has no
/// implementation of, because guessing how many vectors become one would be a
/// hidden choice in the middle of every answer (§3.15).
pub fn load(file: &File, bytes: &[u8]) -> Result<Loaded> {
    let architecture = file.architecture().unwrap_or("unstated");
    if !FAMILIES.contains(&architecture) {
        return Err(Failure::new(
            Category::ArtifactFormatUnsupported,
            Attribution::Artifact,
            Disposition::Refused,
            WHERE,
            "this is not an embedding architecture MCF implements",
        )
        .with_context("declared", architecture.to_owned())
        .with_context("implemented", FAMILIES.join(", ")));
    }

    let key = |name: &str| format!("{architecture}.{name}");
    let shape = Shape {
        blocks: count(file, &key("block_count"))?,
        embedding: count(file, &key("embedding_length"))?,
        heads: count(file, &key("attention.head_count"))?,
        feed_forward: count(file, &key("feed_forward_length"))?,
        context: count(file, &key("context_length"))?,
        vocabulary: file
            .get("tokenizer.ggml.tokens")
            .and_then(crate::gguf::Value::as_list)
            .map(<[crate::gguf::Value]>::len)
            .ok_or_else(|| missing("tokenizer.ggml.tokens"))?,
        token_types: count(file, "tokenizer.ggml.token_type_count")?,
    };
    if shape.heads == 0 || !shape.embedding.is_multiple_of(shape.heads) {
        return Err(malformed(
            "the embedding width is not a whole number of heads",
            &format!("{} across {} heads", shape.embedding, shape.heads),
        ));
    }

    let pooling = match number(file, &key("pooling_type")) {
        Some(1) => Pooling::Mean,
        Some(2) => Pooling::First,
        Some(other) => {
            return Err(Failure::new(
                Category::ArtifactFormatUnsupported,
                Attribution::Artifact,
                Disposition::Refused,
                WHERE,
                "the file declares a pooling MCF does not implement",
            )
            .with_context("declared", other.to_string())
            .with_context("implemented", "1 (mean), 2 (first position)"));
        }
        None => return Err(missing(&key("pooling_type"))),
    };

    let epsilon = float(file, &key("attention.layer_norm_epsilon")).unwrap_or(1e-12);

    let mut tensors = BTreeMap::new();
    for (name, elements) in manifest(&shape) {
        tensors.insert(name.clone(), read_tensor(file, bytes, &name, elements)?);
    }

    Ok(Loaded {
        shape,
        pooling,
        epsilon,
        threads: Threads::definition(),
        tensors,
    })
}

/// Every tensor the shape implies, with how many numbers each must hold.
fn manifest(shape: &Shape) -> Vec<(String, usize)> {
    let width = shape.embedding;
    let square = width.saturating_mul(width);
    let inner = shape.feed_forward;

    let mut wanted = vec![
        (
            "token_embd.weight".to_owned(),
            shape.vocabulary.saturating_mul(width),
        ),
        (
            "position_embd.weight".to_owned(),
            shape.context.saturating_mul(width),
        ),
        (
            "token_types.weight".to_owned(),
            shape.token_types.saturating_mul(width),
        ),
        ("token_embd_norm.weight".to_owned(), width),
        ("token_embd_norm.bias".to_owned(), width),
    ];
    for block in 0..shape.blocks {
        for (suffix, elements) in [
            ("attn_q.weight", square),
            ("attn_q.bias", width),
            ("attn_k.weight", square),
            ("attn_k.bias", width),
            ("attn_v.weight", square),
            ("attn_v.bias", width),
            ("attn_output.weight", square),
            ("attn_output.bias", width),
            ("attn_output_norm.weight", width),
            ("attn_output_norm.bias", width),
            ("ffn_up.weight", inner.saturating_mul(width)),
            ("ffn_up.bias", inner),
            ("ffn_down.weight", inner.saturating_mul(width)),
            ("ffn_down.bias", width),
            ("layer_output_norm.weight", width),
            ("layer_output_norm.bias", width),
        ] {
            wanted.push((format!("blk.{block}.{suffix}"), elements));
        }
    }
    wanted
}

/// What an embedding run produced.
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The vector, one number per embedding width, at unit length.
    pub vector: Vec<f32>,
    /// How many tokens the text became, brackets included.
    pub tokens: usize,
    /// How the positions pooled.
    pub pooling: Pooling,
}

/// Embeds one text and marks the result.
///
/// The mark is the same one a generation carries and means the same thing: this
/// came from MCF's own stand-in, answers a behaviour question, and can never
/// report a speed (D31, B65).
///
/// # Errors
///
/// A token outside the vocabulary, a text longer than the position table, or a
/// tensor the model names and does not have.
pub fn embed(
    model: &Loaded,
    build: &str,
    tokens: &[usize],
) -> Result<Degraded<Behaviour<Embedding>>> {
    let run: Run<StandIn> = Run::at_build(build);
    let embedding = forward(model, tokens)?;
    Ok(run.mark(run.behaviour(embedding)))
}

/// The whole-sequence forward pass, and the pooling.
fn forward(model: &Loaded, tokens: &[usize]) -> Result<Embedding> {
    let width = model.shape.embedding;
    if tokens.is_empty() {
        return Err(malformed("there are no tokens to embed", "an empty text"));
    }
    if tokens.len() > model.shape.context {
        return Err(malformed(
            "the text is longer than the model's position table",
            &format!(
                "{} tokens of {} positions",
                tokens.len(),
                model.shape.context
            ),
        ));
    }

    // Token + position + type-zero, then the embedding normalization. The
    // token-type row is "sentence A" for every position, which is what the
    // reference hard-codes for a single text.
    let embeddings = model.tensor("token_embd.weight")?;
    let positions = model.tensor("position_embd.weight")?;
    let types = model.tensor("token_types.weight")?;
    let norm_w = model.tensor("token_embd_norm.weight")?;
    let norm_b = model.tensor("token_embd_norm.bias")?;

    let mut states: Vec<Vec<f32>> = Vec::with_capacity(tokens.len());
    for (position, token) in tokens.iter().enumerate() {
        if *token >= model.shape.vocabulary {
            return Err(malformed(
                "a token identifier is outside the vocabulary",
                &format!("{token} of {}", model.shape.vocabulary),
            ));
        }
        let token_at = token.saturating_mul(width);
        let position_at = position.saturating_mul(width);
        let mut state: Vec<f32> = Vec::with_capacity(width);
        for lane in 0..width {
            state.push(
                embeddings.get(token_at + lane).copied().unwrap_or(0.0)
                    + positions.get(position_at + lane).copied().unwrap_or(0.0)
                    + types.get(lane).copied().unwrap_or(0.0),
            );
        }
        states.push(ops::layer_norm(&state, norm_w, norm_b, model.epsilon));
    }

    for block in 0..model.shape.blocks {
        states = model.block(block, &states)?;
    }

    // Pool, as declared.
    let mut pooled = vec![0.0_f32; width];
    match model.pooling {
        Pooling::Mean => {
            for state in &states {
                for (slot, value) in pooled.iter_mut().zip(state.iter()) {
                    *slot += value;
                }
            }
            let over = f32::from(u16::try_from(states.len()).unwrap_or(1)).max(1.0);
            for slot in &mut pooled {
                *slot /= over;
            }
        }
        Pooling::First => {
            if let Some(first) = states.first() {
                pooled.copy_from_slice(first);
            }
        }
    }

    // Unit length, and stated everywhere the vector is shown: the trained
    // pipelines these models ship in normalize, the reference tool normalizes
    // by default, and two vectors compared by dot product only mean anything
    // at the same length.
    let mut squares = 0.0_f32;
    for value in &pooled {
        squares = value.mul_add(*value, squares);
    }
    let length = squares.sqrt();
    if length > 0.0 {
        for slot in &mut pooled {
            *slot /= length;
        }
    }

    Ok(Embedding {
        vector: pooled,
        tokens: tokens.len(),
        pooling: model.pooling,
    })
}

impl Loaded {
    /// The same model, dividing each product's rows across `threads` (B-366).
    ///
    /// As [`crate::llama::Loaded::across`], and for the same reason: the
    /// partition decides who computes a row and never how, so the embedding is
    /// the same bytes at any count.
    #[must_use]
    pub const fn across(mut self, threads: Threads) -> Self {
        self.threads = threads;
        self
    }

    /// How many processors this model divides its work across, and whose number
    /// that is.
    #[must_use]
    pub const fn threads(&self) -> Threads {
        self.threads
    }

    /// One matrix-vector product, across whatever this model was given.
    fn product(&self, matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
        ops::matmul_vec_across(matrix, vector, rows, columns, self.threads)
    }

    fn tensor(&self, name: &str) -> Result<&[f32]> {
        self.tensors
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| missing(name))
    }

    /// One block: non-causal attention and an ungated feed-forward, each half
    /// normalized after its residual.
    fn block(&self, block: usize, states: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        let width = self.shape.embedding;
        let head = width.checked_div(self.shape.heads).unwrap_or(1);
        let scale = f32::from(u16::try_from(head).unwrap_or(1)).sqrt().recip();
        let positions = states.len();

        let name = |suffix: &str| format!("blk.{block}.{suffix}");
        let q_w = self.tensor(&name("attn_q.weight"))?;
        let q_b = self.tensor(&name("attn_q.bias"))?;
        let k_w = self.tensor(&name("attn_k.weight"))?;
        let k_b = self.tensor(&name("attn_k.bias"))?;
        let v_w = self.tensor(&name("attn_v.weight"))?;
        let v_b = self.tensor(&name("attn_v.bias"))?;

        let mut queries = Vec::with_capacity(positions);
        let mut keys = Vec::with_capacity(positions);
        let mut values = Vec::with_capacity(positions);
        for state in states {
            queries.push(biased(self.product(q_w, state, width, width), q_b));
            keys.push(biased(self.product(k_w, state, width, width), k_b));
            values.push(biased(self.product(v_w, state, width, width), v_b));
        }

        // Every position attends to every position: the attention is not
        // causal, which is the single deepest difference from the llama line —
        // the fifth word shapes the first word's vector.
        let o_w = self.tensor(&name("attn_output.weight"))?;
        let o_b = self.tensor(&name("attn_output.bias"))?;
        let attn_norm_w = self.tensor(&name("attn_output_norm.weight"))?;
        let attn_norm_b = self.tensor(&name("attn_output_norm.bias"))?;

        let mut after_attention = Vec::with_capacity(positions);
        for (position, state) in states.iter().enumerate() {
            let mut attended = vec![0.0_f32; width];
            for head_index in 0..self.shape.heads {
                let at = head_index.saturating_mul(head);
                let query = queries
                    .get(position)
                    .and_then(|projected| projected.get(at..at + head))
                    .unwrap_or(&[]);
                let mut weights = Vec::with_capacity(positions);
                for other in 0..positions {
                    let key = keys
                        .get(other)
                        .and_then(|projected| projected.get(at..at + head))
                        .unwrap_or(&[]);
                    weights.push(ops::dot(query, key) * scale);
                }
                ops::softmax(&mut weights);
                for (other, weight) in weights.iter().enumerate() {
                    let value = values
                        .get(other)
                        .and_then(|projected| projected.get(at..at + head))
                        .unwrap_or(&[]);
                    for (lane, component) in value.iter().enumerate() {
                        if let Some(slot) = attended.get_mut(at + lane) {
                            *slot = weight.mul_add(*component, *slot);
                        }
                    }
                }
            }
            let projected = biased(self.product(o_w, &attended, width, width), o_b);
            let residual = ops::add(&projected, state);
            after_attention.push(ops::layer_norm(
                &residual,
                attn_norm_w,
                attn_norm_b,
                self.epsilon,
            ));
        }

        // The feed-forward: up, GELU, down — no gate — then the residual and
        // the second normalization.
        let up_w = self.tensor(&name("ffn_up.weight"))?;
        let up_b = self.tensor(&name("ffn_up.bias"))?;
        let down_w = self.tensor(&name("ffn_down.weight"))?;
        let down_b = self.tensor(&name("ffn_down.bias"))?;
        let out_norm_w = self.tensor(&name("layer_output_norm.weight"))?;
        let out_norm_b = self.tensor(&name("layer_output_norm.bias"))?;

        let mut out = Vec::with_capacity(positions);
        for state in &after_attention {
            let mut up = biased(
                self.product(up_w, state, self.shape.feed_forward, width),
                up_b,
            );
            for value in &mut up {
                *value = ops::gelu(*value);
            }
            let down = biased(
                self.product(down_w, &up, width, self.shape.feed_forward),
                down_b,
            );
            let residual = ops::add(&down, state);
            out.push(ops::layer_norm(
                &residual,
                out_norm_w,
                out_norm_b,
                self.epsilon,
            ));
        }
        Ok(out)
    }
}

/// A projection plus its bias.
fn biased(mut projected: Vec<f32>, biases: &[f32]) -> Vec<f32> {
    for (slot, bias) in projected.iter_mut().zip(biases.iter()) {
        *slot += bias;
    }
    projected
}

#[cfg(test)]
mod tests;
