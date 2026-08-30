//! Work that takes longer than a frame.
//!
//! **A download is minutes and a measurement is longer.** The window redraws
//! sixty times a second and cannot wait for either, so a request that answers
//! in many lines runs on its own thread and posts what it hears back down a
//! channel. Each frame the interface drains the channel and draws whatever has
//! arrived, which is why a progress bar moves and the window still closes when
//! you ask it to.
//!
//! **Nothing here interprets an answer.** What comes back is what the daemon
//! said, put in front of the screen that asked for it. A window that
//! summarised a refusal into its own words would be a second opinion about
//! what happened (A2).

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
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
    /// Starts a request that answers in many lines.
    ///
    /// Returns immediately; what comes back arrives through [`Job::drain`].
    #[must_use]
    pub fn start(socket: PathBuf, request: Request, what: String) -> Self {
        let (send, heard) = channel();
        let _worker = std::thread::spawn(move || {
            let line = request.to_line();
            let Ok(mut connection) = UnixStream::connect(&socket) else {
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
                    send.send(Heard::Refused(
                        answer
                            .body
                            .get("what")
                            .and_then(Value::as_text)
                            .unwrap_or("MCF did not say why")
                            .to_owned(),
                    ))
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
        }
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

    /// How far along, between nothing and one, where that is knowable.
    ///
    /// `None` rather than zero where it is not: a bar drawn at zero says the
    /// work has not started, and *MCF cannot say how far along this is* is a
    /// different thing (A7).
    #[must_use]
    pub fn fraction(&self) -> Option<f32> {
        let latest = self.latest()?;
        let arrived = latest.get("arrived").and_then(Value::as_integer)?;
        let total = latest.get("bytes").and_then(Value::as_integer)?;
        if total <= 0 {
            return None;
        }
        // Both are counts of bytes and neither is near f32's limits at any
        // size a model comes in.
        #[expect(
            clippy::cast_precision_loss,
            reason = "byte counts of a file, far inside f32's exact range at these magnitudes"
        )]
        Some((arrived as f32 / total as f32).clamp(0.0, 1.0))
    }
}
