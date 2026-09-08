use std::path::Path;

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

pub const POSITIONS: usize = 120;

pub const PROMPT: &str = "The history of the city of Paris begins";

pub const FURTHEST_RANK: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    pub positions: usize,
    pub agreed: usize,
    pub furthest: usize,
    pub furthest_at: usize,
    pub set_aside: usize,
    pub ranks: Vec<Option<usize>>,
}

impl Agreement {
    #[must_use]
    pub fn within_arithmetic(&self) -> bool {
        self.furthest <= FURTHEST_RANK
    }

    #[must_use]
    pub fn said(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "MCF's own engine read {} position(s) of what the provisioned engine produced, and \
             would have chosen the same token at {}",
            self.positions, self.agreed
        )];
        if self.set_aside > 0 {
            lines.push(format!(
                "{} position(s) set aside: MCF would have ended the turn there, and the other \
                 engine was generating freely — the two are answering different questions at \
                 those positions rather than disagreeing (F40)",
                self.set_aside
            ));
        }
        if self.within_arithmetic() {
            lines.push(format!(
                "AGREE — where they differed, the other engine's token was never worse than \
                 MCF's rank {}; the line is {}, and a swap of the top few is two implementations \
                 summing in a different order rather than one of them being wrong (F27, F41)",
                self.furthest, FURTHEST_RANK
            ));
        } else {
            lines.push(format!(
                "DIVERGE — at position {} MCF ranked the other engine's token {}, past the {} \
                 that separates arithmetic from a defect; one of these two implementations is \
                 wrong and this does not say which — what it says is that the difference is not \
                 summation order (A19, F41)",
                self.furthest_at, self.furthest, FURTHEST_RANK
            ));
        }
        lines.push(
            "neither engine is the authority here: what is compared is two readings of one file, \
             and a disagreement is a finding about one of them (§II, A12)"
                .to_owned(),
        );
        lines
    }

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

pub fn examined(path: &Path, free: Option<u64>) -> Result<gguf::Model, Failure> {
    use std::io::Read as _;

    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        let opened =
            std::fs::File::open(path).and_then(|handle| handle.take(take).read_to_end(&mut prefix));
        if let Err(error) = opened {
            return Err(Failure::new(
                Category::ArtifactMissing,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::crosscheck"),
                "the model file could not be read",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string()));
        }
        match gguf::parse(&prefix) {
            Ok(file) => {
                mcf_standin::llama::covers(&file)?;
                if let Some(available) = free {
                    file.fits_dequantized(available)?;
                }
                return Ok(file);
            }
            Err(failure) => {
                if take >= held {
                    return Err(failure);
                }
            }
        }
    }
    Err(Failure::new(
        Category::ArtifactMissing,
        Attribution::Machine,
        Disposition::Refused,
        Subsystem::new("mcf-serve::crosscheck"),
        "the model file is empty",
    )
    .with_context("path", path.display().to_string()))
}

pub fn against(
    bytes: &[u8],
    prompt: &[usize],
    produced: &[usize],
    free: Option<u64>,
) -> Result<Agreement, Failure> {
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
    if let Some(available) = free {
        file.fits_dequantized(available)?;
    }
    let vocabulary = Vocabulary::read(&file)?;
    let model = mcf_standin::llama::load(&file, bytes)?
        .across(mcf_standin::threads::Threads::what_the_machine_reports());

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
    let mut ranks: Vec<Option<usize>> = Vec::with_capacity(produced.len());

    for next in produced {
        let mut ranked: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
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
        if vocabulary.ending == Some(best.0) && best.0 != *next {
            set_aside = set_aside.saturating_add(1);
            ranks.push(None);
        } else {
            positions = positions.saturating_add(1);
            if best.0 == *next {
                agreed = agreed.saturating_add(1);
                ranks.push(Some(1));
            } else {
                let rank = ranked
                    .iter()
                    .position(|(id, _)| id == next)
                    .unwrap_or(usize::MAX);
                ranks.push(Some(rank.saturating_add(1)));
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
        ranks,
    })
}

#[cfg(test)]
mod tests;
