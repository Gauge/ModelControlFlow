use std::collections::BTreeMap;

use super::Share;
use crate::gguf::{Model, Tensor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mixing {
    Attention,
    Recurrent,
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feed {
    Dense,
    Experts { count: u64, shared: bool },
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub parts: Vec<(String, Vec<u64>)>,
    pub mixing: Mixing,
    pub feed: Feed,
}

impl Shape {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub shape: Shape,
    pub blocks: Vec<u64>,
    pub share: Share,
    pub bits: Option<(u64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    pub families: Vec<Family>,
    pub attending: u64,
    pub recurrent: u64,
    pub outside: Share,
}

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

fn shape_of(tensors: &[&Tensor]) -> Shape {
    let mut parts: Vec<(String, Vec<u64>)> = tensors
        .iter()
        .map(|tensor| (leaf_of(&tensor.name).to_owned(), tensor.dimensions.clone()))
        .collect();
    parts.sort();
    let leaves = || parts.iter().map(|(leaf, _)| leaf.as_str());
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
