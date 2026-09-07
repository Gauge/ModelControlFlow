//! `mcf serve` and `mcf stop`: the daemon, from the command line (B-030,
//! B-210, D1).
//!
//! **What `serve` is.** It starts the process D1 settled MCF is: what `mcf
//! pull` hands a model to, what `mcf run` asks, what `mcf host` holds a model
//! in, and what §3.13's idle rule is *about*.
//!
//! This paragraph said *that process cannot serve a model — there is no
//! engine*, and went on saying it after `mcf provision` built one and `mcf
//! host` began serving models on a port. What a daemon cannot do is a state,
//! not a property: `cannot()` computes it from what is actually installed, and
//! says nothing where an engine is. A sentence in prose cannot do that, which
//! is why the one here now describes what `serve` is for rather than what this
//! milestone had not reached yet (F135).
//!
//! **`stop` is the other half of A26.** A process that can only be killed is a
//! process that leaves no account of why it stopped; `stop` asks, gets an
//! answer, and the daemon says what it was told. B-210 grows this into draining
//! work and releasing held resources when there is work to drain.
//!
//! **Where it listens is where this user can reach and nobody else can.**
//! `$XDG_RUNTIME_DIR/mcf/control.sock` — a directory the platform makes for one
//! user and clears at logout — falling back to the data home, and refusing when
//! neither is set rather than inventing a path (A7, B-036).

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};
use mcf_serve::daemon::{Daemon, Places, Stopped};

use crate::Response;
use crate::models;

/// How long a client waits for the daemon to answer.
///
/// Longer than the daemon's own patience with a silent client, so that a
/// command which arrives while another connection is being waited out is
/// delayed rather than refused (B7's shape: bounded, not absent).
const PATIENCE: Duration = Duration::from_secs(10);

/// Where the control socket lives.
///
/// `None` when neither `XDG_RUNTIME_DIR` nor a data home is set, which is the
/// same answer the record and the model store give in the same situation: MCF
/// does not invent a place to put something (A7).
#[must_use]
pub(crate) fn socket_path() -> Option<PathBuf> {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
        && runtime.is_absolute()
    {
        return Some(runtime.join("mcf").join("control.sock"));
    }
    // The data home is not where a socket belongs — it is for things that
    // outlive a login — but it is somewhere this user owns, and a daemon that
    // refused to start on a machine with no runtime directory would be refusing
    // over a detail of the platform's tidiness.
    models::default_root().map(|models| models.parent().unwrap_or(&models).join("control.sock"))
}

/// Where a daemon should look for everything.
fn places() -> Option<Places> {
    Some(Places {
        socket: socket_path()?,
        journal: mcf_record::journal::default_path()?,
        models: models::default_root()?,
    })
}

/// Starts the daemon and stays there.
/// Makes sure a daemon is up, starting one where there is not.
///
/// **A person opening a window expects the tools to be working.** MCF's
/// surfaces are clients of a daemon, which is right — but it was the operator
/// who had to know that, and a console that draws *MCF is not running* at
/// somebody who has just opened it is a console reporting its own architecture
/// as their problem.
///
/// So a surface asks for a daemon and gets one. If something is already
/// listening, that is the daemon and nothing is started. If a socket is there
/// with nothing behind it — a daemon that was killed, which is a state this
/// machine reached more than once — the stale file is cleared first, because
/// otherwise the new daemon refuses to bind over it.
///
/// Returns what went wrong in words, or nothing where a daemon is now there.
pub(crate) fn ensure_running(socket: &Path) -> Option<String> {
    if UnixStream::connect(socket).is_ok() {
        return None;
    }
    // Something is at that path and nothing is behind it.
    if socket.exists() {
        let _cleared = std::fs::remove_file(socket);
    }
    let Ok(binary) = std::env::current_exe() else {
        return Some("MCF could not find its own program to start a daemon with".to_owned());
    };
    // The same binary, asked to serve. Detached, so closing the window does not
    // take the daemon with it — a model held resident should outlive the thing
    // that was looking at it. Under a memory cap where the system offers one
    // (B-561): a transient scope of the person's own session manager, so
    // that what the kernel reclaims and kills under pressure is MCF's
    // scope and not the desktop's (F243).
    let mut command = match memory_cap() {
        Some(cap) => {
            let mut scoped = Command::new(SYSTEMD_RUN);
            scoped
                .arg("--user")
                .arg("--scope")
                .arg("--quiet")
                .arg(format!("--unit=mcf-serve-{}", std::process::id()))
                .arg(format!("--property=MemoryMax={cap}"))
                .arg("--property=MemorySwapMax=0")
                .arg("--")
                .arg(&binary);
            scoped
        }
        None => Command::new(&binary),
    };
    let started = command
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(error) = started {
        return Some(format!("a daemon could not be started: {error}"));
    }
    // Binding is quick but not instant, and answering before it is ready would
    // be reporting a failure that has not happened.
    for _ in 0..100 {
        if UnixStream::connect(socket).is_ok() {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Some("a daemon was started and did not begin listening".to_owned())
}

/// What holds the daemon's memory, in one line: the cap its scope carries
/// and whether the kernel agreed to take it first.
#[expect(clippy::integer_division, reason = "whole gigabytes are the unit said")]
fn memory_line(cap: Option<u64>, dies_first: &Result<(), String>) -> String {
    let capped = match cap {
        Some(bytes) => format!(
            "held under a memory cap of {} GiB with its servers",
            bytes / (1024 * 1024 * 1024)
        ),
        None => "under no memory cap: the system offered no scope to hold one".to_owned(),
    };
    let first = match dies_first {
        Ok(()) => "and first to go when memory runs out",
        Err(_) => "and the kernel would not let it go first",
    };
    format!("{capped}, {first}")
}

/// The session manager's runner, where the system has one.
const SYSTEMD_RUN: &str = "/usr/bin/systemd-run";

/// The most memory the daemon and its servers may hold together, in bytes,
/// where this machine offers a scope to hold them to; `None` where it does
/// not, or where the machine's memory is not known (A7).
///
/// The cap leaves the desktop a reserve: an eighth of the machine, and
/// never less than eight gigabytes. Under it the kernel reclaims MCF's own
/// pages first and, at the cap, kills inside the scope — so a model that
/// does not fit takes the daemon down and not the session (B-561, F243).
#[expect(
    clippy::integer_division,
    reason = "an eighth of the machine, whole bytes"
)]
fn memory_cap() -> Option<u64> {
    let manager = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|runtime| Path::new(&runtime).join("systemd"))
        .is_some_and(|held| held.is_dir());
    if !manager || !Path::new(SYSTEMD_RUN).is_file() {
        return None;
    }
    let total = match mcf_core::hardware::Machine::read().memory.total {
        mcf_core::attested::Attested::Known(bytes) => bytes.0,
        mcf_core::attested::Attested::Unknown => return None,
    };
    let reserve = (total / 8).max(RESERVE_AT_LEAST);
    total.checked_sub(reserve).filter(|cap| *cap > 0)
}

/// The least the desktop keeps for itself beside a capped daemon.
const RESERVE_AT_LEAST: u64 = 8 * 1024 * 1024 * 1024;

/// Asks the kernel to take this process, and every server it starts, before
/// anything else when memory runs out: the highest adjustment there is,
/// which a process may set on itself without any right (B-561, F243). What
/// the kernel does with it is said back, so that a machine that refused is
/// not reported as one that agreed (A7).
fn prefer_to_die_first() -> Result<(), String> {
    std::fs::write("/proc/self/oom_score_adj", "1000\n").map_err(|error| error.to_string())
}

/// The memory cap this process runs under, in bytes, read from its own
/// control group; `None` where there is none or it cannot be read.
fn cap_in_force() -> Option<u64> {
    let groups = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = groups
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim();
    let held = std::fs::read_to_string(format!("/sys/fs/cgroup{path}/memory.max")).ok()?;
    held.trim().parse::<u64>().ok()
}

/// What engines this machine has, in one line a person can read.
///
/// The daemon used to say "no vendored engine yet" whatever was on the disk,
/// which is how two provisioned engines sat here while it reported none. This
/// reads the disk instead of repeating a sentence.
fn engines_line() -> String {
    const NONE: &str = "no engine is installed yet — `mcf provision llama.cpp` builds one";
    let Some(models) = crate::models::default_root() else {
        return NONE.to_owned();
    };
    let home = models.parent().unwrap_or(&models);
    let engines = mcf_serve::engines::discover(home);
    if engines.is_empty() {
        return NONE.to_owned();
    }
    let cards: usize = engines
        .iter()
        .map(|engine| {
            engine
                .devices(None)
                .unwrap_or_default()
                .iter()
                .filter(|device| device.kind == mcf_serve::engines::Kind::Gpu)
                .count()
        })
        .sum();
    let names: Vec<&str> = engines.iter().map(|engine| engine.name.as_str()).collect();
    if cards == 0 {
        format!("{} ready, on the processor", names.join(" and "))
    } else {
        format!(
            "{} ready, on the processor and {cards} card{}",
            names.join(" and "),
            if cards == 1 { "" } else { "s" }
        )
    }
}

pub(crate) fn run() -> Response {
    let Some(places) = places() else {
        return Response {
            text: "mcf: there is nowhere to run — neither XDG_RUNTIME_DIR, XDG_DATA_HOME nor \
                   HOME is set, and MCF does not invent a place to put a socket (A7)"
                .to_owned(),
            served: false,
        };
    };

    let mut daemon = match Daemon::start(places) {
        Ok(daemon) => daemon,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the daemon did not start", &failure),
                served: false,
            };
        }
    };
    let dies_first = prefer_to_die_first();
    let cap = cap_in_force();

    // Printed before serving rather than after, because after is never: the
    // next thing this process does is block in `accept` until somebody asks it
    // for something.
    let recovered = daemon.recovered();
    // No rule identifiers here. This is the first thing a person ever sees from
    // MCF, and a citation in it sends them to a document they have never read
    // to explain a sentence they could have understood. The rules are cited in
    // the code and carried in the record, which is where a citation is useful.
    println!(
        "mcf is up on {}\n  \
         recovered {} record entr{} and {} model file{}{}{}\n  \
         {}\n  \
         {}\n  \
         it costs nothing while nobody is asking",
        daemon.socket().display(),
        recovered.entries,
        if recovered.entries == 1 { "y" } else { "ies" },
        recovered.held,
        if recovered.held == 1 { "" } else { "s" },
        match &recovered.unreadable {
            Some(what) => format!("\n  PART OF THE RECORD COULD NOT BE READ: {what}"),
            None => String::new(),
        },
        engines_stopped_lines(&recovered.engines_stopped),
        engines_line(),
        memory_line(cap, &dies_first),
    );

    match daemon.serve() {
        Stopped::Asked { reason } => Response {
            text: format!(
                "mcf stopped, because: {}",
                if reason.is_empty() {
                    "no reason was given"
                } else {
                    &reason
                }
            ),
            served: true,
        },
        Stopped::Broken { failure } => Response {
            text: crate::say::refusal("the daemon stopped because it could not go on", &failure),
            served: false,
        },
    }
}

/// What another program can reach.
///
/// Separate from the rest of a status because it is a different kind of fact:
/// everything else is what MCF holds for itself, and this is what it holds for
/// anybody else. A status that omitted it would leave the most consequential
/// thing about the process to be found by looking at the ports (§6.12, B-418).
fn exposed(status: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    match status.get("hosting").and_then(|held| held.get("hosting")) {
        Some(Value::Text(model)) => {
            let address = status
                .get("hosting")
                .and_then(|held| held.get("address"))
                .and_then(Value::as_text)
                .unwrap_or("somewhere MCF did not say");
            lines.push(format!("  hosting: {model}"));
            lines.push(format!(
                "    reachable at {address}, from this computer only"
            ));
            let changed = status
                .get("hosting")
                .and_then(|held| held.get("changed"))
                .and_then(Value::as_list)
                .map(<[Value]>::to_vec)
                .unwrap_or_default();
            for one in &changed {
                if let Some(said) = one.as_text() {
                    lines.push(format!("    {said}"));
                }
            }
        }
        _ => lines.push(
            "  hosting: nothing — `mcf host <model>` holds one where a program can reach it"
                .to_owned(),
        ),
    }
    lines
}

/// Asks a running daemon what it is and what it is holding.
///
/// A22: the headless surface is the complete one. A daemon that answered
/// questions no command could ask would be a capability reachable only through
/// a client, which is what that rule forbids.
pub(crate) fn status() -> Response {
    let Some(socket) = socket_path() else {
        return Response {
            text: "mcf: there is nowhere to look for a daemon — neither XDG_RUNTIME_DIR, \
                   XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };

    let status = match ask(&socket, &Request::Status) {
        Ok(answer) if answer.served => answer.body,
        Ok(answer) => {
            return Response {
                text: format!(
                    "mcf: the daemon would not say\n  {}",
                    crate::say::refused_because(&answer.body)
                ),
                served: false,
            };
        }
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };

    let mut lines = vec![format!("mcf is up on {}", socket.display())];
    if let Some(build) = status.get("build") {
        lines.push(format!("  build: {}", one_line(build)));
    }
    if let Some(up) = status.get("up_nanoseconds").and_then(Value::as_integer) {
        // Seconds, computed without dividing: the workspace denies integer
        // division because a truncated quotient is a silently wrong number.
        lines.push(format!(
            "  up for {} seconds",
            up.checked_div(1_000_000_000).unwrap_or(0)
        ));
    }
    if let Some(recovered) = status.get("recovered") {
        recovered_lines(recovered, &mut lines);
    }
    match status.get("resident") {
        Some(resident) if resident.get("path").is_some() => lines.push(format!(
            "  resident: {} — {} bytes dequantized, since {} (held until displaced or stopped)",
            resident.get("path").and_then(Value::as_text).unwrap_or("?"),
            resident
                .get("bytes_dequantized")
                .and_then(Value::as_integer)
                .unwrap_or(0),
            resident
                .get("since")
                .and_then(Value::as_text)
                .unwrap_or("?"),
        )),
        _ => lines.push(
            "  resident: nothing — the first generation loads its model and holds it".to_owned(),
        ),
    }
    lines.extend(exposed(&status));
    lines.extend(what_is_running(&status));
    for cannot in status
        .get("cannot")
        .and_then(Value::as_list)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_text)
    {
        lines.push(format!("  it cannot: {cannot}"));
    }

    // What it is holding, asked separately because they are separate questions
    // and a client that wanted one should not be sent the other.
    match ask(&socket, &Request::Holding) {
        Ok(answer) if answer.served => {
            let models = answer
                .body
                .get("models")
                .and_then(Value::as_list)
                .map(<[Value]>::to_vec)
                .unwrap_or_default();
            lines.extend(what_is_held(&models));
        }
        Ok(_) | Err(_) => lines.push("  what it is holding could not be read".to_owned()),
    }

    Response {
        text: lines.join("\n"),
        served: true,
    }
}

/// What the daemon is carrying right now (D48, B-460).
///
/// A daemon in the middle of a long turn answers status rather than making
/// the caller wait behind the turn; what it says is what it is carrying, for
/// whom, for how long, and — where the engine is a served one — how far the
/// engine has read. A busy daemon that said nothing about being busy would
/// leave the operator to guess whether the run they started an hour ago is
/// still going.
fn what_is_running(status: &Value) -> Vec<String> {
    let running = status
        .get("running")
        .and_then(Value::as_list)
        .unwrap_or_default();
    let mut lines = Vec::new();
    for carried in running {
        let what = carried.get("doing").and_then(Value::as_text).unwrap_or("?");
        let model = carried.get("model").and_then(Value::as_text).unwrap_or("?");
        let seconds = carried
            .get("nanoseconds")
            .and_then(Value::as_integer)
            .and_then(|nanos| nanos.checked_div(1_000_000_000))
            .unwrap_or(0);
        let engine = match carried.get("engine") {
            Some(engine) if engine.get("read").is_some() => {
                let figure = |key: &str| engine.get(key).and_then(Value::as_integer).unwrap_or(0);
                format!(
                    "; the engine has read {} of {} identifiers and produced {}",
                    figure("read"),
                    figure("of"),
                    figure("produced")
                )
            }
            _ => String::new(),
        };
        lines.push(format!(
            "  running: {what} on {model}, {seconds} seconds in{engine}"
        ));
    }
    lines
}

/// What the daemon is holding, counted the way `mcf list` counts it.
///
/// **Models and companions apart.** B-422 settled that a projector belongs to
/// a model rather than being one, and `mcf list` says so — while this said
/// *holding 11 model file(s)*, so two surfaces gave two counts of one store
/// and neither mentioned the other. The daemon has always sent the
/// distinction on every entry; this was the half that dropped it (B-072,
/// F143).
fn what_is_held(models: &[Value]) -> Vec<String> {
    let companions = models
        .iter()
        .filter(|held| matches!(held.get("companion"), Some(Value::Bool(true))))
        .count();
    let held = models.len().saturating_sub(companions);
    let mut lines = vec![match companions {
        0 => format!("  holding {held} model(s)"),
        _ => format!(
            "  holding {held} model(s), and {companions} companion file(s) that belong to one \
             rather than being one"
        ),
    }];
    for model in models {
        if let Some(path) = model.get("path").and_then(Value::as_text) {
            lines.push(format!("    {path}"));
        }
    }
    lines
}

/// A value on one line, for a surface that is showing rather than recording.
fn one_line(value: &Value) -> String {
    match value.get("version").and_then(Value::as_text) {
        Some(version) => format!(
            "{version} ({})",
            value
                .get("target")
                .and_then(Value::as_text)
                .unwrap_or("an unnamed target")
        ),
        None => value.to_line(),
    }
}

/// Asks a running daemon to stop.
pub(crate) fn stop(reason: &str) -> Response {
    let Some(socket) = socket_path() else {
        return Response {
            text: "mcf: there is nowhere to look for a daemon — neither XDG_RUNTIME_DIR, \
                   XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };

    match ask(
        &socket,
        &Request::Stop {
            reason: reason.to_owned(),
        },
    ) {
        Ok(answer) if answer.served => Response {
            text: format!(
                "asked mcf to stop, because: {}\n  it said it is stopping",
                if reason.is_empty() {
                    "no reason was given"
                } else {
                    reason
                }
            ),
            served: true,
        },
        Ok(answer) => Response {
            text: format!(
                "mcf: the daemon refused to stop\n  {}",
                crate::say::refused_because(&answer.body)
            ),
            served: false,
        },
        Err(text) => Response {
            text,
            served: false,
        },
    }
}

/// Asks a running daemon one thing.
///
/// The failure is text rather than a classified failure because what goes wrong
/// here is *there is nothing there*, which is a fact about this machine rather
/// than about MCF — and saying it plainly beats classifying it (A2's spirit:
/// what matters is that the operator is told).
fn ask(socket: &std::path::Path, request: &Request) -> Result<Answer, String> {
    let mut connection = UnixStream::connect(socket).map_err(|error| {
        format!(
            "mcf: nothing is listening on {}\n  {error}\n  if MCF should be running, `mcf serve` \
             starts it",
            socket.display()
        )
    })?;
    let _deadline = connection.set_read_timeout(Some(PATIENCE));
    let _writing = connection.set_write_timeout(Some(PATIENCE));

    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("mcf: the request could not be sent\n  {error}"))?;

    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|error| format!("mcf: the daemon did not answer\n  {error}"))?;
    Answer::read(line.trim_end()).map_err(|failure| {
        format!("mcf: the daemon answered with something MCF cannot read\n  {failure}")
    })
}

/// What the daemon recovered at its start, as the status shows it.
fn recovered_lines(recovered: &Value, lines: &mut Vec<String>) {
    lines.push(format!(
        "  recovered {} record entries and {} model files",
        recovered
            .get("record_entries")
            .and_then(Value::as_integer)
            .unwrap_or(0),
        recovered
            .get("models_held")
            .and_then(Value::as_integer)
            .unwrap_or(0),
    ));
    if let Some(lost) = recovered.get("record_unreadable").and_then(Value::as_text) {
        lines.push(format!("  PART OF THE RECORD COULD NOT BE READ: {lost}"));
    }
    for stopped in recovered
        .get("engines_stopped")
        .and_then(Value::as_list)
        .unwrap_or_default()
    {
        lines.push(format!(
            "  stopped an engine server whose daemon was gone: {}",
            stopped_said(stopped)
        ));
    }
}

/// One line per engine server the daemon stopped at its start because the
/// daemon that started it was gone (B-574), each on its own line under the
/// recovery line; nothing where there were none, which is the ordinary case.
fn engines_stopped_lines(stopped: &[mcf_serve::orphans::Orphan]) -> String {
    stopped.iter().fold(String::new(), |mut lines, orphan| {
        lines.push_str("\n  stopped an engine server whose daemon was gone: ");
        lines.push_str(&orphan.said());
        lines
    })
}

/// The words for one stopped server, read back from its row in the record.
fn stopped_said(row: &Value) -> String {
    let text = |key: &str| row.get(key).and_then(Value::as_text).map(str::to_owned);
    let model = text("model").map_or_else(
        || "no model named".to_owned(),
        |model| {
            std::path::Path::new(&model)
                .file_name()
                .map_or(model.clone(), |name| name.to_string_lossy().into_owned())
        },
    );
    let held = row
        .get("resident_bytes")
        .and_then(Value::as_integer)
        .map_or_else(String::new, |bytes| {
            #[expect(
                clippy::integer_division,
                reason = "whole mebibytes are the unit shown"
            )]
            let whole = bytes / (1024 * 1024);
            format!(", {whole} MiB resident")
        });
    format!(
        "process {} holding {model} on {}{held}: {}",
        row.get("pid").and_then(Value::as_integer).unwrap_or(0),
        text("reach").unwrap_or_else(|| "nowhere it said".to_owned()),
        text("ended").unwrap_or_else(|| "MCF did not say".to_owned()),
    )
}
