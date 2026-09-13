use super::{Anatomy, Role, declared, declared_per_block};
use crate::gguf::Model;

const CACHE_ELEMENT_BYTES: u64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cache {
    Sized {
        per_token: u64,
        key_heads: u64,
        per_head: u64,
        latent: bool,
        at_context: Option<(u64, u64)>,
        sliding_window: Option<u64>,
        attending: (u64, u64),
        recurrent: u64,
    },
    Unsized(&'static str),
}

impl Cache {
    #[must_use]
    pub fn kept(&self) -> Option<String> {
        match self {
            Self::Sized {
                per_head,
                latent: true,
                ..
            } => Some(format!(
                "one latent of {per_head} per position, which is read back as both key and \
                 value — no value cache"
            )),
            Self::Sized {
                per_head,
                latent: false,
                ..
            } => Some(format!("{per_head} for a key and a value")),
            Self::Unsized(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    pub multiply_adds: u64,
    pub head_width: Option<u64>,
    pub queries_per_key: Option<u64>,
    pub attention_at_context: Option<u64>,
    pub cache: Cache,
}

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
    let heads = declared_per_block(model, "attention.head_count");
    let key_heads = declared_per_block(model, "attention.head_count_kv");
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
    let latent = declared(model, "attention.kv_lora_rank").is_some();
    let attention_at_context = (|| {
        let per_block = heads?.checked_mul(widths?)?.checked_mul(context?)?;
        per_block.checked_mul(body.census.attending)
    })();
    let per_head = if latent { head_width } else { widths };
    let cache = if body.census.attending == 0 && body.census.recurrent > 0 {
        Cache::Unsized(
            "every block keeps a fixed recurrent state rather than keys and values per \
             position, and its width is not among the header's attention widths",
        )
    } else {
        cache_of(key_heads, per_head, latent, context, body, model)
    };
    Work {
        multiply_adds,
        head_width,
        queries_per_key,
        attention_at_context,
        cache,
    }
}

fn cache_of(
    key_heads: Option<u64>,
    per_head: Option<u64>,
    latent: bool,
    context: Option<u64>,
    body: &Anatomy,
    model: &Model,
) -> Cache {
    let per_token = (|| {
        key_heads?
            .checked_mul(per_head?)?
            .checked_mul(body.census.attending)?
            .checked_mul(CACHE_ELEMENT_BYTES)
    })();
    match per_token.zip(key_heads).zip(per_head) {
        Some(((per_token, key_heads), per_head)) => Cache::Sized {
            per_token,
            key_heads,
            per_head,
            latent,
            at_context: context.and_then(|tokens| Some((tokens, tokens.checked_mul(per_token)?))),
            sliding_window: declared(model, "attention.sliding_window"),
            attending: (body.census.attending, body.blocks),
            recurrent: body.census.recurrent,
        },
        None => Cache::Unsized(
            "the header does not name the key/value head count and the head widths, which are \
             what size it",
        ),
    }
}
