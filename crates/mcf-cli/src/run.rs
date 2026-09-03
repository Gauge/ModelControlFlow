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
use mcf_core::failure::Failure;
use mcf_core::hardware::Machine;
use mcf_standin::gguf;
use mcf_standin::llama::load;
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, Stopped, generate};
use mcf_standin::threads::Threads;
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
    turn: &mcf_serve::turn::Turn,
) -> Response {
    run_where(
        crate::serve::socket_path(),
        model,
        prompt,
        limit,
        seed,
        engine,
        turn,
    )
}

/// The same, told where a daemon would be.
///
/// Where the daemon is, is an *input* rather than something looked up in the
/// middle. It was ambient, and a test asserting on the refusal MCF gives for
/// an unreadable file therefore reported on whether a daemon happened to be
/// running on the machine — passing alone and failing beside one, for reasons
/// nothing in the test could see (F46, B-378). §3.12 does not allow a suite
/// whose answer depends on the state it found.
///
/// `None` means *no daemon*, which is both what a machine with no runtime
/// directory gives and what a test wants to say.
pub(crate) fn run_where(
    socket: Option<std::path::PathBuf>,
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    turn: &mcf_serve::turn::Turn,
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
    let listening = socket.and_then(|socket| {
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
            &Asked {
                // Not `unwrap_or(TOKENS)`: the daemon may have a budget
                // somebody derived for this model, and it can only use it
                // if it can tell a caller who said nothing from one who
                // said thirty-two (D43).
                limit,
                seed,
                engine,
                turn,
            },
        );
    }
    // A switch of the template is thrown by the engine that renders it, and
    // MCF's own engine renders none: said here rather than run under no
    // switch (A2, §3.15).
    if turn.asks_anything() {
        return Response {
            text: format!(
                "mcf: {} goes to the model's own template, which the provisioned engine \
                 renders through the daemon, and none is listening\n  `mcf serve` starts one; \
                 without it this process runs MCF's own engine, which runs no template",
                turn.said()
            ),
            served: false,
        };
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

/// What a person asked of the run, beside the prompt: the conditions the
/// account will name back.
#[derive(Clone, Copy)]
struct Asked<'a> {
    limit: Option<usize>,
    seed: u64,
    engine: Option<&'a str>,
    turn: &'a mcf_serve::turn::Turn,
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
    asked: &Asked<'_>,
) -> Response {
    use std::io::{BufRead as _, BufReader, Write as _};

    use mcf_serve::control::{Request, Streamed};

    let Asked {
        limit,
        seed,
        engine,
        turn,
    } = *asked;

    // Between tokens the stand-in can take a second per token on a
    // half-billion-parameter model; between the request and the first token
    // it loads the model. Both are bounded, and the bound is stated here.
    let patience = std::time::Duration::from_secs(600);
    let _deadline = connection.set_read_timeout(Some(patience));
    let _writing = connection.set_write_timeout(Some(patience));
    let mut connection = connection;

    let request = Request::Generate {
        // A person typed this prompt: their text, and the model's answer to
        // it. The category §6.8 protects (B-146).
        whose: mcf_record::content::Whose::User,
        model: path.display().to_string(),
        prompt: prompt.to_owned(),
        limit,
        seed,
        tokens: None,
        engine: engine.map(str::to_owned),
        // A ceiling: a person asking a model a question wants its answer,
        // which ends where the model ends it.
        pinned: false,
        turn: turn.asks_anything().then(|| turn.clone()),
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
             {}{}{}{}{}",
            condition("path"),
            get("prompt_tokens"),
            get("tokens"),
            get("stopped").trim_matches('"'),
            condition("sampler"),
            condition("seed"),
            condition("engine"),
            socket.display(),
            condition("loaded"),
            // MCF addressing a model other than plainly must never be
            // something a reader has to go looking for (§3.15, D43). The
            // account carries it either way; this is where a person sees it.
            match account
                .get("conditions")
                .and_then(|conditions| conditions.get("addressed_as"))
                .and_then(mcf_record::json::Value::as_text)
            {
                Some(how) => format!("\x20 addressed {how}\n"),
                None => String::new(),
            },
            match account
                .get("conditions")
                .and_then(|conditions| conditions.get("budget_from"))
                .and_then(mcf_record::json::Value::as_text)
            {
                Some(why) => format!("\x20 budget    {why}\n"),
                None => String::new(),
            },
            before_the_answer(&account),
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

/// What the model spent before its answer, as a line, where it spent
/// anything: how many tokens inside which marker, who opened it, and whether
/// it closed — a turn that did not close was cut by the budget, and what is
/// printed above is all of it and none of an answer (F106).
fn before_the_answer(account: &mcf_record::json::Value) -> String {
    let Some(before) = account.get("before_the_answer") else {
        return String::new();
    };
    let text = |key: &str| before.get(key).and_then(mcf_record::json::Value::as_text);
    let (Some(inside), Some(opened_by), Some(tokens)) = (
        text("inside"),
        text("opened_by"),
        before
            .get("tokens")
            .and_then(mcf_record::json::Value::as_integer),
    ) else {
        return String::new();
    };
    let closed = before
        .get("closed")
        .and_then(mcf_record::json::Value::as_bool)
        .unwrap_or(false);
    format!(
        "\x20 before   the answer, {tokens} token(s) inside {inside}, opened by {opened_by}; {}\n",
        if closed {
            "closed, and the answer followed"
        } else {
            "NOT closed: the budget ran out inside it, and no answer came"
        }
    )
}

/// What a run produced, with everything a reader needs to judge it.
struct Said {
    text: String,
    tokens: usize,
    prompt_tokens: usize,
    stopped: Stopped,
    /// How many processors the engine divided its work across, and whose number
    /// that was.
    ///
    /// A condition of the run's *cost* and never of its answer: the same input
    /// gives the same bytes at any count (B-366). It is printed because a
    /// reader comparing two runs' durations would otherwise have no way to know
    /// the machines differed in how much of themselves they gave.
    threads: String,
    /// What the engine's own mark says was lost, rendered.
    mark: String,
    engine: String,
}

/// Reads a file's directory from a bounded prefix and refuses early what can
/// be refused early: an architecture MCF has not been taught, and a model that
/// cannot fit dequantized (B-372).
///
/// The reading and the arithmetic are [`mcf_serve::crosscheck::examined`],
/// which the daemon's cross-check applies too (B-072); what this adds is the
/// observation — what the machine says is free — which B4 keeps out of the
/// library and at the surface.
pub(crate) fn examined(path: &Path) -> Result<(), Failure> {
    let free = match Machine::read().memory.available {
        Attested::Known(available) => Some(available.0),
        Attested::Unknown => None,
    };
    mcf_serve::crosscheck::examined(path, free).map(|_| ())
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
    // Every processor the machine reports, and the engine spends only as many
    // of them per product as that product's size earns (B-366, F99). The answer
    // is the same bytes at any count — that is the property the partition was
    // built around — so this changes what the run costs and nothing about what
    // it says.
    let threads = Threads::what_the_machine_reports();
    let model = load(&file, bytes)?.across(threads);

    let prompt_tokens = vocabulary.encode(prompt, true)?;
    let build = mcf_core::build_identity::identifier();
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
            // The model's own end of text, which the file states and MCF was
            // reading and never using: without it a generation always runs to
            // the budget, and *the model finished* is unobservable — which is
            // what the chat-template probe found first (F37).
            stop: vocabulary.ending.into_iter().collect(),
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
        threads: model.threads().describe(),
        mark: degradation,
        engine: mcf_core::build_identity::stand_in_engine(),
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
         \x20 threads  {}\n\
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
        said.threads,
        said.mark,
    )
    .replace("{prompt}", prompt)
}

/// A refusal, said the shared way, plus the sentence that is this command's
/// own: MCF's reader is strict because there is nothing else to fall back to.
fn refused(path: &Path, failure: &Failure) -> String {
    format!(
        "{}\n  MCF's own reader handles {}. A model it refuses may still run on a \
         provisioned engine — `mcf provision llama.cpp` builds one",
        crate::say::refusal(&format!("{} did not run", path.display()), failure),
        // Said rather than counted, so that the sentence cannot go stale the
        // way "one architecture" did once there were four.
        mcf_standin::llama::FAMILIES.join(", ")
    )
}

#[cfg(test)]
mod tests;
