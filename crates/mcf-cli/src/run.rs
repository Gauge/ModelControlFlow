//! `mcf run`: a model on this machine answers something (B-040, D31, B65).
//!
//! **What this is, and what it is emphatically not.** It is the whole path —
//! model file, vocabulary, forward pass, sampler, tokens, text — driven by
//! MCF's own stand-in engine, which D31 put there so that a model no vendored
//! engine will run still runs, *marked*. It is not a benchmark and cannot
//! become one: B65 forbids a stand-in from producing a speed, the type refuses
//! to hand over a bare result, and this surface prints the mark beside every
//! answer rather than under it.
//!
//! **Why it exists before a vendored engine does.** §VI asks that having a
//! model and using a model be one command apart, and B-040 is that command.
//! What MCF can honestly do today is the behaviour half: *what does this model
//! say*, on this machine, from these weights, with a stated seed. What it
//! cannot do is tell you how fast — and saying which half you are getting is
//! the difference between an instrument and a demo.
//!
//! **The conditions travel with the answer.** A generation is a thing somebody
//! could try to reproduce, so the surface prints what would be needed to: the
//! model's digest, the sampler, the seed, the token budget, and the engine that
//! produced it. §3.4's habit at the smallest scale.

use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::Failure;
use mcf_standin::gguf;
use mcf_standin::llama::load;
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, Stopped, generate};
use mcf_standin::tokenizer::Vocabulary;

use crate::Response;
use crate::models;

/// How many tokens a generation produces when nobody says.
///
/// A budget in tokens rather than in seconds, which is B49's shape: a stand-in
/// is slow by design, and a limit in time would make the answer a property of
/// the machine rather than of the model.
pub(crate) const TOKENS: usize = 32;

/// Runs a model and prints what it said.
pub(crate) fn run(model: &str, prompt: &str, limit: Option<usize>, seed: u64) -> Response {
    let Some(path) = resolve(model) else {
        return Response {
            text: format!(
                "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                 holding; a path to a file works too"
            ),
            served: false,
        };
    };

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Response {
                text: format!("mcf: {} could not be read\n  {error}", path.display()),
                served: false,
            };
        }
    };

    let answer = answer(&bytes, prompt, limit.unwrap_or(TOKENS), seed);
    match answer {
        Ok(said) => Response {
            text: render(&path, prompt, seed, &said),
            served: true,
        },
        Err(failure) => Response {
            text: refused(&path, &failure),
            served: false,
        },
    }
}

/// What a run produced, with everything a reader needs to judge it.
struct Said {
    text: String,
    tokens: usize,
    prompt_tokens: usize,
    stopped: Stopped,
    /// What the engine's own mark says was lost, rendered.
    mark: String,
    engine: String,
}

/// Reads the model, runs it, and keeps the mark.
fn answer(bytes: &[u8], prompt: &str, limit: usize, seed: u64) -> Result<Said, Failure> {
    let file = gguf::parse(bytes)?;
    let vocabulary = Vocabulary::read(&file)?;
    let model = load(&file, bytes)?;

    let prompt_tokens = vocabulary.encode(prompt, true)?;
    let build = BuildIdentity::current().version.to_owned();
    let generated = generate(
        &model,
        &build,
        &Request {
            prompt: prompt_tokens.clone(),
            limit,
            // Greedy, and stated: a sampler MCF chose without saying would make
            // two runs of one model differ for a reason nobody recorded (D19,
            // §3.15).
            settings: Settings::Greedy,
            seed,
            stop: Vec::new(),
        },
    )?;

    // The mark cannot be unwrapped away: `Degraded` hands back the value only
    // with its degradation, and this is where both are turned into something a
    // person reads (A5, B-008).
    let degradation = generated.degradation().to_string();
    let behaviour = generated.value();
    let produced = behaviour.observed();

    Ok(Said {
        text: vocabulary.decode(&produced.tokens),
        tokens: produced.tokens.len(),
        prompt_tokens: produced.prompt_length,
        stopped: produced.stopped,
        mark: degradation,
        engine: format!("MCF's own stand-in, build {build}"),
    })
}

/// Where a model is: a path, or something under the store.
pub(crate) fn resolve(named: &str) -> Option<PathBuf> {
    let given = Path::new(named);
    if given.is_file() {
        return Some(given.to_path_buf());
    }
    let root = models::default_root()?;
    // `owner/name:file`, the way a reference is written, and `owner/name/file`,
    // the way it sits on the disk. Both are things somebody will type.
    let under = root.join(named.replace(':', "/"));
    under.is_file().then_some(under)
}

/// What a reader is told, answer and conditions together.
fn render(path: &Path, prompt: &str, seed: u64, said: &Said) -> String {
    let stopped = match said.stopped {
        Stopped::AtStopToken { token } => format!("the model stopped, at token {token}"),
        Stopped::AtLimit => "the token budget ran out".to_owned(),
        Stopped::NothingToRead => "there was nothing to read".to_owned(),
    };

    format!(
        "{}\n\n\
         ── what produced it ─────────────────────────────────────────\n\
         \x20 model    {}\n\
         \x20 prompt   {} token(s)\n\
         \x20 produced {} token(s); {stopped}\n\
         \x20 sampler  greedy, seed {seed}\n\
         \x20 engine   {}\n\
         \x20 MARKED   {}\n\
         \x20 This is a behaviour answer and can never be a speed (B65, D31):\n\
         \x20 MCF's stand-in is written to be read rather than to be fast, and a\n\
         \x20 timing taken from it would measure the stand-in.",
        if said.text.is_empty() {
            "(the model produced no text)"
        } else {
            said.text.trim()
        },
        path.display(),
        said.prompt_tokens,
        said.tokens,
        said.engine,
        said.mark,
    )
    .replace("{prompt}", prompt)
}

/// A refusal, said the shared way, plus the sentence that is this command's
/// own: MCF's reader is strict because there is nothing else to fall back to.
fn refused(path: &Path, failure: &Failure) -> String {
    format!(
        "{}\n  MCF's stand-in implements one architecture and reads GGUF: a model it refuses \
         is one a vendored engine would take, and there is no vendored engine yet (D31, B-320)",
        crate::say::refusal(&format!("{} did not run", path.display()), failure)
    )
}

#[cfg(test)]
mod tests;
