//! Transfers over a real socket, against a server this test is holding.
//!
//! Real TCP on the loopback address rather than a mock: what is being tested is
//! the part that talks to an operating system, and a fake socket would test the
//! fake. Nothing here reaches a network — the listener is on 127.0.0.1, on a
//! port the kernel chose — so the whole file runs in the gating tier (B19, B38).

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::{Deadlines, Exchanged, Tcp, Wire, fetch};
use crate::http::{Request, Url};
use mcf_core::failure::Category;

/// A server that answers from a script, and remembers what it was asked.
struct Server {
    port: u16,
    asked: mpsc::Receiver<String>,
    handle: Option<thread::JoinHandle<()>>,
}

/// What the server does with a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answers {
    /// Sends these, one per connection, in order; the last one repeats.
    These(Vec<String>),
    /// Sends half a header block and closes.
    CutShort,
    /// Accepts the connection and says nothing at all.
    Silence,
}

impl Server {
    /// Starts a server on a port the kernel chooses.
    fn answering(answers: Answers) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("an address").port();
        let (sender, asked) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut served = 0_usize;
            // One connection per request, because the client sends
            // `Connection: close`.
            for connection in listener.incoming() {
                let Ok(stream) = connection else { break };
                let asked_for = read_request(&stream);
                if asked_for.is_empty() {
                    // The connection the test's own `Drop` makes to wake this
                    // thread up. Nothing was asked, so there is nothing to
                    // answer and no more connections are coming.
                    break;
                }
                let _sent = sender.send(asked_for);
                match &answers {
                    Answers::These(scripted) => {
                        let answer = scripted
                            .get(served)
                            .or_else(|| scripted.last())
                            .cloned()
                            .unwrap_or_default();
                        write_answer(stream, answer.as_bytes());
                    }
                    Answers::CutShort => {
                        write_answer(stream, b"HTTP/1.1 200 OK\r\nContent-Len");
                    }
                    Answers::Silence => {
                        // Held open and answered never, which is the hang B7
                        // makes a defined outcome. Long enough for the client's
                        // deadline to pass and short enough not to outlive the
                        // test.
                        thread::sleep(Duration::from_millis(1500));
                        drop(stream);
                    }
                }
                served = served.saturating_add(1);
            }
        });
        Self {
            port,
            asked,
            handle: Some(handle),
        }
    }

    fn url(&self, target: &str) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}{target}", self.port)).expect("a URL")
    }

    /// What the next request said, verbatim.
    fn was_asked(&self) -> String {
        self.asked
            .recv_timeout(Duration::from_secs(5))
            .expect("the server was asked something")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Waking the listener so the thread can end. The connection is dropped
        // *before* the join rather than at the end of this scope: a `let` that
        // held it open would leave the server blocked reading a request that
        // never comes, waiting for a thread that is waiting for this one.
        if let Ok(waker) = TcpStream::connect(("127.0.0.1", self.port)) {
            drop(waker);
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

fn read_request(stream: &TcpStream) -> String {
    // A deadline on this side too: a test server that can be wedged by a
    // connection nobody writes to is a test that hangs a suite rather than
    // failing it.
    let _deadline = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = stream;
    let mut seen = Vec::new();
    let mut byte = [0_u8; 1];
    while reader.read(&mut byte).unwrap_or(0) == 1 {
        seen.push(byte[0]);
        if seen.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&seen).into_owned()
}

fn write_answer(mut stream: TcpStream, bytes: &[u8]) {
    let _written = stream.write_all(bytes);
    let _flushed = stream.flush();
    let _closed = stream.shutdown(std::net::Shutdown::Write);
}

fn wire() -> Tcp {
    Tcp {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            // Short, because two tests below are about a deadline passing and a
            // suite that waits a minute for each is a suite nobody runs.
            idle: Duration::from_millis(400),
        },
    }
}

fn get(server: &Server, target: &str) -> (Vec<u8>, Exchanged) {
    let mut body = Vec::new();
    let exchanged = fetch(&wire(), &Request::get(server.url(target)), &mut body)
        .expect("the exchange completes");
    (body, exchanged)
}

/// The ordinary case, over a socket: the request goes out, the answer comes
/// back, and the body is what the caller asked for.
#[test]
fn a_request_reaches_a_server_and_the_body_comes_back() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello".to_owned(),
    ]));
    let (body, exchanged) = get(&server, "/api/models/owner/model");

    assert_eq!(body, b"hello");
    assert_eq!(exchanged.bytes, 5);
    assert_eq!(exchanged.response.status(), 200);
    assert_eq!(exchanged.redirects, 0);

    let asked = server.was_asked();
    assert!(
        asked.starts_with("GET /api/models/owner/model HTTP/1.1\r\n"),
        "{asked}"
    );
    assert!(asked.contains("Host: 127.0.0.1:"), "{asked}");
}

/// A body larger than the read buffer arrives whole. A model is larger than
/// this machine's memory, so the loop that streams it is the one that matters.
#[test]
fn a_body_larger_than_the_buffer_arrives_whole() {
    let weights = "w".repeat(100_000);
    let server = Server::answering(Answers::These(vec![format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
        weights.len()
    )]));
    let (body, exchanged) = get(&server, "/model.gguf");

    assert_eq!(body.len(), 100_000);
    assert_eq!(exchanged.bytes, 100_000);
    assert!(body.iter().all(|byte| *byte == b'w'));
}

/// A redirect is followed, and the answer records where the bytes actually came
/// from — which for the hub is a CDN and not the hub (§3.4, F9).
#[test]
fn a_redirect_is_followed_and_the_final_host_is_recorded() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 302 Found\r\nLocation: /elsewhere\r\nContent-Length: 0\r\n\r\n".to_owned(),
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_owned(),
    ]));
    let (body, exchanged) = get(&server, "/model.gguf");

    assert_eq!(body, b"ok");
    assert_eq!(exchanged.redirects, 1);
    assert_eq!(exchanged.served_by.target(), "/elsewhere");
    assert_eq!(
        server.was_asked().lines().next(),
        Some("GET /model.gguf HTTP/1.1")
    );
    assert_eq!(
        server.was_asked().lines().next(),
        Some("GET /elsewhere HTTP/1.1")
    );
}

/// A source that redirects for ever is stopped at a stated ceiling, and the
/// refusal names where it was sent. Going round for ever is the failure B7
/// exists to prevent.
#[test]
fn a_redirect_loop_ends_at_the_ceiling() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 302 Found\r\nLocation: /round\r\nContent-Length: 0\r\n\r\n".to_owned(),
    ]));
    let mut body = Vec::new();
    let failure = fetch(&wire(), &Request::get(server.url("/round")), &mut body)
        .expect_err("a loop is refused");

    assert_eq!(failure.category(), Category::HubUnreachable);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.key == "visited" && entry.value.contains("/round")),
        "the refusal does not say where it was sent"
    );
}

/// Resumption asks for the rest, and the server sees the range.
#[test]
fn a_resumed_transfer_asks_the_server_for_the_rest() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-5/6\r\nContent-Length: 2\r\n\r\nef"
            .to_owned(),
    ]));
    let mut body = Vec::new();
    let exchanged = fetch(
        &wire(),
        &Request::get(server.url("/model.gguf")).resuming(4),
        &mut body,
    )
    .expect("the exchange completes");

    assert_eq!(body, b"ef");
    assert_eq!(exchanged.response.status(), 206);
    let range = exchanged
        .response
        .content_range()
        .expect("a range")
        .expect("one is there");
    assert!(range.continues_from(4));
    assert!(server.was_asked().contains("Range: bytes=4-\r\n"));
}

/// And a redirected resumption keeps its offset. A transfer that lost its range
/// on the way to a CDN would start again from zero and report progress that did
/// not happen (A4).
#[test]
fn a_redirect_does_not_lose_the_offset() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 302 Found\r\nLocation: /cdn\r\nContent-Length: 0\r\n\r\n".to_owned(),
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-5/6\r\nContent-Length: 2\r\n\r\nef"
            .to_owned(),
    ]));
    let mut body = Vec::new();
    fetch(
        &wire(),
        &Request::get(server.url("/model.gguf")).resuming(4),
        &mut body,
    )
    .expect("the exchange completes");

    let first = server.was_asked();
    let second = server.was_asked();
    assert!(first.contains("Range: bytes=4-\r\n"), "{first}");
    assert!(second.contains("Range: bytes=4-\r\n"), "{second}");
}

/// A credential is not sent in the clear. Refused rather than downgraded: a
/// token on an unencrypted connection is a token given to everything in between
/// (B-024, A2).
#[test]
fn a_credential_is_not_carried_over_a_wire_that_cannot_keep_it() {
    let server = Server::answering(Answers::These(vec![
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_owned(),
    ]));
    let mut body = Vec::new();
    let failure = fetch(
        &wire(),
        &Request::get(server.url("/private")).offering("hf_token"),
        &mut body,
    )
    .expect_err("refused");

    assert_eq!(failure.category(), Category::ConfigInvalid);
    assert!(
        !format!("{failure:?}").contains("hf_token"),
        "the refusal carries the token"
    );
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("token sent in the clear")),
        "the refusal does not say why"
    );
}

/// A connection that closes mid-header is an interruption, said rather than
/// guessed at.
#[test]
fn a_connection_that_closes_mid_answer_is_an_interruption() {
    let server = Server::answering(Answers::CutShort);
    let mut body = Vec::new();
    let failure =
        fetch(&wire(), &Request::get(server.url("/model.gguf")), &mut body).expect_err("cut short");

    assert_eq!(failure.category(), Category::TransferInterrupted);
    assert!(body.is_empty(), "half an answer was written out as a body");
}

/// A host that accepts a connection and says nothing is the hang B7 makes a
/// defined outcome. The deadline is MCF's, and the refusal says how long it
/// waited.
#[test]
fn a_server_that_never_answers_ends_at_a_deadline() {
    let server = Server::answering(Answers::Silence);
    let mut body = Vec::new();
    let failure =
        fetch(&wire(), &Request::get(server.url("/silence")), &mut body).expect_err("stalled");

    assert_eq!(failure.category(), Category::TransferStalled);
}

/// Nothing is listening, which is the ordinary offline case and a different
/// answer from a slow host.
#[test]
fn a_host_that_refuses_a_connection_is_unreachable() {
    // A port nothing is on: bound and immediately dropped, so the number is
    // real and the listener is gone.
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("a loopback port")
        .local_addr()
        .expect("an address")
        .port();
    let url = Url::parse(&format!("http://127.0.0.1:{port}/x")).expect("a URL");

    let mut body = Vec::new();
    let failure = fetch(&wire(), &Request::get(url), &mut body).expect_err("nothing is there");
    assert_eq!(failure.category(), Category::HubUnreachable);
}

/// A wire says what it is, because it is part of the conditions of anything
/// acquired through it (§3.4).
#[test]
fn a_wire_describes_itself_and_says_what_it_will_not_carry() {
    let tcp = Tcp::default();
    assert!(tcp.describe().contains("TCP"));
    assert!(!tcp.carries_secrets());
    assert_eq!(tcp.deadlines.connect, Duration::from_secs(15));
    assert_eq!(tcp.deadlines.idle, Duration::from_secs(60));
}
