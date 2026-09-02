//! What a model *is*, counted from its tensor directory rather than read from
//! its name (A21, §3.18).
//!
//! **Declared and observed are two columns.** A file says `general.size_label
//! = "8B"`, `llama.attention.head_count = 32`, `llama.vocab_size = 155136`. Its
//! directory also says `blk.0.attn_q.weight` is 4096 by 4096 and
//! `token_embd.weight` is 4096 by 155136, and those shapes are what the
//! engine will multiply. Everything in this module is arithmetic on the second
//! kind: a parameter is an element of a tensor, a block is a `blk.N.` prefix
//! that occurs, a head is a query width divided by a key width. Where the file
//! also declares the figure, [`Agreement`] puts the two side by side and says
//! whether they agree — because a header that disagrees with its own
//! directory is a file an operator should know about before running it, and
//! nothing else in MCF compares them.
//!
//! **No floats.** Bits per weight is bytes-times-eight over elements, kept in
//! hundredths; a share is a count over a count, in parts per million. The
//! arithmetic that turns a directory into a figure is exact and integer, and
//! the rounding happens once, where the figure is printed.
//!
//! **What is not here.** Nothing about behaviour: a parameter count says
//! nothing about what the model does with them, and the module says nothing
//! it did not count. The memory a running model needs is the serving crate's
//! arithmetic ([`mcf_serve::engines`]), which has the machine in hand; this
//! module has only the file.
//!
//! [`mcf_serve::engines`]: https://docs.rs/mcf-serve

use crate::gguf::{Model, Tensor, TensorKind, Value};

/// The part of a model a tensor belongs to, from its name.
///
/// The format's naming is a convention every converter follows (`blk.N.attn_q`,
/// `ffn_up_exps`, `token_embd`), and a tensor whose name follows none of it is
/// [`Role::Other`] with the name kept, rather than guessed into a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// The table that turns a token identifier into a vector.
    Embedding,
    /// The head that turns the last vector back into a score per token.
    Output,
    /// Query, key, value and output projections.
    Attention,
    /// The dense feed-forward of a block, and any expert every token visits.
    FeedForward,
    /// The experts of a mixture, of which a token visits a few.
    Experts,
    /// The router that chooses which experts a token visits.
    Routing,
    /// The projections and decays of a block that keeps a fixed recurrent
    /// state across positions rather than keys and values per position.
    Recurrent,
    /// Normalisation weights and every bias.
    NormsAndBiases,
    /// A name this module does not place.
    Other,
}

impl Role {
    /// What the role is called on a surface.
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

    /// Every role, in the order a surface lists them.
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

/// Which part of the model a tensor is, read from its name.
#[must_use]
pub fn role_of(name: &str) -> Role {
    // `blk.N.ssm_a` carries no `.weight`: a name's last segment is its suffix
    // only when it is one, else it is the leaf itself.
    let (leaf, suffix) = match name.rsplit_once('.') {
        Some((before, after)) if after == "weight" || after == "bias" => {
            (before.rsplit('.').next().unwrap_or(before), after)
        }
        _ => (name.rsplit('.').next().unwrap_or(name), ""),
    };
    // An expert's bias is stacked like its weight and is divided among the
    // experts the same way, so it is counted with them.
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
    // The state-space convention: `ssm_conv1d`, `ssm_a`, `ssm_dt`, `ssm_out`.
    // A recurrent block's own query, key and value projections are still
    // named `attn_qkv` and counted as attention above; this is the rest of it.
    if leaf.starts_with("ssm_") {
        return Role::Recurrent;
    }
    Role::Other
}

/// How much of the model one role or one encoding holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    /// How many tensors.
    pub tensors: u64,
    /// How many elements across them.
    pub elements: u64,
    /// How many bytes, where every tensor's encoding is one this reader sizes.
    pub bytes: Option<u64>,
}

impl Default for Share {
    /// Nothing yet — and nothing yet has a size, zero, which is what the
    /// first tensor adds to. A share that began *unsized* would stay unsized
    /// through every tensor it counted.
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

    /// Bits per element, in hundredths, where the bytes are known.
    #[must_use]
    pub fn hundredths_of_a_bit(&self) -> Option<u64> {
        let bytes = self.bytes?;
        if self.elements == 0 {
            return None;
        }
        // Exact to the hundredth, then truncated: what is printed is the
        // hundredth, and a rounding here would be a rounding twice.
        #[allow(
            clippy::integer_division,
            reason = "the figure is defined in hundredths and the remainder is not shown"
        )]
        Some(bytes.checked_mul(800)? / self.elements)
    }
}

/// A figure the file declares beside the same figure counted from its
/// directory (A21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    /// What is being compared.
    pub what: &'static str,
    /// What the header says, as the header says it.
    pub declared: Option<String>,
    /// What the directory says.
    pub observed: Option<String>,
    /// Whether they agree; `None` where either side is missing, which is not
    /// a disagreement.
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

/// What a mixture of experts activates for one token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    /// How many experts each block holds, from the expert tensors' shape.
    pub experts: u64,
    /// How many of them a token visits, as the header declares — the router
    /// decides at run time and the file only says how many it picks.
    pub used: u64,
    /// Elements a token passes through: everything outside the experts plus
    /// `used` of `experts` of each expert tensor.
    pub elements: u64,
}

/// A model, counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anatomy {
    /// Every element of every tensor.
    pub elements: u64,
    /// Every tensor's bytes, where every encoding is one this reader sizes.
    pub bytes: Option<u64>,
    /// How many tensors are encoded in a way this reader cannot size.
    pub unsized_tensors: u64,
    /// Each role's share, in [`Role::ALL`]'s order, roles with nothing omitted.
    pub roles: Vec<(Role, Share)>,
    /// Each encoding's share, largest by elements first.
    pub kinds: Vec<(TensorKind, Share)>,
    /// How many blocks the directory names.
    pub blocks: u64,
    /// Whether the output head reuses the embedding table — no `output.weight`
    /// in the directory.
    pub output_tied: bool,
    /// What a token activates, where the model is a mixture.
    pub active: Option<Active>,
    /// Header against directory, figure by figure.
    pub agreements: Vec<Agreement>,
    /// The blocks, grouped by what each is made of.
    pub census: blocks::Census,
}

/// Counts a model from its directory.
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

/// The `N` of `blk.N.`, where the name has one.
pub(crate) fn block_index(name: &str) -> Option<u64> {
    name.strip_prefix("blk.")?.split('.').next()?.parse().ok()
}

/// A number the header states under the architecture's prefix.
fn declared(model: &Model, suffix: &str) -> Option<u64> {
    let architecture = model.architecture()?;
    model
        .get(&format!("{architecture}.{suffix}"))
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
}

/// The shape of the first tensor of a name, in any block.
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

/// What a token activates, where the directory holds experts.
fn active_of(model: &Model, roles: &[(Role, Share)], elements: u64) -> Option<Active> {
    let experts = roles
        .iter()
        .find(|(role, _)| *role == Role::Experts)
        .map(|(_, share)| share)?;
    // The expert count is the last dimension of any expert tensor: the
    // directory holds all of them stacked, and that is observed.
    let count = *shape_of(model, "ffn_up_exps.weight")
        .or_else(|| shape_of(model, "ffn_gate_exps.weight"))
        .or_else(|| shape_of(model, "ffn_gate_up_exps.weight"))
        .or_else(|| shape_of(model, "ffn_down_exps.weight"))?
        .last()?;
    let used = declared(model, "expert_used_count")?;
    if count == 0 || used > count {
        return None;
    }
    // Exact: the stacked tensor is `count` experts of equal shape.
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

/// Header against directory, for every figure both state.
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
        // The convention where the header omits it.
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
    // Heads are read off the output projection, whose first dimension is the
    // heads' outputs laid side by side, rather than off the query projection.
    // A query projection may be wider than the heads it serves: one hybrid's
    // carries a gate beside every query (F150), and reading heads from it
    // said thirty-two against a header that declared sixteen. The head's
    // output width is the latent value width where the header declares one,
    // else the value width, else the key width.
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
            declared(model, "attention.head_count"),
            heads_from_output().or_else(|| heads_from("attn_q.weight")),
        ),
        Agreement::of(
            "key/value heads",
            declared(model, "attention.head_count_kv"),
            // A latent-attention model has no key projection: its one
            // key/value head is the latent projection, whose width is the
            // key length the header names.
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
    // Both columns present only where the header declares one; a row that
    // would say *nothing against nothing* is left out.
    found.retain(|held| held.declared.is_some() || held.observed.is_some());
    found.push(parameters_agreement(model, elements));
    if let Some(agreement) = active.and_then(|held| active_agreement(model, held)) {
        found.push(agreement);
    }
    found
}

/// The parameter count, against the header's count or its size label.
fn parameters_agreement(model: &Model, elements: u64) -> Agreement {
    if let Some(count) = model
        .get("general.parameter_count")
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
    {
        return Agreement::of("parameters", Some(count), Some(elements));
    }
    // A size label is a rounded figure in the publisher's words — `8B`,
    // `30B-A3B` — and agreement with it is agreement to the label's own
    // resolution.
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

/// The active count against the label's `A` part, where the label has one.
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

/// Whether a count agrees with a label like `8B` or `2.6B`, to the label's own
/// resolution.
///
/// A label is a count either rounded or truncated to its last digit —
/// publishers do both, and a `30B` with 30.53 billion parameters in it is
/// truncated, not wrong. So a count agrees when it lies from half a unit below
/// the label to one unit above it: `8B` covers 7.5 up to but not including 9
/// billion, `2.6B` covers 2.55 up to 2.7. Anything outside that is a label
/// the count does not support at the label's own precision.
///
/// `None` for a label this does not read — `64x2.6B` names experts times a
/// size, and is not a count.
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
    // The label's own unit, in millions: a whole number of billions has a
    // unit of 1000 million, one decimal 100 million, and so on.
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

/// A count in billions to one decimal, as a label would write it.
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

/// `1,234,567`: a count with its thousands separated, as every surface
/// writes one a person reads rather than compares.
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
