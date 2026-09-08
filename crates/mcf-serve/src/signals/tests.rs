use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixListener;
use std::time::Duration;

use super::{HANGUP, INTERRUPT, TERMINATE, Watch, ask_to_stop, reason_for, watched};
use crate::control::Request;

#[test]
fn the_reason_names_the_signal() {
    assert_eq!(reason_for(TERMINATE), "the process received SIGTERM");
    assert_eq!(reason_for(INTERRUPT), "the process received SIGINT");
    assert_eq!(reason_for(HANGUP), "the process received SIGHUP");
    assert_eq!(reason_for(99), "the process received a signal");
}

#[test]
fn a_signal_becomes_a_stop_request_on_the_daemons_socket() {
    let socket = std::env::temp_dir().join(format!("mcf-signals-{}.sock", std::process::id()));
    let _gone = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("a stand-in daemon listens");

    let asking = std::thread::spawn({
        let socket = socket.clone();
        move || ask_to_stop(&socket, TERMINATE)
    });

    let (mut connection, _) = listener.accept().expect("the request connects");
    connection
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a deadline");
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .expect("the request arrives");
    let request = Request::read(line.trim_end()).expect("it is a request");
    assert_eq!(
        request,
        Request::Stop {
            reason: "the process received SIGTERM".to_owned()
        }
    );
    writeln!(
        connection,
        "{{\"protocol\":1,\"served\":true,\"body\":{{}}}}"
    )
    .expect("answered");
    asking.join().expect("the ask returns");
    let _gone = std::fs::remove_file(&socket);
}

#[test]
fn a_daemon_already_gone_is_not_waited_for() {
    let socket = std::env::temp_dir().join(format!("mcf-signals-gone-{}.sock", std::process::id()));
    let _gone = std::fs::remove_file(&socket);
    ask_to_stop(&socket, TERMINATE);
}

#[test]
fn a_watch_registers_its_socket_and_its_drop_removes_it() {
    let socket =
        std::env::temp_dir().join(format!("mcf-signals-registry-{}.sock", std::process::id()));
    let watch = Watch::over(&socket).expect("the watch goes up");
    assert!(watched().contains(&socket), "{:?}", watched());
    let again = Watch::over(&socket).expect("a second watch on the same socket goes up");
    assert_eq!(watched().iter().filter(|held| **held == socket).count(), 2);
    drop(again);
    assert_eq!(watched().iter().filter(|held| **held == socket).count(), 1);
    drop(watch);
    assert!(!watched().contains(&socket), "{:?}", watched());
}
