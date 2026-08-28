//! MCF's own engine against the one it provisioned, on the same input
//! (B-362, D31, A12, A19, §II).
//!
//! **What this is, and what it is not.** `scripts/check-oracle.sh` compares
//! MCF against a reference implementation and is a *development* tier: it
//! needs a checkout of somebody else's source, and it runs when this
//! repository is being changed. This is the same question asked of the two
//! engines a **user** has — MCF's own and the one MCF built into a prefix
//! (B-367, D39) — on demand, on their machine, about their model.
//!
//! §II is why it exists at all: MCF makes claims about models, and a claim
//! computed by an engine nobody has checked is a claim about the engine. A12
//! is that MCF may not ask to be trusted, and this is the shape of not asking.
//!
//! **Texts cannot be compared, and that is the whole difficulty.** Greedy
//! generation is deterministic, so two correct implementations agree until two
//! tokens are close enough that a different summation order picks a different
//! winner — after which they are writing different sentences and nothing
//! downstream is the same question (F27, F40). So the comparison is *teacher
//! forced*: MCF is made to read the other engine's tokens, and at every
//! position is asked what it would have chosen. That stays comparable however
//! far the free generations have drifted.
//!
//! **What is asserted is the rank, not the margin.** F40 measured both. The
//! margin threshold was calibrated on one parting step per file and raises
//! false alarms when applied at every position of a long comparison; the rank
//! MCF gives the other engine's token separates noise from defect by two
//! orders of magnitude — three against six hundred and eighteen, with the line
//! at eight (F41). A top-two order swap is arithmetic. A token MCF ranks
//! hundredth is not.

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::Value;
use mcf_standin::tokenizer::Vocabulary;

/// How far apart two engines may put one token before it stops being
/// arithmetic.
///
/// Measured rather than chosen (F41): a clean comparison put the other
/// engine's token at worst third, and a deliberately broken attention
/// mechanism put it six hundred and eighteenth. Eight sits in that gap, which
/// is wider than any threshold in this repository — and it is provisional in
/// the direction every threshold here is: a defect that only ever swaps the
/// top two would pass.
pub const FURTHEST_RANK: usize = 8;

/// What the two engines did with the same tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    /// How many positions were compared.
    pub positions: usize,
    /// At how many MCF would have chosen the same token.
    pub agreed: usize,
    /// The worst rank MCF gave a token the other engine chose.
    pub furthest: usize,
    /// Where that happened.
    pub furthest_at: usize,
    /// Positions not compared because MCF would have ended the turn there.
    ///
    /// The other engine was generating freely and MCF is reading its output,
    /// so a position where MCF would stop is one where the two are being asked
    /// different questions — not a disagreement about the next token (F40's
    /// artifact, which cost an apparent defect an order of magnitude past any
    /// real one).
    pub set_aside: usize,
}

impl Agreement {
    /// Whether this is arithmetic rather than a defect, by the stated rule.
    #[must_use]
    pub fn within_arithmetic(&self) -> bool {
        self.furthest <= FURTHEST_RANK
    }

    /// The record's shape.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
        Value::map([
            ("positions", count(self.positions)),
            ("agreed", count(self.agreed)),
            ("furthest_rank", count(self.furthest)),
            ("furthest_at", count(self.furthest_at)),
            ("set_aside", count(self.set_aside)),
            ("furthest_rank_allowed", count(FURTHEST_RANK)),
            ("within_arithmetic", Value::Bool(self.within_arithmetic())),
        ])
    }
}

/// Reads the other engine's tokens with MCF's engine, position by position.
///
/// # Errors
///
/// Whatever loading the model reports. A model MCF cannot read is not a
/// disagreement — it is MCF declining to answer, and saying so.
pub fn against(bytes: &[u8], prompt: &[usize], produced: &[usize]) -> Result<Agreement, Failure> {
    // Before the model is loaded, not after. Nothing to compare against is a
    // fact about the *other* engine's answer and needs no model at all —
    // loading one to discover it is the shape F44 is about, paying for an
    // experiment whose result was already unavailable.
    if produced.is_empty() {
        return Err(Failure::new(
            Category::ProbeInconclusive,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-serve::crosscheck"),
            "the other engine produced no tokens to compare against, so there is nothing to \
             read with MCF's own — which is *could not tell*, not *the engines agree* (A7)",
        )
        .with_context("tokens_produced", "0")
        .with_context("prompt_tokens", prompt.len().to_string())
        .with_context(
            "what_to_do",
            "an engine that prints text and exits cannot be cross-checked at all; the \
             provisioned server hands back the identifiers it produced and can (B-376)",
        ));
    }

    let file = mcf_standin::gguf::parse(bytes)?;
    let vocabulary = Vocabulary::read(&file)?;
    let model = mcf_standin::llama::load(&file, bytes)?;

    let mut cache = mcf_standin::llama::Cache::for_model(&model.shape);
    let mut position = 0_usize;
    let mut logits = Vec::new();
    for token in prompt {
        logits = model.forward(*token, position, &mut cache)?;
        position = position.saturating_add(1);
    }

    let mut agreed = 0_usize;
    let mut furthest = 0_usize;
    let mut furthest_at = 0_usize;
    let mut set_aside = 0_usize;
    let mut positions = 0_usize;

    for next in produced {
        let mut ranked: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
        // Highest first, lower identifier on a tie — the order the sampler
        // uses, so this reports the choice that would actually have been made.
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let Some(best) = ranked.first().copied() else {
            break;
        };
        // A position where MCF would end the turn is one where the two engines
        // are answering different questions, not one where they disagree.
        if vocabulary.ending == Some(best.0) && best.0 != *next {
            set_aside = set_aside.saturating_add(1);
        } else {
            positions = positions.saturating_add(1);
            if best.0 == *next {
                agreed = agreed.saturating_add(1);
            } else {
                let rank = ranked
                    .iter()
                    .position(|(id, _)| id == next)
                    .unwrap_or(usize::MAX);
                if rank > furthest {
                    furthest = rank;
                    furthest_at = position;
                }
            }
        }
        logits = model.forward(*next, position, &mut cache)?;
        position = position.saturating_add(1);
    }

    Ok(Agreement {
        positions,
        agreed,
        furthest,
        furthest_at,
        set_aside,
    })
}

#[cfg(test)]
mod tests;
