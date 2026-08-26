//! `mcf serve` and `mcf stop`: the daemon, from the command line (B-030,
//! B-210, D1).
//!
//! **What `serve` is at M2's start, and what it is not.** It starts the process
//! D1 settled MCF is, and that process cannot serve a model — there is no
//! engine. The command is still worth having and worth this name: it is what
//! `mcf pull` will hand a model to, what `mcf run` will ask, and what §3.13's
//! idle rule is *about*. A daemon that appeared only when it could do
//! everything would be a daemon nobody could measure the idle cost of.
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
use std::path::PathBuf;
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

    // Printed before serving rather than after, because after is never: the
    // next thing this process does is block in `accept` until somebody asks it
    // for something.
    let recovered = daemon.recovered();
    println!(
        "mcf is up on {}\n  \
         recovered {} record entr{} and {} model file{}{}\n  \
         it cannot serve a model yet: no inference engine is vendored (B-320)\n  \
         idle costs nothing — this process is blocked in accept until asked (§3.13)",
        daemon.socket().display(),
        recovered.entries,
        if recovered.entries == 1 { "y" } else { "ies" },
        recovered.held,
        if recovered.held == 1 { "" } else { "s" },
        match &recovered.unreadable {
            Some(what) => format!("\n  PART OF THE RECORD COULD NOT BE READ: {what}"),
            None => String::new(),
        }
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
                text: format!("mcf: the daemon would not say\n  {}", answer.body.to_line()),
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
    }
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
            lines.push(format!("  holding {} model file(s)", models.len()));
            for model in models {
                if let Some(path) = model.get("path").and_then(Value::as_text) {
                    lines.push(format!("    {path}"));
                }
            }
        }
        Ok(_) | Err(_) => lines.push("  what it is holding could not be read".to_owned()),
    }

    Response {
        text: lines.join("\n"),
        served: true,
    }
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
                answer.body.to_line()
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
