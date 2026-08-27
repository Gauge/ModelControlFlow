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

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::hardware::Machine;
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
pub(crate) fn run(
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };

    // A daemon that is listening serves the generation; this process runs it
    // only when nothing is (B-034, PR9). Which one did is part of the account,
    // because it is a condition: the same file through the same engine in
    // another process is another process's memory, cache and clock.
    let listening = crate::serve::socket_path().and_then(|socket| {
        std::os::unix::net::UnixStream::connect(&socket)
            .ok()
            .map(|c| (socket, c))
    });
    if let Some((socket, connection)) = listening {
        return served(
            connection,
            &socket,
            &path,
            prompt,
            limit.unwrap_or(TOKENS),
            seed,
            engine,
        );
    }
    if engine.is_some_and(|engine| engine != "stand-in") {
        return Response {
            text: format!(
                "mcf: the {} engine runs through the daemon, and none is listening\n  `mcf serve` \
                 starts one; without it this process runs MCF's own engine (D39)",
                engine.unwrap_or_default()
            ),
            served: false,
        };
    }

    // The directory first, from a bounded read: whether this model can run on
    // MCF's own engine is answerable from the header, and answering it after
    // reading sixteen gigabytes is a seventy-second refusal (B-372). Only here,
    // where MCF's own engine is the one that will run: a daemon decides for
    // itself, and a provisioned engine covers what it covers (B-032).
    if let Err(failure) = examined(&path) {
        return Response {
            text: refused(&path, &failure),
            served: false,
        };
    }

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

/// The generation through the daemon: tokens printed as they arrive, the
/// account printed when it comes.
///
/// The tokens go to the terminal *as they are read* — that is the whole point
/// of a stream (B-035, D24) — and the conditions block follows the last one,
/// so what a person sees is what `render` prints for an in-process run, with
/// one more line saying which process produced it.
#[allow(
    clippy::too_many_lines,
    reason = "the client side of one protocol exchange: send, stream, account. Splitting it \
              would put the three ways the stream can end in three places, and they are \
              one decision"
)]
fn served(
    connection: std::os::unix::net::UnixStream,
    socket: &Path,
    path: &Path,
    prompt: &str,
    limit: usize,
    seed: u64,
    engine: Option<&str>,
) -> Response {
    use std::io::{BufRead as _, BufReader, Write as _};

    use mcf_serve::control::{Request as Ask, Streamed};

    // Between tokens the stand-in can take a second per token on a
    // half-billion-parameter model; between the request and the first token
    // it loads the model. Both are bounded, and the bound is stated here.
    let patience = std::time::Duration::from_secs(600);
    let _deadline = connection.set_read_timeout(Some(patience));
    let _writing = connection.set_write_timeout(Some(patience));
    let mut connection = connection;

    let request = Ask::Generate {
        model: path.display().to_string(),
        prompt: prompt.to_owned(),
        limit,
        seed,
        engine: engine.map(str::to_owned),
    };
    if let Err(error) =
        writeln!(connection, "{}", request.to_line()).and_then(|()| connection.flush())
    {
        return Response {
            text: format!(
                "mcf: the daemon at {} would not take the request\n  {error}",
                socket.display()
            ),
            served: false,
        };
    }

    let mut out = std::io::stdout();
    let mut produced = 0_usize;
    let mut account: Option<mcf_record::json::Value> = None;
    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        match Streamed::read(line.trim_end()) {
            Ok(Streamed::Token { text, .. }) => {
                produced = produced.saturating_add(1);
                let _printed = write!(out, "{text}");
                let _flushed = out.flush();
            }
            Ok(Streamed::Done(done)) => {
                account = Some(done);
                break;
            }
            Err(_) => break,
        }
    }
    let _newline = writeln!(out);

    // The stream ended without its account: the daemon died, or the wire
    // did. What was received was received (A4), and the absence of the
    // account is said rather than filled in (A7).
    let Some(account) = account else {
        return Response {
            text: format!(
                "\n── the stream ended before its account ──────────────────────\n  \
                 {produced} token(s) were received from the daemon at {} and printed above;\n  \
                 the terminating line never came, so the conditions of this answer are\n  \
                 unknown here — the daemon's record has them if it lived to write them (A4, A26)",
                socket.display()
            ),
            served: false,
        };
    };

    // A failure with tokens before it is an engine that died mid-answer: what
    // arrived is kept and printed above, and the account says how it ended
    // (A4, B-033). A failure with none is a refusal.
    if let Some(failure) = account.get("failure")
        && account
            .get("tokens")
            .and_then(mcf_record::json::Value::as_integer)
            == Some(0)
    {
        return Response {
            text: format!(
                "mcf: {} did not run\n  the daemon at {} refused it:\n  {}",
                path.display(),
                socket.display(),
                failure.to_line()
            ),
            served: false,
        };
    }

    let get = |key: &str| {
        account
            .get(key)
            .map_or_else(|| "?".to_owned(), mcf_record::json::Value::to_line)
    };
    let conditions = account.get("conditions");
    let condition = |key: &str| {
        conditions.and_then(|c| c.get(key)).map_or_else(
            || "?".to_owned(),
            |v| v.as_text().map_or_else(|| v.to_line(), str::to_owned),
        )
    };
    let degraded = account
        .get("degraded")
        .and_then(mcf_record::json::Value::as_text);
    let died = account.get("failure").map(mcf_record::json::Value::to_line);
    let served = died.is_none();
    Response {
        text: format!(
            "\n── what produced it ─────────────────────────────────────────\n\
             \x20 model    {}\n\
             \x20 prompt   {} token(s)\n\
             \x20 produced {} token(s); stopped: {}\n\
             \x20 sampler  {}, seed {}\n\
             \x20 engine   {}\n\
             \x20 served   by the daemon at {}, model loaded {}\n\
             {}{}",
            condition("path"),
            get("prompt_tokens"),
            get("tokens"),
            get("stopped").trim_matches('"'),
            condition("sampler"),
            condition("seed"),
            condition("engine"),
            socket.display(),
            condition("loaded"),
            match degraded {
                Some(mark) => format!(
                    "\x20 MARKED   {mark}\n\x20 This is a behaviour answer and can never be a speed \
                     (B65, D31)."
                ),
                None => "\x20 a real engine: nothing here is marked degraded, and a timing taken \
                         under stated conditions would be a measurement (D39)"
                    .to_owned(),
            },
            match died {
                Some(failure) => format!(
                    "\n\x20 THE ENGINE DIED mid-answer; what arrived is above (A4):\n\x20 {failure}"
                ),
                None => String::new(),
            },
        ),
        served,
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
pub(crate) fn examined(path: &Path) -> Result<(), Failure> {
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
                Subsystem::new("mcf-cli::run"),
                "the model file could not be read",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string()));
        }
        match gguf::parse(&prefix) {
            Ok(file) => {
                mcf_standin::llama::covers(&file)?;
                return fits_in_memory(&file);
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
    Ok(())
}

/// Refuses a file the stand-in cannot hold, before a tensor is read (B-372).
///
/// The observation lives here and the arithmetic in the library: this reads
/// what the machine says is free, and [`gguf::Model::fits_dequantized`] is the
/// same judgement wherever it is asked. Where the machine's memory is unknown,
/// MCF proceeds — refusing on an unknown would turn A7's honesty about not
/// knowing into a limit nobody measured.
pub(crate) fn fits_in_memory(file: &gguf::Model) -> Result<(), Failure> {
    match Machine::read().memory.available {
        Attested::Known(available) => file.fits_dequantized(available.0),
        Attested::Unknown => Ok(()),
    }
}

/// Reads the model, runs it, and keeps the mark.
fn answer(bytes: &[u8], prompt: &str, limit: usize, seed: u64) -> Result<Said, Failure> {
    let file = gguf::parse(bytes)?;
    fits_in_memory(&file)?;
    // Asked before the vocabulary, because when both are unsupported the
    // architecture is what the operator needs to hear: a vocabulary MCF cannot
    // read is one component of a model it might otherwise run, and an
    // architecture it cannot read is the whole model. An embedding model
    // refused for its tokenizer sounds like a tokenizer problem.
    mcf_standin::llama::covers(&file)?;
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

/// Where a model is: a path, or something under one of the stores.
///
/// `Ok(None)` means nothing of that name is held anywhere. `Err` means it is
/// held in **more than one** store, and MCF will not choose for the operator:
/// two files under one name are two artifacts, possibly of different sizes and
/// certainly with two provenances, and picking the first silently would make
/// every measurement taken afterwards a measurement of whichever one MCF
/// happened to reach (§3.15, A1).
pub(crate) fn resolve(named: &str) -> Result<Option<PathBuf>, Vec<PathBuf>> {
    let given = Path::new(named);
    if given.is_file() {
        return Ok(Some(given.to_path_buf()));
    }
    // `owner/name:file`, the way a reference is written, and `owner/name/file`,
    // the way it sits on the disk. Both are things somebody will type.
    let relative = named.replace(':', "/");
    let found: Vec<PathBuf> = models::stores()
        .into_iter()
        .map(|root| root.join(&relative))
        .filter(|candidate| candidate.is_file())
        .collect();
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.into_iter().next()),
        _ => Err(found),
    }
}

/// What a surface says when a name is held in more than one store.
pub(crate) fn ambiguous(named: &str, found: &[PathBuf]) -> String {
    let mut lines = vec![format!(
        "mcf: {named} names {} files, in different stores:",
        found.len()
    )];
    for path in found {
        lines.push(format!("  {}", path.display()));
    }
    lines.push(
        "  name one of those paths. MCF will not choose: they are two artifacts with two \
         provenances, and a measurement against whichever one MCF reached first would be a \
         measurement nobody could reproduce (§3.15)"
            .to_owned(),
    );
    lines.join("\n")
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
        "{}\n  MCF's own engine reads GGUF and implements {}: a model it refuses is one a \
         vendored engine would take, and there is no vendored engine yet (D31, B-320)",
        crate::say::refusal(&format!("{} did not run", path.display()), failure),
        // Said rather than counted, so that the sentence cannot go stale the
        // way "one architecture" did once there were four.
        mcf_standin::llama::FAMILIES.join(", ")
    )
}

#[cfg(test)]
mod tests;
