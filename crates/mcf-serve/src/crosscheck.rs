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
//!
//! **One implementation, three surfaces** (B-072, B-424). The daemon runs the
//! comparison — `Request::CrossCheck` — and the console, the window and the
//! terminal ask it to and print what it said. The prompt, the length and the
//! sentences live here so that no surface can quietly compare something
//! else, or say the same figures in different words.

use std::path::Path;

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

/// How much of a generation to compare.
///
/// Long enough to leave the region where two engines agree by construction —
/// F40 found them parting at step four — and short enough that MCF's own
/// engine, which pays a forward pass per position, answers in minutes rather
/// than an afternoon.
pub const POSITIONS: usize = 120;

/// What both engines are asked.
pub const PROMPT: &str = "The history of the city of Paris begins";

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
    /// Every position in order: the rank MCF gave the other engine's token,
    /// one being agreement, or `None` where the position was set aside. The
    /// raw reading the figures above are arithmetic over (D54, D16).
    pub ranks: Vec<Option<usize>>,
}

impl Agreement {
    /// Whether this is arithmetic rather than a defect, by the stated rule.
    #[must_use]
    pub fn within_arithmetic(&self) -> bool {
        self.furthest <= FURTHEST_RANK
    }

    /// The comparison in sentences, written so a reader can disagree with the
    /// rule as well as the answer — the same sentences on every surface.
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

/// Reads a file's directory from a bounded prefix and refuses early what can
/// be refused early: an architecture MCF has not been taught, and a model that
/// cannot fit dequantized (B-372).
///
/// `gguf::parse` was written to read the directory of a file it does not hold
/// all of — B-213's pre-acquisition fitment needs exactly that — so this reads
/// sixteen mebibytes, and two hundred and fifty-six only if the metadata alone
/// outgrows that. The growth rule needs no knowledge of which failure means
/// "truncated": a prefix that failed to parse is only retried *larger*, and a
/// whole file that failed to parse is what failing honestly looks like.
///
/// `free` is what the platform says is available, observed by the caller —
/// B4 keeps hardware sampling out of here, and a caller that does not know
/// passes `None`, which is not a refusal (A7). The directory is handed back
/// because the caller that weighed a file usually reads its vocabulary next.
///
/// # Errors
///
/// The file could not be read; its directory could not be parsed; its
/// architecture is not covered; or, dequantized, it is larger than `free`.
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
                // The directory may simply be longer than this prefix; try the
                // next size up rather than deciding anything from a partial
                // read.
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

/// Reads the other engine's tokens with MCF's engine, position by position.
///
/// `free` is what the platform says is available, observed by the caller —
/// B4 keeps hardware sampling out of here, and a caller that does not know
/// passes `None`, which is not a refusal (A7).
///
/// # Errors
///
/// Whatever loading the model reports, and `resource.memory.exhausted` where
/// this model cannot be held dequantized. A model MCF cannot read is not a
/// disagreement — it is MCF declining to answer, and saying so.
pub fn against(
    bytes: &[u8],
    prompt: &[usize],
    produced: &[usize],
    free: Option<u64>,
) -> Result<Agreement, Failure> {
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
    // Weighed before it is dequantized, the same arithmetic every other path
    // uses. A cross-check exists to compare two engines and it cannot do that
    // from inside a process the kernel has ended (B-372, F136).
    if let Some(available) = free {
        file.fits_dequantized(available)?;
    }
    let vocabulary = Vocabulary::read(&file)?;
    // Threads change what this costs and not what it produces (B-366), which is
    // exactly what a cross-check needs: the logits compared here are the same
    // bytes at any count, and getting them takes less of the operator's day.
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
