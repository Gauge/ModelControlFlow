//! A hub on a socket, for the tests and scenarios that need one (B-028, D26).
//!
//! **Why the laboratory owns this.** `mcf_lab::hub` simulates what MCF observes
//! *through the `Source` interface*; this simulates what MCF observes *through
//! a socket*. Both are the same discipline — the observation is simulated and
//! the cause never is — and the second one exists because B-322 gave MCF code
//! that talks to an operating system, and code that talks to an operating
//! system cannot be tested against a value.
//!
//! **It answers from a script and decides nothing.** A path in, an answer out,
//! and a record of what it was asked. There is no HTTP parser here beyond
//! finding the end of the request, because the thing under test is MCF's
//! client: a server clever enough to be wrong is a second implementation of
//! somebody else's software, which is exactly what A12 warns about.
//!
//! **It leaves nothing behind.** The listener is on the loopback address, on a
//! port the kernel chooses, and dropping the handle wakes the thread and waits
//! for it — so a scenario that uses one can still claim to leave no residue
//! (A27, B58).

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// How long the server waits for a request before giving up on a connection.
const PATIENCE: Duration = Duration::from_secs(5);

/// What the server does with a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Behaviour {
    /// Answers by path, and `404`s anything the script does not name.
    Scripted(BTreeMap<String, String>),
    /// Accepts the connection and says nothing at all, which is the hang B7
    /// makes a defined outcome.
    Silence,
    /// Says this the moment a connection arrives, without waiting to be asked.
    ///
    /// What a plain HTTP server does to a client that opened a TLS handshake:
    /// the first thing it hears is not a TLS record, and it can say so at once
    /// rather than waiting for a deadline (B-322).
    Blurting(String),
}

/// A server on the loopback address, answering from a script.
pub struct Serving {
    port: u16,
    asked: Arc<Mutex<Vec<String>>>,
    /// Set by `Drop`, read by the thread. A server that only stopped when a
    /// connection said nothing could not be stopped at all once it started
    /// answering before it listened (`Blurting`), and the test that used one
    /// would hang rather than fail.
    stopping: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl core::fmt::Debug for Serving {
    /// The port and how much it has been asked. The script is not printed: it
    /// is whole HTTP answers, and a `Debug` that dumped them would bury a test
    /// failure in the fixture rather than in the failure.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Serving")
            .field("port", &self.port)
            .field("asked", &self.asked().len())
            .field("running", &self.handle.is_some())
            .field("stopping", &self.stopping)
            .finish()
    }
}

impl Serving {
    /// A server that answers these paths and refuses the rest.
    ///
    /// `None` when no loopback port can be had, which is a machine this cannot
    /// run on rather than a failure to report.
    #[must_use]
    pub fn answering(answers: BTreeMap<String, String>) -> Option<Self> {
        Self::start(Behaviour::Scripted(answers))
    }

    /// A server that accepts a connection and never answers.
    #[must_use]
    pub fn holding_open() -> Option<Self> {
        Self::start(Behaviour::Silence)
    }

    /// A server that speaks first and is not speaking the caller's protocol.
    #[must_use]
    pub fn blurting(answer: &str) -> Option<Self> {
        Self::start(Behaviour::Blurting(answer.to_owned()))
    }

    fn start(behaviour: Behaviour) -> Option<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        let asked = Arc::new(Mutex::new(Vec::new()));
        let recording = Arc::clone(&asked);
        let stopping = Arc::new(AtomicBool::new(false));
        let told_to_stop = Arc::clone(&stopping);

        let handle = thread::spawn(move || {
            // Connections the silent behaviour is holding open. They close when
            // this thread ends, which is when the handle is dropped: a client's
            // deadline is what ends the wait, and a server that closed first
            // would be demonstrating a different failure.
            let mut held = Vec::new();
            for connection in listener.incoming() {
                if told_to_stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = connection else { break };
                if let Behaviour::Blurting(answer) = &behaviour {
                    // Not a word is read: the point is that the far end speaks
                    // first, and speaks something else.
                    write_answer(stream, answer.as_bytes());
                    continue;
                }
                let request = read_request(&stream);
                if request.is_empty() {
                    // The connection `Drop` makes to wake this thread: nothing
                    // was asked, so nothing more is coming.
                    break;
                }
                if let Ok(mut seen) = recording.lock() {
                    seen.push(request.clone());
                }
                match &behaviour {
                    Behaviour::Scripted(answers) => {
                        let target = request
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or_default()
                            .to_owned();
                        let answer = answers.get(&target).cloned().unwrap_or_else(|| {
                            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_owned()
                        });
                        write_answer(stream, answer.as_bytes());
                    }
                    Behaviour::Silence => held.push(stream),
                    // Answered above, before anything was read.
                    Behaviour::Blurting(_) => {}
                }
            }
            drop(held);
        });

        Some(Self {
            port,
            asked,
            stopping,
            handle: Some(handle),
        })
    }

    /// The port the kernel gave it.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Where MCF would point at it.
    #[must_use]
    pub fn base(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    /// Every request it was sent, verbatim, in order.
    #[must_use]
    pub fn asked(&self) -> Vec<String> {
        self.asked
            .lock()
            .map(|seen| seen.clone())
            .unwrap_or_default()
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        // The connection is dropped *before* the join: one held open would
        // leave the server reading a request that never comes, waiting for a
        // thread that is waiting for this one.
        if let Ok(waker) = TcpStream::connect(("127.0.0.1", self.port)) {
            drop(waker);
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

/// An ordinary answer with a body.
#[must_use]
pub fn answer(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// An answer with a status and no body, for the refusals a hub makes.
#[must_use]
pub fn status(code: u16, reason: &str) -> String {
    format!("HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\n\r\n")
}

fn read_request(stream: &TcpStream) -> String {
    let _deadline = stream.set_read_timeout(Some(PATIENCE));
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

#[cfg(test)]
mod tests;
