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

pub(crate) const TOKENS: usize = 32;

#[allow(
    clippy::too_many_arguments,
    reason = "what a person asked, each of which the account names back"
)]
pub(crate) fn run(
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    turn: &mcf_serve::turn::Turn,
    image: Option<&Path>,
    started: mcf_serve::declared::Started,
) -> Response {
    run_where(
        crate::serve::socket_path(),
        model,
        prompt,
        limit,
        seed,
        engine,
        turn,
        image,
        started,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "what a person asked, each of which the account names back"
)]
pub(crate) fn run_where(
    socket: Option<std::path::PathBuf>,
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    turn: &mcf_serve::turn::Turn,
    image: Option<&Path>,
    started: mcf_serve::declared::Started,
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
                limit,
                seed,
                engine,
                turn,
                image,
                started,
            },
        );
    }
    if let Some(refusal) = needs_the_daemon(engine, turn, image, started) {
        return Response {
            text: refusal,
            served: false,
        };
    }

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

fn needs_the_daemon(
    engine: Option<&str>,
    turn: &mcf_serve::turn::Turn,
    image: Option<&Path>,
    started: mcf_serve::declared::Started,
) -> Option<String> {
    let without = "`mcf serve` starts one; without it this process runs MCF's own engine";
    if let Some(image) = image {
        return Some(format!(
            "mcf: a picture ({}) goes to the provisioned engine through the daemon, and none is \
             listening\n  {without}, which reads text only",
            image.display()
        ));
    }
    if started.asks_anything() {
        return Some(format!(
            "mcf: {} is the provisioned engine's to start, through the daemon, and none is \
             listening\n  {without}, which runs the weights as the file lays them out",
            started.said()
        ));
    }
    if turn.asks_anything() {
        return Some(format!(
            "mcf: {} goes to the model's own template, which the provisioned engine renders \
             through the daemon, and none is listening\n  {without}, which runs no template",
            turn.said()
        ));
    }
    engine.filter(|engine| *engine != "stand-in").map(|engine| {
        format!(
            "mcf: the {engine} engine runs through the daemon, and none is listening\n  \
                 {without} (D39)"
        )
    })
}

#[derive(Clone, Copy)]
struct Asked<'a> {
    limit: Option<usize>,
    seed: u64,
    engine: Option<&'a str>,
    turn: &'a mcf_serve::turn::Turn,
    image: Option<&'a Path>,
    started: mcf_serve::declared::Started,
}

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
        image,
        started,
    } = *asked;
    let image = match image.map(|image| image.canonicalize().map_err(|error| (image, error))) {
        Some(Err((image, error))) => {
            return Response {
                text: format!(
                    "mcf: the picture at {} could not be found\n  {error}",
                    image.display()
                ),
                served: false,
            };
        }
        Some(Ok(whole)) => Some(whole),
        None => None,
    };

    let patience = std::time::Duration::from_secs(3600);
    let _deadline = connection.set_read_timeout(Some(patience));
    let _writing = connection.set_write_timeout(Some(patience));
    let mut connection = connection;

    let request = Request::Generate {
        whose: mcf_record::content::Whose::User,
        model: path.display().to_string(),
        prompt: prompt.to_owned(),
        limit,
        seed,
        tokens: None,
        pieces: None,
        engine: engine.map(str::to_owned),
        pinned: false,
        turn: turn.asks_anything().then(|| turn.clone()),
        image: image.as_ref().map(|whole| whole.display().to_string()),
        started,
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
            Ok(Streamed::Progress {
                read,
                of,
                produced: made,
                seconds,
            }) => {
                eprintln!(
                    "  … the engine has read {read} of {of} identifiers and produced {made}, \
                     {seconds} s in"
                );
            }
            Ok(Streamed::Done(done)) => {
                account = Some(done);
                break;
            }
            Err(_) => break,
        }
    }
    let _newline = writeln!(out);

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
             {}{}{}{}{}{}{}",
            condition("path"),
            get("prompt_tokens"),
            get("tokens"),
            get("stopped").trim_matches('"'),
            condition("sampler"),
            condition("seed"),
            condition("engine"),
            socket.display(),
            condition("loaded"),
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
            shown_line(&account),
            beyond_the_load(&account),
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
                    "\n\x20 THE ENGINE DIED mid-answer; what arrived is above:\n\x20 {failure}"
                ),
                None => String::new(),
            },
        ),
        served,
    }
}

fn shown_line(account: &mcf_record::json::Value) -> String {
    let Some(shown) = account
        .get("conditions")
        .and_then(|conditions| conditions.get("shown"))
        .filter(|shown| !matches!(shown, mcf_record::json::Value::Null))
    else {
        return String::new();
    };
    let text = |value: Option<&mcf_record::json::Value>| {
        value
            .and_then(mcf_record::json::Value::as_text)
            .unwrap_or("?")
            .to_owned()
    };
    let picture = shown.get("picture");
    let bytes = picture
        .and_then(|picture| picture.get("bytes"))
        .and_then(mcf_record::json::Value::as_integer)
        .unwrap_or(0);
    let projector = text(shown.get("projector"));
    let projector = projector.rsplit('/').next().unwrap_or("?").to_owned();
    format!(
        "\x20 shown     {} ({bytes} bytes) through {projector}, placed {};\n\
         \x20           the turn went as {}\n",
        text(picture.and_then(|picture| picture.get("path"))),
        text(shown.get("placed")),
        text(shown.get("read_as")),
    )
}

fn beyond_the_load(account: &mcf_record::json::Value) -> String {
    let under = |key: &str| {
        account
            .get("conditions")
            .and_then(|conditions| conditions.get(key))
    };
    let started = under("started_with").map(mcf_serve::declared::Started::from_value);
    let asked = started
        .filter(mcf_serve::declared::Started::asks_anything)
        .map_or_else(String::new, |started| {
            format!("\x20 started   {}\n", started.said())
        });
    let left = under("declares")
        .map(mcf_serve::declared::Declared::from_value)
        .and_then(|declared| declared.not_started(started.unwrap_or_default()))
        .map_or_else(String::new, |left| format!("\x20 declares  {left}\n"));
    format!("{asked}{left}")
}

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
            "NOT closed: the budget ran out inside it, and no answer came — a thought is the \
             size of its question, and --limit allows more"
        }
    )
}

struct Said {
    text: String,
    tokens: usize,
    prompt_tokens: usize,
    stopped: Stopped,
    threads: String,
    mark: String,
    engine: String,
}

pub(crate) fn examined(path: &Path) -> Result<(), Failure> {
    let free = match Machine::read().memory.available {
        Attested::Known(available) => Some(available.0),
        Attested::Unknown => None,
    };
    mcf_serve::crosscheck::examined(path, free).map(|_| ())
}

pub(crate) fn fits_in_memory(file: &gguf::Model) -> Result<(), Failure> {
    match Machine::read().memory.available {
        Attested::Known(available) => file.fits_dequantized(available.0),
        Attested::Unknown => Ok(()),
    }
}

fn answer(bytes: &[u8], prompt: &str, limit: usize, seed: u64) -> Result<Said, Failure> {
    let file = gguf::parse(bytes)?;
    fits_in_memory(&file)?;
    mcf_standin::llama::covers(&file)?;
    let vocabulary = Vocabulary::read(&file)?;
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
            settings: Settings::Greedy,
            seed,
            stop: vocabulary.ending.into_iter().collect(),
        },
    )?;

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

pub(crate) fn resolve(named: &str) -> Result<Option<PathBuf>, Vec<PathBuf>> {
    let given = Path::new(named);
    if given.is_file() {
        return Ok(Some(given.to_path_buf()));
    }
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

fn refused(path: &Path, failure: &Failure) -> String {
    format!(
        "{}\n  MCF's own reader handles {}. A model it refuses may still run on a \
         provisioned engine — `mcf provision llama.cpp` builds one",
        crate::say::refusal(&format!("{} did not run", path.display()), failure),
        mcf_standin::llama::FAMILIES.join(", ")
    )
}

#[cfg(test)]
mod tests;
