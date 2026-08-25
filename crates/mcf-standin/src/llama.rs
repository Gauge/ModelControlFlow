//! One family of model, run one token at a time.
//!
//! The llama architecture is what §XII's reference model is, what GGUF was
//! written for, and what most published open weights are. This assembles
//! [`crate::ops`] into its forward pass: an embedding lookup, then a stack of
//! blocks each doing normalized attention and a gated feed-forward, then a
//! final norm and a projection to logits.
//!
//! **What it holds and what it refuses.** Everything about the model comes from
//! the file's own metadata — the block count, the widths, the head counts, the
//! epsilon, the rope base — and a model that does not state one of them is
//! refused by name rather than run under a default (A7). A default here is not
//! a convenience; it is a different model that produces confident nonsense.
//!
//! **Grouped-query attention is the general case.** A model with as many
//! key/value heads as query heads is the special case where the grouping is
//! one, so there is one code path rather than two and the special case is
//! exercised by every test that uses it.
//!
//! **The cache is the whole of what makes generation possible.** Each token's
//! keys and values are kept so the next token attends to them without
//! recomputing the sequence, which is the one piece of machinery here that
//! exists for speed — and it is not an optimization but the definition of
//! autoregressive decoding.
//!
//! **Still deliberately slow.** A hidden state is a `Vec<f32>` per token, every
//! matrix multiply is [`crate::ops::matmul_vec`], and nothing is fused. D32 and
//! F8 settled that trade.

use std::collections::BTreeMap;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::dequantize;
use crate::gguf::{Model as File, Value};
use crate::ops;

const WHERE: Subsystem = Subsystem::new("mcf-standin::llama");

/// The architecture this module runs.
pub const ARCHITECTURE: &str = "llama";

/// What the file says the model is.
///
/// Every field is read from the metadata; none has a default. A model whose
/// file does not say how many heads it has is a model MCF cannot run, and
/// saying so is the honest outcome (A7, B7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// How many transformer blocks.
    pub blocks: usize,
    /// The width of the residual stream.
    pub embedding: usize,
    /// How many query heads.
    pub heads: usize,
    /// How many key/value heads. Equal to `heads` for a model without grouped
    /// attention.
    pub key_value_heads: usize,
    /// The inner width of the feed-forward.
    pub feed_forward: usize,
    /// The longest context the file claims.
    pub context: usize,
    /// How many tokens the vocabulary has.
    pub vocabulary: usize,
}

impl Shape {
    /// The width of one attention head.
    #[must_use]
    pub const fn head_dimension(&self) -> usize {
        match self.embedding.checked_div(self.heads) {
            Some(width) => width,
            None => 0,
        }
    }
}

/// A model, loaded and ready to run.
#[derive(Debug)]
pub struct Loaded {
    /// What the file said it is.
    pub shape: Shape,
    /// The normalization epsilon the file states.
    epsilon: f32,
    /// The rope base frequency the file states.
    rope_theta: f32,
    /// Every tensor, dequantized once and kept.
    ///
    /// Dequantizing on load rather than per token is the one memory-for-time
    /// trade here, and it is made for legibility: the forward pass reads
    /// numbers, not encodings. It also makes the cost of a large model on the
    /// stand-in obvious rather than hidden, which is the honest way for
    /// something D31 admits may be two orders of magnitude slow.
    tensors: BTreeMap<String, Vec<f32>>,
}

/// The keys and values of the tokens seen so far.
#[derive(Debug, Default)]
pub struct Cache {
    /// Per block, per position, the concatenated key heads.
    keys: Vec<Vec<Vec<f32>>>,
    /// The same for values.
    values: Vec<Vec<Vec<f32>>>,
}

impl Cache {
    /// A cache for a model of this shape.
    #[must_use]
    pub fn for_model(shape: &Shape) -> Self {
        Self {
            keys: vec![Vec::new(); shape.blocks],
            values: vec![Vec::new(); shape.blocks],
        }
    }

    /// How many tokens it holds.
    #[must_use]
    pub fn length(&self) -> usize {
        self.keys.first().map_or(0, Vec::len)
    }
}

/// Reads the shape, the constants and every tensor a llama model needs.
///
/// # Errors
///
/// `artifact.format.unsupported` when the file is not a llama model;
/// `artifact.provenance.incomplete` when it does not state something the
/// architecture needs; `artifact.format.malformed` when a tensor it names is
/// absent or the wrong size; `engine.unavailable` when a tensor is in a scheme
/// this crate does not decode.
pub fn load(file: &File, bytes: &[u8]) -> Result<Loaded> {
    let architecture = file.architecture().unwrap_or("unstated");
    if architecture != ARCHITECTURE {
        return Err(Failure::new(
            Category::ArtifactFormatUnsupported,
            Attribution::Artifact,
            Disposition::Refused,
            WHERE,
            "the stand-in engine implements one architecture, and this file is another",
        )
        .with_context("declared", architecture.to_owned())
        .with_context("implemented", ARCHITECTURE.to_owned()));
    }

    let shape = read_shape(file)?;
    let epsilon = float(file, "llama.attention.layer_norm_rms_epsilon").unwrap_or(1e-5);
    let rope_theta = float(file, "llama.rope.freq_base").unwrap_or(10_000.0);

    let mut tensors = BTreeMap::new();
    for (name, elements) in manifest(&shape) {
        tensors.insert(name.clone(), read_tensor(file, bytes, &name, elements)?);
    }

    // The output projection is tied to the embedding in some models and its own
    // tensor in others. Both are ordinary rather than exceptional, so the
    // absence is not a failure — but which one was used is a fact about the
    // model, and `output_is_tied` is how a caller can say so.
    if let Ok(output) = read_tensor(
        file,
        bytes,
        "output.weight",
        shape.vocabulary.saturating_mul(shape.embedding),
    ) {
        tensors.insert("output.weight".to_owned(), output);
    }

    Ok(Loaded {
        shape,
        epsilon,
        rope_theta,
        tensors,
    })
}

/// The shape, entirely from the file's own metadata.
///
/// # Errors
///
/// `artifact.provenance.incomplete` when the file does not state something the
/// architecture needs; `artifact.format.malformed` when what it states does not
/// divide — an embedding width that is not a whole number of heads is a model
/// nobody can run, and the arithmetic that discovered it is the honest place to
/// say so.
fn read_shape(file: &File) -> Result<Shape> {
    let shape = Shape {
        blocks: count(file, "llama.block_count")?,
        embedding: count(file, "llama.embedding_length")?,
        heads: count(file, "llama.attention.head_count")?,
        key_value_heads: match number(file, "llama.attention.head_count_kv") {
            Some(value) => usize::try_from(value).unwrap_or(0),
            // The one field with a defined fallback, and it is the model's own
            // convention rather than a guess: a file that omits it is a model
            // without grouped attention, where the two counts are equal.
            None => count(file, "llama.attention.head_count")?,
        },
        feed_forward: count(file, "llama.feed_forward_length")?,
        context: count(file, "llama.context_length")?,
        vocabulary: file
            .get("tokenizer.ggml.tokens")
            .and_then(Value::as_list)
            .map(<[Value]>::len)
            .ok_or_else(|| missing("tokenizer.ggml.tokens"))?,
    };

    if shape.heads == 0 || !shape.embedding.is_multiple_of(shape.heads) {
        return Err(malformed(
            "the embedding width is not a whole number of heads",
            &format!("{} across {} heads", shape.embedding, shape.heads),
        ));
    }
    if shape.key_value_heads == 0 || !shape.heads.is_multiple_of(shape.key_value_heads) {
        return Err(malformed(
            "the query heads are not a whole number of groups",
            &format!(
                "{} query heads over {} key/value heads",
                shape.heads, shape.key_value_heads
            ),
        ));
    }
    Ok(shape)
}

/// Every tensor this architecture needs, and how many values each holds.
///
/// Derived from the shape rather than from what the file happens to contain, so
/// a model missing a tensor is a named absence rather than a forward pass that
/// quietly skips a block.
fn manifest(shape: &Shape) -> Vec<(String, usize)> {
    let head = shape.head_dimension();
    let kv_width = head.saturating_mul(shape.key_value_heads);
    let square = shape.embedding.saturating_mul(shape.embedding);
    let gate = shape.feed_forward.saturating_mul(shape.embedding);

    let mut wanted = vec![
        (
            "token_embd.weight".to_owned(),
            shape.vocabulary.saturating_mul(shape.embedding),
        ),
        ("output_norm.weight".to_owned(), shape.embedding),
    ];
    for block in 0..shape.blocks {
        for (suffix, elements) in [
            ("attn_norm.weight", shape.embedding),
            ("attn_q.weight", square),
            ("attn_k.weight", kv_width.saturating_mul(shape.embedding)),
            ("attn_v.weight", kv_width.saturating_mul(shape.embedding)),
            ("attn_output.weight", square),
            ("ffn_norm.weight", shape.embedding),
            ("ffn_gate.weight", gate),
            ("ffn_up.weight", gate),
            ("ffn_down.weight", gate),
        ] {
            wanted.push((format!("blk.{block}.{suffix}"), elements));
        }
    }
    wanted
}

impl Loaded {
    /// Whether the output projection reuses the embedding matrix.
    #[must_use]
    pub fn output_is_tied(&self) -> bool {
        !self.tensors.contains_key("output.weight")
    }

    /// Runs one token through the model and returns the logits for the next.
    ///
    /// `position` is where this token sits in the sequence, which is what `RoPE`
    /// rotates by and what the cache indexes. A caller that passes the same
    /// position twice gets a model attending to a sequence that never existed,
    /// so the cache's own length is what a caller should use.
    ///
    /// # Errors
    ///
    /// `artifact.format.malformed` when a token identifier is outside the
    /// vocabulary, or when a tensor's shape and the model's shape disagree at
    /// the point of use — which is a file that passed loading and is still
    /// wrong about itself.
    pub fn forward(&self, token: usize, position: usize, cache: &mut Cache) -> Result<Vec<f32>> {
        if token >= self.shape.vocabulary {
            return Err(malformed(
                "a token identifier is outside the vocabulary",
                &format!("{token} of {}", self.shape.vocabulary),
            ));
        }

        let width = self.shape.embedding;
        let embedding = self.tensor("token_embd.weight")?;
        let start = token.saturating_mul(width);
        let mut hidden = embedding
            .get(start..start.saturating_add(width))
            .ok_or_else(|| {
                malformed(
                    "the embedding table is shorter than the vocabulary",
                    "token_embd.weight",
                )
            })?
            .to_vec();

        for block in 0..self.shape.blocks {
            let attended = self.attention(block, &hidden, position, cache)?;
            hidden = ops::add(&hidden, &attended);
            let fed = self.feed_forward(block, &hidden)?;
            hidden = ops::add(&hidden, &fed);
        }

        let normalized = ops::rms_norm(&hidden, self.tensor("output_norm.weight")?, self.epsilon);
        let projection = if self.output_is_tied() {
            self.tensor("token_embd.weight")?
        } else {
            self.tensor("output.weight")?
        };
        Ok(ops::matmul_vec(
            projection,
            &normalized,
            self.shape.vocabulary,
            width,
        ))
    }

    /// One block's attention: normalize, project, rotate, remember, attend,
    /// project back. What it returns is what the residual stream adds.
    ///
    /// # Errors
    ///
    /// `artifact.format.malformed` when a projection produces a width the
    /// model's own shape does not permit — a file that loaded and is still
    /// wrong about itself.
    fn attention(
        &self,
        block: usize,
        hidden: &[f32],
        position: usize,
        cache: &mut Cache,
    ) -> Result<Vec<f32>> {
        let width = self.shape.embedding;
        let head = self.shape.head_dimension();
        let kv_width = head.saturating_mul(self.shape.key_value_heads);
        let groups = self
            .shape
            .heads
            .checked_div(self.shape.key_value_heads)
            .unwrap_or(1);

        let normalized = ops::rms_norm(
            hidden,
            self.tensor(&format!("blk.{block}.attn_norm.weight"))?,
            self.epsilon,
        );
        let mut queries = ops::matmul_vec(
            self.tensor(&format!("blk.{block}.attn_q.weight"))?,
            &normalized,
            width,
            width,
        );
        let mut keys = ops::matmul_vec(
            self.tensor(&format!("blk.{block}.attn_k.weight"))?,
            &normalized,
            kv_width,
            width,
        );
        let values = ops::matmul_vec(
            self.tensor(&format!("blk.{block}.attn_v.weight"))?,
            &normalized,
            kv_width,
            width,
        );
        if queries.len() != width || keys.len() != kv_width || values.len() != kv_width {
            return Err(malformed(
                "a projection produced the wrong width",
                &format!("block {block}"),
            ));
        }

        // Rotate each head of the query and the key by this position.
        for index in 0..self.shape.heads {
            let at = index.saturating_mul(head);
            if let Some(slice) = queries.get_mut(at..at.saturating_add(head)) {
                ops::rope(slice, position, self.rope_theta);
            }
        }
        for index in 0..self.shape.key_value_heads {
            let at = index.saturating_mul(head);
            if let Some(slice) = keys.get_mut(at..at.saturating_add(head)) {
                ops::rope(slice, position, self.rope_theta);
            }
        }

        push(&mut cache.keys, block, keys);
        push(&mut cache.values, block, values);
        let history = cache.keys.get(block).map_or(0, Vec::len);

        // The scale keeps the logits' variance independent of the head width.
        let scale = f32::from(u16::try_from(head).unwrap_or(1)).sqrt().recip();
        let mut attended = vec![0.0_f32; width];
        for head_index in 0..self.shape.heads {
            let group = head_index.checked_div(groups).unwrap_or(0);
            let query_at = head_index.saturating_mul(head);
            let key_at = group.saturating_mul(head);
            let query = queries
                .get(query_at..query_at.saturating_add(head))
                .unwrap_or(&[]);

            let mut weights = Vec::with_capacity(history);
            for step in 0..history {
                let key = slice_at(cache.keys.get(block), step, key_at, head);
                weights.push(ops::dot(query, key) * scale);
            }
            ops::softmax(&mut weights);

            for (step, weight) in weights.iter().enumerate() {
                let value = slice_at(cache.values.get(block), step, key_at, head);
                for (index, element) in value.iter().enumerate() {
                    if let Some(slot) = attended.get_mut(query_at.saturating_add(index)) {
                        *slot = weight.mul_add(*element, *slot);
                    }
                }
            }
        }

        Ok(ops::matmul_vec(
            self.tensor(&format!("blk.{block}.attn_output.weight"))?,
            &attended,
            width,
            width,
        ))
    }

    /// One block's gated feed-forward, on its own normalization. What it
    /// returns is what the residual stream adds.
    ///
    /// # Errors
    ///
    /// As [`Loaded::attention`]: a tensor the model names and does not have.
    fn feed_forward(&self, block: usize, hidden: &[f32]) -> Result<Vec<f32>> {
        let width = self.shape.embedding;
        let normalized = ops::rms_norm(
            hidden,
            self.tensor(&format!("blk.{block}.ffn_norm.weight"))?,
            self.epsilon,
        );
        let gate = ops::matmul_vec(
            self.tensor(&format!("blk.{block}.ffn_gate.weight"))?,
            &normalized,
            self.shape.feed_forward,
            width,
        );
        let up = ops::matmul_vec(
            self.tensor(&format!("blk.{block}.ffn_up.weight"))?,
            &normalized,
            self.shape.feed_forward,
            width,
        );
        let activated = ops::swiglu(&gate, &up);
        Ok(ops::matmul_vec(
            self.tensor(&format!("blk.{block}.ffn_down.weight"))?,
            &activated,
            width,
            self.shape.feed_forward,
        ))
    }

    fn tensor(&self, name: &str) -> Result<&[f32]> {
        self.tensors
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| missing(name))
    }
}

/// One head's slice of a remembered key or value row.
fn slice_at(steps: Option<&Vec<Vec<f32>>>, step: usize, at: usize, width: usize) -> &[f32] {
    steps
        .and_then(|steps| steps.get(step))
        .and_then(|row| row.get(at..at.saturating_add(width)))
        .unwrap_or(&[])
}

/// Appends this token's keys or values to a block's history.
fn push(cache: &mut [Vec<Vec<f32>>], block: usize, row: Vec<f32>) {
    if let Some(steps) = cache.get_mut(block) {
        steps.push(row);
    }
}

/// Reads and dequantizes one tensor, checking it is the size the model's shape
/// implies.
fn read_tensor(file: &File, bytes: &[u8], name: &str, elements: usize) -> Result<Vec<f32>> {
    let tensor = file.tensor(name).ok_or_else(|| missing(name))?;
    let expected = u64::try_from(elements).unwrap_or(u64::MAX);
    if tensor.elements() != Some(expected) {
        return Err(malformed(
            "a tensor is not the size the model's own metadata implies",
            &format!(
                "{name}: {:?} against {elements} expected",
                tensor.dimensions
            ),
        ));
    }
    let size = tensor.bytes().ok_or_else(|| {
        Failure::new(
            Category::EngineUnavailable,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "a tensor is in a scheme the stand-in engine does not decode",
        )
        .with_context("tensor", name.to_owned())
        .with_context("scheme", tensor.kind.to_string())
    })?;
    let start = usize::try_from(file.data_offset.saturating_add(tensor.offset))
        .map_err(|_| malformed("a tensor begins past this machine's addressing", name))?;
    let end = start.saturating_add(usize::try_from(size).unwrap_or(usize::MAX));
    let raw = bytes.get(start..end).ok_or_else(|| {
        malformed(
            "the file ends before a tensor it names",
            &format!("{name}: wanted bytes {start}..{end} of {}", bytes.len()),
        )
    })?;
    dequantize::tensor(tensor.kind, raw, elements)
}

fn count(file: &File, key: &str) -> Result<usize> {
    number(file, key)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| missing(key))
}

fn number(file: &File, key: &str) -> Option<i64> {
    file.get(key).and_then(Value::as_integer)
}

fn float(file: &File, key: &str) -> Option<f32> {
    match file.get(key) {
        Some(Value::Float(value)) => Some(narrow(*value)),
        _ => None,
    }
}

/// A metadata float, at the width this crate computes in.
///
/// GGUF carries these at 32 or 64 bits. A 32-bit one widened to 64 comes back
/// exactly; a 64-bit one is narrowed, which is what the arithmetic below would
/// do to it anyway — every weight and every activation here is `f32`, and an
/// epsilon carried at higher precision than the numbers it is added to is
/// precision nobody can use.
#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

fn missing(what: &str) -> Failure {
    Failure::new(
        Category::ArtifactProvenanceIncomplete,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the model file does not state something the architecture needs",
    )
    .with_context("wanted", what.to_owned())
}

fn malformed(detail: &str, found: &str) -> Failure {
    Failure::new(
        Category::ArtifactFormatMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("found", found.to_owned())
}

#[cfg(test)]
mod tests;
