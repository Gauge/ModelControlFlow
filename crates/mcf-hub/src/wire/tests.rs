use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::{Deadlines, Exchanged, Tcp, Wire, fetch};
use crate::http::{Request, Url};
use mcf_core::failure::Category;

struct Server {
    port: u16,
    asked: mpsc::Receiver<String>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Answers {
    These(Vec<String>),
    CutShort,
    Silence,
}

impl Server {
    fn answering(answers: Answers) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("an address").port();
        let (sender, asked) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut served = 0_usize;
            for connection in listener.incoming() {
                let Ok(stream) = connection else { break };
                let asked_for = read_request(&stream);
                if asked_for.is_empty() {
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

    fn was_asked(&self) -> String {
        self.asked
            .recv_timeout(Duration::from_secs(5))
            .expect("the server was asked something")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Ok(waker) = TcpStream::connect(("127.0.0.1", self.port)) {
            drop(waker);
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

fn read_request(stream: &TcpStream) -> String {
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

#[test]
fn a_connection_that_closes_mid_answer_is_an_interruption() {
    let server = Server::answering(Answers::CutShort);
    let mut body = Vec::new();
    let failure =
        fetch(&wire(), &Request::get(server.url("/model.gguf")), &mut body).expect_err("cut short");

    assert_eq!(failure.category(), Category::TransferInterrupted);
    assert!(body.is_empty(), "half an answer was written out as a body");
}

#[test]
fn a_server_that_never_answers_ends_at_a_deadline() {
    let server = Server::answering(Answers::Silence);
    let mut body = Vec::new();
    let failure =
        fetch(&wire(), &Request::get(server.url("/silence")), &mut body).expect_err("stalled");

    assert_eq!(failure.category(), Category::TransferStalled);
}

#[test]
fn a_host_that_refuses_a_connection_is_unreachable() {
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

#[test]
fn a_name_that_will_not_resolve_says_so_without_guessing_why() {
    let url = Url::parse("http://this-name-does-not-exist.invalid/x").expect("a URL");
    let mut body = Vec::new();
    let failure = fetch(&wire(), &Request::get(url), &mut body).expect_err("no such name");

    assert_eq!(failure.category(), Category::HubUnreachable);
    assert!(
        failure
            .context_value("what_this_does_not_say")
            .is_some_and(|said| said.contains("three causes")),
        "the refusal does not say which question it is leaving open"
    );
    assert!(
        failure.context_value("what_to_do").is_some_and(
            |said| said.contains("without a network") || said.contains("with no network")
        ),
        "the refusal does not say what still works"
    );
}

#[test]
fn a_refusal_is_distinguished_from_no_route() {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("a loopback port")
        .local_addr()
        .expect("an address")
        .port();
    let url = Url::parse(&format!("http://127.0.0.1:{port}/x")).expect("a URL");
    let mut body = Vec::new();
    let failure = fetch(&wire(), &Request::get(url), &mut body).expect_err("nothing is there");

    assert_eq!(failure.category(), Category::HubUnreachable);
    assert!(
        failure
            .context_value("what_this_says")
            .is_some_and(|said| said.contains("working path")),
        "a refusal was reported as though there were no way to reach the host"
    );
}

#[test]
fn a_wire_describes_itself_and_says_what_it_will_not_carry() {
    let tcp = Tcp::default();
    assert!(tcp.describe().contains("TCP"));
    assert!(!tcp.carries_secrets());
    assert_eq!(tcp.deadlines.connect, Duration::from_secs(15));
    assert_eq!(tcp.deadlines.idle, Duration::from_secs(60));
}

#[test]
fn an_interrupted_read_is_retried_rather_than_classified() {
    use std::io::Read;

    struct Twitchy {
        left: usize,
    }

    impl Read for Twitchy {
        fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
            if self.left > 0 {
                self.left -= 1;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "a signal arrived",
                ));
            }
            let held = b"ok";
            into.get_mut(..held.len())
                .ok_or_else(|| std::io::Error::other("the buffer is too small"))?
                .copy_from_slice(held);
            Ok(held.len())
        }
    }

    let mut buffer = [0_u8; 8];
    let read = super::patiently(&mut Twitchy { left: 2 }, &mut buffer)
        .expect("an interrupted read is retried, not reported");
    assert_eq!(read, 2);
    assert_eq!(buffer.get(..2), Some(b"ok".as_slice()));
}

#[test]
fn any_other_error_is_still_reported() {
    use std::io::Read;

    struct Broken;

    impl Read for Broken {
        fn read(&mut self, _into: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "the far end went away",
            ))
        }
    }

    let mut buffer = [0_u8; 8];
    let held = super::patiently(&mut Broken, &mut buffer);
    assert_eq!(
        held.map(|_| ()).unwrap_err().kind(),
        std::io::ErrorKind::ConnectionReset
    );
}
