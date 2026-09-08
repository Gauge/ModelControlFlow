use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use std::time::Instant;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request, Streamed};

#[derive(Debug)]
pub struct Job {
    pub what: String,
    heard: Receiver<Heard>,
    pub answers: Vec<Value>,
    pub refused: Option<String>,
    pub finished: bool,
    started: Instant,
    connection: Option<UnixStream>,
    pub stopped: bool,
    pub stopping: bool,
    asked: std::sync::Arc<std::sync::Mutex<Option<std::process::ChildStdin>>>,
    child: std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>>,
}

#[derive(Debug)]
enum Heard {
    Answer(Value),
    Refused(String),
    Ended,
}

impl Job {
    #[must_use]
    pub fn already(what: String, answers: Vec<Value>) -> Self {
        let (_sent, heard) = std::sync::mpsc::channel();
        Self {
            what,
            heard,
            answers,
            refused: None,
            finished: true,
            started: Instant::now(),
            connection: None,
            stopped: false,
            stopping: false,
            asked: std::sync::Arc::new(std::sync::Mutex::new(None)),
            child: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    #[must_use]
    pub fn spawned(mut command: std::process::Command, what: String) -> Self {
        let (send, heard) = channel();
        let spawned = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                let _sent = send.send(Heard::Refused(format!(
                    "the command could not be started: {error}"
                )));
                return Self {
                    what,
                    heard,
                    answers: Vec::new(),
                    refused: None,
                    finished: false,
                    started: Instant::now(),
                    connection: None,
                    stopped: false,
                    stopping: false,
                    asked: std::sync::Arc::new(std::sync::Mutex::new(None)),
                    child: std::sync::Arc::new(std::sync::Mutex::new(None)),
                };
            }
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let asked = std::sync::Arc::new(std::sync::Mutex::new(child.stdin.take()));
        let held = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
        let waited = std::sync::Arc::clone(&held);
        let _worker = std::thread::spawn(move || {
            if let Some(stdout) = stdout {
                for read in BufReader::new(stdout).lines() {
                    let Ok(read) = read else { break };
                    if send
                        .send(Heard::Answer(Value::map([
                            ("line", Value::text(read)),
                            ("done", Value::Bool(false)),
                        ])))
                        .is_err()
                    {
                        return;
                    }
                }
            }
            let mut last_words = String::new();
            if let Some(stderr) = stderr {
                let _read =
                    std::io::Read::read_to_string(&mut BufReader::new(stderr), &mut last_words);
            }
            let exit = waited
                .lock()
                .ok()
                .and_then(|mut slot| slot.as_mut().and_then(|child| child.wait().ok()));
            let code = exit.and_then(|status| status.code()).unwrap_or(-1);
            let _sent = send.send(Heard::Answer(Value::map([
                ("done", Value::Bool(true)),
                ("exit", Value::Integer(i64::from(code))),
                ("last_words", Value::text(last_words.trim().to_owned())),
            ])));
        });
        Self {
            what,
            heard,
            answers: Vec::new(),
            refused: None,
            finished: false,
            started: Instant::now(),
            connection: None,
            stopped: false,
            stopping: false,
            asked,
            child: held,
        }
    }

    #[must_use]
    pub fn start(socket: &Path, request: Request, what: String) -> Self {
        let (send, heard) = channel();
        let connected = UnixStream::connect(socket).ok();
        let kept = connected.as_ref().and_then(|held| held.try_clone().ok());
        let _worker = std::thread::spawn(move || {
            let line = request.to_line();
            let Some(mut connection) = connected else {
                let _sent = send.send(Heard::Refused(
                    "MCF is not answering on this computer".to_owned(),
                ));
                return;
            };
            if writeln!(connection, "{line}")
                .and_then(|()| connection.flush())
                .is_err()
            {
                let _sent = send.send(Heard::Refused("MCF stopped listening".to_owned()));
                return;
            }
            let streaming = matches!(request, Request::Generate { .. });
            let reader = BufReader::new(&connection);
            for read in reader.lines() {
                let Ok(read) = read else { break };
                if streaming {
                    let Ok(streamed) = Streamed::read(read.trim_end()) else {
                        continue;
                    };
                    let body = match streamed {
                        Streamed::Token { at, text } => Value::map([
                            ("token", Value::text(text)),
                            ("at", Value::Integer(i64::try_from(at).unwrap_or(i64::MAX))),
                            ("done", Value::Bool(false)),
                        ]),
                        Streamed::Progress {
                            read,
                            of,
                            produced,
                            seconds,
                        } => {
                            let figure =
                                |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
                            Value::map([
                                (
                                    "progress",
                                    Value::map([
                                        ("read", figure(read)),
                                        ("of", figure(of)),
                                        ("produced", figure(produced)),
                                        ("seconds", figure(seconds)),
                                    ]),
                                ),
                                ("done", Value::Bool(false)),
                            ])
                        }
                        Streamed::Done(account) => {
                            Value::map([("account", account), ("done", Value::Bool(true))])
                        }
                    };
                    let ended = matches!(body.get("done"), Some(Value::Bool(true)));
                    if send.send(Heard::Answer(body)).is_err() {
                        return;
                    }
                    if ended {
                        return;
                    }
                    continue;
                }
                let Ok(answer) = Answer::read(read.trim_end()) else {
                    continue;
                };
                let posted = if answer.served {
                    send.send(Heard::Answer(answer.body))
                } else {
                    send.send(Heard::Refused(refused_because(&answer.body)))
                };
                if posted.is_err() {
                    return;
                }
            }
            let _sent = send.send(Heard::Ended);
        });
        Self {
            what,
            heard,
            answers: Vec::new(),
            refused: None,
            finished: false,
            started: Instant::now(),
            connection: kept,
            stopped: false,
            stopping: false,
            asked: std::sync::Arc::new(std::sync::Mutex::new(None)),
            child: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn stop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(connection) = self.connection.take() {
            let _closed = connection.shutdown(std::net::Shutdown::Both);
        }
        if !self.stopping
            && let Ok(mut input) = self.asked.lock()
            && let Some(mut stdin) = input.take()
        {
            use std::io::Write as _;
            let _asked = writeln!(stdin, "stop");
            let _flushed = stdin.flush();
            drop(stdin);
            self.stopping = true;
            return;
        }
        if let Ok(mut slot) = self.child.lock()
            && let Some(mut child) = slot.take()
        {
            let _killed = child.kill();
            let _reaped = child.wait();
        }
        self.stopped = true;
        self.finished = true;
        self.refused = Some(format!(
            "stopped at your asking after {} s; what had been measured is above, and nothing \
             was recorded",
            self.ran()
        ));
    }

    #[must_use]
    pub fn ran(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    pub fn drain(&mut self) -> bool {
        let mut anything = false;
        loop {
            match self.heard.try_recv() {
                Ok(Heard::Answer(body)) => {
                    if matches!(body.get("done"), Some(Value::Bool(true))) {
                        self.finished = true;
                    }
                    self.answers.push(body);
                    anything = true;
                }
                Ok(Heard::Refused(why)) => {
                    self.refused = Some(why);
                    self.finished = true;
                    anything = true;
                }
                Ok(Heard::Ended) | Err(TryRecvError::Disconnected) => {
                    if !self.finished {
                        self.refused =
                            Some("MCF stopped answering before it said it had finished".to_owned());
                        self.finished = true;
                        anything = true;
                    }
                    return anything;
                }
                Err(TryRecvError::Empty) => return anything,
            }
        }
    }

    #[must_use]
    pub fn latest(&self) -> Option<&Value> {
        self.answers.last()
    }

    #[must_use]
    pub fn conclusion(&self) -> Option<&Value> {
        self.answers
            .iter()
            .rev()
            .find(|body| matches!(body.get("done"), Some(Value::Bool(true))))
    }

    #[must_use]
    pub fn progress(&self) -> Option<(u64, u64)> {
        let latest = self.latest()?;
        let arrived = latest.get("arrived").and_then(Value::as_integer)?;
        let total = latest.get("bytes").and_then(Value::as_integer)?;
        if total <= 0 {
            return None;
        }
        Some((
            u64::try_from(arrived)
                .unwrap_or(0)
                .min(u64::try_from(total).unwrap_or(0)),
            u64::try_from(total).unwrap_or(0),
        ))
    }
}

#[must_use]
pub fn refused_because(body: &Value) -> String {
    mcf_record::decode::failure_said(body).unwrap_or_else(|| {
        format!(
            "MCF refused with something that is not a failure: {}",
            body.to_line()
        )
    })
}
