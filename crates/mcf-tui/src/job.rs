//! Work that takes longer than a frame.
//!
//! **A download is minutes and a measurement is longer.** The window redraws
//! sixty times a second and the console redraws on a key, and neither can
//! wait for either, so a request that answers in many lines runs on its own
//! thread and posts what it hears back down a channel. Each pass the surface
//! drains the channel and draws whatever has arrived, which is why a progress
//! line moves and the window still closes when you ask it to.
//!
//! **One job for both surfaces** (B-072). The window had this first; the
//! console's buttons were drawn and did nothing, and giving them the same job
//! is what makes a run from the console the run the window makes.
//!
//! **Nothing here interprets an answer.** What comes back is what the daemon
//! said, put in front of the screen that asked for it. A window that
//! summarised a refusal into its own words would be a second opinion about
//! what happened (A2).

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use std::time::Instant;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request, Streamed};

/// What a running job has said so far.
#[derive(Debug)]
pub struct Job {
    /// What it is: a sentence for the screen that started it.
    pub what: String,
    heard: Receiver<Heard>,
    /// Every answer so far, oldest first.
    pub answers: Vec<Value>,
    /// Why it stopped, if it stopped badly.
    pub refused: Option<String>,
    /// Whether the daemon said it was finished.
    pub finished: bool,
    /// When it was started, so how long it took is measured rather than
    /// estimated (A7: the screen may only show a figure it actually took).
    started: Instant,
    /// The connection the request went out on, kept so the job can be cut
    /// short: closing it is how the daemon learns the asker has gone, and
    /// the daemon stops the engine and the ladder on seeing that.
    connection: Option<UnixStream>,
    /// Whether it was cut short from this side.
    pub stopped: bool,
    /// Whether the process was asked to stop and is finishing the unit in
    /// hand: the job goes on until the process ends of its own accord, and
    /// a second Stop kills it (B-571).
    pub stopping: bool,
    /// The process's own input, where the job is a command MCF ran: the
    /// word `stop` on it asks the process to finish what it is doing,
    /// record it, and end.
    asked: std::sync::Arc<std::sync::Mutex<Option<std::process::ChildStdin>>>,
    /// The process behind it, where the job is a command MCF ran rather
    /// than a request to the daemon (B-519): the reader thread waits on it
    /// once its output ends, and stopping kills it.
    child: std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>>,
}

/// One thing heard from the daemon.
#[derive(Debug)]
enum Heard {
    /// An answer it served.
    Answer(Value),
    /// A refusal, in the daemon's own words.
    Refused(String),
    /// The connection ended.
    Ended,
}

impl Job {
    /// A job that is already over, carrying what was heard.
    ///
    /// For a report in hand rather than one being waited on. The screens draw
    /// from `answers` and `finished`, and a caller that already has the answer
    /// — a test drawing a finished report, or one read back from the record —
    /// has nothing to wait on and should not have to invent a daemon to say so.
    #[must_use]
    pub fn already(what: String, answers: Vec<Value>) -> Self {
        // A channel with no sender: `drain` sees it closed and reports the job
        // over, which is what it is.
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

    /// Starts a command of MCF's own and reads its lines as answers, each
    /// `{"line": …}`, with `{"done": true, "exit": …}` when it ends (B-519).
    ///
    /// The command is MCF itself with arguments — `mcf eval <model>` from
    /// the window — never something acquired: what a model wrote runs
    /// inside that command's container, not here.
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

    /// Starts a request that answers in many lines.
    ///
    /// Returns immediately; what comes back arrives through [`Job::drain`].
    #[must_use]
    pub fn start(socket: &Path, request: Request, what: String) -> Self {
        let (send, heard) = channel();
        // Connected here rather than on the thread, so that a handle to the
        // connection stays with the job: the thread reads it, and the job
        // can close it to cut the run short.
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
            // No read deadline. A measurement is minutes by design and a
            // download can be longer; a timeout here would turn patience into
            // a report of a broken daemon.
            if writeln!(connection, "{line}")
                .and_then(|()| connection.flush())
                .is_err()
            {
                let _sent = send.send(Heard::Refused("MCF stopped listening".to_owned()));
                return;
            }
            // A generation does not answer in `Answer`s. It writes one
            // `Streamed::Token` a token and a `Streamed::Done` carrying the
            // account, which is a different shape on the same socket — so
            // which is expected is decided by what was asked, rather than by
            // guessing at each line (A2).
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
                // Nobody is listening any more: the window moved on, or
                // closed. Stopping is right and saying so is not needed.
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

    /// Cuts the run short.
    ///
    /// The connection is closed, which is the one thing the daemon watches
    /// for while an engine runs: it stops the engine at its next glance and
    /// climbs no further. What was heard so far stays; nothing is invented
    /// about what was not, and the job says it was stopped rather than that
    /// MCF failed (A2, A7).
    pub fn stop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(connection) = self.connection.take() {
            let _closed = connection.shutdown(std::net::Shutdown::Both);
        }
        // A command MCF ran is asked first: the word `stop` on its input,
        // and the input closed. It finishes the unit in hand, records it,
        // says what it kept and ends on its own; the job goes on until it
        // does. Asked twice, it is killed (B-571).
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

    /// How long this job has been running, in whole seconds.
    ///
    /// **Measured from the moment the request went out**, which is what an
    /// operator waited, rather than the daemon's own idea of the work. A run
    /// still going reports what it has taken so far.
    #[must_use]
    pub fn ran(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// Takes everything said since the last frame. Returns whether anything was.
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
                    // The connection ended without a final answer. That is
                    // not success: something stopped, and a window that drew
                    // a finished bar here would be reporting a completion
                    // nobody claimed (A2).
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

    /// The most recent answer, which is what a progress display wants.
    #[must_use]
    pub fn latest(&self) -> Option<&Value> {
        self.answers.last()
    }

    /// The answer that says it finished, which is where the results are.
    #[must_use]
    pub fn conclusion(&self) -> Option<&Value> {
        self.answers
            .iter()
            .rev()
            .find(|body| matches!(body.get("done"), Some(Value::Bool(true))))
    }

    /// How far along, as bytes arrived of bytes expected, where the daemon
    /// says both.
    ///
    /// `None` rather than a zero pair where it does not: a bar drawn at zero
    /// says the work has not started, and *MCF cannot say how far along this
    /// is* is a different thing (A7).
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

/// Why the daemon refused, read from the body it refused with (A2).
#[must_use]
pub fn refused_because(body: &Value) -> String {
    mcf_record::decode::failure_said(body).unwrap_or_else(|| {
        format!(
            "MCF refused with something that is not a failure: {}",
            body.to_line()
        )
    })
}
