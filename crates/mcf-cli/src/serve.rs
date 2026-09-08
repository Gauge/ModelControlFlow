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

const PATIENCE: Duration = Duration::from_secs(10);

#[must_use]
pub(crate) fn socket_path() -> Option<PathBuf> {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
        && runtime.is_absolute()
    {
        return Some(runtime.join("mcf").join("control.sock"));
    }
    models::default_root().map(|models| models.parent().unwrap_or(&models).join("control.sock"))
}

fn places() -> Option<Places> {
    Some(Places {
        socket: socket_path()?,
        journal: mcf_record::journal::default_path()?,
        models: models::default_root()?,
    })
}

pub(crate) fn ensure_running(socket: &Path) -> Option<String> {
    if UnixStream::connect(socket).is_ok() {
        return None;
    }
    if socket.exists() {
        let _cleared = std::fs::remove_file(socket);
    }
    let Ok(binary) = std::env::current_exe() else {
        return Some("MCF could not find its own program to start a daemon with".to_owned());
    };
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
    for _ in 0..100 {
        if UnixStream::connect(socket).is_ok() {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Some("a daemon was started and did not begin listening".to_owned())
}

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

const SYSTEMD_RUN: &str = "/usr/bin/systemd-run";

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

const RESERVE_AT_LEAST: u64 = 8 * 1024 * 1024 * 1024;

fn prefer_to_die_first() -> Result<(), String> {
    std::fs::write("/proc/self/oom_score_adj", "1000\n").map_err(|error| error.to_string())
}

fn cap_in_force() -> Option<u64> {
    let groups = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = groups
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?
        .trim();
    let held = std::fs::read_to_string(format!("/sys/fs/cgroup{path}/memory.max")).ok()?;
    held.trim().parse::<u64>().ok()
}

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

    let recovered = daemon.recovered();
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

fn one_line(value: &Value) -> String {
    let text = |key: &str| value.get(key).and_then(Value::as_text);
    match text("version") {
        Some(version) => {
            let revision = text("revision").map_or_else(String::new, |held| {
                format!(" · {}", held.chars().take(7).collect::<String>())
            });
            format!(
                "{version}{revision} ({})",
                text("target").unwrap_or("an unnamed target")
            )
        }
        None => value.to_line(),
    }
}

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

pub(crate) fn ask(socket: &std::path::Path, request: &Request) -> Result<Answer, String> {
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

fn engines_stopped_lines(stopped: &[mcf_serve::orphans::Orphan]) -> String {
    stopped.iter().fold(String::new(), |mut lines, orphan| {
        lines.push_str("\n  stopped an engine server whose daemon was gone: ");
        lines.push_str(&orphan.said());
        lines
    })
}

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
