use crate::gguf::{Model, Tensor, TensorKind, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Embedding,
    Output,
    Attention,
    FeedForward,
    Experts,
    Routing,
    Recurrent,
    NormsAndBiases,
    Other,
}

impl Role {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Output => "output head",
            Self::Attention => "attention",
            Self::FeedForward => "feed-forward",
            Self::Experts => "experts",
            Self::Routing => "routing",
            Self::Recurrent => "recurrent state",
            Self::NormsAndBiases => "norms and biases",
            Self::Other => "other",
        }
    }

    pub const ALL: [Self; 9] = [
        Self::Embedding,
        Self::Output,
        Self::Attention,
        Self::FeedForward,
        Self::Experts,
        Self::Routing,
        Self::Recurrent,
        Self::NormsAndBiases,
        Self::Other,
    ];
}

#[must_use]
pub fn role_of(name: &str) -> Role {
    let (leaf, suffix) = match name.rsplit_once('.') {
        Some((before, after)) if after == "weight" || after == "bias" => {
            (before.rsplit('.').next().unwrap_or(before), after)
        }
        _ => (name.rsplit('.').next().unwrap_or(name), ""),
    };
    if leaf.starts_with("ffn_") && leaf.ends_with("_exps") {
        return Role::Experts;
    }
    if suffix == "bias" || leaf.ends_with("_norm") || leaf == "output_norm" {
        return Role::NormsAndBiases;
    }
    if name.starts_with("token_embd") || name.starts_with("position_embd") {
        return Role::Embedding;
    }
    if name.starts_with("output.") {
        return Role::Output;
    }
    if leaf == "ffn_gate_inp" {
        return Role::Routing;
    }
    if leaf.starts_with("ffn_") {
        return Role::FeedForward;
    }
    if leaf.starts_with("attn_") {
        return Role::Attention;
    }
    if leaf.starts_with("ssm_") {
        return Role::Recurrent;
    }
    Role::Other
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub tensors: u64,
    pub elements: u64,
    pub bytes: Option<u64>,
}

impl Default for Share {
    fn default() -> Self {
        Self {
            tensors: 0,
            elements: 0,
            bytes: Some(0),
        }
    }
}

impl Share {
    pub(crate) fn add(&mut self, tensor: &Tensor) {
        self.tensors = self.tensors.saturating_add(1);
        self.elements = self.elements.saturating_add(tensor.elements().unwrap_or(0));
        self.bytes = match (self.bytes, tensor.bytes()) {
            (Some(held), Some(more)) => held.checked_add(more),
            _ => None,
        };
    }

    #[must_use]
    pub fn hundredths_of_a_bit(&self) -> Option<u64> {
        let bytes = self.bytes?;
        if self.elements == 0 {
            return None;
        }
        #[allow(
            clippy::integer_division,
            reason = "the figure is defined in hundredths and the remainder is not shown"
        )]
        Some(bytes.checked_mul(800)? / self.elements)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    pub what: &'static str,
    pub declared: Option<String>,
    pub observed: Option<String>,
    pub agrees: Option<bool>,
}

impl Agreement {
    fn of(what: &'static str, declared: Option<u64>, observed: Option<u64>) -> Self {
        Self {
            what,
            declared: declared.map(|held| held.to_string()),
            observed: observed.map(|held| held.to_string()),
            agrees: declared.zip(observed).map(|(one, other)| one == other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub experts: u64,
    pub used: u64,
    pub elements: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anatomy {
    pub elements: u64,
    pub bytes: Option<u64>,
    pub unsized_tensors: u64,
    pub roles: Vec<(Role, Share)>,
    pub kinds: Vec<(TensorKind, Share)>,
    pub blocks: u64,
    pub output_tied: bool,
    pub active: Option<Active>,
    pub agreements: Vec<Agreement>,
    pub census: blocks::Census,
}

#[must_use]
pub fn of(model: &Model) -> Anatomy {
    let mut elements: u64 = 0;
    let mut bytes: Option<u64> = Some(0);
    let mut unsized_tensors: u64 = 0;
    let mut roles: Vec<(Role, Share)> = Role::ALL
        .iter()
        .map(|role| (*role, Share::default()))
        .collect();
    let mut kinds: Vec<(TensorKind, Share)> = Vec::new();
    let mut blocks: u64 = 0;
    for tensor in &model.tensors {
        elements = elements.saturating_add(tensor.elements().unwrap_or(0));
        bytes = match (bytes, tensor.bytes()) {
            (Some(held), Some(more)) => held.checked_add(more),
            _ => None,
        };
        if tensor.bytes().is_none() {
            unsized_tensors = unsized_tensors.saturating_add(1);
        }
        let role = role_of(&tensor.name);
        if let Some((_, share)) = roles.iter_mut().find(|(held, _)| *held == role) {
            share.add(tensor);
        }
        if let Some((_, share)) = kinds.iter_mut().find(|(held, _)| *held == tensor.kind) {
            share.add(tensor);
        } else {
            let mut share = Share::default();
            share.add(tensor);
            kinds.push((tensor.kind, share));
        }
        if let Some(index) = block_index(&tensor.name) {
            blocks = blocks.max(index.saturating_add(1));
        }
    }
    roles.retain(|(_, share)| share.tensors > 0);
    kinds.sort_by_key(|(_, share)| core::cmp::Reverse(share.elements));
    let output_tied = model.tensor("output.weight").is_none();
    let active = active_of(model, &roles, elements);
    let agreements = agreements_of(model, elements, blocks, active.as_ref());
    Anatomy {
        elements,
        bytes,
        unsized_tensors,
        roles,
        kinds,
        blocks,
        output_tied,
        active,
        agreements,
        census: blocks::of(model),
    }
}

pub(crate) fn block_index(name: &str) -> Option<u64> {
    name.strip_prefix("blk.")?.split('.').next()?.parse().ok()
}

fn declared(model: &Model, suffix: &str) -> Option<u64> {
    let architecture = model.architecture()?;
    model
        .get(&format!("{architecture}.{suffix}"))
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
}

/// A model whose blocks are not all the same kind declares some figures once per block
/// rather than once for the model, writing a zero against every block the figure does not
/// apply to. The figure itself is what the blocks that do have it say, so a zero is a block
/// to pass over rather than a figure to take. Where the blocks disagree, the largest is the
/// one that does not under-count what they cost together.
fn declared_per_block(model: &Model, suffix: &str) -> Option<u64> {
    let architecture = model.architecture()?;
    let held = model.get(&format!("{architecture}.{suffix}"))?;
    let Some(per_block) = held.as_list() else {
        return held
            .as_integer()
            .and_then(|whole| u64::try_from(whole).ok());
    };
    per_block
        .iter()
        .filter_map(Value::as_integer)
        .filter_map(|block| u64::try_from(block).ok())
        .filter(|block| *block > 0)
        .max()
}

fn shape_of(model: &Model, leaf: &str) -> Option<Vec<u64>> {
    model
        .tensors
        .iter()
        .find(|tensor| {
            tensor
                .name
                .strip_prefix("blk.")
                .and_then(|rest| rest.split_once('.'))
                .is_some_and(|(_, name)| name == leaf)
        })
        .map(|tensor| tensor.dimensions.clone())
}

fn active_of(model: &Model, roles: &[(Role, Share)], elements: u64) -> Option<Active> {
    let experts = roles
        .iter()
        .find(|(role, _)| *role == Role::Experts)
        .map(|(_, share)| share)?;
    let count = *shape_of(model, "ffn_up_exps.weight")
        .or_else(|| shape_of(model, "ffn_gate_exps.weight"))
        .or_else(|| shape_of(model, "ffn_gate_up_exps.weight"))
        .or_else(|| shape_of(model, "ffn_down_exps.weight"))?
        .last()?;
    let used = declared(model, "expert_used_count")?;
    if count == 0 || used > count {
        return None;
    }
    #[allow(
        clippy::integer_division,
        reason = "a stacked expert tensor is a whole number of experts by construction"
    )]
    let per_expert = experts.elements / count;
    let visited = per_expert.checked_mul(used)?;
    let outside = elements.checked_sub(experts.elements)?;
    Some(Active {
        experts: count,
        used,
        elements: outside.checked_add(visited)?,
    })
}

fn agreements_of(
    model: &Model,
    elements: u64,
    blocks: u64,
    active: Option<&Active>,
) -> Vec<Agreement> {
    let embedding = model
        .tensor("token_embd.weight")
        .map(|held| held.dimensions.clone());
    let width = embedding.as_ref().and_then(|held| held.first().copied());
    let vocabulary = embedding.as_ref().and_then(|held| held.get(1).copied());
    let tokens = model
        .get("tokenizer.ggml.tokens")
        .and_then(Value::as_list)
        .and_then(|held| u64::try_from(held.len()).ok());
    let key = declared(model, "attention.key_length").or_else(|| {
        let heads = declared(model, "attention.head_count")?;
        let width = declared(model, "embedding_length")?;
        #[allow(
            clippy::integer_division,
            reason = "the width divides across the heads by convention"
        )]
        (heads > 0).then(|| width / heads)
    });
    let heads_from = |leaf: &str| {
        let shape = shape_of(model, leaf)?;
        let projected = *shape.get(1)?;
        let key = key?;
        (key > 0 && projected % key == 0).then(|| {
            #[allow(clippy::integer_division, reason = "checked exact by the guard")]
            {
                projected / key
            }
        })
    };
    let value = declared(model, "attention.value_length_mla")
        .or_else(|| declared(model, "attention.value_length"))
        .or(key);
    let heads_from_output = || {
        let shape = shape_of(model, "attn_output.weight")?;
        let gathered = *shape.first()?;
        let value = value?;
        (value > 0 && gathered % value == 0).then(|| {
            #[allow(clippy::integer_division, reason = "checked exact by the guard")]
            {
                gathered / value
            }
        })
    };
    let mut found = vec![
        Agreement::of("blocks", declared(model, "block_count"), Some(blocks)),
        Agreement::of(
            "embedding width",
            declared(model, "embedding_length"),
            width,
        ),
        Agreement::of(
            "vocabulary, embedding rows",
            declared(model, "vocab_size"),
            vocabulary,
        ),
        Agreement::of(
            "vocabulary, tokens listed",
            declared(model, "vocab_size"),
            tokens,
        ),
        Agreement::of(
            "attention heads",
            declared_per_block(model, "attention.head_count"),
            heads_from_output().or_else(|| heads_from("attn_q.weight")),
        ),
        Agreement::of(
            "key/value heads",
            declared_per_block(model, "attention.head_count_kv"),
            heads_from("attn_k.weight").or_else(|| heads_from("attn_kv_a_mqa.weight")),
        ),
        Agreement::of(
            "feed-forward width",
            declared(model, "feed_forward_length"),
            shape_of(model, "ffn_up.weight").and_then(|held| held.get(1).copied()),
        ),
        Agreement::of(
            "expert feed-forward width",
            declared(model, "expert_feed_forward_length"),
            shape_of(model, "ffn_up_exps.weight").and_then(|held| held.get(1).copied()),
        ),
        Agreement::of(
            "experts",
            declared(model, "expert_count"),
            shape_of(model, "ffn_up_exps.weight").and_then(|held| held.get(2).copied()),
        ),
    ];
    found.retain(|held| held.declared.is_some() || held.observed.is_some());
    found.push(parameters_agreement(model, elements));
    if let Some(agreement) = active.and_then(|held| active_agreement(model, held)) {
        found.push(agreement);
    }
    found
}

fn parameters_agreement(model: &Model, elements: u64) -> Agreement {
    if let Some(count) = model
        .get("general.parameter_count")
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
    {
        return Agreement::of("parameters", Some(count), Some(elements));
    }
    let label = model
        .get("general.size_label")
        .and_then(Value::as_text)
        .map(str::to_owned);
    let total = label.as_deref().and_then(|held| held.split('-').next());
    Agreement {
        what: "parameters",
        declared: label.as_deref().map(|held| format!("{held} (a label)")),
        observed: Some(format!("{elements} ({}B)", billions(elements))),
        agrees: total.and_then(|held| label_agrees(held, elements)),
    }
}

fn active_agreement(model: &Model, active: &Active) -> Option<Agreement> {
    let label = model.get("general.size_label")?.as_text()?;
    let part = label.split('-').find_map(|held| held.strip_prefix('A'))?;
    Some(Agreement {
        what: "parameters a token activates",
        declared: Some(format!("A{part} (a label)")),
        observed: Some(format!(
            "{} ({}B)",
            active.elements,
            billions(active.elements)
        )),
        agrees: label_agrees(part, active.elements),
    })
}

fn label_agrees(label: &str, elements: u64) -> Option<bool> {
    const MILLION: u64 = 1_000_000;
    let number = label.strip_suffix('B')?;
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty() || fraction.len() > 3 {
        return None;
    }
    let whole: u64 = whole.parse().ok()?;
    let fraction_value: u64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    let unit = 10_u64.pow(3_u32.saturating_sub(u32::try_from(fraction.len()).ok()?));
    let labelled = whole
        .checked_mul(1000)?
        .checked_add(fraction_value.checked_mul(unit)?)?;
    #[allow(
        clippy::integer_division,
        reason = "millions is the resolution the comparison is made at"
    )]
    let counted = elements / MILLION;
    let floor = labelled.saturating_sub(unit.div_ceil(2));
    let ceiling = labelled.checked_add(unit)?;
    Some(counted >= floor && counted < ceiling)
}

#[must_use]
pub fn billions(elements: u64) -> String {
    const TENTH: u64 = 100_000_000;
    #[allow(
        clippy::integer_division,
        reason = "one decimal is the resolution shown; the remainder is what it drops"
    )]
    let tenths = elements.saturating_add(TENTH / 2) / TENTH;
    #[allow(
        clippy::integer_division,
        reason = "the same, split into the whole and the tenth"
    )]
    let (whole, tenth) = (tenths / 10, tenths % 10);
    format!("{whole}.{tenth}")
}

#[must_use]
pub fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let mut out = String::with_capacity(digits.len().saturating_mul(4).div_ceil(3));
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len().saturating_sub(index)).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub mod blocks;
pub mod vocabulary;
pub mod work;

#[cfg(test)]
mod tests;
