use std::io::{BufRead as _, Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use mcf_core::Failure;
use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
use mcf_record::json::{self, Value};

use crate::adapters::ProvisionedLlama;

pub const ATTEMPTS: usize = 600;

const BETWEEN: std::time::Duration = std::time::Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    Eos,
    Limit,
    Word,
    Other(String),
}

impl Stop {
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

#[derive(Debug, Clone)]
pub struct Completed {
    pub text: String,
    pub predicted: usize,
    pub evaluated: usize,
    pub stop: Stop,
    pub produced: Vec<usize>,
    pub timings: Option<Value>,
}

impl Completed {
    #[must_use]
    pub fn words(&self) -> &[usize] {
        match (&self.stop, self.produced.split_last()) {
            (Stop::Eos, Some((_, before))) => before,
            _ => &self.produced,
        }
    }
}

fn ready_on(port: u16) -> bool {
    got_on(port, "/health")
        .is_some_and(|said| said.contains("200 OK") && said.contains("\"status\":\"ok\""))
}

#[must_use]
pub fn metrics_on(port: u16) -> Option<String> {
    let said = got_on(port, "/metrics")?;
    let (_, body) = said.split_once("\r\n\r\n")?;
    Some(body.to_owned())
}

#[must_use]
pub fn metrics_via(reach: &Reach) -> Option<String> {
    page_via(reach, "/metrics")
}

fn page_via(reach: &Reach, path: &str) -> Option<String> {
    let said = match reach {
        Reach::Port { port, key } => got_on_with(*port, key.as_deref(), path)?,
        Reach::Socket(socket) => got_via(socket, path)?,
    };
    let (_, body) = said.split_once("\r\n\r\n")?;
    Some(body.to_owned())
}

#[must_use]
pub fn tokens_in_flight(reach: &Reach) -> Option<u64> {
    let answer = page_via(reach, "/slots")?;
    let Ok(Value::List(slots)) = json::parse(&answer) else {
        return None;
    };
    Some(
        slots
            .iter()
            .filter(|slot| {
                slot.get("is_processing")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .filter_map(|slot| {
                slot.get("next_token")
                    .and_then(|next| match next {
                        Value::List(items) => items.first(),
                        other => Some(other),
                    })
                    .and_then(|next| next.get("n_decoded"))
                    .and_then(Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
            })
            .fold(0, u64::saturating_add),
    )
}

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

fn got_on(port: u16, path: &str) -> Option<String> {
    got_on_with(port, None, path)
}

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

#[derive(Debug, Default)]
pub struct Progress {
    pub seen: AtomicBool,
    pub read: AtomicU64,
    pub of: AtomicU64,
    pub produced: AtomicU64,
}

impl Progress {
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Waiting<'a> {
    pub client: Option<&'a UnixStream>,
    pub told: Option<&'a UnixStream>,
    pub stopping: Option<&'a AtomicBool>,
    pub progress: Option<&'a Progress>,
}

impl Waiting<'_> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Closed {
    ClientLeft,
    Stopping,
}

const GLANCE: std::time::Duration = std::time::Duration::from_millis(250);

const TOLD_EVERY: std::time::Duration = std::time::Duration::from_secs(10);

struct Done {
    flag: std::sync::Mutex<bool>,
    wake: std::sync::Condvar,
}

impl Done {
    const fn new() -> Self {
        Self {
            flag: std::sync::Mutex::new(false),
            wake: std::sync::Condvar::new(),
        }
    }

    fn is_set(&self) -> bool {
        *self
            .flag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn set(&self) {
        *self
            .flag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        self.wake.notify_all();
    }

    fn wait(&self, within: std::time::Duration) -> bool {
        let held = self
            .flag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (held, _timed_out) = self
            .wake
            .wait_timeout_while(held, within, |set| !*set)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *held
    }
}

fn watched(reach: &Reach, request: &Link, waiting: Waiting<'_>, done: &Done) -> Option<Closed> {
    let client = waiting.client.and_then(|client| client.try_clone().ok());
    if let Some(client) = &client {
        let _mode = client.set_nonblocking(true);
    }
    let began = std::time::Instant::now();
    let mut last_told: Option<std::time::Instant> = None;
    let mut byte = [0_u8; 1];
    while !done.wait(GLANCE) {
        let gone = match client.as_ref().map(|client| (&*client).read(&mut byte)) {
            Some(Ok(0)) => true,
            Some(Err(error)) => !matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            Some(Ok(_)) | None => false,
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
                && !done.is_set()
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
            let held = figure(slot, "n_prompt_tokens").unwrap_or(0);
            let of = held.saturating_sub(produced).max(read);
            (read, of, produced)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Startup {
    pub attempts: usize,
    pub gpu_layers: u32,
    pub context: u64,
    pub projector: Option<PathBuf>,
    pub started: crate::declared::Started,
    pub threads: Option<u32>,
    pub batch: Option<u32>,
    pub ubatch: Option<u32>,
    pub parallel: Option<u32>,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Extras {
    pub cached: bool,
    pub json_schema: Option<Value>,
    pub ranked: usize,
}

#[derive(Debug)]
pub struct Served {
    child: Child,
    reach: Reach,
    last_words: std::sync::Arc<std::sync::Mutex<String>>,
    pub model: PathBuf,
    pub commit: String,
    pub window: u64,
    pub projector: Option<PathBuf>,
    pub media_marker: Option<String>,
    pub prefix: PathBuf,
    pub gpu_layers: u32,
    pub started: crate::declared::Started,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    Socket(PathBuf),
    Port { port: u16, key: Option<String> },
}

impl Reach {
    fn appeared(&self) -> bool {
        match self {
            Self::Socket(socket) => socket.exists(),
            Self::Port { .. } => true,
        }
    }

    #[must_use]
    pub fn said(&self) -> String {
        match self {
            Self::Socket(socket) => socket.display().to_string(),
            Self::Port { port, .. } => format!("{}:{port}", crate::hosting::LOOPBACK),
        }
    }
}

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

    fn shutdown(&self) {
        let _closed = match self {
            Self::Socket(stream) => stream.shutdown(std::net::Shutdown::Both),
            Self::Port(stream, _) => stream.shutdown(std::net::Shutdown::Both),
        };
    }

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
            .arg("--metrics")
            .arg("--ctx-size")
            .arg(context.to_string())
            .arg("-ngl")
            .arg(gpu_layers.to_string())
            .arg("--no-webui")
            .arg("--no-warmup")
            .arg("--special");
        if let Some(projector) = projector {
            command.arg("--mmproj").arg(projector);
        }
        command.args(started.arguments());
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

    fn wait_until_listening(&mut self, attempts: usize) -> Result<(), Failure> {
        for _ in 0..attempts {
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

    #[must_use]
    pub fn peak_resident_bytes(&self) -> Option<u64> {
        crate::adapters::peak_resident_of(self.child.id())
    }

    #[must_use]
    pub fn metrics(&self) -> Option<String> {
        metrics_via(&self.reach)
    }

    #[must_use]
    pub const fn reach(&self) -> &Reach {
        &self.reach
    }

    #[must_use]
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    #[must_use]
    pub fn resident_bytes(&self) -> Option<u64> {
        crate::adapters::resident_of(self.child.id())
    }

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
        interpret(&self.request_while("POST", "/completion", Some(&body), waiting)?)
            .map_err(|failure| self.with_last_words(failure))
    }

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
            return Err(self.with_last_words(Failure::new(
                Category::EngineExitMidstream,
                Attribution::Machine,
                Disposition::Aborted,
                Subsystem::new("mcf-serve::served"),
                "the provisioned server's answer had no body",
            )));
        }
        let mut completed = interpret(&end).map_err(|failure| self.with_last_words(failure))?;
        if completed.produced.is_empty() {
            completed.produced = produced;
        }
        if completed.text.is_empty() {
            completed.text = text;
        }
        Ok(completed)
    }

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
        Ok(ranked_in(&answered, wanted))
    }

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

    pub fn render_with_tools(&self, messages: Value, tools: Value) -> Result<String, Failure> {
        let body = Value::map([("messages", messages), ("tools", tools)]).to_line();
        let answered = self.request("POST", "/apply-template", Some(&body))?;
        text_in(
            &answered,
            "prompt",
            "the provisioned server's answer rendered no prompt",
        )
    }

    fn request(&self, method: &str, path: &str, body: Option<&str>) -> Result<String, Failure> {
        self.request_while(method, path, body, Waiting::NOBODY)
    }

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

#[derive(Debug)]
enum Interrupted {
    Connecting(std::io::Error),
    Sending(std::io::Error),
    Reading(std::io::Error),
    Closed(Closed),
}
use Interrupted::{Reading, Sending};

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

fn while_watched<T>(
    reach: &Reach,
    waiting: Waiting<'_>,
    work: impl FnOnce(&Link) -> Result<T, Interrupted>,
) -> Result<T, Interrupted> {
    let connection = Link::open(reach).map_err(Interrupted::Connecting)?;
    let done = Done::new();
    let (answer, closed) = std::thread::scope(|scope| {
        let watcher = waiting
            .watches_anything()
            .then(|| scope.spawn(|| watched(reach, &connection, waiting, &done)));
        let answer = work(&connection);
        done.set();
        let closed = watcher.and_then(|watcher| watcher.join().ok().flatten());
        (answer, closed)
    });
    match closed {
        Some(closed) => Err(Interrupted::Closed(closed)),
        None => answer,
    }
}

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
    let mut ending = [0_u8; 2];
    let _ended = reader.read_exact(&mut ending);
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

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

const LAST_WORDS: usize = 4096;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub id: usize,
    pub piece: String,
}

impl Token {
    #[must_use]
    pub fn from_bytes(id: usize, bytes: &[u8]) -> Self {
        Self {
            id,
            piece: spelled(bytes),
        }
    }
}

fn spelled(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    core::str::from_utf8(bytes).map_or_else(
        |_not_text| {
            bytes.iter().fold(String::new(), |mut out, byte| {
                let _written = write!(out, "<0x{byte:02X}>");
                out
            })
        },
        str::to_owned,
    )
}

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

#[must_use]
#[expect(
    clippy::integer_division,
    reason = "nanonats scaled to millibits; what is discarded is under a millibit"
)]
pub fn millibits(held: &Value) -> Option<i64> {
    let written = match held {
        Value::Integer(whole) => return Some(whole.saturating_mul(1_442_695) / 1_000),
        Value::ForeignNumber(_) => held.to_line(),
        _ => return None,
    };
    let (negative, digits) = written
        .strip_prefix('-')
        .map_or((false, written.as_str()), |rest| (true, rest));
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
        None => Stop::Other("nothing".to_owned()),
    };
    let timings = value.get("timings").and_then(|held| match held {
        Value::Map(fields) => {
            let mut fields = fields.clone();
            if let Some(kept) = value.get("tokens_cached") {
                let _added = fields.insert("tokens_cached".to_owned(), kept.clone());
            }
            Some(Value::Map(fields))
        }
        _ => None,
    });
    Ok(Completed {
        text: value
            .get("content")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned(),
        predicted: number("tokens_predicted"),
        timings,
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

#[derive(Debug, Clone)]
pub struct Live {
    pub child: u32,
    pub model: PathBuf,
    pub commit: String,
    pub window: u64,
    pub reach: Reach,
    pub spent_at_start: crate::power::Spent,
}

static LIVE: std::sync::OnceLock<std::sync::Mutex<Vec<Live>>> = std::sync::OnceLock::new();

fn live_list() -> &'static std::sync::Mutex<Vec<Live>> {
    LIVE.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

#[must_use]
pub fn live() -> Vec<Live> {
    live_list()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

impl Served {
    fn register(&self) {
        let entry = Live {
            child: self.child.id(),
            model: self.model.clone(),
            commit: self.commit.clone(),
            window: self.window,
            reach: self.reach.clone(),
            spent_at_start: crate::power::spent(),
        };
        let mut held = live_list()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.retain(|live| live.child != entry.child);
        held.push(entry);
        crate::power::watch();
    }

    fn unregister(&self) {
        let child = self.child.id();
        let mut held = live_list()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let was = held.len();
        held.retain(|live| live.child != child);
        if held.len() < was {
            crate::power::release();
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.unregister();
        let _killed = self.child.kill();
        let _waited = self.child.wait();
        if let Reach::Socket(socket) = &self.reach {
            let _gone = std::fs::remove_file(socket);
        }
    }
}

fn fresh_marker() -> String {
    use std::hash::{BuildHasher as _, Hasher as _};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    format!("<__media_{:016x}__>", hasher.finish())
}

#[derive(Debug, Clone, Copy)]
pub enum Prompt<'a> {
    Identifiers(&'a [usize]),
    Shown { text: &'a str, picture: &'a [u8] },
}

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
        (
            "temperature",
            if draw.is_greedy() {
                Value::Integer(0)
            } else {
                Value::exact_thousandths(draw.temperature)
            },
        ),
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
        ("return_tokens", Value::Bool(true)),
        ("cache_prompt", Value::Bool(false)),
        ("ignore_eos", Value::Bool(pinned)),
        ("stream", Value::Bool(streaming)),
    ])
}

#[cfg(test)]
mod request_tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use mcf_core::configuration::Thousandths;

    use super::{Prompt, base64, completion_body};
    use crate::generation::{Draw, Stated, Truncation, Whose};

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

    #[test]
    fn bytes_are_written_in_the_engines_alphabet() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xef, 0xbf]), "/++/");
    }

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
    #![allow(clippy::panic, clippy::expect_used)]

    use super::ranked_in;

    const ANSWERED: &str = r#"{"content":"x","completion_probabilities":[{"id":11,"token":",",
        "top_logprobs":[{"id":12095,"token":" Paris","logprob":-1.15},
                        {"id":7407,"token":" located","logprob":-2.55},
                        {"id":1128,"token":" what","logprob":-3.25}]}]}"#;

    #[test]
    fn a_token_in_the_list_is_ranked_from_one() {
        assert_eq!(ranked_in(ANSWERED, 12095).0, Some(1));
        assert_eq!(ranked_in(ANSWERED, 7407).0, Some(2));
        assert_eq!(ranked_in(ANSWERED, 1128).0, Some(3));
    }

    #[test]
    fn the_engines_figure_is_carried_and_not_computed() {
        let (_, said) = ranked_in(ANSWERED, 12095);
        assert_eq!(said.as_deref(), Some("-1.15"));
    }

    #[test]
    fn a_token_outside_the_list_has_no_rank() {
        assert_eq!(ranked_in(ANSWERED, 999_999), (None, None));
    }

    #[test]
    fn nothing_is_read_out_of_something_that_is_not_one() {
        assert_eq!(ranked_in("not json at all", 1), (None, None));
        assert_eq!(ranked_in(r#"{"error":"context is full"}"#, 1), (None, None));
    }
}

#[cfg(test)]
mod tokenize_tests {
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use super::{Token, tokens_in};

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
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use std::io::Write as _;
    use std::os::unix::net::{UnixListener, UnixStream};

    use super::{Link, Reach, plain_request, sent_and_streamed};

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

    fn answering(name: &str, answer: String) -> (Link, std::thread::JoinHandle<()>) {
        let socket = std::env::temp_dir().join(format!("mcf-stream-{}-{name}", std::process::id()));
        let _gone = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the socket binds");
        let serving = std::thread::spawn(move || {
            if let Ok((mut connection, _)) = listener.accept() {
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
