//! Generating tokens, and the mark every result carries (A5, D31, B65).
//!
//! [`crate::llama`] produces logits and [`crate::sample`] chooses from them.
//! This is the loop that puts them together — and, more importantly, the place
//! where what comes out is *typed as a stand-in result*.
//!
//! **Why that matters more than the loop.** A5: a result produced under reduced
//! capability is always accompanied by an explicit statement of what was lost,
//! and an unmarked degraded result is a corrupted one. The reduced capability
//! here is *the engine the artifact was meant to run on*, and the mark is
//! [`mcf_core::engine::Run<StandIn>::mark`], which produces a `Degraded` value
//! with no way out that drops it.
//!
//! **What a stand-in may answer.** Behaviour-class questions only (B31, B65):
//! did the loop terminate, did the format hold, did the call parse, what did it
//! say. Never how fast, and the type system rather than this paragraph is what
//! enforces it — there is no `timing` method on a stand-in run.
//!
//! **A run states its own conditions.** The seed, the settings and the stopping
//! rule are values a caller passed, and they travel in the outcome: D18 makes
//! sampling identity and D19 makes the seed a condition, so a generation that
//! did not say what it sampled under would be a result nobody can compare or
//! reproduce.

use mcf_core::degradation::Degraded;
use mcf_core::engine::{Behaviour, Run, StandIn};
use mcf_core::failure::Result;

use crate::llama::{Cache, Loaded};
use crate::sample::{Rng, Settings};

/// What a generation was asked to do.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// The tokens to read before generating, which is the prompt already turned
    /// into identifiers. The tokenizer that produces them is not in this crate
    /// yet, and a caller that has one passes its output here.
    pub prompt: Vec<usize>,
    /// How many tokens to generate at most.
    ///
    /// A budget in *tokens* rather than in time, which is B49's shape: a task
    /// that failed for want of a wall clock is a measurement of the machine,
    /// and a stand-in is slow by design.
    pub limit: usize,
    /// How the next token is chosen.
    pub settings: Settings,
    /// The seed, which is a condition of the result (D19).
    pub seed: u64,
    /// Tokens that end the generation when produced.
    pub stop: Vec<usize>,
}

/// What a generation produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// The tokens produced, in order, not including the prompt.
    pub tokens: Vec<usize>,
    /// Why it stopped.
    pub stopped: Stopped,
    /// How many tokens were read before generation began.
    pub prompt_length: usize,
}

/// Why a generation ended.
///
/// A named outcome rather than a length a caller has to interpret: *the model
/// stopped* and *MCF stopped it* are different facts about a run, and a
/// behaviour-class laboratory is asking exactly that question (B31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// The model produced a token the request named as an ending.
    AtStopToken {
        /// Which one.
        token: usize,
    },
    /// The token budget ran out.
    AtLimit,
    /// There was nothing to generate: an empty prompt with nothing to read.
    NothingToRead,
}

/// Runs a generation and marks the result.
///
/// The return type is the point. A `Degraded<Behaviour<Generated>>` cannot be
/// unwrapped into a bare `Generated` — `Degraded` has no `into_inner` — so a
/// caller that wants the tokens carries the mark to wherever it renders them
/// (A5, B-008).
///
/// # Errors
///
/// Whatever the forward pass fails with: a token outside the vocabulary, a
/// tensor the model names and does not have.
pub fn generate(
    model: &Loaded,
    build: &str,
    request: &Request,
) -> Result<Degraded<Behaviour<Generated>>> {
    generate_streaming(model, build, request, &mut |_token| {})
}

/// The same, telling the caller each token as it is produced.
///
/// What a serving surface needs (B-034, PR9): a first token is observable only
/// if it is handed over before the last one exists. The mark is applied to the
/// whole at the end exactly as for [`generate`] — a token streamed early is
/// still a token from the stand-in, and the terminating line is where the
/// caller is told so.
///
/// # Errors
///
/// As [`generate`].
pub fn generate_streaming(
    model: &Loaded,
    build: &str,
    request: &Request,
    on_token: &mut dyn FnMut(usize),
) -> Result<Degraded<Behaviour<Generated>>> {
    let run: Run<StandIn> = Run::at_build(build);
    let generated = run_loop(model, request, on_token)?;
    Ok(run.mark(run.behaviour(generated)))
}

/// The loop itself, separated from the marking so that the mark cannot be
/// forgotten by a future caller reaching for the loop.
fn run_loop(
    model: &Loaded,
    request: &Request,
    on_token: &mut dyn FnMut(usize),
) -> Result<Generated> {
    let mut cache = Cache::for_model(&model.shape);
    let mut rng = Rng::seeded(request.seed);
    let mut logits = Vec::new();
    let mut position = 0;

    // Read the prompt. Every token is run through the model so the cache holds
    // it; only the last one's logits decide the first generated token.
    for token in &request.prompt {
        logits = model.forward(*token, position, &mut cache)?;
        position = position.saturating_add(1);
    }
    if logits.is_empty() {
        return Ok(Generated {
            tokens: Vec::new(),
            stopped: Stopped::NothingToRead,
            prompt_length: request.prompt.len(),
        });
    }

    let mut tokens = Vec::new();
    let mut stopped = Stopped::AtLimit;
    for _ in 0..request.limit {
        let Some(next) = crate::sample::next(&logits, request.settings, &mut rng) else {
            break;
        };
        if request.stop.contains(&next) {
            stopped = Stopped::AtStopToken { token: next };
            break;
        }
        tokens.push(next);
        on_token(next);
        logits = model.forward(next, position, &mut cache)?;
        position = position.saturating_add(1);
    }

    Ok(Generated {
        tokens,
        stopped,
        prompt_length: request.prompt.len(),
    })
}

#[cfg(test)]
mod tests;
