//! Tests for the signal watch.
//!
//! The handler is called as a function here rather than raised as a signal:
//! a real signal reaches the whole test process, and what is under test is
//! what the watch does with one, not the kernel's delivery of it.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixListener;
use std::time::Duration;

use super::{HANGUP, INTERRUPT, TERMINATE, Watch, on_signal, reason_for};
use crate::control::Request;

#[test]
fn the_reason_names_the_signal() {
    assert_eq!(reason_for(TERMINATE), "the process received SIGTERM");
    assert_eq!(reason_for(INTERRUPT), "the process received SIGINT");
    assert_eq!(reason_for(HANGUP), "the process received SIGHUP");
    assert_eq!(reason_for(99), "the process received a signal");
}

/// A signal becomes the stop request a client would send, on the daemon's
/// own socket, with the signal as its reason.
#[test]
fn a_signal_becomes_a_stop_request_on_the_daemons_socket() {
    let socket = std::env::temp_dir().join(format!("mcf-signals-{}.sock", std::process::id()));
    let _gone = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("a stand-in daemon listens");

    let watch = Watch::over(&socket).expect("the watch goes up");
    on_signal(TERMINATE);

    let (mut connection, _) = listener.accept().expect("the watch connects");
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
    // Answer it, as the daemon would, so the thread's read returns.
    writeln!(
        connection,
        "{{\"protocol\":1,\"served\":true,\"body\":{{}}}}"
    )
    .expect("answered");

    drop(watch);
    let _gone = std::fs::remove_file(&socket);
}

/// A watch dropped without a signal ends its thread and sends nothing.
#[test]
fn a_watch_taken_down_quietly_sends_nothing() {
    let socket =
        std::env::temp_dir().join(format!("mcf-signals-quiet-{}.sock", std::process::id()));
    let _gone = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("a stand-in daemon listens");
    listener
        .set_nonblocking(true)
        .expect("the listener does not block");

    let watch = Watch::over(&socket).expect("the watch goes up");
    drop(watch);

    assert!(
        listener.accept().is_err(),
        "something connected to the socket without a signal"
    );
    let _gone = std::fs::remove_file(&socket);
}
