#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use mcf_record::json::Value;

const TERMINATE: i32 = 15;
const KILL: i32 = 9;
const GRACE: Duration = Duration::from_secs(3);
const AFTER_KILL: Duration = Duration::from_secs(2);
const LOOK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    pub pid: u32,
    pub binary: PathBuf,
    pub model: Option<PathBuf>,
    pub reach: Option<String>,
    pub resident_bytes: Option<u64>,
    pub ended: Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    OnRequest,
    Killed,
    WouldNotStop,
}

impl Ended {
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

fn is_engine_server(binary: &Path, engines: &Path) -> bool {
    binary
        .file_name()
        .is_some_and(|name| name == "llama-server")
        && binary.starts_with(engines)
}

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

fn parent_is_gone(parent: u32) -> bool {
    if parent <= 1 {
        return true;
    }
    match std::fs::read_to_string(format!("/proc/{parent}/comm")) {
        Ok(name) => matches!(name.trim(), "systemd" | "init"),
        Err(_) => true,
    }
}

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
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
fn signal(pid: u32, which: i32) {
    let Ok(pid) = i32::try_from(pid) else {
        return;
    };
    if pid <= 1 {
        return;
    }
    let _status = unsafe { kill(pid, which) };
}

#[cfg(not(unix))]
fn signal(_pid: u32, _which: i32) {}

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

fn arguments_of(pid: u32) -> Option<Vec<String>> {
    let line = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let arguments: Vec<String> = line
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    (!arguments.is_empty()).then_some(arguments)
}

fn parent_of(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parent_in(&stat)
}

fn state_of(pid: u32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    state_in(&stat)
}

fn after_command(stat: &str) -> Option<&str> {
    stat.rsplit_once(')').map(|(_, rest)| rest)
}

fn parent_in(stat: &str) -> Option<u32> {
    after_command(stat)?.split_whitespace().nth(1)?.parse().ok()
}

fn state_in(stat: &str) -> Option<char> {
    after_command(stat)?
        .split_whitespace()
        .next()?
        .chars()
        .next()
}

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
