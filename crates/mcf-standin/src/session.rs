use mcf_core::degradation::Degraded;
use mcf_core::engine::{Behaviour, Run, StandIn};
use mcf_core::failure::Result;

use crate::llama::{Cache, Loaded};
use crate::sample::{Rng, Settings};

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub prompt: Vec<usize>,
    pub limit: usize,
    pub settings: Settings,
    pub seed: u64,
    pub stop: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    pub tokens: Vec<usize>,
    pub stopped: Stopped,
    pub prompt_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    AtStopToken { token: usize },
    AtLimit,
    NothingToRead,
}

pub fn generate(
    model: &Loaded,
    build: &str,
    request: &Request,
) -> Result<Degraded<Behaviour<Generated>>> {
    generate_streaming(model, build, request, &mut |_token| {})
}

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

fn run_loop(
    model: &Loaded,
    request: &Request,
    on_token: &mut dyn FnMut(usize),
) -> Result<Generated> {
    let mut cache = Cache::for_model(&model.shape);
    let mut rng = Rng::seeded(request.seed);
    let mut logits = Vec::new();
    let mut position = 0;

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
