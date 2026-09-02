//! The blocks, taken one by one and grouped by what each is made of.
//!
//! **A block count is not a description.** `block_count = 48` says nothing
//! about whether the forty-eight are alike, and increasingly they are not: a
//! model that keeps a recurrent state in three blocks of every four and
//! attends over its context in the fourth, a mixture whose first block is
//! dense and whose others are experts, a file quantised more finely in its
//! first blocks than its last. Each of those is read off the directory — the
//! tensors a block holds, their shapes, their encodings — and none of it is
//! read off the header, which does not say (A21).
//!
//! **Why it matters before running anything.** The key/value cache and the
//! attention arithmetic in [`super::work`] are per block *that attends*; a
//! model whose blocks mostly keep a fixed state instead has a cache a quarter
//! the size the block count would suggest. Sizing it from the block count was
//! wrong by exactly that factor, and this module is where the right count
//! comes from.

use std::collections::BTreeMap;

use super::Share;
use crate::gguf::{Model, Tensor};

/// How a block mixes information across positions, read from its tensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mixing {
    /// Attention over the context: keys and values kept per position.
    Attention,
    /// A recurrent state of fixed size, carried forward — nothing kept per
    /// position.
    Recurrent,
    /// Neither is named in the block.
    Nothing,
}

/// What feeds forward in a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    /// One feed-forward every token passes.
    Dense,
    /// Experts, of which a token visits a few.
    Experts {
        /// How many the expert tensors stack.
        count: u64,
        /// Whether a shared expert sits beside them, visited by every token.
        shared: bool,
    },
    /// No feed-forward tensor in the block.
    Nothing,
}

/// A block's make-up: which tensors it holds, at what shapes.
///
/// Two blocks with the same parts at the same shapes are one shape, whatever
/// their encodings — the encoding is counted, not part of the identity, so
/// that a file quantised unevenly still shows as one architecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    /// Each tensor's leaf name and dimensions, sorted by name.
    pub parts: Vec<(String, Vec<u64>)>,
    /// How it mixes across positions.
    pub mixing: Mixing,
    /// What feeds forward.
    pub feed: Feed,
}

impl Shape {
    /// What the shape is made of, in the words every surface uses.
    ///
    /// Here rather than on a surface so that the window and the command line
    /// cannot describe one block two ways (B-072).
    #[must_use]
    pub fn said(&self) -> String {
        let mixing = match self.mixing {
            Mixing::Attention => "attention over the context, keys and values kept per position",
            Mixing::Recurrent => "a recurrent state of fixed size, nothing kept per position",
            Mixing::Nothing => "no mixing across positions",
        };
        let feed = match self.feed {
            Feed::Dense => "one feed-forward every token passes".to_owned(),
            Feed::Experts {
                count,
                shared: true,
            } => {
                format!("{count} experts and a shared one every token passes")
            }
            Feed::Experts {
                count,
                shared: false,
            } => format!("{count} experts"),
            Feed::Nothing => "no feed-forward".to_owned(),
        };
        format!("{mixing}; {feed}")
    }
}

/// Every block of one shape, and what they hold together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    /// The shape.
    pub shape: Shape,
    /// Which blocks, ascending.
    pub blocks: Vec<u64>,
    /// Everything in them.
    pub share: Share,
    /// The least and the most finely encoded block, in hundredths of a bit
    /// per element — where every block's bytes are known. Equal where the
    /// file encodes them alike.
    pub bits: Option<(u64, u64)>,
}

/// The blocks, grouped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    /// Each shape's blocks, in the order the first block of each appears.
    pub families: Vec<Family>,
    /// How many blocks attend over the context, which is how many keep keys
    /// and values per position.
    pub attending: u64,
    /// How many keep a recurrent state instead.
    pub recurrent: u64,
    /// What sits outside every block: the embedding, the output head, the
    /// final norm.
    pub outside: Share,
}

/// Groups a model's blocks by what they are made of.
#[must_use]
pub fn of(model: &Model) -> Census {
    let mut held: BTreeMap<u64, Vec<&Tensor>> = BTreeMap::new();
    let mut outside = Share::default();
    for tensor in &model.tensors {
        match super::block_index(&tensor.name) {
            Some(index) => held.entry(index).or_default().push(tensor),
            None => outside.add(tensor),
        }
    }
    let mut families: Vec<Family> = Vec::new();
    for (index, tensors) in held {
        let shape = shape_of(&tensors);
        let mut held_by_block = Share::default();
        for tensor in &tensors {
            held_by_block.add(tensor);
        }
        let bits = held_by_block.hundredths_of_a_bit();
        match families.iter_mut().find(|family| family.shape == shape) {
            Some(family) => {
                family.blocks.push(index);
                family.share.tensors = family.share.tensors.saturating_add(held_by_block.tensors);
                family.share.elements =
                    family.share.elements.saturating_add(held_by_block.elements);
                family.share.bytes = match (family.share.bytes, held_by_block.bytes) {
                    (Some(held), Some(more)) => held.checked_add(more),
                    _ => None,
                };
                family.bits = match (family.bits, bits) {
                    (Some((least, most)), Some(held)) => Some((least.min(held), most.max(held))),
                    _ => None,
                };
            }
            None => families.push(Family {
                shape,
                blocks: vec![index],
                share: held_by_block,
                bits: bits.map(|held| (held, held)),
            }),
        }
    }
    let count = |mixing: Mixing| {
        families
            .iter()
            .filter(|family| family.shape.mixing == mixing)
            .map(|family| u64::try_from(family.blocks.len()).unwrap_or(u64::MAX))
            .fold(0_u64, u64::saturating_add)
    };
    Census {
        attending: count(Mixing::Attention),
        recurrent: count(Mixing::Recurrent),
        families,
        outside,
    }
}

/// What one block is made of.
fn shape_of(tensors: &[&Tensor]) -> Shape {
    let mut parts: Vec<(String, Vec<u64>)> = tensors
        .iter()
        .map(|tensor| (leaf_of(&tensor.name).to_owned(), tensor.dimensions.clone()))
        .collect();
    parts.sort();
    let leaves = || parts.iter().map(|(leaf, _)| leaf.as_str());
    // A recurrent block may still carry an `attn_qkv` — its input projection
    // — so the state tensors decide first.
    let mixing = if leaves().any(|leaf| leaf.starts_with("ssm_")) {
        Mixing::Recurrent
    } else if leaves().any(|leaf| leaf.starts_with("attn_k") || leaf.starts_with("attn_qkv")) {
        Mixing::Attention
    } else {
        Mixing::Nothing
    };
    let experts = parts
        .iter()
        .find(|(leaf, _)| leaf.starts_with("ffn_") && leaf.ends_with("_exps"))
        .and_then(|(_, dimensions)| dimensions.last().copied());
    let feed = match experts {
        Some(count) => Feed::Experts {
            count,
            shared: leaves().any(|leaf| leaf.ends_with("_shexp")),
        },
        None if leaves().any(|leaf| leaf.starts_with("ffn_up") || leaf.starts_with("ffn_down")) => {
            Feed::Dense
        }
        None => Feed::Nothing,
    };
    Shape {
        parts,
        mixing,
        feed,
    }
}

/// `attn_q` of `blk.3.attn_q.weight`: the name without the block and without
/// the `.weight` or `.bias`.
fn leaf_of(name: &str) -> &str {
    let after_block = name
        .strip_prefix("blk.")
        .and_then(|rest| rest.split_once('.'))
        .map_or(name, |(_, rest)| rest);
    after_block
        .strip_suffix(".weight")
        .or_else(|| after_block.strip_suffix(".bias"))
        .unwrap_or(after_block)
}

/// Blocks as ranges: `0–2, 4–6, 8` — and, past `most` ranges, `…`.
#[must_use]
pub fn ranges(blocks: &[u64], most: usize) -> String {
    let mut runs: Vec<(u64, u64)> = Vec::new();
    for &block in blocks {
        match runs.last_mut() {
            Some((_, end)) if end.saturating_add(1) == block => *end = block,
            _ => runs.push((block, block)),
        }
    }
    let mut said: Vec<String> = runs
        .iter()
        .take(most)
        .map(|(start, end)| {
            if start == end {
                start.to_string()
            } else {
                format!("{start}–{end}")
            }
        })
        .collect();
    if runs.len() > most {
        said.push("…".to_owned());
    }
    said.join(", ")
}
