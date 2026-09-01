//! What one token costs this model, in arithmetic the header and directory
//! fix before anything runs.
//!
//! **Three figures, each a product of numbers the file states.** A weight is
//! multiplied once per token it serves, so the weights a token passes through
//! are its multiply-adds. Attention over the context reads every earlier
//! position's key and value once per head per block, so the context length
//! times the attention widths is what the last token of a full window costs on
//! top of that. And every position keeps its keys and values for the next
//! token to read, so the key and value widths across the blocks are what one
//! token holds in the cache — which, times the context, is the memory the
//! cache takes when the window is full.
//!
//! **None of it is a measurement** (A20). It is the arithmetic an engine's
//! own planner does, shown so that an operator can see the shape of the cost
//! before choosing whether to pay it. A speed is measured by `mcf bench`, not
//! computed here. And where the arithmetic does not apply — a compressed
//! latent cache, whose width is not the key width the header names — the
//! figure is withheld with the reason, rather than computed from the wrong
//! formula and shown with confidence (A7).

use super::{Anatomy, Role, declared};
use crate::gguf::Model;

/// Bytes per cached element at the width engines keep a cache in by default.
const CACHE_ELEMENT_BYTES: u64 = 2;

/// The key-and-value cache one token holds, or why it is not sized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cache {
    /// Sized from the header's widths.
    Sized {
        /// Bytes per token, at sixteen bits an element.
        per_token: u64,
        /// The declared context length and the bytes a full window holds.
        at_context: Option<(u64, u64)>,
        /// A sliding window the header declares, which bounds how much of the
        /// context some blocks read — the full-window figure is then an upper
        /// bound, not the figure.
        sliding_window: Option<u64>,
    },
    /// Not sized, and why.
    Unsized(&'static str),
}

/// One token's arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    /// Multiply-adds through the weights a token passes: the active elements
    /// less the embedding table, which is looked up rather than multiplied —
    /// unless the output head reuses it, in which case it is multiplied once.
    pub multiply_adds: u64,
    /// The width of one attention head, as the header declares it or as the
    /// embedding width divides by the head count.
    pub head_width: Option<u64>,
    /// How many query heads share one key/value head, where it divides.
    pub queries_per_key: Option<u64>,
    /// What attention over a full declared context adds per token, summed
    /// across the blocks.
    pub attention_at_context: Option<u64>,
    /// The cache.
    pub cache: Cache,
}

/// One token's arithmetic, from the header and the counted directory.
#[must_use]
pub fn of(model: &Model, body: &Anatomy) -> Work {
    let active = body
        .active
        .as_ref()
        .map_or(body.elements, |held| held.elements);
    let embedding = body
        .roles
        .iter()
        .find(|(role, _)| *role == Role::Embedding)
        .map_or(0, |(_, share)| share.elements);
    let multiply_adds = if body.output_tied {
        active
    } else {
        active.saturating_sub(embedding)
    };
    let heads = declared(model, "attention.head_count");
    let key_heads = declared(model, "attention.head_count_kv");
    let head_width = declared(model, "attention.key_length").or_else(|| {
        let (width, count) = (declared(model, "embedding_length")?, heads?);
        if count > 0 && width.is_multiple_of(count) {
            width.checked_div(count)
        } else {
            None
        }
    });
    let queries_per_key = heads.zip(key_heads).and_then(|(all, keys)| {
        if keys > 0 && all.is_multiple_of(keys) {
            all.checked_div(keys)
        } else {
            None
        }
    });
    let value_width = declared(model, "attention.value_length").or(head_width);
    let context = declared(model, "context_length");
    let widths = head_width
        .zip(value_width)
        .and_then(|(key, value)| key.checked_add(value));
    // A latent-attention model (`kv_lora_rank`) attends through a compressed
    // vector, and neither the cache nor the attention arithmetic is the
    // product of the widths the header names under the usual keys.
    let latent = declared(model, "attention.kv_lora_rank").is_some();
    let attention_at_context = (|| {
        if latent {
            return None;
        }
        let per_block = heads?.checked_mul(widths?)?.checked_mul(context?)?;
        per_block.checked_mul(body.blocks)
    })();
    let cache = if latent {
        Cache::Unsized(
            "this model caches a compressed latent rather than its keys and values, and its \
             width is not the key width the header names",
        )
    } else {
        cache_of(key_heads, widths, context, body.blocks, model)
    };
    Work {
        multiply_adds,
        head_width,
        queries_per_key,
        attention_at_context,
        cache,
    }
}

/// The cache, where the header names the widths that size it.
fn cache_of(
    key_heads: Option<u64>,
    widths: Option<u64>,
    context: Option<u64>,
    blocks: u64,
    model: &Model,
) -> Cache {
    let per_token = (|| {
        key_heads?
            .checked_mul(widths?)?
            .checked_mul(blocks)?
            .checked_mul(CACHE_ELEMENT_BYTES)
    })();
    match per_token {
        Some(per_token) => Cache::Sized {
            per_token,
            at_context: context.and_then(|tokens| Some((tokens, tokens.checked_mul(per_token)?))),
            sliding_window: declared(model, "attention.sliding_window"),
        },
        None => Cache::Unsized(
            "the header does not name the key/value head count and the head widths, which are \
             what size it",
        ),
    }
}
