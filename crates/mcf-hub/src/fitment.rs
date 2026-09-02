//! Whether a model will run here, computed before a byte is fetched (B-213,
//! [PR3]).
//!
//! §6.3 makes *this will not run here, because it needs 131 GiB and you have
//! 24* a complete success of §III. PR3's observation is that the information
//! needed to say so arrives **before** the download, and is currently used only
//! to justify a refusal rather than to inform a choice — while a repository
//! publishing twenty quantizations makes choosing blind cost tens of gigabytes
//! a guess.
//!
//! **This is the arithmetic half, and it is exact.** Weights, plus the
//! key/value cache at the requested context, plus what a runtime holds beyond
//! the weights, against what the machine has. The *other* half — projecting
//! throughput from local history — is an estimate, is B-214's, and A20 keeps
//! the two apart: nothing here produces a number about speed.
//!
//! **Every input is untrusted** (§3.7). The sizes come from the hub's listing
//! and the shape from a model file's header, both of which a hostile source
//! chooses. So every arithmetic step is checked: a shape that multiplies out
//! past what can be counted, or a total that overflows, is a refusal to plan
//! rather than a plan built on a wrapped number.
//!
//! **What it does not do is decide.** A verdict here is *fits*, *fits without
//! room for your context*, or *does not fit*, with the numbers that produced
//! it. Which of several that fit a user should take is §IV's question and needs
//! measurements this machine has not taken yet (B34, B29).
//!
//! [PR3]: ../../../doc/proposals.md#pr3--pre-acquisition-planning

use mcf_record::json::Value;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::measurement::Bytes;

const WHERE: Subsystem = Subsystem::new("mcf-hub::fitment");

/// What a runtime holds beyond the weights and the cache.
///
/// Activations, the graph, the allocator's slack. A single conservative figure
/// rather than a model of an engine MCF has not admitted yet (D23, B-320): a
/// wrong model would produce confident bad guidance, and PR3's own caveat is
/// that the projection must be conservative. When an engine is vendored and
/// measured, this becomes a measurement and stops being a constant.
pub const RUNTIME_OVERHEAD: Bytes = Bytes(512 * 1024 * 1024);
/// The share of memory a plan leaves for everything else on the machine.
///
/// Stated as a fraction in per-cent so the arithmetic stays integral (A6's
/// habit: `Quantity` is `Ord` because no measurement here needs a float).
/// Ninety per cent, because a machine with nothing left is a machine that
/// swaps, and a plan that fills memory exactly is a plan that was wrong.
pub const USABLE_PER_CENT: u64 = 90;

/// The shape a key/value cache is computed from.
///
/// Every field comes from the model file's own metadata. There are no defaults:
/// a shape MCF guessed at would produce a plan about a different model (A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// How many blocks hold a key/value cache.
    ///
    /// Not *how many blocks the model has*. A model whose configuration lists
    /// its layer types — some full attention, some linear — caches only in the
    /// full-attention ones, and counting every block would overstate the cache
    /// by the ratio between them. The reference model publishes exactly that
    /// shape: sixty-four blocks, sixteen of them full attention, so the plain
    /// count is four times the truth ([findings.md](../../../doc/findings.md)
    /// F16).
    pub blocks: u64,
    /// How many key/value heads — the grouped count, not the query count, and
    /// getting that wrong overstates the cache by the grouping factor.
    pub key_value_heads: u64,
    /// How many elements one key/value head keeps per position, keys and
    /// values together.
    ///
    /// Twice the head width where a key and a value are each one head wide,
    /// which is what a configuration's `head_dim` describes. Not always: a
    /// model that caches a compressed latent keeps one latent per position
    /// and reads its values back out of it, and a shape that assumed a value
    /// for every key would have its cache nearly twice its size (F151).
    pub per_head: u64,
    /// How many bytes one cached element occupies.
    ///
    /// Two for the half-precision caches every engine uses by default. It is a
    /// parameter because a cache can be quantized, and a plan that assumed
    /// otherwise would refuse models that fit.
    pub bytes_per_element: u64,
}

impl Shape {
    /// The shape a model's own `config.json` states.
    ///
    /// Every field is read; none is defaulted. A model whose configuration does
    /// not say how many key/value heads it has is one MCF will not plan for,
    /// because the grouping factor is exactly what a guess would get wrong —
    /// eight heads planned as thirty-two overstates the cache fourfold, and the
    /// operator would be told a variant does not fit that does (A7).
    ///
    /// `head_dim` where the configuration states one; otherwise the width
    /// divided by the attention heads, which is the same number written another
    /// way and the shape older configurations use.
    ///
    /// The cache element is a *parameter of the run* rather than a fact about
    /// the model — two bytes for the half-precision caches engines use by
    /// default — so it is passed in rather than read.
    /// **Nested configurations are read where the model puts them.** A
    /// multimodal repository publishes one `config.json` describing several
    /// models, with the transformer's fields under `text_config` and the top
    /// level holding the composition. MCF looks at the top level first and then
    /// there — not because it is guessing, but because that is where the field
    /// is written (F16). A configuration with neither is one MCF will not plan
    /// for.
    #[must_use]
    pub fn from_configuration(configuration: &Value, bytes_per_element: u64) -> Option<Self> {
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
            // `checked_div` rather than `/`: a configuration claiming zero
            // attention heads is a configuration, not a division by zero
            // (§3.7).
            None => field("hidden_size")?.checked_div(field("num_attention_heads")?)?,
        };
        let declared_blocks = field("num_hidden_layers")?;
        Some(Self {
            // Only the blocks that cache. Where a configuration lists its layer
            // types, MCF counts the full-attention ones; where it does not,
            // every block caches, which is what a transformer without a hybrid
            // attention scheme does. Both are readings of the declaration
            // rather than assumptions about the model (§3.18, A21).
            blocks: caching_blocks(list("layer_types"), declared_blocks)?,
            key_value_heads: field("num_key_value_heads")?,
            // One key and one value per head, each the head's width.
            per_head: head_dimension.checked_mul(2)?,
            bytes_per_element,
        })
    }

    /// How many bytes one token of context costs, keys and values together.
    ///
    /// `None` when the arithmetic overflows, which is a shape from a hostile or
    /// broken file rather than a model (§3.7).
    #[must_use]
    pub fn bytes_per_token(&self) -> Option<u64> {
        self.blocks
            .checked_mul(self.key_value_heads)?
            .checked_mul(self.per_head)?
            .checked_mul(self.bytes_per_element)
    }
}

/// How many blocks hold a key/value cache.
///
/// `None` when the configuration lists layer types and *none* of them is a full
/// attention layer: a model that caches nothing at all is either a shape MCF
/// does not understand or a configuration that is wrong, and both are reasons
/// to refuse a plan rather than to produce one claiming a variant costs no
/// memory (A7).
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

/// One variant of a model, and what holding it would cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// What the variant is called — the file, usually, since that is how a
    /// quantization is named.
    pub name: String,
    /// The weights, as the hub's listing states them.
    ///
    /// The hub's *claim* (A21): B-021 checks it against what arrives, and a
    /// disagreement is a finding rather than an error here.
    pub weights: Bytes,
    /// The shape its cache is computed from.
    pub shape: Shape,
}

impl Requirement {
    /// What this variant needs at a stated context length.
    ///
    /// # Errors
    ///
    /// `artifact.format.malformed` when the arithmetic overflows — a shape or a
    /// size that cannot be added up is one MCF will not plan against.
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

/// What a plan says about one variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It fits at the context asked for, with this much left over.
    Fits {
        /// What the variant needs.
        needs: Bytes,
        /// What would remain of the usable memory.
        headroom: Bytes,
    },
    /// The weights fit and the context does not, and this is the longest
    /// context that would.
    ///
    /// A distinct answer rather than a refusal, because it is the one an
    /// operator can act on: the same download at a shorter context is a
    /// different, workable configuration.
    FitsWithoutContextHeadroom {
        /// What it needs at the context asked for.
        needs: Bytes,
        /// The longest context that fits.
        longest_context: u64,
    },
    /// It does not fit at any context, and this is by how much.
    DoesNotFit {
        /// What the weights alone need, with the runtime's overhead.
        needs: Bytes,
        /// How much more memory the machine would have to have.
        short_by: Bytes,
    },
}

impl Verdict {
    /// The outcome's name, as the record writes it.
    ///
    /// Stable for life (C5), and none of the three is a failure: all three are
    /// answers to *will this run here*, which A9 makes results.
    #[must_use]
    pub const fn outcome(&self) -> &'static str {
        match *self {
            Self::Fits { .. } => "fits",
            Self::FitsWithoutContextHeadroom { .. } => "fits_at_a_shorter_context",
            Self::DoesNotFit { .. } => "does_not_fit",
        }
    }

    /// Whether this variant can be run here at all.
    #[must_use]
    pub const fn is_runnable(&self) -> bool {
        !matches!(self, Self::DoesNotFit { .. })
    }
}

/// A plan, as the record keeps it (B-086, A9, §6.3).
///
/// **A9's other half.** *"Does not fit here" is a finding, not a failure.*
/// §6.3 already calls *this will not run here, because it needs 131 GiB and
/// you have 24* a complete success of §III, and this is where that success
/// stops being a sentence printed once and becomes something the register can
/// answer questions from: *what has this machine been told it cannot run* is
/// the question an operator asks before downloading tens of gigabytes a second
/// time.
///
/// Written whichever way each variant came out, because a record that kept
/// only the refusals could not answer *when was this last known to fit* (A1).
/// The numbers that produced each verdict travel with it, since a verdict
/// without them is an opinion (§3.4).
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

/// A count, saturating rather than wrapping into a negative number.
fn whole(held: u64) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

/// Judges one variant against what a machine has.
///
/// `available` is what the machine reports free, and the plan uses
/// [`USABLE_PER_CENT`] of it: a plan that fills memory exactly is a plan that
/// was wrong.
///
/// # Errors
///
/// As [`Requirement::at_context`].
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

    // The weights and the runtime's overhead alone: what a context of zero
    // would cost. If even that does not fit, no context does.
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

/// Judges every variant a repository publishes, in the order given.
///
/// This is PR3's sentence — *seven fit here with headroom, four fit without room
/// for your context, nine do not fit* — and it is produced without fetching any
/// of them.
///
/// # Errors
///
/// As [`assess`]: one variant whose arithmetic does not add up refuses the
/// whole plan rather than being dropped from it, because a plan missing a row
/// nobody mentioned is worse than no plan (A1).
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
