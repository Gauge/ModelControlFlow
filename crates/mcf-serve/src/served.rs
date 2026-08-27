//! The provisioned engine driven as a *server* rather than as a completion
//! tool (B-376).
//!
//! F36 built the first provisioned engine as a subprocess per generation: a
//! command line in, text out, the exit classified by the stage it died in. It
//! runs the reference model and it is honest, and it cannot be probed. Two
//! things a probe needs from an engine are missing from that shape, and both
//! are properties of the *interface* rather than of llama.cpp:
//!
//!   - **A turn of token identifiers has nowhere to go.** A chat turn is a
//!     sequence of tokens (B-374), and a command line takes text. MCF refuses
//!     to parse control tokens out of prompt text (F26, rightly), so the turn
//!     cannot be spelled either. Every probe was therefore confined to MCF's
//!     own engine.
//!   - **It does not say why it stopped.** The tool prints text and exits,
//!     and *the model finished* and *the budget ran out* look identical from
//!     outside. F38 is the whole of why that matters: those two, plus a third
//!     the tool also cannot show, are the observation a chat-template probe is
//!     made of.
//!
//! The server has both. It accepts `"prompt": [1, 4093, …]` — identifiers, sent
//! as themselves — and answers with `stop_type`, `tokens_predicted` and
//! `tokens_evaluated`. Confirmed before any of this was written: the sixteen
//! identifiers MCF assembles for a small instruct model came back as `tokens_evaluated: 16`,
//! and the generation ended `stop_type: "eos"` after 250 tokens.
//!
//! **It listens on a Unix socket, not a port.** `--host` binds one when the
//! address ends in `.sock`. This is not a stylistic preference: four projects
//! share this machine (§XVII), a port is a machine-wide resource two of them
//! can collide over, and a socket under MCF's own runtime directory cannot be
//! reached by anything that has not been told where it is. Nothing listens on
//! the network.
//!
//! **The context is the model's own.** `-c 0` means *loaded from the model*,
//! so the window is a thing the file declares rather than a number MCF chose
//! and did not mention (§3.15). What that number turns out to be is B-055's
//! question, and this does not prejudge it.

use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::{self, Value};

use crate::adapters::ProvisionedLlama;

/// How many times the server is asked whether it is ready, before the start is
/// given up on.
///
/// A bounded wait in the thread that asked for the server, not a timer: §6.9
/// forbids background timers, and this is the caller waiting for the thing it
/// asked for. Loading a 16 GB model off a cold disk is the case this has to
/// cover, and F35 measured that as seconds rather than minutes.
pub const ATTEMPTS: usize = 600;

/// How long each attempt waits.
const BETWEEN: std::time::Duration = std::time::Duration::from_millis(100);

/// Why a generation ended, as the engine itself reports it.
///
/// The point of the whole module. `Other` carries the word rather than
/// collapsing to *something else*, because a stop reason MCF has not been
/// taught is exactly the thing A7 says to keep rather than to round off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// The model emitted its own end-of-turn token.
    Eos,
    /// The budget ran out.
    Limit,
    /// A stop word was reached.
    Word,
    /// Something this engine says that MCF has not been taught.
    Other(String),
}

impl Stop {
    /// The reason as the record spells it.
    #[must_use]
    pub fn written(&self) -> String {
        match self {
            Self::Eos => "stop_token".to_owned(),
            Self::Limit => "limit".to_owned(),
            Self::Word => "stop_word".to_owned(),
            Self::Other(said) => format!("engine_said_{said}"),
        }
    }
}

/// One completed generation, as the server accounts for it.
#[derive(Debug, Clone)]
pub struct Completed {
    /// What the model produced.
    pub text: String,
    /// How many tokens it produced.
    pub predicted: usize,
    /// How many tokens of prompt it read.
    ///
    /// Kept because it is the check that the turn arrived intact: MCF knows
    /// how many identifiers it sent, and an engine that read a different
    /// number read something else (D46).
    pub evaluated: usize,
    /// Why it stopped.
    pub stop: Stop,
}

/// A running `llama-server`, holding one model.
///
/// The model stays loaded between requests, which is the residency F36 left
/// open — there, every generation paid the load again, and a 16 GB model
/// cannot sensibly be probed that way.
#[derive(Debug)]
pub struct Served {
    child: Child,
    socket: PathBuf,
    /// The model it holds. A request for a different one needs a different
    /// server, and the caller has to be able to tell.
    pub model: PathBuf,
    /// The pin it was built from, for the conditions.
    pub commit: String,
}

impl Served {
    /// Starts a server holding one model, and waits for it to answer.
    ///
    /// # Errors
    ///
    /// `engine.spawn.not_found` if the binary is not in the prefix,
    /// `engine.spawn.refused` if it cannot be started, and
    /// `engine.hang.no_output` if it never begins listening.
    pub fn start(llama: &ProvisionedLlama, model: &Path, runtime: &Path) -> Result<Self, Failure> {
        Self::start_within(llama, model, runtime, ATTEMPTS)
    }

    /// The same, waiting a stated number of attempts.
    ///
    /// The bound is a parameter so that the laboratory can produce the *did
    /// not begin listening* failure in a scenario rather than MCF claiming a
    /// category nothing can reach (A13, B-010). A scenario that had to wait a
    /// minute to demonstrate a timeout would not be run.
    ///
    /// # Errors
    ///
    /// As [`Served::start`].
    pub fn start_within(
        llama: &ProvisionedLlama,
        model: &Path,
        runtime: &Path,
        attempts: usize,
    ) -> Result<Self, Failure> {
        let binary = llama.prefix.join("build").join("bin").join("llama-server");
        if !binary.exists() {
            return Err(Failure::new(
                Category::EngineSpawnNotFound,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "this provisioned prefix has no llama-server: it was built before MCF asked for \
                 one, and `mcf provision llama.cpp` again produces it",
            )
            .with_context("looked_for", binary.display().to_string()));
        }

        // The socket goes under MCF's own runtime directory, named for the
        // process that owns it, so a server left behind by a killed daemon is
        // identifiable rather than anonymous (A27, B58).
        let socket = runtime.join(format!("llama-{}.sock", std::process::id()));
        let _gone = std::fs::remove_file(&socket);
        if let Some(parent) = socket.parent() {
            let _made = std::fs::create_dir_all(parent);
        }

        let mut command = Command::new(&binary);
        command
            .arg("--model")
            .arg(model)
            .arg("--host")
            .arg(&socket)
            .arg("--ctx-size")
            .arg("0")
            .arg("-ngl")
            .arg("0")
            .arg("--no-webui")
            .arg("--no-warmup")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = command.spawn().map_err(|error| {
            Failure::new(
                Category::EngineSpawnRefused,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server could not be started",
            )
            .with_context("error", error.to_string())
        })?;

        let mut served = Self {
            child,
            socket,
            model: model.to_path_buf(),
            commit: llama.commit.clone(),
        };
        served.wait_until_listening(attempts)?;
        Ok(served)
    }

    /// Waits for the server to be *ready*, which is three conditions deep.
    ///
    /// The socket file appears before anything is listening; the listener
    /// accepts before the model is loaded; and a request made in between is
    /// answered — with an error, in a shape close enough to a real answer to
    /// be mistaken for one. The first version of this waited for the
    /// connection to be accepted and the probe's opening trial read a
    /// still-loading server as an engine that would not say why it stopped.
    /// So the wait is on `/health` saying `ok`, which is the server's own
    /// statement that it has a model.
    fn wait_until_listening(&mut self, attempts: usize) -> Result<(), Failure> {
        for _ in 0..attempts {
            // A child that has already exited will never listen, and waiting
            // the full span for it would report a hang where there was a
            // death (A2).
            if let Ok(Some(status)) = self.child.try_wait() {
                return Err(Failure::new(
                    Category::EngineExitImmediate,
                    Attribution::Machine,
                    Disposition::Aborted,
                    Subsystem::new("mcf-serve::served"),
                    "the provisioned server exited before it began listening",
                )
                .with_context("exit", status.to_string()));
            }
            if self.socket.exists()
                && self
                    .request("GET", "/health", None)
                    .is_ok_and(|answer| answer.contains("\"status\":\"ok\""))
            {
                return Ok(());
            }
            std::thread::sleep(BETWEEN);
        }
        Err(Failure::new(
            Category::EngineHangNoOutput,
            Attribution::Machine,
            Disposition::Aborted,
            Subsystem::new("mcf-serve::served"),
            "the provisioned server did not begin listening",
        )
        .with_context("socket", self.socket.display().to_string())
        .with_context(
            "waited",
            format!("{} ms", attempts as u128 * BETWEEN.as_millis()),
        ))
    }

    /// One greedy generation from a turn of token identifiers.
    ///
    /// # Errors
    ///
    /// `engine.protocol.malformed` for an answer MCF cannot read, and
    /// `engine.exit.midstream` if the connection ends before one arrives.
    pub fn complete(
        &self,
        tokens: &[usize],
        limit: usize,
        seed: u64,
    ) -> Result<Completed, Failure> {
        let identifiers = Value::List(
            tokens
                .iter()
                .map(|token| Value::Integer(i64::try_from(*token).unwrap_or(i64::MAX)))
                .collect(),
        );
        let body = Value::map([
            ("prompt", identifiers),
            (
                "n_predict",
                Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
            ),
            (
                "seed",
                Value::Integer(i64::try_from(seed).unwrap_or(i64::MAX)),
            ),
            ("temperature", Value::Integer(0)),
            // Every request starts from the same state, or a trial would be
            // measuring what the previous trial left behind (§3.12).
            ("cache_prompt", Value::Bool(false)),
        ])
        .to_line();

        interpret(&self.request("POST", "/completion", Some(&body))?)
    }

    /// The smallest HTTP a request needs.
    ///
    /// `Connection: close` so the body ends at end of stream and there is no
    /// chunked encoding or keep-alive framing to get wrong — B15 admits weight
    /// only against a stated cost, and a general HTTP client is weight this
    /// does not need to carry.
    fn request(&self, method: &str, path: &str, body: Option<&str>) -> Result<String, Failure> {
        let died = |what: &str, error: &std::io::Error| {
            Failure::new(
                Category::EngineExitMidstream,
                Attribution::Machine,
                Disposition::Aborted,
                Subsystem::new("mcf-serve::served"),
                what,
            )
            .with_context("error", error.to_string())
        };

        let mut connection = UnixStream::connect(&self.socket).map_err(|error| {
            died(
                "the provisioned server stopped accepting connections",
                &error,
            )
        })?;
        let body = body.unwrap_or("");
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        connection
            .write_all(request.as_bytes())
            .and_then(|()| connection.flush())
            .map_err(|error| died("the request could not be sent to the server", &error))?;

        let mut answer = Vec::new();
        connection
            .read_to_end(&mut answer)
            .map_err(|error| died("the server's answer ended early", &error))?;
        let answer = String::from_utf8_lossy(&answer).into_owned();
        answer.split_once("\r\n\r\n").map_or_else(
            || {
                Err(Failure::new(
                    Category::EngineProtocolMalformed,
                    Attribution::Machine,
                    Disposition::Aborted,
                    Subsystem::new("mcf-serve::served"),
                    "the provisioned server's answer had no body",
                )
                .with_context("engine_said", answer.chars().take(400).collect::<String>()))
            },
            |(_head, body)| Ok(body.to_owned()),
        )
    }
}

/// What the server said, read as a completion.
///
/// Its own function, taking the answer as text, so that the laboratory can
/// produce the *answer MCF cannot read* failure without needing a server that
/// misbehaves on demand (A13, B-010). Every classification this makes is the
/// one the daemon makes.
///
/// # Errors
///
/// `engine.protocol.malformed` for an answer that is not JSON, and for one
/// that is an error rather than a generation.
pub fn interpret(answer: &str) -> Result<Completed, Failure> {
    let malformed = |what: &str| {
        Failure::new(
            Category::EngineProtocolMalformed,
            Attribution::Machine,
            Disposition::Aborted,
            Subsystem::new("mcf-serve::served"),
            what,
        )
        .with_context("engine_said", answer.chars().take(400).collect::<String>())
    };
    let value = json::parse(answer).map_err(|error| {
        malformed("the provisioned server answered with something that is not JSON")
            .with_context("error", error.to_string())
    })?;

    // An error is not a short answer. The server reports one in a shape a
    // reader can mistake for a completion — no `stop_type`, no content — and
    // reading it as *the engine did not say why it stopped* would attribute
    // the server's problem to the model (A2).
    //
    // It is *not* `engine.protocol.malformed` either. That was the first
    // classification and it was wrong: an engine that answers "this request is
    // 2048 tokens and the context is 2048" has been understood perfectly, and
    // calling its answer unparseable blames the engine for a request that was
    // out of bounds. A refusal MCF asked for is a refusal, attributed to the
    // request (F42). Malformed is kept for an answer that genuinely cannot be
    // read.
    if let Some(said) = value.get("error") {
        let message = said
            .get("message")
            .and_then(Value::as_text)
            .unwrap_or("the engine did not say")
            .to_owned();
        return Err(Failure::new(
            Category::ConfigInvalid,
            Attribution::User,
            Disposition::Refused,
            Subsystem::new("mcf-serve::served"),
            "the provisioned server refused the request, in its own words",
        )
        .with_context("engine_said", message)
        .with_context(
            "engine_said_in_full",
            said.to_line().chars().take(400).collect::<String>(),
        ));
    }

    let number = |name: &str| -> usize {
        value
            .get(name)
            .and_then(Value::as_integer)
            .and_then(|found| usize::try_from(found).ok())
            .unwrap_or(0)
    };
    let stop = match value.get("stop_type").and_then(Value::as_text) {
        Some("eos") => Stop::Eos,
        Some("limit") => Stop::Limit,
        Some("word") => Stop::Word,
        Some(other) => Stop::Other(other.to_owned()),
        // No `stop_type` at all is not "it hit the limit": it is an engine
        // MCF has not been taught, and saying so is A7.
        None => Stop::Other("nothing".to_owned()),
    };
    Ok(Completed {
        text: value
            .get("content")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned(),
        predicted: number("tokens_predicted"),
        evaluated: number("tokens_evaluated"),
        stop,
    })
}

impl Drop for Served {
    /// The server goes when the daemon does, and takes its socket with it.
    ///
    /// A27: what MCF starts, MCF stops. A server left running holds a model in
    /// memory on a machine three other projects are sharing, which is the
    /// least neighbourly failure available here.
    fn drop(&mut self) {
        let _killed = self.child.kill();
        let _waited = self.child.wait();
        let _gone = std::fs::remove_file(&self.socket);
    }
}
