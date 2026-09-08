use std::path::PathBuf;

use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{Conditions, Floor};

use super::interposed;

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mcf-cost-{name}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&root));
    std::fs::create_dir_all(&root).expect("a place to write");
    root
}

#[test]
fn a_socket_with_nothing_behind_it_measures_nothing() {
    let root = scratch("nobody");
    let socket = root.join("control.sock");
    assert!(interposed(&socket, 8, conditions()).is_none());
    drop(std::fs::remove_dir_all(&root));
}

#[test]
fn a_running_daemon_answers_and_the_reading_names_what_it_omits() {
    let root = scratch("interposed");
    let places = crate::daemon::Places {
        socket: root.join("control.sock"),
        journal: root.join("record.jsonl"),
        models: root.join("models"),
    };
    let mut daemon = crate::daemon::Daemon::start(places.clone()).expect("a daemon starts");
    let serving = std::thread::spawn(move || daemon.serve());

    let measured = interposed(&places.socket, 20, conditions()).expect("a reading");
    assert!(measured.round_trip.n() >= 2, "too few trials completed");
    assert!(
        !measured.excludes.is_empty(),
        "a partial reading that names nothing it omits is a whole one"
    );
    assert!(
        measured
            .excludes
            .iter()
            .any(|missing| missing.contains("engine")),
        "the missing half is the engine's, and it is not named: {:?}",
        measured.excludes
    );

    let asked = ask_to_stop(&places.socket);
    assert!(asked, "the daemon would not stop");
    drop(serving.join());
    drop(std::fs::remove_dir_all(&root));
}

fn ask_to_stop(socket: &std::path::Path) -> bool {
    use std::io::{BufRead as _, BufReader, Write as _};
    let Ok(mut connection) = std::os::unix::net::UnixStream::connect(socket) else {
        return false;
    };
    let request = crate::control::Request::Stop {
        reason: "the measurement is done".to_owned(),
    };
    if writeln!(connection, "{}", request.to_line()).is_err() {
        return false;
    }
    let mut answer = String::new();
    BufReader::new(&connection).read_line(&mut answer).is_ok()
}
