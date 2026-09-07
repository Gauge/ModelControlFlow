//! Engine servers whose daemon is gone, found and stopped when the next daemon
//! starts (B-574, A27).
//!
//! **What was observed.** A daemon that ends by signal — `SIGTERM` from a
//! scope being stopped, `SIGKILL` from the kernel's memory killer, which
//! B-561 arranged for the daemon to draw first — never reaches the `Drop` that
//! stops its engine server. The server is reparented to the session's
//! manager and carries on holding its model: nine of them, from daemons
//! stopped during one afternoon's work, sat on this machine holding twenty
//! gigabytes between them, and nothing MCF showed named one (F261's
//! neighbour, B-574). That is the least neighbourly failure available here,
//! and the one A27 is about: what MCF starts, MCF stops.
//!
//! **What is done about it.** Not a death signal on the child — that is
//! per-thread on Linux and would tie the server's life to whichever thread
//! spawned it — but a sweep at the next daemon's start. An engine server is
//! one of ours when its executable is under this MCF home's `provisioned`
//! directory; its daemon is gone when the process it reports as its parent
//! is the reparenting target — process 1, or the session manager that stands
//! in for it — which is the one shape a live daemon's server never has. Each
//! one found is asked to stop, then made to, and what was stopped goes into
//! the record with the daemon's start (A4): pid, model, where it listened,
//! how much it held, and how it ended.
//!
//! **What is not done.** A server whose parent is any live process is left
//! alone, whichever daemon or test that is. A server from another home's
//! engines is not this daemon's to stop. The socket file a stopped server
//! listened on is removed, and so is a socket named for a daemon that is not
//! there; nothing else on the disk is touched.
//!
//! **Why this module takes the `unsafe_code` opt-out.** The standard library
//! can signal only a child it spawned, and an orphan by definition is not one.
//! What is admitted is `kill(2)` with a process id and a signal number, its
//! status read, and nothing else — the third such module in the workspace,
//! and the rule being a `deny` rather than a `forbid` for exactly this reason
//! (build.md §4).

// The reason is above.
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use mcf_record::json::Value;

/// `SIGTERM`, the number the platform gives it.
const TERMINATE: i32 = 15;
/// `SIGKILL`.
const KILL: i32 = 9;
/// How long a server is given to leave on request before it is made to.
const GRACE: Duration = Duration::from_secs(3);
/// How long the kernel is given to take a killed server away.
const AFTER_KILL: Duration = Duration::from_secs(2);
/// How often the process table is looked at while waiting.
const LOOK: Duration = Duration::from_millis(100);

/// One engine server found running with no daemon over it, and what was
/// done about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    /// Its process id.
    pub pid: u32,
    /// The executable it ran.
    pub binary: PathBuf,
    /// The model it held, where its command line said.
    pub model: Option<PathBuf>,
    /// Where it listened: a socket path, or `port N`.
    pub reach: Option<String>,
    /// What it held resident when found, in bytes, where the kernel said.
    pub resident_bytes: Option<u64>,
    /// How it ended.
    pub ended: Ended,
}

/// How a stopped server went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// It left when asked.
    OnRequest,
    /// It did not leave when asked, and was made to.
    Killed,
    /// It was still in the process table after both, which is reported
    /// rather than assumed away (A7).
    WouldNotStop,
}

impl Ended {
    /// The word the record carries.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::OnRequest => "stopped on request",
            Self::Killed => "killed",
            Self::WouldNotStop => "would not stop",
        }
    }
}

impl Orphan {
    /// The row the record carries for this server.
    #[must_use]
    pub fn as_value(&self) -> Value {
        Value::map([
            ("pid", Value::Integer(i64::from(self.pid))),
            ("binary", Value::text(self.binary.display().to_string())),
            (
                "model",
                self.model.as_ref().map_or(Value::Null, |model| {
                    Value::text(model.display().to_string())
                }),
            ),
            (
                "reach",
                self.reach
                    .as_ref()
                    .map_or(Value::Null, |reach| Value::text(reach.clone())),
            ),
            (
                "resident_bytes",
                self.resident_bytes.map_or(Value::Null, |bytes| {
                    Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                }),
            ),
            ("ended", Value::text(self.ended.said())),
        ])
    }

    /// One line a person reads about it.
    #[must_use]
    pub fn said(&self) -> String {
        let model = self.model.as_ref().map_or_else(
            || "no model named".to_owned(),
            |model| {
                model.file_name().map_or_else(
                    || model.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                )
            },
        );
        let reach = self.reach.as_deref().unwrap_or("nowhere it said");
        // Whole mebibytes: a person reading how much a stray server held
        // wants the size, not the remainder.
        #[expect(
            clippy::integer_division,
            reason = "whole mebibytes are the unit shown"
        )]
        let held = self.resident_bytes.map_or_else(String::new, |bytes| {
            format!(", {} MiB resident", bytes / (1024 * 1024))
        });
        format!(
            "process {} holding {model} on {reach}{held}: {}",
            self.pid,
            self.ended.said()
        )
    }
}

/// Finds every engine server of this home whose daemon is gone, stops each,
/// and clears the socket files left behind.
///
/// `home` is the MCF data home — the directory whose `provisioned` holds the
/// engines — and `runtime` is where engine sockets go. Empty where the
/// platform has no process table to read, which is not a claim that nothing
/// is running.
#[must_use]
pub fn stop_all(home: &Path, runtime: &Path) -> Vec<Orphan> {
    let engines = home.join("provisioned");
    let mut stopped = Vec::new();
    for pid in processes() {
        let Some(found) = orphan_at(pid, &engines) else {
            continue;
        };
        let ended = stop(pid);
        if let Some(socket) = found
            .reach
            .as_deref()
            .filter(|reach| reach.starts_with('/'))
        {
            let _gone = std::fs::remove_file(socket);
        }
        stopped.push(Orphan { ended, ..found });
    }
    clear_stale_sockets(runtime);
    stopped.sort_by_key(|orphan| orphan.pid);
    stopped
}

/// Whether a process is one of this home's engine servers with no daemon
/// over it, and what it is, without touching it.
fn orphan_at(pid: u32, engines: &Path) -> Option<Orphan> {
    if pid == std::process::id() {
        return None;
    }
    let arguments = arguments_of(pid)?;
    let binary = PathBuf::from(arguments.first()?);
    if !is_engine_server(&binary, engines) {
        return None;
    }
    let parent = parent_of(pid)?;
    if !parent_is_gone(parent) {
        return None;
    }
    let mut found = described(&binary, &arguments);
    found.pid = pid;
    found.resident_bytes = resident_of(pid);
    Some(found)
}

/// An `llama-server` under the home's engines.
fn is_engine_server(binary: &Path, engines: &Path) -> bool {
    binary
        .file_name()
        .is_some_and(|name| name == "llama-server")
        && binary.starts_with(engines)
}

/// What a server's command line says about it. The pid and the resident size
/// are filled in by the caller; here is only what the arguments say.
fn described(binary: &Path, arguments: &[String]) -> Orphan {
    let after = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|at| arguments.get(at.checked_add(1)?))
            .cloned()
    };
    let model = after("--model").or_else(|| after("-m")).map(PathBuf::from);
    let host = after("--host");
    let reach = match host {
        Some(host) if host.starts_with('/') => Some(host),
        _ => after("--port").map(|port| format!("port {port}")),
    };
    Orphan {
        pid: 0,
        binary: binary.to_path_buf(),
        model,
        reach,
        resident_bytes: None,
        ended: Ended::WouldNotStop,
    }
}

/// A daemon is gone when its server's parent is the reparenting target:
/// process 1, or the session manager standing in for it. Any other live
/// parent — a daemon, a test, a shell — is a process that will stop the
/// server itself.
fn parent_is_gone(parent: u32) -> bool {
    if parent <= 1 {
        return true;
    }
    match std::fs::read_to_string(format!("/proc/{parent}/comm")) {
        Ok(name) => matches!(name.trim(), "systemd" | "init"),
        // No such process any more: the parent went between the two reads.
        Err(_) => true,
    }
}

/// Asks the server to stop, waits, makes it, waits.
fn stop(pid: u32) -> Ended {
    signal(pid, TERMINATE);
    if wait_until_gone(pid, GRACE) {
        return Ended::OnRequest;
    }
    signal(pid, KILL);
    if wait_until_gone(pid, AFTER_KILL) {
        Ended::Killed
    } else {
        Ended::WouldNotStop
    }
}

/// Whether the process left the table — or became a zombie its new parent
/// has yet to collect, which is as gone as a process gets — within the time.
fn wait_until_gone(pid: u32, within: Duration) -> bool {
    let started = std::time::Instant::now();
    loop {
        match state_of(pid) {
            None | Some('Z' | 'X') => return true,
            Some(_) => {}
        }
        if started.elapsed() >= within {
            return false;
        }
        std::thread::sleep(LOOK);
    }
}

/// Removes the socket files named for daemons that are not there.
///
/// A server's socket is `llama-<daemon pid>.sock`; a file whose pid is not in
/// the process table is a name the kernel has forgotten, and a fresh daemon
/// that drew that pid would remove it before binding anyway.
fn clear_stale_sockets(runtime: &Path) {
    let Ok(entries) = std::fs::read_dir(runtime) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.strip_prefix("llama-"))
            .and_then(|rest| rest.strip_suffix(".sock"))
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() && state_of(pid).is_none() {
            let _gone = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(unix)]
unsafe extern "C" {
    /// `int kill(pid_t pid, int sig)`.
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Sends a signal to a process that is not this one's child. The status is
/// not acted on: a process that is already gone is the outcome wanted, and
/// one that refuses the signal is found by the wait that follows.
#[cfg(unix)]
fn signal(pid: u32, which: i32) {
    let Ok(pid) = i32::try_from(pid) else {
        return;
    };
    if pid <= 1 {
        return;
    }
    // SAFETY: `kill` takes two integers and touches no memory of this
    // process; the pid is above 1, so it names neither every process nor the
    // reparenting target.
    let _status = unsafe { kill(pid, which) };
}

#[cfg(not(unix))]
fn signal(_pid: u32, _which: i32) {}

/// Every process id the process table lists.
fn processes() -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut found: Vec<u32> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .collect();
    found.sort_unstable();
    found
}

/// A process's arguments, NUL-separated in the table.
fn arguments_of(pid: u32) -> Option<Vec<String>> {
    let line = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let arguments: Vec<String> = line
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    (!arguments.is_empty()).then_some(arguments)
}

/// The parent a process reports.
fn parent_of(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parent_in(&stat)
}

/// The state letter a process reports.
fn state_of(pid: u32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    state_in(&stat)
}

/// The fields after the command, which is parenthesized and may hold spaces
/// and parentheses of its own, so they are counted from the last `)`.
fn after_command(stat: &str) -> Option<&str> {
    stat.rsplit_once(')').map(|(_, rest)| rest)
}

/// The parent pid from a `stat` line: the second field after the command.
fn parent_in(stat: &str) -> Option<u32> {
    after_command(stat)?.split_whitespace().nth(1)?.parse().ok()
}

/// The state from a `stat` line: the first field after the command.
fn state_in(stat: &str) -> Option<char> {
    after_command(stat)?
        .split_whitespace()
        .next()?
        .chars()
        .next()
}

/// What a process holds resident, from `VmRSS` in its status.
fn resident_of(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    resident_in(&status)
}

fn resident_in(status: &str) -> Option<u64> {
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kib.checked_mul(1024)
}

#[cfg(test)]
mod tests;
