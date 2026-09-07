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

use std::io::{BufRead as _, Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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
    /// The identifiers it produced, where it was asked for them.
    ///
    /// Text cannot be compared between two engines once their generations
    /// part — they are writing different sentences by then — so a cross-check
    /// needs the tokens themselves to feed the other engine (B-362, F40).
    pub produced: Vec<usize>,
}

impl Completed {
    /// The identifiers that are the model's words: what it produced, less the
    /// end-of-turn token where it stopped on one.
    ///
    /// The server lists that token among `tokens` — it was sampled — but it
    /// is the model ending its turn, not saying anything, and the completion
    /// tool's path holds its `[end of text]` back for the same reason (F142).
    /// The server's `content` is not these words either: it leaves every
    /// marker out, so a `</think>` the model wrote — the one token that says
    /// where its answer began (B-451) — is not in it.
    #[must_use]
    pub fn words(&self) -> &[usize] {
        match (&self.stop, self.produced.split_last()) {
            (Stop::Eos, Some((_, before))) => before,
            _ => &self.produced,
        }
    }
}

/// Whether a hosted engine is answering yet.
///
/// One line of HTTP over a loopback socket rather than a client: what is being
/// asked is *are you up*, and a client for that would be a client MCF
/// maintains for one question.
fn ready_on(port: u16) -> bool {
    // `200` and a body that says so. A `503 Loading model` is the engine
    // answering that it is not ready, which is a different thing from up.
    got_on(port, "/health")
        .is_some_and(|said| said.contains("200 OK") && said.contains("\"status\":\"ok\""))
}

/// The engine's own counters, as it publishes them on `/metrics` when
/// started with them on: tokens prompted and predicted, the rates, the
/// cache in use, requests in hand. Read on request and never on a timer
/// (B4). `None` where the port does not answer.
#[must_use]
pub fn metrics_on(port: u16) -> Option<String> {
    let said = got_on(port, "/metrics")?;
    let (_, body) = said.split_once("\r\n\r\n")?;
    Some(body.to_owned())
}

/// The engine's own counters over whichever way it is reached: the
/// daemon's own server for a run answers on a socket of its own, and what
/// it is doing is read off it the same way a hosted one's is (B-573).
#[must_use]
pub fn metrics_via(reach: &Reach) -> Option<String> {
    let said = match reach {
        // A keyed server refuses its counters to a reader without the key,
        // the same as any other request (B-580).
        Reach::Port { port, key } => got_on_with(*port, key.as_deref(), "/metrics")?,
        Reach::Socket(socket) => got_via(socket, "/metrics")?,
    };
    let (_, body) = said.split_once("\r\n\r\n")?;
    Some(body.to_owned())
}

/// One GET over the engine's socket, bounded as [`got_on`] is.
fn got_via(socket: &Path, path: &str) -> Option<String> {
    use std::io::{Read as _, Write as _};
    let Ok(mut connection) = UnixStream::connect(socket) else {
        return None;
    };
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    if write!(
        connection,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .and_then(|()| connection.flush())
    .is_err()
    {
        return None;
    }
    let mut said = String::new();
    let mut held = [0_u8; 4096];
    while let Ok(read) = connection.read(&mut held) {
        if read == 0 {
            break;
        }
        said.push_str(&String::from_utf8_lossy(
            held.get(..read).unwrap_or_default(),
        ));
        if said.len() > 65_536 {
            break;
        }
    }
    Some(said)
}

/// One GET on the engine's port, bounded: the status line and a short body.
fn got_on(port: u16, path: &str) -> Option<String> {
    got_on_with(port, None, path)
}

/// [`got_on`] presenting the key the server wants, where it wants one.
fn got_on_with(port: u16, key: Option<&str>, path: &str) -> Option<String> {
    use std::io::{Read as _, Write as _};
    let Ok(mut connection) = std::net::TcpStream::connect((crate::hosting::LOOPBACK, port)) else {
        return None;
    };
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    let bearer = key.map_or_else(String::new, |key| {
        format!("Authorization: Bearer {key}\r\n")
    });
    if write!(
        connection,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\n{bearer}Connection: close\r\n\r\n"
    )
    .and_then(|()| connection.flush())
    .is_err()
    {
        return None;
    }
    let mut said = String::new();
    // Bounded: what is wanted is the first line and a short body, and a server
    // that streamed forever must not become a read that never ends.
    let mut held = [0_u8; 4096];
    while let Ok(read) = connection.read(&mut held) {
        if read == 0 {
            break;
        }
        said.push_str(&String::from_utf8_lossy(
            held.get(..read).unwrap_or_default(),
        ));
        if said.len() > 65_536 {
            break;
        }
    }
    Some(said)
}

/// How far the engine has got with the request in flight, published for
/// anybody asking while it runs (D48, B-460).
///
/// Read off the server's own slot: how many identifiers of the turn it has
/// read, how many the turn holds, and how many it has produced. Zero before
/// the first reading, and `seen` says whether there has been one — a zero
/// that means *not looked yet* is not a zero that means *nothing read* (A7).
#[derive(Debug, Default)]
pub struct Progress {
    /// Whether the slot has been read at all.
    pub seen: AtomicBool,
    /// Identifiers of the turn the engine has read so far.
    pub read: AtomicU64,
    /// Identifiers the turn holds.
    pub of: AtomicU64,
    /// Identifiers the engine has produced so far.
    pub produced: AtomicU64,
}

impl Progress {
    /// The reading as a value for a page.
    #[must_use]
    pub fn to_value(&self) -> Value {
        if !self.seen.load(Ordering::Relaxed) {
            return Value::Null;
        }
        let figure = |held: &AtomicU64| {
            Value::Integer(i64::try_from(held.load(Ordering::Relaxed)).unwrap_or(i64::MAX))
        };
        Value::map([
            ("read", figure(&self.read)),
            ("of", figure(&self.of)),
            ("produced", figure(&self.produced)),
        ])
    }

    fn take(&self, read: u64, of: u64, produced: u64) {
        self.read.store(read, Ordering::Relaxed);
        self.of.store(of, Ordering::Relaxed);
        self.produced.store(produced, Ordering::Relaxed);
        self.seen.store(true, Ordering::Relaxed);
    }
}

/// Who is waiting for the engine's answer, so that nobody waits for nothing
/// and nothing runs for nobody (D48).
///
/// A request the engine takes hours over — the usable-context probe asked a
/// 27B model for 262,143 identifiers on a processor — ran on for hours after
/// its client had given up, with every request behind it queued and the
/// daemon unable to say so. The three things here are the three parts of
/// the answer: the client's connection is watched and the engine's request
/// closed when the client goes (B-459), which the engine takes as the
/// cancellation it is; the daemon's stop closes every request in flight the
/// same way; and how far the engine has read is published as it goes, both
/// to the client as lines and to the daemon for its status (B-458, B-460).
#[derive(Debug, Clone, Copy, Default)]
pub struct Waiting<'a> {
    /// The connection that asked. Read to its end, the client has gone.
    pub client: Option<&'a UnixStream>,
    /// Where progress lines go, where the client reads a stream that has
    /// room for them.
    pub told: Option<&'a UnixStream>,
    /// Raised when the daemon is stopping.
    pub stopping: Option<&'a AtomicBool>,
    /// Where the engine's progress is published.
    pub progress: Option<&'a Progress>,
}

impl Waiting<'_> {
    /// Nobody in particular: the request is waited for as long as it takes,
    /// which is how a request MCF makes for itself is waited for.
    pub const NOBODY: Waiting<'static> = Waiting {
        client: None,
        told: None,
        stopping: None,
        progress: None,
    };

    fn watches_anything(&self) -> bool {
        self.client.is_some() || self.stopping.is_some() || self.progress.is_some()
    }
}

/// Why a request in flight was closed before the engine answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closed {
    /// The client that asked went away.
    ClientLeft,
    /// The daemon was asked to stop.
    Stopping,
}

/// How often the client's connection and the engine's slot are looked at
/// while a request is in flight.
const GLANCE: std::time::Duration = std::time::Duration::from_millis(250);

/// How long between progress lines to a client, and before the first: a
/// turn the engine answers inside this many seconds is never told how it is
/// going, because it is already done.
const TOLD_EVERY: std::time::Duration = std::time::Duration::from_secs(10);

/// Watches over one request in flight, from beside the thread that made it.
///
/// Not a timer in §6.9's sense: it lives exactly as long as the request the
/// caller is waiting on and is the caller's own patience made concrete, and
/// it ends when the answer arrives or the request is closed. Looks four
/// times a second at whether the client is still there and whether the
/// daemon is stopping, and at the engine's slot for how far it has read;
/// tells the client every ten seconds where the engine has got to; and
/// closes the engine's request when nobody is waiting for it any more.
fn watched(
    reach: &Reach,
    request: &Link,
    waiting: Waiting<'_>,
    done: &AtomicBool,
) -> Option<Closed> {
    let client = waiting.client.and_then(|client| client.try_clone().ok());
    if let Some(client) = &client {
        let _timeout = client.set_read_timeout(Some(GLANCE));
    }
    let began = std::time::Instant::now();
    let mut last_told: Option<std::time::Instant> = None;
    let mut byte = [0_u8; 1];
    while !done.load(Ordering::Acquire) {
        let gone = match client.as_ref().map(|client| (&*client).read(&mut byte)) {
            Some(Ok(0)) => true,
            Some(Err(error)) => !matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            Some(Ok(_)) | None => {
                if client.is_none() {
                    std::thread::sleep(GLANCE);
                }
                false
            }
        };
        let closed = if gone {
            Some(Closed::ClientLeft)
        } else if waiting
            .stopping
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Some(Closed::Stopping)
        } else {
            None
        };
        if let Some(closed) = closed {
            request.shutdown();
            return Some(closed);
        }
        if waiting.progress.is_none() && waiting.told.is_none() {
            continue;
        }
        let due = last_told.unwrap_or(began).elapsed() >= TOLD_EVERY;
        if !due {
            continue;
        }
        last_told = Some(std::time::Instant::now());
        if let Some((read, of, produced)) = slot_progress(reach) {
            if let Some(progress) = waiting.progress {
                progress.take(read, of, produced);
            }
            if let Some(mut told) = waiting.told
                && !done.load(Ordering::Acquire)
            {
                let line = crate::control::Streamed::Progress {
                    read,
                    of,
                    produced,
                    seconds: began.elapsed().as_secs(),
                }
                .to_line();
                let _written = writeln!(told, "{line}").and_then(|()| told.flush());
            }
        }
    }
    None
}

/// The processing slot's reading: identifiers read, of how many, and how
/// many produced. `None` where no slot is processing or the answer could
/// not be read.
fn slot_progress(reach: &Reach) -> Option<(u64, u64, u64)> {
    let answer = plain_request(reach, "GET", "/slots", None).ok()?;
    let slots = json::parse(&answer).ok()?;
    let Value::List(slots) = slots else {
        return None;
    };
    let figure = |slot: &Value, key: &str| {
        slot.get(key)
            .and_then(Value::as_integer)
            .and_then(|figure| u64::try_from(figure).ok())
    };
    slots
        .iter()
        .find(|slot| {
            slot.get("is_processing")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .map(|slot| {
            let produced = slot
                .get("next_token")
                .and_then(|next| match next {
                    Value::List(items) => items.first(),
                    other => Some(other),
                })
                .and_then(|next| figure(next, "n_decoded"))
                .unwrap_or(0);
            let read = figure(slot, "n_prompt_tokens_processed").unwrap_or(0);
            // The slot's `n_prompt_tokens` is what its context holds, which
            // is the prompt while the prompt is being read and the prompt
            // plus what has been produced so far once it is answering (the
            // pinned server pushes each sampled identifier onto the same
            // list). The prompt's length is that figure less what has been
            // produced; never less than what has been read of it, which
            // the server counts a step behind.
            let held = figure(slot, "n_prompt_tokens").unwrap_or(0);
            let of = held.saturating_sub(produced).max(read);
            (read, of, produced)
        })
}

/// A running `llama-server`, holding one model.
///
/// The model stays loaded between requests, which is the residency F36 left
/// open — there, every generation paid the load again, and a 16 GB model
/// cannot sensibly be probed that way.
/// How a server is started: the plain load, and the settings a measurement
/// varies (D52).
///
/// Everything a measurement does not set is the engine's own default, so
/// that a server started for a generation is the same command it always
/// was, and a measurement that varied one thing varied one thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Startup {
    /// How many times to look for the server before giving up.
    pub attempts: usize,
    /// How many layers go on a card.
    pub gpu_layers: u32,
    /// The window it opens, in tokens.
    pub context: u64,
    /// The projector beside the model, where a picture has to reach it.
    pub projector: Option<PathBuf>,
    /// What it is started with beyond the plain load.
    pub started: crate::declared::Started,
    /// How many threads compute, where a measurement sets it.
    pub threads: Option<u32>,
    /// The logical batch a prompt is read in, where a measurement sets it;
    /// the physical batch is set with it, since the engine reads no more
    /// at once than the smaller of the two.
    pub batch: Option<u32>,
    /// The physical batch, where a measurement sets it apart from the
    /// logical one.
    pub ubatch: Option<u32>,
    /// How many requests it serves at once, where a measurement sets it.
    pub parallel: Option<u32>,
    /// The type the key-value cache holds its values in, as the engine
    /// names it, where a measurement sets it (B-534).
    pub cache: Option<&'static str>,
}

impl Default for Startup {
    fn default() -> Self {
        Self {
            attempts: ATTEMPTS,
            gpu_layers: 0,
            context: 4096,
            projector: None,
            started: crate::declared::Started::default(),
            threads: None,
            batch: None,
            ubatch: None,
            parallel: None,
            cache: None,
        }
    }
}

/// What a completion asks for beyond the prompt, the limit and the draw
/// (D52): whether the engine may keep the prompt's prefix between
/// requests, a schema the answer is constrained to, and how many of the
/// engine's ranked candidates come back with each token.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Extras {
    /// Whether the engine keeps what it read of a prompt for the next
    /// request that shares its prefix. Off for every measurement but the
    /// one that measures it (§3.12).
    pub cached: bool,
    /// A JSON schema the engine constrains the answer to, where asked.
    pub json_schema: Option<Value>,
    /// How many ranked candidates come back with each produced token.
    pub ranked: usize,
}

/// A provisioned server MCF started, held for as long as this value is.
#[derive(Debug)]
pub struct Served {
    child: Child,
    /// Where it answers: a socket of its own, or the port it was hosted on.
    reach: Reach,
    /// The last of what the engine wrote to its error stream, kept by a
    /// thread as it arrives.
    ///
    /// **An engine that stops says why, and the sentence was being thrown
    /// away.** A server that exits before it answers exits with a status and
    /// a line — *unknown model architecture*, *failed to allocate*, *segmentation
    /// fault* — and with its error stream sent to nowhere the person got the
    /// status alone (A2, A4). The tail is bounded so a chatty engine cannot
    /// grow it without limit; the last lines are the ones that say why.
    last_words: std::sync::Arc<std::sync::Mutex<String>>,
    /// The model it holds. A request for a different one needs a different
    /// server, and the caller has to be able to tell.
    pub model: PathBuf,
    /// The pin it was built from, for the conditions.
    pub commit: String,
    /// The context window it was started with, in tokens. A request that
    /// needs a larger one needs a different server: this one refuses a turn
    /// longer than its window, and a refusal is not a reading (F152).
    pub window: u64,
    /// The projector it was started with, where the model has one beside
    /// it: what lets a picture reach the model. `None` is text only, and a
    /// picture sent to it is refused before the engine is asked (B-452).
    pub projector: Option<PathBuf>,
    /// Where a picture goes in the text this engine reads.
    ///
    /// **The engine's marker, and this engine's alone.** The pinned server
    /// makes one up at random for each process unless it is told one, so
    /// that no text a person typed can stand where a picture goes; MCF has
    /// to know the marker to place a picture, so it tells the engine one it
    /// made up itself, fresh for each engine it starts, and keeps it here.
    /// `None` for an engine hosted on a port, which places its own.
    pub media_marker: Option<String>,
    /// The build it is: the prefix the binary was started from. Two builds
    /// of the same commit — one for the processor, one for a card — hold
    /// the same model as two different engines, and a request that resolved
    /// to one is not answered by the other (F133).
    pub prefix: PathBuf,
    /// How many layers it was told to put on a card. A server holding the
    /// whole model on the card does not answer for a request that asked for
    /// none of it there: the figure would be the card's under the
    /// processor's name.
    pub gpu_layers: u32,
    /// What it was started with beyond the plain load: a draft head, a rope
    /// scaling. An engine holding a model under one set of these does not
    /// answer for another — the tokens are drawn from a different
    /// arrangement of the same weights — so the caller compares before it
    /// reuses one (B-456).
    pub started: crate::declared::Started,
}

/// Where a server answers: a socket of its own under MCF's runtime
/// directory, or the port somebody hosted it on, with the key they set.
///
/// One request path for both (B-072, B-480): a message from the window
/// goes to the server hosted for its model the same way a diagnostic goes
/// to the one MCF started for itself, and the hosted server's own counters
/// move for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// A socket file the server was told to listen on.
    Socket(PathBuf),
    /// A loopback port, and the key a caller must present where one is set.
    Port {
        /// The port it listens on.
        port: u16,
        /// The key it wants, where somebody set one.
        key: Option<String>,
    },
}

impl Reach {
    /// Whether there is anything to connect to yet: a socket file appears
    /// before the server listens on it; a port has nothing to appear.
    fn appeared(&self) -> bool {
        match self {
            Self::Socket(socket) => socket.exists(),
            Self::Port { .. } => true,
        }
    }

    /// The place, for a failure's context.
    #[must_use]
    pub fn said(&self) -> String {
        match self {
            Self::Socket(socket) => socket.display().to_string(),
            Self::Port { port, .. } => format!("{}:{port}", crate::hosting::LOOPBACK),
        }
    }
}

/// One connection to a server, by whichever way it is reached.
enum Link {
    Socket(UnixStream),
    Port(std::net::TcpStream, Option<String>),
}

impl Link {
    fn open(reach: &Reach) -> std::io::Result<Self> {
        match reach {
            Reach::Socket(socket) => UnixStream::connect(socket).map(Self::Socket),
            Reach::Port { port, key } => {
                std::net::TcpStream::connect((crate::hosting::LOOPBACK, *port))
                    .map(|stream| Self::Port(stream, key.clone()))
            }
        }
    }

    /// Closes both directions, which is how a request in flight is ended.
    fn shutdown(&self) {
        let _closed = match self {
            Self::Socket(stream) => stream.shutdown(std::net::Shutdown::Both),
            Self::Port(stream, _) => stream.shutdown(std::net::Shutdown::Both),
        };
    }

    /// The head of one request: the line, the host, the body's type and
    /// length, and the key where the server wants one.
    fn head(&self, method: &str, path: &str, length: usize) -> String {
        let key = match self {
            Self::Port(_, Some(key)) => format!("Authorization: Bearer {key}\r\n"),
            Self::Port(_, None) | Self::Socket(_) => String::new(),
        };
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
             Content-Length: {length}\r\n{key}Connection: close\r\n\r\n"
        )
    }
}

impl std::io::Read for &Link {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match *self {
            Link::Socket(stream) => (&*stream).read(buf),
            Link::Port(stream, _) => (&*stream).read(buf),
        }
    }
}

impl std::io::Write for &Link {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match *self {
            Link::Socket(stream) => (&*stream).write(buf),
            Link::Port(stream, _) => (&*stream).write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match *self {
            Link::Socket(stream) => (&*stream).flush(),
            Link::Port(stream, _) => (&*stream).flush(),
        }
    }
}

impl Served {
    /// Starts a server holding one model, and waits for it to answer.
    ///
    /// # Errors
    ///
    /// `engine.spawn.not_found` if the binary is not in the prefix,
    /// `engine.spawn.refused` if it cannot be started, and
    /// `engine.hang.no_output` if it never begins listening.
    #[allow(
        clippy::too_many_arguments,
        reason = "one engine's start, each condition of which the account names"
    )]
    pub fn start(
        llama: &ProvisionedLlama,
        model: &Path,
        runtime: &Path,
        gpu_layers: u32,
        context: u64,
        projector: Option<&Path>,
        started: crate::declared::Started,
    ) -> Result<Self, Failure> {
        Self::start_within(
            llama, model, runtime, ATTEMPTS, gpu_layers, context, projector, started,
        )
    }

    /// Starts a server under settings somebody chose, listening on a port.
    ///
    /// **This is what hosting is.** MCF does not implement an inference API;
    /// it provisions an engine that has one and binds it where a caller can
    /// reach it, under settings that are written down. What is returned holds
    /// the child so that dropping it stops the server (A27).
    ///
    /// # Errors
    ///
    /// As [`Self::start`].
    pub fn hosted(
        llama: &ProvisionedLlama,
        model: &Path,
        settings: &crate::hosting::Hosting,
        report: &mut dyn FnMut(u64),
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
        let mut command = Command::new(&binary);
        command
            .args(settings.arguments(&model.display().to_string(), settings.bind()))
            .arg("--port")
            .arg(settings.port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            Failure::new(
                Category::EngineSpawnRefused,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server could not be started",
            )
            .with_context("error", error.to_string())
        })?;
        // A hosted server answers over its port, with the key somebody set
        // on it where they set one, so a request MCF sends it — a message
        // from the window, a run — reaches the same engine a caller on the
        // port sees (B-480).
        let last_words = kept_last_words(&mut child);
        let mut served = Self {
            child,
            reach: Reach::Port {
                port: settings.port,
                key: settings.api_key.clone(),
            },
            last_words,
            model: model.to_path_buf(),
            commit: llama.commit.clone(),
            prefix: llama.prefix.clone(),
            gpu_layers: settings.gpu_layers,
            window: settings.context,
            projector: settings.projector.as_ref().map(PathBuf::from),
            media_marker: None,
            started: settings.started,
        };
        served
            .wait_until_answering(settings.port, ATTEMPTS, report)
            .map_err(|failure| served.with_last_words(failure))?;
        served.register();
        Ok(served)
    }

    /// Waits for a hosted server to report itself ready.
    ///
    /// A model is loaded before it answers, and on a large one that is tens of
    /// seconds — during which the engine answers `503 Loading model`. Treating
    /// that as *up* would hand a caller a server that refuses everything.
    /// **And says how far the load has got while it waits.** A model is read
    /// from disk before the server answers, and on a large one that is
    /// minutes of silence; the engine's resident memory grows as the pages
    /// come in, and once a second that figure goes to `report`, which is
    /// what a screen needs to say *6 GB of 17 read* rather than only how long
    /// it has waited (A7).
    fn wait_until_answering(
        &mut self,
        port: u16,
        attempts: usize,
        report: &mut dyn FnMut(u64),
    ) -> Result<(), Failure> {
        for glance in 0..attempts.saturating_mul(4) {
            if glance % 4 == 3 {
                report(self.resident_bytes().unwrap_or(0));
            }
            if let Some(status) = self.child.try_wait().ok().flatten() {
                return Err(Failure::new(
                    Category::EngineSpawnRefused,
                    Attribution::Machine,
                    Disposition::Refused,
                    Subsystem::new("mcf-serve::served"),
                    "the provisioned server stopped before it began answering",
                )
                .with_context("status", status.to_string()));
            }
            if ready_on(port) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        Err(Failure::new(
            Category::EngineHangNoOutput,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-serve::served"),
            "the provisioned server did not begin answering",
        )
        .with_context("port", port.to_string()))
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
    ///
    /// `projector` is the model's own, found beside it by [`crate::projector`]
    /// where the caller looked: the engine loads it with the model, so that
    /// a picture in a request reaches the model instead of being refused as
    /// something a text-only engine cannot take. It costs memory the model's
    /// own file does not declare, which the peak the account carries
    /// measures rather than assumes (B-452).
    #[allow(
        clippy::too_many_arguments,
        reason = "one engine's start, each condition of which the account names"
    )]
    pub fn start_within(
        llama: &ProvisionedLlama,
        model: &Path,
        runtime: &Path,
        attempts: usize,
        gpu_layers: u32,
        context: u64,
        projector: Option<&Path>,
        started: crate::declared::Started,
    ) -> Result<Self, Failure> {
        Self::start_as(
            llama,
            model,
            runtime,
            &Startup {
                attempts,
                gpu_layers,
                context,
                projector: projector.map(Path::to_path_buf),
                started,
                ..Startup::default()
            },
        )
    }

    /// Starts a server under a [`Startup`]: the plain load, or one with
    /// its threads, batch or slots set for a measurement that varies them
    /// (D52).
    ///
    /// # Errors
    ///
    /// As [`Self::start`].
    #[allow(
        clippy::too_many_lines,
        reason = "one start read straight through: the binary, the fit, the socket, every switch, the wait"
    )]
    pub fn start_as(
        llama: &ProvisionedLlama,
        model: &Path,
        runtime: &Path,
        startup: &Startup,
    ) -> Result<Self, Failure> {
        let (attempts, gpu_layers, context, started) = (
            startup.attempts,
            startup.gpu_layers,
            startup.context,
            startup.started,
        );
        let projector = startup.projector.as_deref();
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

        // **A server that would not fit beside what is resident is refused
        // here, before anything is allocated** (B-560, F243): the kernel's
        // answer to a model that does not fit is to kill something, and what
        // it killed was the desktop.
        if let Some((needs, available)) = crate::engines::would_not_fit(model, context, gpu_layers)
        {
            return Err(Failure::new(
                Category::ResourceMemoryExhausted,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "a server for this model would not fit in the memory left beside what is \
                 already resident: a server holding this or another model has the rest, and \
                 unhosting it or stopping the run that holds it makes room",
            )
            .with_context("needs_bytes", needs.to_string())
            .with_context("available_bytes", available.to_string())
            .with_context("window", context.to_string()));
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
            // Its counters published, so that the model under test can be
            // watched the way a hosted one is (B-573).
            .arg("--metrics")
            // **How large a window the engine holds open.** This was the
            // literal `0`, which llama.cpp reads as *the model's whole trained
            // context* — 262,144 tokens on a model MCF was planning against at
            // 4,096. The engine then allocated a cache for a window nobody
            // asked for: 42 GiB resident for a 17.6 GiB model, one request
            // still running after seventy-five minutes, and a single-threaded
            // daemon blocked behind it so every other client was refused.
            //
            // The same defect as the `-ngl` literal below, in the argument
            // beside it: a hidden value that was not the stated condition
            // (A6, A12, F133). The window MCF resolved for this model on this
            // machine is the one it opens.
            .arg("--ctx-size")
            .arg(context.to_string())
            // **How much of the model goes on the card.** This was the
            // literal `0` — no layers, ever — so MCF resolved a model to a
            // graphics card, said so, and ran it on the processor. It cost
            // 4.9× on this machine and every figure MCF produced through a
            // provisioned engine carried a device label that was false
            // (F133).
            .arg("-ngl")
            .arg(gpu_layers.to_string())
            .arg("--no-webui")
            .arg("--no-warmup")
            // **The engine's own pieces carry the markers.** A turn watched
            // as it arrives is the same text as the turn accounted for at
            // the end, and without this the engine's pieces leave every
            // special token out — so a `</think>` the model wrote would be
            // on the page in the account and missing from the stream a
            // person actually read (B-454, B-451, A4).
            .arg("--special");
        // **The projector goes in with the model, or a picture has nowhere
        // to go** (B-452). `mcf host` started it and the daemon's own engine
        // did not, so the same model took a picture on the port and reported
        // *vision: false* on the socket — two answers to one question.
        if let Some(projector) = projector {
            command.arg("--mmproj").arg(projector);
        }
        // **What a file declares is not what an engine starts** (B-456). A
        // draft head sits in the weights and is not read in at all unless
        // the engine is told to use it, and a rope scaling the file does not
        // declare is nobody's to apply but the person who asked for it.
        // Nothing here is derived from the model: these are switches, and an
        // empty set of them is the plain load.
        command.args(started.arguments());
        // **What a measurement varies, where it varies it** (D52): the
        // engine's own defaults otherwise, so a server started for a
        // generation is byte-for-byte the command it was before.
        if let Some(threads) = startup.threads {
            command.arg("--threads").arg(threads.to_string());
        }
        if let Some(batch) = startup.batch {
            command
                .arg("--batch-size")
                .arg(batch.to_string())
                .arg("--ubatch-size")
                .arg(batch.min(startup.ubatch.unwrap_or(batch)).to_string());
        }
        if let Some(parallel) = startup.parallel {
            command.arg("--parallel").arg(parallel.to_string());
        }
        if let Some(cache) = startup.cache {
            // A quantized value cache needs the fused attention path, which
            // the engine does not turn on by itself for every model.
            command
                .arg("--cache-type-k")
                .arg(cache)
                .arg("--cache-type-v")
                .arg(cache)
                .arg("--flash-attn")
                .arg("on");
        }
        let media_marker = fresh_marker();
        command
            .env("LLAMA_MEDIA_MARKER", &media_marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            Failure::new(
                Category::EngineSpawnRefused,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server could not be started",
            )
            .with_context("error", error.to_string())
        })?;

        let last_words = kept_last_words(&mut child);
        let mut served = Self {
            child,
            reach: Reach::Socket(socket),
            last_words,
            model: model.to_path_buf(),
            commit: llama.commit.clone(),
            prefix: llama.prefix.clone(),
            gpu_layers,
            window: context,
            projector: projector.map(Path::to_path_buf),
            media_marker: Some(media_marker),
            started,
        };
        served
            .wait_until_listening(attempts)
            .map_err(|failure| served.with_last_words(failure))?;
        served.register();
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
            if self.reach.appeared()
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
        .with_context("reach", self.reach.said())
        .with_context(
            "waited",
            format!("{} ms", attempts as u128 * BETWEEN.as_millis()),
        ))
    }

    /// The most memory the server's process has held resident, in bytes, as
    /// the kernel keeps it — or `None` where the kernel does not say.
    ///
    /// A high-water mark rather than the moment's figure: what a request
    /// costs in memory is what it took at its peak, which for a turn is the
    /// cache and the working buffers at their fullest. A server reused across
    /// requests carries the mark of the largest so far, which is what a
    /// ceiling is (B-424).
    #[must_use]
    pub fn peak_resident_bytes(&self) -> Option<u64> {
        crate::adapters::peak_resident_of(self.child.id())
    }

    /// The engine's own counters, where it publishes them (B-573).
    #[must_use]
    pub fn metrics(&self) -> Option<String> {
        metrics_via(&self.reach)
    }

    /// Where the server answers.
    #[must_use]
    pub const fn reach(&self) -> &Reach {
        &self.reach
    }

    /// The server's process, whose memory the kernel reports.
    #[must_use]
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    /// The memory the server holds resident now, in bytes — what stopping
    /// it would give back — or `None` where the kernel does not say.
    #[must_use]
    pub fn resident_bytes(&self) -> Option<u64> {
        crate::adapters::resident_of(self.child.id())
    }

    /// One generation from a turn of token identifiers, drawn as `draw` says.
    ///
    /// `pinned` tells the engine to run past the model's end of text to the
    /// limit, so that `limit` is the length rather than a ceiling on it; the
    /// count that comes back in [`Completed::predicted`] is what proves it
    /// did (B-396).
    ///
    /// # Errors
    ///
    /// `engine.protocol.malformed` for an answer MCF cannot read, and
    /// `engine.exit.midstream` if the connection ends before one arrives.
    pub fn complete(
        &self,
        prompt: Prompt<'_>,
        limit: usize,
        draw: crate::generation::Draw,
        pinned: bool,
        waiting: Waiting<'_>,
    ) -> Result<Completed, Failure> {
        self.complete_with(prompt, limit, draw, pinned, &Extras::default(), waiting)
    }

    /// The same, with what a measurement asks for beyond the plain
    /// request (D52).
    ///
    /// # Errors
    ///
    /// As [`Self::complete`].
    pub fn complete_with(
        &self,
        prompt: Prompt<'_>,
        limit: usize,
        draw: crate::generation::Draw,
        pinned: bool,
        extras: &Extras,
        waiting: Waiting<'_>,
    ) -> Result<Completed, Failure> {
        let mut body = completion_body(prompt, limit, draw, pinned, false);
        if let Value::Map(fields) = &mut body {
            if extras.cached {
                fields.insert("cache_prompt".to_owned(), Value::Bool(true));
            }
            if let Some(schema) = &extras.json_schema {
                fields.insert("json_schema".to_owned(), schema.clone());
            }
            if extras.ranked > 0 {
                fields.insert(
                    "n_probs".to_owned(),
                    Value::Integer(i64::try_from(extras.ranked).unwrap_or(i64::MAX)),
                );
            }
        }
        let body = body.to_line();
        // A refusal comes back with the server's one line for it; the reason
        // is on the engine's error stream, where it wrote one, and goes on
        // the refusal too, so that a person reads why and not only that
        // (A4, B-452).
        interpret(&self.request_while("POST", "/completion", Some(&body), waiting)?)
            .map_err(|failure| self.with_last_words(failure))
    }

    /// The same, watched as it arrives (B-454).
    ///
    /// **A long thought is watched rather than waited for.** The whole-answer
    /// path returns nothing until the model has finished, so a turn that
    /// takes ten minutes is ten minutes of a blank terminal — and an engine
    /// that dies in the ninth leaves nothing at all, when what it had
    /// produced was the most useful thing it could leave (A2, A4). Here each
    /// piece is handed on as the engine writes it, with how many tokens came
    /// before it, and what has arrived stays arrived whatever happens next.
    ///
    /// The pieces are the engine's own, markers included, so the text watched
    /// is the text accounted for; the identifiers come back the same way, so
    /// the account is built from the same figures as the whole-answer path.
    ///
    /// # Errors
    ///
    /// As [`Self::complete`]. A failure after pieces have arrived is a
    /// failure *after* them: the caller has them already.
    pub fn complete_while(
        &self,
        prompt: Prompt<'_>,
        limit: usize,
        draw: crate::generation::Draw,
        pinned: bool,
        waiting: Waiting<'_>,
        arriving: &mut dyn FnMut(usize, &str),
    ) -> Result<Completed, Failure> {
        let body = completion_body(prompt, limit, draw, pinned, true).to_line();
        let mut produced: Vec<usize> = Vec::new();
        let mut text = String::new();
        let mut last = String::new();
        let mut on_event = |event: &str| {
            let Ok(value) = json::parse(event) else {
                return;
            };
            // The end of the stream carries the account and no words: the
            // words were the pieces. An engine that ignored the asking and
            // answered whole sends that account as its only event, which is
            // read the same way — its words are then in the account, and the
            // caller writes them once (B-454).
            let ends = matches!(value.get("stop"), Some(Value::Bool(true)))
                || value.get("stop_type").is_some()
                || value.get("error").is_some();
            if ends {
                event.clone_into(&mut last);
                return;
            }
            let piece = value.get("content").and_then(Value::as_text).unwrap_or("");
            if let Some(Value::List(tokens)) = value.get("tokens") {
                produced.extend(
                    tokens
                        .iter()
                        .filter_map(Value::as_integer)
                        .filter_map(|token| usize::try_from(token).ok()),
                );
            }
            if piece.is_empty() {
                return;
            }
            text.push_str(piece);
            arriving(produced.len().saturating_sub(1), piece);
        };
        let answer = self.streamed_while("/completion", &body, waiting, &mut on_event)?;
        let end = if last.is_empty() { answer } else { last };
        if end.trim().is_empty() {
            // Nothing at all came back: a connection that closed before the
            // engine wrote, which is an engine that died with the request in
            // hand — and its last words are the account of that (A4).
            return Err(self.with_last_words(Failure::new(
                Category::EngineExitMidstream,
                Attribution::Machine,
                Disposition::Aborted,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server's answer had no body",
            )));
        }
        let mut completed = interpret(&end).map_err(|failure| self.with_last_words(failure))?;
        // In this shape the engine sends its words as it goes and its
        // account at the end, so what came from the pieces is what the
        // account would otherwise have carried (B-454).
        if completed.produced.is_empty() {
            completed.produced = produced;
        }
        if completed.text.is_empty() {
            completed.text = text;
        }
        Ok(completed)
    }

    /// A request whose answer arrives in events, each handed on as it comes.
    ///
    /// # Errors
    ///
    /// As [`Self::request_while`].
    fn streamed_while(
        &self,
        path: &str,
        body: &str,
        waiting: Waiting<'_>,
        on_event: &mut dyn FnMut(&str),
    ) -> Result<String, Failure> {
        match while_watched(&self.reach, waiting, |connection| {
            sent_and_streamed(connection, "POST", path, body, on_event)
        }) {
            Ok(answer) => Ok(answer),
            Err(Interrupted::Closed(closed)) => {
                Err(closed_failure(&self.model, closed, waiting.progress))
            }
            Err(Interrupted::Connecting(error) | Interrupted::Reading(error)) => Err(self
                .with_last_words(
                    Failure::new(
                        Category::EngineExitMidstream,
                        Attribution::Machine,
                        Disposition::Aborted,
                        Subsystem::new("mcf-serve::served"),
                        "the provisioned server stopped while it was answering",
                    )
                    .with_context("error", error.to_string()),
                )),
            Err(Interrupted::Sending(error)) => Err(Failure::new(
                Category::EngineExitMidstream,
                Attribution::Machine,
                Disposition::Aborted,
                Subsystem::new("mcf-serve::served"),
                "the request could not be sent to the server",
            )
            .with_context("error", error.to_string())),
        }
    }

    /// Where the model ranked the token that actually came next, and what it
    /// said the token was worth.
    ///
    /// **The question a reader is asking when they ask which words matter.**
    /// A token the model ranked first was one it would have written anyway, so
    /// the writer supplied no information by writing it; one it ranked
    /// hundredth, or did not have in its list at all, is where the prompt told
    /// the model something. That is a property of this model and this prefix
    /// and nothing else, and it does not depend on comparing two answers — so
    /// it is not touched by the drift that makes an ablation an ordering
    /// rather than a measure (§3.8, A19).
    ///
    /// Returns the rank counting from one, and the engine's own figure for the
    /// token as the text it sent. `None` for the rank where the token is
    /// outside the list asked for, which is a bound and not an absence: it was
    /// worse than the last one that fitted (A7).
    ///
    /// # Errors
    ///
    /// Whatever the server answered with, where that was not a completion.
    pub fn ranked_next(
        &self,
        prefix: &[usize],
        wanted: usize,
        how_many: usize,
    ) -> Result<(Option<usize>, Option<String>), Failure> {
        let identifiers = Value::List(
            prefix
                .iter()
                .map(|held| Value::Integer(i64::try_from(*held).unwrap_or(0)))
                .collect(),
        );
        let body = Value::map([
            ("prompt", identifiers),
            // One token, only so that the server produces a distribution at
            // this position; what it picks is not read.
            ("n_predict", Value::Integer(1)),
            ("temperature", Value::Integer(0)),
            (
                "n_probs",
                Value::Integer(i64::try_from(how_many).unwrap_or(0)),
            ),
            // **Cached here, where every other request turns it off.** The
            // prefixes asked for are each one token longer than the last, so
            // without it the work is quadratic in the prompt. Nothing is being
            // timed and nothing is generated: what is read is a distribution
            // at one position, which the cache does not change.
            ("cache_prompt", Value::Bool(true)),
        ])
        .to_line();
        let answered = self.request("POST", "/completion", Some(&body))?;
        Ok(ranked_in(&answered, wanted))
    }

    /// The engine's ranked candidates for the token after `prefix`: each
    /// identifier with the log-probability the engine gives it, in
    /// millibits, highest first (D52, B-491, B-499).
    ///
    /// **Millibits, not a float.** The engine writes a natural
    /// log-probability as a decimal; it is read here to thousandths of a
    /// bit so that a shipped crate holds an integer and a record can be
    /// ordered (A6). What is *not* in the list is bounded rather than
    /// lost: the caller reads a token that is absent as ranked past
    /// `how_many` and no likelier than the last one listed.
    ///
    /// Cached, for the reason [`Self::ranked_next`] is: prefixes each one
    /// token longer than the last, nothing timed, nothing generated.
    ///
    /// # Errors
    ///
    /// Whatever the server answered with, where that was not a completion
    /// carrying its candidates.
    pub fn distribution_at(
        &self,
        prefix: &[usize],
        how_many: usize,
    ) -> Result<Vec<(usize, i64)>, Failure> {
        let identifiers = Value::List(
            prefix
                .iter()
                .map(|held| Value::Integer(i64::try_from(*held).unwrap_or(0)))
                .collect(),
        );
        let body = Value::map([
            ("prompt", identifiers),
            ("n_predict", Value::Integer(1)),
            ("temperature", Value::Integer(0)),
            (
                "n_probs",
                Value::Integer(i64::try_from(how_many).unwrap_or(0)),
            ),
            ("cache_prompt", Value::Bool(true)),
        ])
        .to_line();
        let answered = self.request("POST", "/completion", Some(&body))?;
        distribution_in(&answered).ok_or_else(|| {
            Failure::new(
                Category::EngineProtocolMalformed,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server's answer carried no ranked candidates",
            )
        })
    }

    /// Some text as this server's model receives it: the identifiers, and
    /// what each one spells (B-441).
    ///
    /// **The tokenizer that generates is the tokenizer that reads.** The
    /// prompt report counted and ranked a prompt through MCF's own
    /// segmentation while the answers it compared came through this server,
    /// so one report carried two readings of the prompt and, for a
    /// vocabulary MCF's tokenizer does not segment, none (F158). This asks
    /// the engine itself, which is what every generation through it is
    /// tokenized by.
    ///
    /// `with_beginning` asks for the model's own convention — a beginning
    /// marker where the file says to add one, none where it says not to.
    ///
    /// **`as_markers` says whose text this is.** The model's own — a marker
    /// from its template, a turn its template rendered — is read with its
    /// control tokens taken as themselves. A person's is not: the server
    /// reads `<|im_start|>` typed into a prompt as the token that opens a
    /// turn when it is told to parse specials, and the first cut told it to
    /// for every text, so on the path every prompt now takes a person could
    /// type their way into a marker — the thing D46's safety property
    /// exists to prevent and MCF's own tokenizer refuses (F26, F161).
    ///
    /// # Errors
    ///
    /// Whatever the server answered with, where that was not a list of
    /// tokens.
    pub fn tokenize(
        &self,
        text: &str,
        with_beginning: bool,
        as_markers: bool,
    ) -> Result<Vec<Token>, Failure> {
        let body = Value::map([
            ("content", Value::text(text.to_owned())),
            ("add_special", Value::Bool(with_beginning)),
            ("parse_special", Value::Bool(as_markers)),
            ("with_pieces", Value::Bool(true)),
        ])
        .to_line();
        let answered = self.request("POST", "/tokenize", Some(&body))?;
        tokens_in(&answered)
    }

    /// Identifiers as text, spelled by the engine — markers included, since
    /// what is asked is what these tokens *are* and a marker left out would
    /// make a turn that closed its thinking look like one that never opened
    /// it (A1).
    ///
    /// # Errors
    ///
    /// Whatever the server answered with, where that was not text.
    pub fn detokenize(&self, tokens: &[usize]) -> Result<String, Failure> {
        let identifiers = Value::List(
            tokens
                .iter()
                .map(|token| Value::Integer(i64::try_from(*token).unwrap_or(i64::MAX)))
                .collect(),
        );
        let body = Value::map([("tokens", identifiers)]).to_line();
        let answered = self.request("POST", "/detokenize", Some(&body))?;
        text_in(
            &answered,
            "content",
            "the provisioned server's answer spelled no text",
        )
    }

    /// A conversation as the model's own template renders it, rendered by
    /// the engine (D47).
    ///
    /// MCF runs no template (D46); the engine runs this one for every chat
    /// turn it serves, and here it is asked to say what it would send. The
    /// switches — thinking on or off, how hard to reason — are the
    /// template's own words, passed through as the caller named them, and a
    /// template that does not know a switch renders without it, which the
    /// caller tells by comparing (A4).
    ///
    /// # Errors
    ///
    /// Whatever the server answered with, where that was not a rendering —
    /// a template that raised on the switches it was given says so in its
    /// own words.
    pub fn render(&self, messages: Value, switches: Value) -> Result<String, Failure> {
        let body =
            Value::map([("messages", messages), ("chat_template_kwargs", switches)]).to_line();
        let answered = self.request("POST", "/apply-template", Some(&body))?;
        text_in(
            &answered,
            "prompt",
            "the provisioned server's answer rendered no prompt",
        )
    }

    /// A conversation with tools declared, as the model's own template
    /// renders them: the tools go in the template's own place and form,
    /// which is how a caller of the hosted server declares them (D52,
    /// B-517).
    ///
    /// # Errors
    ///
    /// As [`Self::render`].
    pub fn render_with_tools(&self, messages: Value, tools: Value) -> Result<String, Failure> {
        let body = Value::map([("messages", messages), ("tools", tools)]).to_line();
        let answered = self.request("POST", "/apply-template", Some(&body))?;
        text_in(
            &answered,
            "prompt",
            "the provisioned server's answer rendered no prompt",
        )
    }

    /// The smallest HTTP a request needs.
    ///
    /// `Connection: close` so the body ends at end of stream and there is no
    /// chunked encoding or keep-alive framing to get wrong — B15 admits weight
    /// only against a stated cost, and a general HTTP client is weight this
    /// does not need to carry.
    fn request(&self, method: &str, path: &str, body: Option<&str>) -> Result<String, Failure> {
        self.request_while(method, path, body, Waiting::NOBODY)
    }

    /// A request somebody is waiting on, watched over while it is in flight
    /// (D48): closed when the client leaves or the daemon stops, its
    /// progress published as it goes.
    ///
    /// # Errors
    ///
    /// As [`Self::request`], and `lab.interrupted` where the request was
    /// closed before the engine answered — saying who left, and how far the
    /// engine had read when it was closed.
    fn request_while(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        waiting: Waiting<'_>,
    ) -> Result<String, Failure> {
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
        let answer = match exchange(&self.reach, method, path, body, waiting) {
            Ok(answer) => answer,
            Err(Interrupted::Closed(closed)) => {
                return Err(closed_failure(&self.model, closed, waiting.progress));
            }
            Err(Interrupted::Connecting(error)) => {
                return Err(self.with_last_words(died(
                    "the provisioned server stopped accepting connections",
                    &error,
                )));
            }
            Err(Interrupted::Sending(error)) => {
                return Err(died("the request could not be sent to the server", &error));
            }
            Err(Interrupted::Reading(error)) => {
                return Err(self.with_last_words(died("the server's answer ended early", &error)));
            }
        };
        answer.split_once("\r\n\r\n").map_or_else(
            || {
                // No head at all is a connection that closed before the
                // server wrote: an engine that died with the request in hand,
                // and its last words are the account of that (A4).
                Err(self.with_last_words(
                    Failure::new(
                        Category::EngineExitMidstream,
                        Attribution::Machine,
                        Disposition::Aborted,
                        Subsystem::new("mcf-serve::served"),
                        "the provisioned server's answer had no body",
                    )
                    .with_context("engine_said", answer.chars().take(400).collect::<String>()),
                ))
            },
            |(_head, body)| Ok(body.to_owned()),
        )
    }

    /// A failure with the engine's own last words on it, where it wrote any.
    ///
    /// Given a moment to arrive: the stream is read by another thread, and an
    /// engine that has just died may not have been read yet.
    fn with_last_words(&self, failure: Failure) -> Failure {
        for _ in 0..20 {
            if let Ok(held) = self.last_words.lock()
                && !held.trim().is_empty()
            {
                return failure.with_context("engine_last_words", held.trim().to_owned());
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        failure
    }
}

/// Why a request did not come back with an answer.
#[derive(Debug)]
enum Interrupted {
    /// The engine's socket would not take a connection.
    Connecting(std::io::Error),
    /// The request could not be written.
    Sending(std::io::Error),
    /// The answer could not be read to its end.
    Reading(std::io::Error),
    /// The request was closed by the side that was waiting for it (D48).
    Closed(Closed),
}
use Interrupted::{Reading, Sending};

/// Asks the engine at `socket` once and reads the whole answer, watched over
/// for whoever is waiting: the request is closed when the client leaves or
/// the daemon stops, and the engine's progress reaches `waiting` while it
/// runs.
fn exchange(
    reach: &Reach,
    method: &str,
    path: &str,
    body: Option<&str>,
    waiting: Waiting<'_>,
) -> Result<String, Interrupted> {
    while_watched(reach, waiting, |connection| {
        sent_and_read(connection, method, path, body)
    })
}

/// One exchange with somebody watching over it (D48).
///
/// The connection is opened, the work is done on it, and a watcher beside it
/// closes it where the client that asked has left or the daemon is stopping
/// — which is what stops an engine answering a question nobody is waiting
/// for. Written once, because a second copy of it would be a second answer
/// to *when does a request end* (B-072).
fn while_watched<T>(
    reach: &Reach,
    waiting: Waiting<'_>,
    work: impl FnOnce(&Link) -> Result<T, Interrupted>,
) -> Result<T, Interrupted> {
    let connection = Link::open(reach).map_err(Interrupted::Connecting)?;
    let done = AtomicBool::new(false);
    let (answer, closed) = std::thread::scope(|scope| {
        let watcher = waiting
            .watches_anything()
            .then(|| scope.spawn(|| watched(reach, &connection, waiting, &done)));
        let answer = work(&connection);
        done.store(true, Ordering::Release);
        let closed = watcher.and_then(|watcher| watcher.join().ok().flatten());
        (answer, closed)
    });
    match closed {
        Some(closed) => Err(Interrupted::Closed(closed)),
        None => answer,
    }
}

/// Asks whatever listens at `socket` on behalf of `waiting`, with no server
/// of MCF's own behind it.
///
/// What the laboratory uses to produce a request closed because the client
/// that asked for it left (A13): the daemon's own path runs through the same
/// exchange with a served engine behind the socket. `model` is named in the
/// failure, as it is there.
///
/// # Errors
///
/// `lab.interrupted` where the request was closed before an answer came,
/// saying who left; `engine.exit.midstream` where the wire failed.
pub fn asked_while(
    socket: &Path,
    model: &Path,
    method: &str,
    path: &str,
    body: Option<&str>,
    waiting: Waiting<'_>,
) -> Result<String, Failure> {
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
    match exchange(
        &Reach::Socket(socket.to_path_buf()),
        method,
        path,
        body,
        waiting,
    ) {
        Ok(answer) => Ok(answer),
        Err(Interrupted::Closed(closed)) => Err(closed_failure(model, closed, waiting.progress)),
        Err(Interrupted::Connecting(error)) => {
            Err(died("the server stopped accepting connections", &error))
        }
        Err(Interrupted::Sending(error)) => {
            Err(died("the request could not be sent to the server", &error))
        }
        Err(Interrupted::Reading(error)) => Err(died("the server's answer ended early", &error)),
    }
}

/// The failure a request closed before its answer is reported as: who
/// stopped waiting, and how far the engine had got.
/// What a request is told when the client that asked for it went away. One
/// sentence in one place, because a ladder reads it back to know whether to
/// climb on: a run whose asker has left is a run nobody will read.
pub const CLIENT_LEFT: &str = "the client that asked for this left before the engine answered, \
                               so the request was closed and the engine stopped";

fn closed_failure(model: &Path, closed: Closed, progress: Option<&Progress>) -> Failure {
    let (what, attribution) = match closed {
        Closed::ClientLeft => (CLIENT_LEFT, Attribution::User),
        Closed::Stopping => (
            "the daemon was asked to stop while the engine was answering, so the \
             request was closed and the engine stopped",
            Attribution::User,
        ),
    };
    let failure = Failure::new(
        Category::LabInterrupted,
        attribution,
        Disposition::Aborted,
        Subsystem::new("mcf-serve::served"),
        what,
    )
    .with_context("model", model.display().to_string());
    match progress.map(Progress::to_value) {
        Some(Value::Map(fields)) => {
            let figure = |key: &str| fields.get(key).map_or_else(String::new, Value::to_line);
            failure
                .with_context("engine_read", figure("read"))
                .with_context("engine_of", figure("of"))
                .with_context("engine_produced", figure("produced"))
        }
        _ => failure,
    }
}

/// Writes one request and reads the whole answer.
fn sent_and_read(
    mut connection: &Link,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, Interrupted> {
    let body = body.unwrap_or("");
    let request = format!("{}{body}", connection.head(method, path, body.len()));
    connection
        .write_all(request.as_bytes())
        .and_then(|()| connection.flush())
        .map_err(Sending)?;
    let mut answer = Vec::new();
    connection.read_to_end(&mut answer).map_err(Reading)?;
    Ok(String::from_utf8_lossy(&answer).into_owned())
}

/// Sends one request and hands on each event of the answer as it arrives.
///
/// **The framing, read rather than assumed.** The engine writes an answer it
/// is streaming in chunked transfer encoding — a size in hexadecimal, the
/// bytes, and again — and an answer it refuses before it starts streaming as
/// one ordinary body. Both are read here: the head says which, and a body
/// that is not chunked is one event of itself, which is how a refusal
/// reaches the same reader as an answer.
///
/// Each event is a `data:` line of server-sent events. Returns the last one,
/// which is the engine's account of the whole turn.
fn sent_and_streamed(
    mut connection: &Link,
    method: &str,
    path: &str,
    body: &str,
    on_event: &mut dyn FnMut(&str),
) -> Result<String, Interrupted> {
    let request = format!("{}{body}", connection.head(method, path, body.len()));
    connection
        .write_all(request.as_bytes())
        .and_then(|()| connection.flush())
        .map_err(Sending)?;

    let mut reader = std::io::BufReader::new(connection);
    let mut chunked = false;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(Reading)?;
        if read == 0 || line.trim().is_empty() {
            break;
        }
        if line.to_ascii_lowercase().starts_with("transfer-encoding:")
            && line.to_ascii_lowercase().contains("chunked")
        {
            chunked = true;
        }
    }

    let mut held = String::new();
    let mut last = String::new();
    let mut hand_on = |held: &mut String, at_end: bool| {
        while let Some(at) = held.find("\n\n") {
            let event: String = held.drain(..at).collect();
            held.drain(.."\n\n".len().min(held.len()));
            deliver(&event, &mut last, on_event);
        }
        if at_end && !held.trim().is_empty() {
            let event = std::mem::take(held);
            deliver(&event, &mut last, on_event);
        }
    };

    loop {
        let piece = if chunked {
            match next_chunk(&mut reader)? {
                Some(piece) => piece,
                None => break,
            }
        } else {
            let mut bytes = [0_u8; 4096];
            let read = reader.read(&mut bytes).map_err(Reading)?;
            if read == 0 {
                break;
            }
            String::from_utf8_lossy(bytes.get(..read).unwrap_or_default()).into_owned()
        };
        held.push_str(&piece);
        hand_on(&mut held, false);
    }
    hand_on(&mut held, true);
    Ok(last)
}

/// One event of the stream, to whoever is watching, keeping the last.
fn deliver(event: &str, last: &mut String, on_event: &mut dyn FnMut(&str)) {
    let payload = event
        .trim()
        .strip_prefix("data:")
        .map_or_else(|| event.trim(), str::trim);
    if payload.is_empty() {
        return;
    }
    last.clear();
    last.push_str(payload);
    on_event(payload);
}

/// The next chunk of a chunked body, or nothing at its end.
fn next_chunk(reader: &mut std::io::BufReader<&Link>) -> Result<Option<String>, Interrupted> {
    let mut line = String::new();
    let read = reader.read_line(&mut line).map_err(Reading)?;
    if read == 0 {
        return Ok(None);
    }
    let size = usize::from_str_radix(line.trim(), 16).unwrap_or(0);
    if size == 0 {
        return Ok(None);
    }
    let mut bytes = vec![0_u8; size];
    reader.read_exact(&mut bytes).map_err(Reading)?;
    // The two bytes that end a chunk, read and thrown away.
    let mut ending = [0_u8; 2];
    let _ended = reader.read_exact(&mut ending);
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

/// A request to a server by its socket alone, with nobody watching: what
/// the watcher itself asks the slot with.
fn plain_request(
    reach: &Reach,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, Interrupted> {
    let connection = Link::open(reach).map_err(Sending)?;
    let answer = sent_and_read(&connection, method, path, body)?;
    Ok(answer
        .split_once("\r\n\r\n")
        .map_or(answer.clone(), |(_head, body)| body.to_owned()))
}

/// How much of the engine's error stream is kept: the end of it.
const LAST_WORDS: usize = 4096;

/// Keeps the tail of a child's error stream as it is written.
///
/// The thread ends when the stream does, which is when the engine does; it
/// holds no reference to the server, so a server dropped mid-read is not
/// kept alive by it.
fn kept_last_words(child: &mut Child) -> std::sync::Arc<std::sync::Mutex<String>> {
    let kept = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    if let Some(mut stream) = child.stderr.take() {
        let into = std::sync::Arc::clone(&kept);
        let _reader = std::thread::Builder::new()
            .name("engine-last-words".to_owned())
            .spawn(move || {
                let mut held = [0_u8; 1024];
                while let Ok(read) = stream.read(&mut held) {
                    if read == 0 {
                        break;
                    }
                    let Ok(mut kept) = into.lock() else {
                        break;
                    };
                    kept.push_str(&String::from_utf8_lossy(
                        held.get(..read).unwrap_or_default(),
                    ));
                    if kept.len() > LAST_WORDS {
                        let cut = kept.len().saturating_sub(LAST_WORDS);
                        let at = (cut..kept.len())
                            .find(|at| kept.is_char_boundary(*at))
                            .unwrap_or(kept.len());
                        kept.drain(..at);
                    }
                }
            });
    }
    kept
}

/// One token as the server reads it: the identifier and its spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The identifier.
    pub id: usize,
    /// What it spells. A piece that is not text on its own — one byte of a
    /// character the vocabulary splits — is written as the engine writes a
    /// byte token, `<0xE2>`, so that it is on the page rather than lost (A1).
    pub piece: String,
}

impl Token {
    /// A token by the bytes it contributes, spelled the one way every reader
    /// of a piece spells them.
    #[must_use]
    pub fn from_bytes(id: usize, bytes: &[u8]) -> Self {
        Self {
            id,
            piece: spelled(bytes),
        }
    }
}

/// Bytes as a piece on the page: text where they are text, and each byte
/// written the way the engine writes a byte token, `<0xE2>`, where they are
/// not — one byte of a character the vocabulary splits is on the page rather
/// than lost or shown as a replacement mark (A1, F19).
fn spelled(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    core::str::from_utf8(bytes).map_or_else(
        |_not_text| {
            bytes.iter().fold(String::new(), |mut out, byte| {
                // The write cannot fail: the target is a `String`.
                let _written = write!(out, "<0x{byte:02X}>");
                out
            })
        },
        str::to_owned,
    )
}

/// One text field of an answer, or why not: the engine's refusal in its own
/// words where it refused, and a malformed answer where it did not say.
fn text_in(answer: &str, field: &str, missing: &str) -> Result<String, Failure> {
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
        .with_context("reason", message));
    }
    value
        .get(field)
        .and_then(Value::as_text)
        .map(str::to_owned)
        .ok_or_else(|| malformed(missing))
}

/// The tokens in the server's answer to `/tokenize`.
fn tokens_in(answer: &str) -> Result<Vec<Token>, Failure> {
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
    if let Some(said) = value.get("error") {
        let message = said
            .get("message")
            .and_then(Value::as_text)
            .unwrap_or("the engine did not say")
            .to_owned();
        return Err(
            malformed("the provisioned server refused to tokenize").with_context("reason", message)
        );
    }
    let listed = value
        .get("tokens")
        .and_then(Value::as_list)
        .ok_or_else(|| malformed("the provisioned server's answer listed no tokens"))?;
    listed
        .iter()
        .map(|held| {
            let id = held
                .get("id")
                .and_then(Value::as_integer)
                .and_then(|id| usize::try_from(id).ok())
                .ok_or_else(|| malformed("a token in the server's answer had no identifier"))?;
            let piece = match held.get("piece") {
                Some(Value::Text(text)) => text.clone(),
                Some(Value::List(bytes)) => bytes
                    .iter()
                    .map(|byte| {
                        byte.as_integer()
                            .and_then(|held| u8::try_from(held).ok())
                            .map_or_else(|| "<?>".to_owned(), |held| spelled(&[held]))
                    })
                    .collect(),
                _ => return Err(malformed("a token in the server's answer had no spelling")),
            };
            Ok(Token { id, piece })
        })
        .collect()
}

/// Where `wanted` sits in the distribution the server sent back.
///
/// The engine's figure for the token is carried as the text it wrote. The
/// record has no floating-point variant on purpose — a value nobody can do
/// arithmetic on cannot become a measurement by accident — and a log
/// probability is exactly the sort of number that would grow a division
/// somewhere else. What is ordered here is the rank, which is a count.
fn ranked_in(answer: &str, wanted: usize) -> (Option<usize>, Option<String>) {
    let Ok(value) = json::parse(answer) else {
        return (None, None);
    };
    let Some(first) = value
        .get("completion_probabilities")
        .and_then(Value::as_list)
        .and_then(<[Value]>::first)
    else {
        return (None, None);
    };
    let Some(ranked) = first.get("top_logprobs").and_then(Value::as_list) else {
        return (None, None);
    };
    for (at, candidate) in ranked.iter().enumerate() {
        let held = candidate
            .get("id")
            .and_then(Value::as_integer)
            .and_then(|held| usize::try_from(held).ok());
        if held == Some(wanted) {
            let said = candidate
                .get("logprob")
                .map(mcf_record::json::Value::to_line);
            return (Some(at.saturating_add(1)), said);
        }
    }
    (None, None)
}

/// The ranked candidates in a one-token completion, each with its
/// log-probability in millibits.
fn distribution_in(answer: &str) -> Option<Vec<(usize, i64)>> {
    let value = json::parse(answer).ok()?;
    let first = value
        .get("completion_probabilities")
        .and_then(Value::as_list)
        .and_then(<[Value]>::first)?;
    let ranked = first.get("top_logprobs").and_then(Value::as_list)?;
    let mut out = Vec::with_capacity(ranked.len());
    for candidate in ranked {
        let id = candidate
            .get("id")
            .and_then(Value::as_integer)
            .and_then(|held| usize::try_from(held).ok())?;
        let logprob = candidate.get("logprob").and_then(millibits)?;
        out.push((id, logprob));
    }
    Some(out)
}

/// A natural log-probability as the engine writes it — an integer, or a
/// decimal MCF does not otherwise carry — read to millibits: thousandths
/// of a bit, negative for anything under certainty.
///
/// The decimal is parsed by hand and only here: the sign, the whole part
/// and up to nine places of fraction, then scaled by the bit's worth in
/// nats in integers. A shipped crate holds no float (A6, A1), and a
/// log-probability read to a thousandth of a bit is read finer than any
/// engine states it.
#[must_use]
#[expect(
    clippy::integer_division,
    reason = "nanonats scaled to millibits; what is discarded is under a millibit"
)]
pub fn millibits(held: &Value) -> Option<i64> {
    let written = match held {
        Value::Integer(whole) => return Some(whole.saturating_mul(1_442_695) / 1_000),
        // As the engine wrote it: the record's reader keeps a decimal as
        // its text, and its line is that text.
        Value::ForeignNumber(_) => held.to_line(),
        _ => return None,
    };
    let (negative, digits) = written
        .strip_prefix('-')
        .map_or((false, written.as_str()), |rest| (true, rest));
    // An exponent: the engine writes `-1.5e-05` for a near-certain token,
    // which is under a millibit either way.
    let (mantissa, exponent) = digits
        .split_once(['e', 'E'])
        .map_or((digits, 0_i32), |(mantissa, exponent)| {
            (mantissa, exponent.parse::<i32>().unwrap_or(0))
        });
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let whole: i128 = whole
        .parse()
        .ok()
        .or_else(|| whole.is_empty().then_some(0))?;
    let mut nanos: i128 = 0;
    for (place, digit) in fraction.bytes().take(9).enumerate() {
        let digit = i128::from(digit.checked_sub(b'0')?);
        if digit > 9 {
            return None;
        }
        nanos += digit * 10_i128.pow(8 - u32::try_from(place).ok()?);
    }
    // Nanonats, then scaled by the exponent, then to millibits: a nat is
    // 1.442695 bits.
    let mut nanonats = whole * 1_000_000_000 + nanos;
    match exponent.cmp(&0) {
        std::cmp::Ordering::Greater => {
            nanonats = nanonats.saturating_mul(10_i128.pow(u32::try_from(exponent).ok()?));
        }
        std::cmp::Ordering::Less => {
            nanonats /= 10_i128.pow(u32::try_from(-exponent).ok()?);
        }
        std::cmp::Ordering::Equal => {}
    }
    let millibits = nanonats * 1_442_695 / 1_000_000_000_000;
    let signed = if negative { -millibits } else { millibits };
    i64::try_from(signed).ok()
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
        produced: match value.get("tokens") {
            Some(Value::List(tokens)) => tokens
                .iter()
                .filter_map(Value::as_integer)
                .filter_map(|token| usize::try_from(token).ok())
                .collect(),
            _ => Vec::new(),
        },
    })
}

/// A server MCF started and still holds, as the daemon lists what is under
/// test: enough to read its counters without holding whoever owns it — a
/// measurement's step, a generation's slot, a hold (B-573).
#[derive(Debug, Clone)]
pub struct Live {
    /// The server's process.
    pub child: u32,
    /// The model it holds.
    pub model: PathBuf,
    /// The engine's commit.
    pub commit: String,
    /// The window it was opened at.
    pub window: u64,
    /// Where it answers.
    pub reach: Reach,
}

/// Every server alive now, oldest first: registered as one starts
/// answering, struck as it is dropped.
static LIVE: std::sync::OnceLock<std::sync::Mutex<Vec<Live>>> = std::sync::OnceLock::new();

fn live_list() -> &'static std::sync::Mutex<Vec<Live>> {
    LIVE.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// The servers alive now, oldest first.
#[must_use]
pub fn live() -> Vec<Live> {
    live_list()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

impl Served {
    /// Puts this server on the list of the live.
    fn register(&self) {
        let entry = Live {
            child: self.child.id(),
            model: self.model.clone(),
            commit: self.commit.clone(),
            window: self.window,
            reach: self.reach.clone(),
        };
        let mut held = live_list()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|live| live.child != entry.child);
        held.push(entry);
    }

    /// Strikes this server from the list of the live.
    fn unregister(&self) {
        let child = self.child.id();
        let mut held = live_list()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|live| live.child != child);
    }
}

impl Drop for Served {
    /// The server goes when the daemon does, and takes its socket with it.
    ///
    /// A27: what MCF starts, MCF stops. A server left running holds a model in
    /// memory on a machine three other projects are sharing, which is the
    /// least neighbourly failure available here.
    fn drop(&mut self) {
        self.unregister();
        let _killed = self.child.kill();
        let _waited = self.child.wait();
        if let Reach::Socket(socket) = &self.reach {
            let _gone = std::fs::remove_file(socket);
        }
    }
}

/// The request one generation sends, every condition of it stated.
/// A marker no text a person typed can contain, for one engine's life.
///
/// The engine stands a picture where it finds the marker and wraps what the
/// projector makes of it in the markers the model was trained to see around
/// one; a marker anybody could guess would let a prompt stand a picture's
/// place where no picture is (B-452). Sixteen hex digits from the standard
/// library's own random keys, which are seeded from the operating system.
fn fresh_marker() -> String {
    use std::hash::{BuildHasher as _, Hasher as _};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    format!("<__media_{:016x}__>", hasher.finish())
}

/// What a completion is asked from.
#[derive(Debug, Clone, Copy)]
pub enum Prompt<'a> {
    /// A turn of token identifiers, sent as themselves.
    Identifiers(&'a [usize]),
    /// A turn of text with a picture in it, at the engine's
    /// [`Served::media_marker`].
    ///
    /// **Text, because the engine gives a picture no other door.** A prompt
    /// carrying a picture is a string the engine reads with every marker in
    /// it taken as a marker — the template's and the person's alike. That
    /// is the engine's condition and not a choice of MCF's, and the account
    /// says the turn was read that way (F26, F161).
    Shown {
        /// The rendered turn, with the marker where the picture goes.
        text: &'a str,
        /// The picture's bytes, as the file held them.
        picture: &'a [u8],
    },
}

/// Bytes as the engine takes a picture: the sixty-four-character alphabet
/// with `=` padding, and no line breaks.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let letter = |six: u32| {
        ALPHABET
            .get(usize::try_from(six & 0x3f).unwrap_or(0))
            .copied()
            .unwrap_or(b'A') as char
    };
    let mut out = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for chunk in bytes.chunks(3) {
        let mut held = [0_u8; 3];
        for (slot, byte) in held.iter_mut().zip(chunk) {
            *slot = *byte;
        }
        let packed = (u32::from(held[0]) << 16) | (u32::from(held[1]) << 8) | u32::from(held[2]);
        out.push(letter(packed >> 18));
        out.push(letter(packed >> 12));
        out.push(if chunk.len() > 1 {
            letter(packed >> 6)
        } else {
            '='
        });
        out.push(if chunk.len() > 2 { letter(packed) } else { '=' });
    }
    out
}

fn completion_body(
    prompt: Prompt<'_>,
    limit: usize,
    draw: crate::generation::Draw,
    pinned: bool,
    streaming: bool,
) -> Value {
    let prompt = match prompt {
        Prompt::Identifiers(tokens) => Value::List(
            tokens
                .iter()
                .map(|token| Value::Integer(i64::try_from(*token).unwrap_or(i64::MAX)))
                .collect(),
        ),
        Prompt::Shown { text, picture } => Value::map([
            ("prompt_string", Value::text(text.to_owned())),
            (
                "multimodal_data",
                Value::List(vec![Value::text(base64(picture))]),
            ),
        ]),
    };
    Value::map([
        ("prompt", prompt),
        (
            "n_predict",
            Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
        ),
        (
            "seed",
            Value::Integer(i64::try_from(draw.seed).unwrap_or(i64::MAX)),
        ),
        // The engine's API takes a decimal, and this is the one place MCF
        // writes a number that is not an integer: it is the temperature a
        // caller stated, written as they stated it, to a request that is
        // not a record (A1). Nought is the integer, so a greedy request
        // is byte-for-byte what it was before there was a temperature.
        (
            "temperature",
            if draw.is_greedy() {
                Value::Integer(0)
            } else {
                Value::exact_thousandths(draw.temperature)
            },
        ),
        // **The cut is stated, so the server fills nothing in** (B-440,
        // F157): left unsaid, it applies the file's `general.sampling.*`
        // and then its own house values, and a seeded draw runs under a
        // condition nobody named. Nought, one and nought are *off*.
        (
            "top_k",
            Value::Integer(i64::from(draw.truncation.top_k_sent())),
        ),
        (
            "top_p",
            Value::exact_thousandths(draw.truncation.top_p_sent()),
        ),
        (
            "min_p",
            Value::exact_thousandths(draw.truncation.min_p_sent()),
        ),
        // The identifiers as well as the text. They cost nothing to ask
        // for and are the only form in which two engines can be compared
        // past the point where their generations part (B-362).
        ("return_tokens", Value::Bool(true)),
        // Every request starts from the same state, or a trial would be
        // measuring what the previous trial left behind (§3.12).
        ("cache_prompt", Value::Bool(false)),
        // The model's end of text is not an end where the length is
        // pinned: the server keeps sampling to `n_predict` and says it
        // stopped at the limit, which the caller reads back (B-396).
        ("ignore_eos", Value::Bool(pinned)),
        // Whether the answer arrives token by token or whole. Asked for
        // only where somebody is watching it arrive: a probe that reads
        // nothing until the end has no use for the pieces, and the
        // whole-answer path is the one every other caller runs on
        // (B-454, D48).
        ("stream", Value::Bool(streaming)),
    ])
}

#[cfg(test)]
mod request_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use mcf_core::configuration::Thousandths;

    use super::{Prompt, base64, completion_body};
    use crate::generation::{Draw, Stated, Truncation, Whose};

    /// Two engines never share a marker, and a marker is a thing a prompt
    /// would have to guess.
    #[test]
    fn a_marker_is_fresh_for_each_engine() {
        let one = super::fresh_marker();
        let two = super::fresh_marker();
        assert_ne!(one, two);
        assert!(
            one.starts_with("<__media_") && one.ends_with("__>"),
            "{one}"
        );
        assert_eq!(one.len(), "<__media_".len() + 16 + "__>".len());
    }

    /// A log-probability as the engine writes it is read to millibits:
    /// a nat is 1.443 bits, an exponent is honoured, and a token near
    /// certainty rounds to nought (D52).
    #[test]
    fn a_log_probability_is_read_to_millibits() {
        use mcf_record::json::{Value, parse};
        let read = |text: &str| super::millibits(&parse(text).unwrap_or(Value::Null));
        assert_eq!(read("-0.693147"), Some(-999));
        assert_eq!(read("-2"), Some(-2885));
        assert_eq!(read("-1.5e-05"), Some(0));
        assert_eq!(read("-1.5e+01"), Some(-21640));
        assert_eq!(read("0"), Some(0));
        assert_eq!(read("\"text\""), None);
    }

    /// The alphabet and the padding, against the reference values.
    #[test]
    fn bytes_are_written_in_the_engines_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xef, 0xbf]), "/++/");
    }

    /// A picture goes as text with the marker and the bytes beside it, in
    /// the shape the engine reads (B-452).
    #[test]
    fn a_picture_goes_as_text_with_its_bytes_beside_it() {
        let body = completion_body(
            Prompt::Shown {
                text: "look: <__media__>what is it",
                picture: b"foo",
            },
            8,
            Draw::greedy(0),
            false,
            false,
        )
        .to_line();
        assert!(
            body.contains(
                r#""prompt":{"multimodal_data":["Zm9v"],"prompt_string":"look: <__media__>what is it"}"#
            ),
            "{body}"
        );
    }

    /// **Nothing is left for the server to fill in** (B-440, F157). A draw
    /// with nothing declared sends the three cuts as *off* — nought, one and
    /// nought — rather than sending nothing, which the server would read as
    /// *use the file's, then mine*.
    #[test]
    fn a_draw_with_nothing_declared_states_the_cut_as_off() {
        let draw = Draw {
            seed: 7,
            temperature: Thousandths(700),
            truncation: Truncation::OFF,
        };
        let body = completion_body(Prompt::Identifiers(&[1, 2]), 8, draw, false, false).to_line();
        assert!(body.contains(r#""top_k":0"#), "{body}");
        assert!(body.contains(r#""top_p":1.000"#), "{body}");
        assert!(body.contains(r#""min_p":0.000"#), "{body}");
        assert!(body.contains(r#""temperature":0.700"#), "{body}");
    }

    /// What the file declared is sent as read, and what it did not is off —
    /// parameter by parameter.
    #[test]
    fn what_the_file_declared_is_sent_as_read() {
        let draw = Draw {
            seed: 7,
            temperature: Thousandths(700),
            truncation: Truncation {
                top_k: Stated::Declared(20),
                top_p: Stated::Declared(Thousandths(950)),
                min_p: Stated::Off,
                whose: Whose::File,
            },
        };
        let body = completion_body(Prompt::Identifiers(&[1]), 8, draw, false, false).to_line();
        assert!(body.contains(r#""top_k":20"#), "{body}");
        assert!(body.contains(r#""top_p":0.950"#), "{body}");
        assert!(body.contains(r#""min_p":0.000"#), "{body}");
    }

    /// A greedy draw states the cut too: the request is the same shape
    /// whatever the temperature, so nothing is filled in either way.
    #[test]
    fn a_greedy_draw_states_the_cut_as_well() {
        let body =
            completion_body(Prompt::Identifiers(&[1]), 8, Draw::greedy(0), false, false).to_line();
        assert!(body.contains(r#""temperature":0,"#), "{body}");
        assert!(body.contains(r#""top_k":0"#), "{body}");
    }
}

#[cfg(test)]
mod ranking_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::ranked_in;

    /// The shape the provisioned server actually sends.
    const ANSWERED: &str = r#"{"content":"x","completion_probabilities":[{"id":11,"token":",",
        "top_logprobs":[{"id":12095,"token":" Paris","logprob":-1.15},
                        {"id":7407,"token":" located","logprob":-2.55},
                        {"id":1128,"token":" what","logprob":-3.25}]}]}"#;

    /// Counting from one, because a reader saying *its second choice* means the
    /// second and not the one after the first.
    #[test]
    fn a_token_in_the_list_is_ranked_from_one() {
        assert_eq!(ranked_in(ANSWERED, 12095).0, Some(1));
        assert_eq!(ranked_in(ANSWERED, 7407).0, Some(2));
        assert_eq!(ranked_in(ANSWERED, 1128).0, Some(3));
    }

    /// The engine's own figure travels as the text it wrote.
    ///
    /// The record has no floating-point variant on purpose: a log probability
    /// parsed into a float is one division away from a measurement nobody
    /// checked.
    #[test]
    fn the_engines_figure_is_carried_and_not_computed() {
        let (_, said) = ranked_in(ANSWERED, 12095);
        assert_eq!(said.as_deref(), Some("-1.15"));
    }

    /// A token the engine did not list is *outside the list*, not rank zero
    /// and not absent (A7).
    #[test]
    fn a_token_outside_the_list_has_no_rank() {
        assert_eq!(ranked_in(ANSWERED, 999_999), (None, None));
    }

    /// An answer that is not a completion is not a ranking.
    #[test]
    fn nothing_is_read_out_of_something_that_is_not_one() {
        assert_eq!(ranked_in("not json at all", 1), (None, None));
        assert_eq!(ranked_in(r#"{"error":"context is full"}"#, 1), (None, None));
    }
}

#[cfg(test)]
mod tokenize_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use super::{Token, tokens_in};

    /// The server's shape, read as identifiers with their spellings.
    #[test]
    fn the_servers_tokens_are_read_with_their_spellings() {
        let answer =
            r#"{"tokens":[{"id":151644,"piece":"<|im_start|>"},{"id":872,"piece":" user"}]}"#;
        assert_eq!(
            tokens_in(answer).unwrap(),
            vec![
                Token {
                    id: 151_644,
                    piece: "<|im_start|>".to_owned()
                },
                Token {
                    id: 872,
                    piece: " user".to_owned()
                },
            ]
        );
    }

    /// A piece that is not text on its own is written byte by byte, the way
    /// the engine writes a byte token, and not dropped (A1).
    #[test]
    fn a_piece_that_is_not_text_is_written_as_bytes() {
        let answer = r#"{"tokens":[{"id":5,"piece":[226,128]},{"id":6,"piece":[168]}]}"#;
        let pieces: Vec<String> = tokens_in(answer)
            .unwrap()
            .into_iter()
            .map(|held| held.piece)
            .collect();
        assert_eq!(pieces, vec!["<0xE2><0x80>".to_owned(), "<0xA8>".to_owned()]);
    }

    /// An error, a list without spellings and something that is not JSON are
    /// each a failure with the server's words in it, never an empty prompt.
    #[test]
    fn what_is_not_a_token_list_is_a_failure_and_not_an_empty_prompt() {
        for answer in [
            "not json",
            r#"{"error":{"message":"model is loading"}}"#,
            r#"{"tokens":[1,2,3]}"#,
            r#"{"tokens":[{"id":"x","piece":"a"}]}"#,
        ] {
            assert!(tokens_in(answer).is_err(), "{answer}");
        }
        assert_eq!(tokens_in(r#"{"tokens":[]}"#).unwrap(), Vec::<Token>::new());
    }
}

#[cfg(test)]
mod streaming_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use std::io::Write as _;
    use std::os::unix::net::{UnixListener, UnixStream};

    use super::{Link, Reach, plain_request, sent_and_streamed};

    /// The events of a stream as the engine frames them: each one a chunk
    /// of its own true length, and the zero chunk that ends the body.
    fn chunked(events: &[&str]) -> String {
        let mut out = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                       Transfer-Encoding: chunked\r\n\r\n"
            .to_owned();
        for event in events {
            let piece = format!("data: {event}\n\n");
            let size = format!("{:x}\r\n", piece.len());
            out.push_str(&size);
            out.push_str(&piece);
            out.push_str("\r\n");
        }
        out.push_str("0\r\n\r\n");
        out
    }

    /// A server that writes one answer and closes, on a socket of its own.
    fn answering(name: &str, answer: String) -> (Link, std::thread::JoinHandle<()>) {
        let socket = std::env::temp_dir().join(format!("mcf-stream-{}-{name}", std::process::id()));
        let _gone = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the socket binds");
        let serving = std::thread::spawn(move || {
            if let Ok((mut connection, _)) = listener.accept() {
                // The request, read and thrown away: this server answers the
                // same thing to anything.
                let mut byte = [0_u8; 4096];
                let _read = std::io::Read::read(&mut connection, &mut byte);
                let _written = connection.write_all(answer.as_bytes());
                let _flushed = connection.flush();
            }
        });
        let connection = UnixStream::connect(&socket).expect("a connection");
        let _gone = std::fs::remove_file(&socket);
        (Link::Socket(connection), serving)
    }

    /// An answer sent in chunked pieces arrives as its events, in order.
    #[test]
    fn events_arrive_as_the_engine_writes_them() {
        let answer = chunked(&[
            "{\"content\":\"one\"}",
            "{\"content\":\"two\"}",
            "{\"stop\":true}",
        ]);
        let (connection, serving) = answering("chunked", answer);
        let mut seen = Vec::new();
        let last = sent_and_streamed(&connection, "POST", "/completion", "{}", &mut |event| {
            seen.push(event.to_owned());
        })
        .unwrap_or_else(|why| panic!("the stream is read: {why:?}"));
        let _joined = serving.join();
        assert_eq!(
            seen,
            vec![
                "{\"content\":\"one\"}".to_owned(),
                "{\"content\":\"two\"}".to_owned(),
                "{\"stop\":true}".to_owned(),
            ]
        );
        assert_eq!(last, "{\"stop\":true}");
    }

    /// A refusal comes back as one ordinary body and reaches the same
    /// reader: the engine says no before it says anything else.
    #[test]
    fn a_body_that_is_not_a_stream_is_one_event() {
        let body = "{\"error\":{\"message\":\"no\"}}";
        let answer = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (connection, serving) = answering("plain", answer);
        let mut seen = Vec::new();
        let last = sent_and_streamed(&connection, "POST", "/completion", "{}", &mut |event| {
            seen.push(event.to_owned());
        })
        .unwrap_or_else(|why| panic!("the body is read: {why:?}"));
        let _joined = serving.join();
        assert_eq!(seen, vec![body.to_owned()]);
        assert_eq!(last, body);
    }

    /// A server reached by its port gets the same request as one reached by
    /// its socket, with the key it was hosted under where one was set: what
    /// lets a message from the window reach a hosted server (B-480).
    #[test]
    fn a_port_is_reached_with_its_key() {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind((crate::hosting::LOOPBACK, 0)).expect("a port");
        let port = listener.local_addr().expect("an address").port();
        let serving = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().expect("a caller");
            let mut request = [0_u8; 1024];
            let read = connection.read(&mut request).unwrap_or(0);
            let heard =
                String::from_utf8_lossy(request.get(..read).unwrap_or_default()).into_owned();
            let body = "{\"status\":\"ok\"}";
            let _written = write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            heard
        });
        let reach = Reach::Port {
            port,
            key: Some("s3cret".to_owned()),
        };
        let answer = plain_request(&reach, "GET", "/health", None)
            .unwrap_or_else(|why| panic!("the port answers: {why:?}"));
        let heard = serving.join().expect("the server's side");
        assert_eq!(answer, "{\"status\":\"ok\"}");
        assert!(heard.starts_with("GET /health HTTP/1.1\r\n"), "{heard}");
        assert!(
            heard.contains("Authorization: Bearer s3cret\r\n"),
            "{heard}"
        );
        assert_eq!(reach.said(), format!("127.0.0.1:{port}"));
        assert!(reach.appeared());
    }
}
