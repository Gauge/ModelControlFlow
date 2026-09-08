use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::{
    Ended, described, is_engine_server, parent_in, parent_is_gone, parent_of, resident_in,
    state_in, state_of, stop_all,
};

#[test]
fn what_a_server_said_on_its_command_line_is_read_back() {
    let binary =
        Path::new("/home/x/.local/share/mcf/provisioned/llama.cpp@abc/build/bin/llama-server");
    let socket = described(
        binary,
        &[
            binary.display().to_string(),
            "--model".to_owned(),
            "/models/a.gguf".to_owned(),
            "--host".to_owned(),
            "/run/user/1/mcf/llama-77.sock".to_owned(),
            "--metrics".to_owned(),
        ],
    );
    assert_eq!(socket.model.as_deref(), Some(Path::new("/models/a.gguf")));
    assert_eq!(
        socket.reach.as_deref(),
        Some("/run/user/1/mcf/llama-77.sock")
    );

    let port = described(
        binary,
        &[
            binary.display().to_string(),
            "-m".to_owned(),
            "/models/b.gguf".to_owned(),
            "--host".to_owned(),
            "0.0.0.0".to_owned(),
            "--port".to_owned(),
            "17817".to_owned(),
        ],
    );
    assert_eq!(port.model.as_deref(), Some(Path::new("/models/b.gguf")));
    assert_eq!(port.reach.as_deref(), Some("port 17817"));

    let bare = described(binary, &[binary.display().to_string()]);
    assert_eq!(bare.model, None);
    assert_eq!(bare.reach, None);
    assert_eq!(bare.binary, binary);
}

#[test]
fn a_stat_line_is_read_from_after_the_command_whatever_the_command_holds() {
    let stat = "4242 (llama (b) x) S 1 4242 4242 0 -1 4194560 100 0 0 0 5 3 0 0 20 0 9 0 12 1 2";
    assert_eq!(state_in(stat), Some('S'));
    assert_eq!(parent_in(stat), Some(1));
    assert_eq!(state_in("garbage"), None);
    assert_eq!(parent_in("4242 (x)"), None);
}

#[test]
fn what_a_process_holds_resident_is_the_rss_line_in_bytes() {
    let status = "Name:\tllama-server\nVmPeak:\t 1000 kB\nVmRSS:\t   2048 kB\nThreads:\t4\n";
    assert_eq!(resident_in(status), Some(2_097_152));
    assert_eq!(resident_in("Name:\tx\n"), None);
}

#[test]
fn only_a_server_under_this_homes_engines_is_ours() {
    let engines = Path::new("/home/x/.local/share/mcf/provisioned");
    assert!(is_engine_server(
        &engines.join("llama.cpp@abc/build/bin/llama-server"),
        engines
    ));
    assert!(!is_engine_server(
        &engines.join("llama.cpp@abc/build/bin/llama-cli"),
        engines
    ));
    assert!(!is_engine_server(
        Path::new("/opt/llama/build/bin/llama-server"),
        engines
    ));
    assert!(!is_engine_server(
        Path::new("/home/y/.local/share/mcf/provisioned/llama.cpp@abc/build/bin/llama-server"),
        engines
    ));
}

#[test]
fn this_process_has_a_parent_that_is_not_gone() {
    let parent = parent_of(std::process::id()).expect("this process is in the table");
    assert!(!parent_is_gone(parent) || parent <= 1);
    assert!(state_of(std::process::id()).is_some());
}

const STAND_IN: &str = "trap 'kill $! 2>/dev/null; exit 0' TERM; sleep 60 & wait $!";

fn a_home(name: &str) -> (PathBuf, PathBuf) {
    let home = std::env::temp_dir().join(format!("mcf-orphans-{}-{name}", std::process::id()));
    let engine = home
        .join("provisioned")
        .join("llama.cpp@fixture")
        .join("build")
        .join("bin");
    std::fs::create_dir_all(&engine).expect("the fixture home is made");
    std::fs::create_dir_all(home.join("run")).expect("the fixture runtime is made");
    let server = engine.join("llama-server");
    std::fs::write(&server, b"#!/bin/sh\nexit 0\n").expect("the fixture server is written");
    (home, server)
}

fn alive(pid: u32) -> bool {
    !matches!(state_of(pid), None | Some('Z' | 'X'))
}

fn wait_for(what: impl Fn() -> bool, within: Duration) -> bool {
    let started = Instant::now();
    while !what() {
        if started.elapsed() > within {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

#[test]
fn a_server_whose_daemon_is_gone_is_stopped_and_one_whose_parent_lives_is_not() {
    let Ok(bash) = Command::new("bash").arg("-c").arg("exit 0").status() else {
        return;
    };
    if !bash.success() {
        return;
    }
    let (home, server) = a_home("sweep");
    let runtime = home.join("run");

    let orphan_script =
        "( exec -a \"$0\" sh -c \"$2\" --model \"$1\" --port 4242 >/dev/null 2>&1 ) & echo $!";
    let started = Command::new("bash")
        .arg("-c")
        .arg(orphan_script)
        .arg(&server)
        .arg("/models/orphaned.gguf")
        .arg(STAND_IN)
        .output()
        .expect("bash starts the orphan");
    let orphan: u32 = String::from_utf8_lossy(&started.stdout)
        .trim()
        .parse()
        .expect("bash printed the orphan's pid");

    let mut kept = {
        use std::os::unix::process::CommandExt as _;
        let mut command = Command::new("sh");
        command
            .arg0(&server)
            .arg("-c")
            .arg(STAND_IN)
            .arg("--model")
            .arg("/models/kept.gguf")
            .arg("--port")
            .arg("4243")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.spawn().expect("the kept server starts")
    };

    let stale = runtime.join("llama-4000000000.sock");
    let own = runtime.join(format!("llama-{}.sock", std::process::id()));
    std::fs::write(&stale, b"").expect("the stale socket is written");
    std::fs::write(&own, b"").expect("this process's socket is written");

    assert!(
        wait_for(
            || parent_of(orphan).is_some_and(parent_is_gone),
            Duration::from_secs(5)
        ),
        "the orphan {orphan} was not reparented within five seconds: parent {:?} named {:?}",
        parent_of(orphan),
        parent_of(orphan)
            .and_then(|parent| std::fs::read_to_string(format!("/proc/{parent}/comm")).ok())
    );
    assert!(alive(orphan), "the orphan died before the sweep");

    let stopped = stop_all(&home, &runtime);

    let found = stopped
        .iter()
        .find(|found| found.pid == orphan)
        .unwrap_or_else(|| panic!("the orphan {orphan} was not found: {stopped:?}"));
    assert_eq!(found.ended, Ended::OnRequest);
    assert_eq!(
        found.model.as_deref(),
        Some(Path::new("/models/orphaned.gguf"))
    );
    assert_eq!(found.reach.as_deref(), Some("port 4242"));
    assert_eq!(found.binary, server);
    assert!(found.resident_bytes.is_some_and(|bytes| bytes > 0));
    assert!(!alive(orphan), "the orphan is still in the table");
    assert!(found.said().contains("orphaned.gguf"));
    assert!(found.said().contains("port 4242"));

    let kept_pid = kept.id();
    assert!(
        !stopped.iter().any(|found| found.pid == kept_pid),
        "the server whose parent lives was stopped"
    );
    assert!(alive(kept_pid), "the kept server is gone");
    assert!(!stale.exists(), "the stale socket was left");
    assert!(own.exists(), "this process's own socket was removed");

    let _killed = kept.kill();
    let _waited = kept.wait();
    let _gone = std::fs::remove_dir_all(&home);
}

#[test]
fn a_home_with_nothing_running_stops_nothing() {
    let (home, _server) = a_home("empty");
    let runtime = home.join("run");
    assert_eq!(stop_all(&home, &runtime), Vec::new());
    assert_eq!(
        stop_all(Path::new("/nowhere/at/all"), Path::new("/nowhere")),
        Vec::new()
    );
    let _gone = std::fs::remove_dir_all(&home);
}

#[test]
fn the_row_carries_every_field_and_the_words_say_how_it_ended() {
    let orphan = super::Orphan {
        pid: 77,
        binary: PathBuf::from("/x/provisioned/e/build/bin/llama-server"),
        model: Some(PathBuf::from("/models/a.gguf")),
        reach: Some("port 17817".to_owned()),
        resident_bytes: Some(3 * 1024 * 1024),
        ended: Ended::Killed,
    };
    let row = orphan.as_value();
    assert_eq!(
        row.get("pid").and_then(mcf_record::json::Value::as_integer),
        Some(77)
    );
    assert_eq!(
        row.get("model").and_then(mcf_record::json::Value::as_text),
        Some("/models/a.gguf")
    );
    assert_eq!(
        row.get("reach").and_then(mcf_record::json::Value::as_text),
        Some("port 17817")
    );
    assert_eq!(
        row.get("resident_bytes")
            .and_then(mcf_record::json::Value::as_integer),
        Some(3 * 1024 * 1024)
    );
    assert_eq!(
        row.get("ended").and_then(mcf_record::json::Value::as_text),
        Some("killed")
    );
    assert_eq!(
        orphan.said(),
        "process 77 holding a.gguf on port 17817, 3 MiB resident: killed"
    );
    assert_eq!(Ended::OnRequest.said(), "stopped on request");
    assert_eq!(Ended::WouldNotStop.said(), "would not stop");
}
