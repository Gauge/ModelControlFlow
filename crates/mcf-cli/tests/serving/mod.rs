#![allow(
    unreachable_pub,
    dead_code,
    reason = "a stand-in engine for the whole-system test, reached only from it"
)]

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PATIENCE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Behaviour {
    Scripted(BTreeMap<String, String>),
    Silence,
    Blurting(String),
}

pub struct Serving {
    port: u16,
    asked: Arc<Mutex<Vec<String>>>,
    stopping: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl core::fmt::Debug for Serving {
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
    #[must_use]
    pub fn answering(answers: BTreeMap<String, String>) -> Option<Self> {
        Self::start(Behaviour::Scripted(answers))
    }

    #[must_use]
    pub fn holding_open() -> Option<Self> {
        Self::start(Behaviour::Silence)
    }

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
            let mut held = Vec::new();
            for connection in listener.incoming() {
                if told_to_stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = connection else { break };
                if let Behaviour::Blurting(answer) = &behaviour {
                    write_answer(stream, answer.as_bytes());
                    continue;
                }
                let request = read_request(&stream);
                if request.is_empty() {
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

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn base(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

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
        if let Ok(waker) = TcpStream::connect(("127.0.0.1", self.port)) {
            drop(waker);
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

#[must_use]
pub fn answer(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

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
