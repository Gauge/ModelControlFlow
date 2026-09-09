use mcf_record::json::Value;

use mcf_core::configuration::CacheType;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::measurement::Bytes;

const WHERE: Subsystem = Subsystem::new("mcf-hub::fitment");

pub const RUNTIME_OVERHEAD: Bytes = Bytes(512 * 1024 * 1024);
pub const USABLE_PER_CENT: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    pub blocks: u64,
    pub key_value_heads: u64,
    pub per_head: u64,
    pub cache: CacheType,
}

impl Shape {
    #[must_use]
    pub fn from_configuration(configuration: &Value, cache: CacheType) -> Option<Self> {
        let text = configuration.get("text_config");
        let field = |name: &str| {
            configuration
                .get(name)
                .or_else(|| text.and_then(|nested| nested.get(name)))
                .and_then(Value::as_integer)
                .and_then(|value| u64::try_from(value).ok())
                .filter(|value| *value > 0)
        };
        let list = |name: &str| {
            configuration
                .get(name)
                .or_else(|| text.and_then(|nested| nested.get(name)))
                .and_then(Value::as_list)
        };
        let head_dimension = match field("head_dim") {
            Some(stated) => stated,
            None => field("hidden_size")?.checked_div(field("num_attention_heads")?)?,
        };
        let declared_blocks = field("num_hidden_layers")?;
        Some(Self {
            blocks: caching_blocks(list("layer_types"), declared_blocks)?,
            key_value_heads: field("num_key_value_heads")?,
            per_head: head_dimension.checked_mul(2)?,
            cache,
        })
    }

    #[must_use]
    pub fn bytes_per_token(&self) -> Option<u64> {
        self.cache.bytes_for(self.elements_per_token()?)
    }

    #[must_use]
    pub fn elements_per_token(&self) -> Option<u64> {
        self.blocks
            .checked_mul(self.key_value_heads)?
            .checked_mul(self.per_head)
    }

    #[must_use]
    pub fn held_as(self, cache: CacheType) -> Self {
        Self { cache, ..self }
    }
}

fn caching_blocks(layer_types: Option<&[Value]>, declared: u64) -> Option<u64> {
    let Some(layers) = layer_types else {
        return Some(declared);
    };
    let caching = layers
        .iter()
        .filter_map(Value::as_text)
        .filter(|kind| kind.contains("full_attention"))
        .count();
    u64::try_from(caching).ok().filter(|count| *count > 0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub weights: Bytes,
    pub shape: Shape,
}

impl Requirement {
    pub fn at_context(&self, context: u64) -> Result<Bytes> {
        let per_token = self.shape.bytes_per_token().ok_or_else(|| {
            overflowed(
                "the shape multiplies out past what can be counted",
                &self.name,
            )
        })?;
        let cache = per_token.checked_mul(context).ok_or_else(|| {
            overflowed(
                "the cache at that context is larger than can be counted",
                &self.name,
            )
        })?;
        self.weights
            .0
            .checked_add(cache)
            .and_then(|total| total.checked_add(RUNTIME_OVERHEAD.0))
            .map(Bytes)
            .ok_or_else(|| overflowed("the total is larger than can be counted", &self.name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Fits { needs: Bytes, headroom: Bytes },
    FitsWithoutContextHeadroom { needs: Bytes, longest_context: u64 },
    DoesNotFit { needs: Bytes, short_by: Bytes },
}

impl Verdict {
    #[must_use]
    pub const fn outcome(&self) -> &'static str {
        match *self {
            Self::Fits { .. } => "fits",
            Self::FitsWithoutContextHeadroom { .. } => "fits_at_a_shorter_context",
            Self::DoesNotFit { .. } => "does_not_fit",
        }
    }

    #[must_use]
    pub const fn is_runnable(&self) -> bool {
        !matches!(self, Self::DoesNotFit { .. })
    }
}

#[must_use]
pub fn planned(verdicts: &[(String, Verdict)], context: u64, available: Bytes) -> Value {
    Value::map([
        ("context", Value::Integer(whole(context))),
        ("available_bytes", Value::Integer(whole(available.0))),
        ("usable_per_cent", Value::Integer(whole(USABLE_PER_CENT))),
        (
            "runtime_overhead_bytes",
            Value::Integer(whole(RUNTIME_OVERHEAD.0)),
        ),
        (
            "variants",
            Value::List(
                verdicts
                    .iter()
                    .map(|(name, verdict)| {
                        let mut fields = vec![
                            ("name".to_owned(), Value::text(name.clone())),
                            ("outcome".to_owned(), Value::text(verdict.outcome())),
                        ];
                        match verdict {
                            Verdict::Fits { needs, headroom } => {
                                fields.push((
                                    "needs_bytes".to_owned(),
                                    Value::Integer(whole(needs.0)),
                                ));
                                fields.push((
                                    "headroom_bytes".to_owned(),
                                    Value::Integer(whole(headroom.0)),
                                ));
                                fields.push(("longest_context".to_owned(), Value::Null));
                                fields.push(("short_by_bytes".to_owned(), Value::Null));
                            }
                            Verdict::FitsWithoutContextHeadroom {
                                needs,
                                longest_context,
                            } => {
                                fields.push((
                                    "needs_bytes".to_owned(),
                                    Value::Integer(whole(needs.0)),
                                ));
                                fields.push(("headroom_bytes".to_owned(), Value::Null));
                                fields.push((
                                    "longest_context".to_owned(),
                                    Value::Integer(whole(*longest_context)),
                                ));
                                fields.push(("short_by_bytes".to_owned(), Value::Null));
                            }
                            Verdict::DoesNotFit { needs, short_by } => {
                                fields.push((
                                    "needs_bytes".to_owned(),
                                    Value::Integer(whole(needs.0)),
                                ));
                                fields.push(("headroom_bytes".to_owned(), Value::Null));
                                fields.push(("longest_context".to_owned(), Value::Null));
                                fields.push((
                                    "short_by_bytes".to_owned(),
                                    Value::Integer(whole(short_by.0)),
                                ));
                            }
                        }
                        Value::map(fields)
                    })
                    .collect(),
            ),
        ),
    ])
}

fn whole(held: u64) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

pub fn assess(requirement: &Requirement, context: u64, available: Bytes) -> Result<Verdict> {
    let usable = available
        .0
        .checked_mul(USABLE_PER_CENT)
        .and_then(|scaled| scaled.checked_div(100))
        .unwrap_or(0);

    let needs = requirement.at_context(context)?;
    if needs.0 <= usable {
        return Ok(Verdict::Fits {
            needs,
            headroom: Bytes(usable.saturating_sub(needs.0)),
        });
    }

    let floor = requirement.at_context(0)?;
    if floor.0 > usable {
        return Ok(Verdict::DoesNotFit {
            needs: floor,
            short_by: Bytes(floor.0.saturating_sub(usable)),
        });
    }

    let per_token = requirement.shape.bytes_per_token().ok_or_else(|| {
        overflowed(
            "the shape multiplies out past what can be counted",
            &requirement.name,
        )
    })?;
    let longest = usable
        .saturating_sub(floor.0)
        .checked_div(per_token.max(1))
        .unwrap_or(0);
    Ok(Verdict::FitsWithoutContextHeadroom {
        needs,
        longest_context: longest,
    })
}

pub fn plan(
    requirements: &[Requirement],
    context: u64,
    available: Bytes,
) -> Result<Vec<(String, Verdict)>> {
    let mut out = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        out.push((
            requirement.name.clone(),
            assess(requirement, context, available)?,
        ));
    }
    Ok(out)
}

fn overflowed(detail: &str, name: &str) -> Failure {
    Failure::new(
        Category::ArtifactFormatMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        detail,
    )
    .with_context("variant", name.to_owned())
}

#[cfg(test)]
mod tests;
