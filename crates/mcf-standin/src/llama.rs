use std::collections::BTreeMap;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::dequantize;
use crate::gguf::{Model as File, Value};
use crate::ops;
use crate::threads::Threads;

const WHERE: Subsystem = Subsystem::new("mcf-standin::llama");

pub use crate::architecture::FAMILIES;

pub fn covers(file: &File) -> Result<()> {
    let architecture = file.architecture().unwrap_or("unstated");
    if FAMILIES.contains(&architecture) {
        return Ok(());
    }
    Err(Failure::new(
        Category::ArtifactFormatUnsupported,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "MCF's engine has not been taught this architecture",
    )
    .with_context("declared", architecture.to_owned())
    .with_context("implemented", FAMILIES.join(", "))
    .with_context(
        "what_to_do",
        "B-365 is the register item that grows this list, and it grows by somebody reading \
         the architecture rather than by MCF guessing that one shaped like another will do \
         (A19)",
    ))
}

pub const ARCHITECTURE: &str = "llama";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub blocks: usize,
    pub embedding: usize,
    pub heads: usize,
    pub key_value_heads: usize,
    pub feed_forward: usize,
    pub context: usize,
    pub vocabulary: usize,
    pub stated_head_dimension: Option<usize>,
    pub experts: usize,
    pub experts_used: usize,
    pub sliding_window: Option<usize>,
    pub sliding_window_pattern: usize,
}

impl Shape {
    #[must_use]
    pub fn is_sliding(&self, block: usize) -> bool {
        if self.sliding_window.is_none() || self.sliding_window_pattern == 0 {
            return false;
        }
        block
            .checked_rem(self.sliding_window_pattern)
            .is_some_and(|within| within < self.sliding_window_pattern.saturating_sub(1))
    }
}

impl Shape {
    #[must_use]
    pub const fn head_dimension(&self) -> usize {
        if let Some(stated) = self.stated_head_dimension {
            return stated;
        }
        match self.embedding.checked_div(self.heads) {
            Some(width) => width,
            None => 0,
        }
    }

    #[must_use]
    pub const fn query_width(&self) -> usize {
        self.head_dimension().saturating_mul(self.heads)
    }

    #[must_use]
    pub const fn key_value_width(&self) -> usize {
        self.head_dimension().saturating_mul(self.key_value_heads)
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub shape: Shape,
    epsilon: f32,
    rope_theta: f32,
    rope_theta_swa: f32,
    habits: crate::architecture::Habits,
    threads: Threads,
    tensors: BTreeMap<String, Vec<f32>>,
}

#[derive(Debug, Default)]
pub struct Cache {
    keys: Vec<Vec<Vec<f32>>>,
    values: Vec<Vec<Vec<f32>>>,
}

impl Cache {
    #[must_use]
    pub fn for_model(shape: &Shape) -> Self {
        Self {
            keys: vec![Vec::new(); shape.blocks],
            values: vec![Vec::new(); shape.blocks],
        }
    }

    #[must_use]
    pub fn length(&self) -> usize {
        self.keys.first().map_or(0, Vec::len)
    }
}

pub fn load(file: &File, bytes: &[u8]) -> Result<Loaded> {
    let architecture = file.architecture().unwrap_or("unstated");
    covers(file)?;

    let shape = read_shape(file, architecture)?;
    let epsilon = float(
        file,
        &format!("{architecture}.attention.layer_norm_rms_epsilon"),
    )
    .unwrap_or(1e-5);
    let rope_theta = float(file, &format!("{architecture}.rope.freq_base")).unwrap_or(10_000.0);
    let rope_theta_swa =
        float(file, &format!("{architecture}.rope.freq_base_swa")).unwrap_or(10_000.0);

    let mut tensors = BTreeMap::new();
    for (name, elements) in manifest(&shape) {
        tensors.insert(name.clone(), read_tensor(file, bytes, &name, elements)?);
    }

    for (name, elements) in optional(&shape) {
        if file.tensor(&name).is_some() {
            tensors.insert(name.clone(), read_tensor(file, bytes, &name, elements)?);
        }
    }

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
        rope_theta_swa,
        habits: crate::architecture::habits(architecture),
        threads: Threads::definition(),
        tensors,
    })
}

fn read_shape(file: &File, architecture: &str) -> Result<Shape> {
    let key = |name: &str| format!("{architecture}.{name}");
    let shape = Shape {
        blocks: count(file, &key("block_count"))?,
        embedding: count(file, &key("embedding_length"))?,
        heads: count(file, &key("attention.head_count"))?,
        key_value_heads: match number(file, &key("attention.head_count_kv")) {
            Some(value) => usize::try_from(value).unwrap_or(0),
            None => count(file, &key("attention.head_count"))?,
        },
        feed_forward: count(file, &key("feed_forward_length"))?,
        context: count(file, &key("context_length"))?,
        stated_head_dimension: number(file, &key("attention.key_length"))
            .and_then(|value| usize::try_from(value).ok())
            .filter(|width| *width > 0),
        experts: number(file, &key("expert_count"))
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0),
        experts_used: number(file, &key("expert_used_count"))
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0),
        sliding_window: number(file, &key("attention.sliding_window"))
            .and_then(|value| usize::try_from(value).ok())
            .filter(|window| *window > 0),
        sliding_window_pattern: number(file, &key("attention.sliding_window_pattern"))
            .and_then(|value| usize::try_from(value).ok())
            .filter(|period| *period > 0)
            .unwrap_or(6),
        vocabulary: file
            .get("tokenizer.ggml.tokens")
            .and_then(Value::as_list)
            .map(<[Value]>::len)
            .ok_or_else(|| missing("tokenizer.ggml.tokens"))?,
    };

    if shape.experts > 0 && shape.experts_used == 0 {
        return Err(missing(&key("expert_used_count")));
    }
    if shape.experts_used > shape.experts {
        return Err(malformed(
            "the file routes each token to more experts than it has",
            &format!("{} used of {}", shape.experts_used, shape.experts),
        ));
    }

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

fn manifest(shape: &Shape) -> Vec<(String, usize)> {
    let kv_width = shape.key_value_width();
    let query = shape.query_width().saturating_mul(shape.embedding);
    let gate = shape.feed_forward.saturating_mul(shape.embedding);

    let mut wanted = vec![
        (
            "token_embd.weight".to_owned(),
            shape.vocabulary.saturating_mul(shape.embedding),
        ),
        ("output_norm.weight".to_owned(), shape.embedding),
    ];
    let stack = gate.saturating_mul(shape.experts);
    let feed_forward: Vec<(&str, usize)> = if shape.experts > 0 {
        vec![
            (
                "ffn_gate_inp.weight",
                shape.embedding.saturating_mul(shape.experts),
            ),
            ("ffn_gate_exps.weight", stack),
            ("ffn_up_exps.weight", stack),
            ("ffn_down_exps.weight", stack),
        ]
    } else {
        vec![
            ("ffn_gate.weight", gate),
            ("ffn_up.weight", gate),
            ("ffn_down.weight", gate),
        ]
    };

    for block in 0..shape.blocks {
        for (suffix, elements) in [
            ("attn_norm.weight", shape.embedding),
            ("attn_q.weight", query),
            ("attn_k.weight", kv_width.saturating_mul(shape.embedding)),
            ("attn_v.weight", kv_width.saturating_mul(shape.embedding)),
            ("attn_output.weight", query),
            ("ffn_norm.weight", shape.embedding),
        ] {
            wanted.push((format!("blk.{block}.{suffix}"), elements));
        }
        for (suffix, elements) in &feed_forward {
            wanted.push((format!("blk.{block}.{suffix}"), *elements));
        }
    }
    wanted
}

fn optional(shape: &Shape) -> Vec<(String, usize)> {
    let head = shape.head_dimension();
    let mut wanted = Vec::new();
    for block in 0..shape.blocks {
        for suffix in ["attn_q_norm.weight", "attn_k_norm.weight"] {
            wanted.push((format!("blk.{block}.{suffix}"), head));
        }
        for suffix in ["post_attention_norm.weight", "post_ffw_norm.weight"] {
            wanted.push((format!("blk.{block}.{suffix}"), shape.embedding));
        }
    }
    wanted
}

fn normalize_each_head(values: &mut [f32], head: usize, weights: &[f32], epsilon: f32) {
    if head == 0 {
        return;
    }
    let heads = values.len().wrapping_div(head);
    for index in 0..heads {
        let at = index.saturating_mul(head);
        let Some(slice) = values.get(at..at.saturating_add(head)) else {
            continue;
        };
        let normalized = ops::rms_norm(slice, weights, epsilon);
        if let Some(slot) = values.get_mut(at..at.saturating_add(head)) {
            slot.copy_from_slice(&normalized);
        }
    }
}

impl Loaded {
    #[must_use]
    pub const fn across(mut self, threads: Threads) -> Self {
        self.threads = threads;
        self
    }

    #[must_use]
    pub const fn threads(&self) -> Threads {
        self.threads
    }

    fn product(&self, matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
        ops::matmul_vec_across(matrix, vector, rows, columns, self.threads)
    }

    fn carried(&self, name: &str) -> Option<&[f32]> {
        self.tensors.get(name).map(Vec::as_slice)
    }

    #[must_use]
    pub fn output_is_tied(&self) -> bool {
        !self.tensors.contains_key("output.weight")
    }

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

        if self.habits.scales_the_embedding {
            let scale = f32::from(u16::try_from(width).unwrap_or(1)).sqrt();
            for value in &mut hidden {
                *value *= scale;
            }
        }

        for block in 0..self.shape.blocks {
            let mut attended = self.attention(block, &hidden, position, cache)?;
            if let Some(weights) = self.carried(&format!("blk.{block}.post_attention_norm.weight"))
            {
                attended = ops::rms_norm(&attended, weights, self.epsilon);
            }
            hidden = ops::add(&hidden, &attended);

            let mut fed = self.feed_forward(block, &hidden)?;
            if let Some(weights) = self.carried(&format!("blk.{block}.post_ffw_norm.weight")) {
                fed = ops::rms_norm(&fed, weights, self.epsilon);
            }
            hidden = ops::add(&hidden, &fed);
        }

        let normalized = ops::rms_norm(&hidden, self.tensor("output_norm.weight")?, self.epsilon);
        let projection = if self.output_is_tied() {
            self.tensor("token_embd.weight")?
        } else {
            self.tensor("output.weight")?
        };
        Ok(self.product(projection, &normalized, self.shape.vocabulary, width))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one block's attention is one sequence — project, normalize each head, rotate, \
                  cache, weigh, gather, project back — and every step of it depends on the \
                  widths bound at the top. Splitting it would put the widths in one function \
                  and their use in another, which is how a head width comes to be wrong \
                  somewhere and right elsewhere (F19)"
    )]
    fn attention(
        &self,
        block: usize,
        hidden: &[f32],
        position: usize,
        cache: &mut Cache,
    ) -> Result<Vec<f32>> {
        let width = self.shape.embedding;
        let head = self.shape.head_dimension();
        let query_width = self.shape.query_width();
        let kv_width = self.shape.key_value_width();
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
        let mut queries = self.product(
            self.tensor(&format!("blk.{block}.attn_q.weight"))?,
            &normalized,
            query_width,
            width,
        );
        let mut keys = self.product(
            self.tensor(&format!("blk.{block}.attn_k.weight"))?,
            &normalized,
            kv_width,
            width,
        );
        let values = self.product(
            self.tensor(&format!("blk.{block}.attn_v.weight"))?,
            &normalized,
            kv_width,
            width,
        );
        if queries.len() != query_width || keys.len() != kv_width || values.len() != kv_width {
            return Err(malformed(
                "a projection produced the wrong width",
                &format!("block {block}"),
            ));
        }

        if let Some(weights) = self.carried(&format!("blk.{block}.attn_q_norm.weight")) {
            normalize_each_head(&mut queries, head, weights, self.epsilon);
        }
        if let Some(weights) = self.carried(&format!("blk.{block}.attn_k_norm.weight")) {
            normalize_each_head(&mut keys, head, weights, self.epsilon);
        }

        let sliding = self.shape.is_sliding(block);
        let theta = if sliding {
            self.rope_theta_swa
        } else {
            self.rope_theta
        };

        for index in 0..self.shape.heads {
            let at = index.saturating_mul(head);
            if let Some(slice) = queries.get_mut(at..at.saturating_add(head)) {
                ops::rope(slice, position, theta, self.habits.rotation);
            }
        }
        for index in 0..self.shape.key_value_heads {
            let at = index.saturating_mul(head);
            if let Some(slice) = keys.get_mut(at..at.saturating_add(head)) {
                ops::rope(slice, position, theta, self.habits.rotation);
            }
        }

        push(&mut cache.keys, block, keys);
        push(&mut cache.values, block, values);
        let history = cache.keys.get(block).map_or(0, Vec::len);

        let scale = f32::from(u16::try_from(head).unwrap_or(1)).sqrt().recip();
        let mut attended = vec![0.0_f32; query_width];
        for head_index in 0..self.shape.heads {
            let group = head_index.checked_div(groups).unwrap_or(0);
            let query_at = head_index.saturating_mul(head);
            let key_at = group.saturating_mul(head);
            let query = queries
                .get(query_at..query_at.saturating_add(head))
                .unwrap_or(&[]);

            let mut weights = Vec::with_capacity(history);
            for step in 0..history {
                if let Some(window) = self.shape.sliding_window
                    && sliding
                    && position.saturating_sub(step) >= window
                {
                    weights.push(f32::NEG_INFINITY);
                    continue;
                }
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

        Ok(self.product(
            self.tensor(&format!("blk.{block}.attn_output.weight"))?,
            &attended,
            width,
            query_width,
        ))
    }

    fn feed_forward(&self, block: usize, hidden: &[f32]) -> Result<Vec<f32>> {
        let normalized = ops::rms_norm(
            hidden,
            self.tensor(&format!("blk.{block}.ffn_norm.weight"))?,
            self.epsilon,
        );
        if self.shape.experts > 0 {
            return self.experts(block, &normalized);
        }
        self.dense(block, &normalized)
    }

    fn dense(&self, block: usize, normalized: &[f32]) -> Result<Vec<f32>> {
        let width = self.shape.embedding;
        let inner = self.shape.feed_forward;
        let gate = self.product(
            self.tensor(&format!("blk.{block}.ffn_gate.weight"))?,
            normalized,
            inner,
            width,
        );
        let up = self.product(
            self.tensor(&format!("blk.{block}.ffn_up.weight"))?,
            normalized,
            inner,
            width,
        );
        let activated = ops::gated(&gate, &up, self.habits.activation);
        Ok(self.product(
            self.tensor(&format!("blk.{block}.ffn_down.weight"))?,
            &activated,
            width,
            inner,
        ))
    }

    fn experts(&self, block: usize, normalized: &[f32]) -> Result<Vec<f32>> {
        let width = self.shape.embedding;
        let inner = self.shape.feed_forward;
        let experts = self.shape.experts;
        let used = self.shape.experts_used.min(experts);

        let mut scores = self.product(
            self.tensor(&format!("blk.{block}.ffn_gate_inp.weight"))?,
            normalized,
            experts,
            width,
        );
        ops::softmax(&mut scores);

        let mut chosen: Vec<usize> = Vec::with_capacity(used);
        for _ in 0..used {
            let mut best: Option<usize> = None;
            for expert in 0..experts {
                if chosen.contains(&expert) {
                    continue;
                }
                let score = scores.get(expert).copied().unwrap_or(f32::NEG_INFINITY);
                let standing = best
                    .and_then(|index| scores.get(index).copied())
                    .unwrap_or(f32::NEG_INFINITY);
                if score > standing {
                    best = Some(expert);
                }
            }
            match best {
                Some(expert) => chosen.push(expert),
                None => break,
            }
        }

        let mut total = 0.0_f32;
        for expert in &chosen {
            total += scores.get(*expert).copied().unwrap_or(0.0);
        }
        let total = total.max(6.103_515_6e-5);

        let gates = self.tensor(&format!("blk.{block}.ffn_gate_exps.weight"))?;
        let ups = self.tensor(&format!("blk.{block}.ffn_up_exps.weight"))?;
        let downs = self.tensor(&format!("blk.{block}.ffn_down_exps.weight"))?;
        let per_expert = inner.saturating_mul(width);

        let mut out = vec![0.0_f32; width];
        for expert in chosen {
            let at = expert.saturating_mul(per_expert);
            let end = at.saturating_add(per_expert);
            let (Some(gate_weights), Some(up_weights), Some(down_weights)) =
                (gates.get(at..end), ups.get(at..end), downs.get(at..end))
            else {
                return Err(malformed(
                    "an expert is outside the stack the file said it holds",
                    &format!("block {block}, expert {expert} of {experts}"),
                ));
            };

            let gate = self.product(gate_weights, normalized, inner, width);
            let up = self.product(up_weights, normalized, inner, width);
            let activated = ops::gated(&gate, &up, self.habits.activation);
            let produced = self.product(down_weights, &activated, width, inner);

            let weight = scores.get(expert).copied().unwrap_or(0.0) / total;
            for (slot, value) in out.iter_mut().zip(produced.iter()) {
                *slot = weight.mul_add(*value, *slot);
            }
        }
        Ok(out)
    }

    fn tensor(&self, name: &str) -> Result<&[f32]> {
        self.tensors
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| missing(name))
    }
}

fn slice_at(steps: Option<&Vec<Vec<f32>>>, step: usize, at: usize, width: usize) -> &[f32] {
    steps
        .and_then(|steps| steps.get(step))
        .and_then(|row| row.get(at..at.saturating_add(width)))
        .unwrap_or(&[])
}

fn push(cache: &mut [Vec<Vec<f32>>], block: usize, row: Vec<f32>) {
    if let Some(steps) = cache.get_mut(block) {
        steps.push(row);
    }
}

pub(crate) fn read_tensor(
    file: &File,
    bytes: &[u8],
    name: &str,
    elements: usize,
) -> Result<Vec<f32>> {
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

pub(crate) fn count(file: &File, key: &str) -> Result<usize> {
    number(file, key)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| missing(key))
}

pub(crate) fn number(file: &File, key: &str) -> Option<i64> {
    file.get(key).and_then(Value::as_integer)
}

pub(crate) fn float(file: &File, key: &str) -> Option<f32> {
    match file.get(key) {
        Some(Value::Float(value)) => Some(narrow(*value)),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn narrow(value: f64) -> f32 {
    value as f32
}

pub(crate) fn missing(what: &str) -> Failure {
    Failure::new(
        Category::ArtifactProvenanceIncomplete,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the model file does not state something the architecture needs",
    )
    .with_context("wanted", what.to_owned())
}

pub(crate) fn malformed(detail: &str, found: &str) -> Failure {
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
pub(crate) mod tests;
