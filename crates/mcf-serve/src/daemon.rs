//! The daemon: long-lived, restartable, and idle almost all the time (B-030,
//! B-031, B-036, D1).
//!
//! **What it is for at M2's start.** D1 settled that MCF is a process with
//! clients attached rather than a command that exits. This is that process. It
//! cannot serve a model — there is no engine — and it says so when asked, which
//! is the honest shape of a daemon that exists before the thing it will host
//! (A19).
//!
//! **Idle costs nothing, and that is structural rather than careful.** §3.13
//! asks that an idle MCF be indistinguishable from nothing, and D24 states it as
//! a prohibition: zero timer wakeups. The way to get that is not a small
//! interval — it is *no* interval. This daemon blocks in `accept` and does
//! nothing at all until somebody connects: no tick, no poll, no watcher, no
//! heartbeat. B-031's condition is measurable because of that shape, and
//! `checks/tests/soak.rs` measures it.
//!
//! **Local by construction, not by configuration.** The control plane is a Unix
//! socket in a directory only this user can enter (B-036, §6.12). There is no
//! bind address, no port and no flag to expose it: exposure is not something
//! MCF can do today, so it is not something a mistake can do either. When §XI's
//! remote surface arrives it arrives as a deliberate, recorded act with its own
//! decision behind it.
//!
//! **What survives a restart is what was written down.** The daemon keeps no
//! state a crash could lose: what it knows on start is what the record and the
//! model store say, both read fresh. A9's habit — a daemon that recovered from
//! its own memory would be a daemon whose memory is the record, and D20 already
//! settled that the journal is the record.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::{Clock as _, Instant, Monotonic, SystemClock, Timestamp};
use mcf_record::journal::{Entry, EntryKind};
use mcf_record::json::Value;

use crate::control::{Answer, REQUEST_CEILING, Request, VERSION};

const WHERE: Subsystem = Subsystem::new("mcf-serve::daemon");

/// How long the daemon waits for a client to say something.
///
/// Two seconds. Long enough that a slow client on a busy machine is not cut
/// off, short enough that one which says nothing is not worth waiting for —
/// and stated here rather than chosen at the call site, because it is the bound
/// on how long one client can delay another (B7).
pub const PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

/// What a daemon was told about where things are.
///
/// Passed in rather than discovered, for the reason every other path in MCF
/// takes one: a process that decides where to put its socket is a process a
/// test cannot put somewhere else, and the laboratory needs to run one without
/// touching the operator's own (B19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// Where the control socket goes.
    pub socket: PathBuf,
    /// The record, which is what the daemon recovers from.
    pub journal: PathBuf,
    /// The model store, which is what it reports holding.
    pub models: PathBuf,
}

/// A running daemon.
#[derive(Debug)]
pub struct Daemon {
    places: Places,
    listener: UnixListener,
    started: Timestamp,
    since: Instant<Monotonic>,
    /// What the record said when this process started, which is what *recovered
    /// across a restart* means concretely.
    recovered: Recovered,
    /// The one model held between requests, if any (D41, §7.18).
    resident: std::sync::Mutex<Option<crate::generation::Resident>>,
}

/// What was there when the daemon started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// How many entries the record held.
    pub entries: usize,
    /// What a replay could not read, where anything could not be.
    ///
    /// Kept rather than reduced to a count: B62 requires a replay report what
    /// was lost, and a daemon that recovered past a damaged record without
    /// saying so would be the silent failure A2 calls worse than a crash.
    pub unreadable: Option<String>,
    /// How many artifacts the store held.
    pub held: usize,
}

/// Why the daemon stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    /// A client asked it to, and said why.
    Asked {
        /// The reason the client gave.
        reason: String,
    },
    /// The listener will not answer any more, and the failure says why.
    Broken {
        /// What went wrong.
        failure: Box<Failure>,
    },
}

impl Daemon {
    /// Starts a daemon: recovers what is on the disk, then listens.
    ///
    /// # Errors
    ///
    /// `config.conflict` when something is already listening on that socket —
    /// which is one MCF already running, and starting a second would give two
    /// processes one record (D20). `resource.disk.readonly` when the socket
    /// cannot be made.
    pub fn start(places: Places) -> Result<Self> {
        if let Some(parent) = places.socket.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                unusable("the directory the control socket lives in", parent, &error)
            })?;
        }

        // A socket file left by a process that died is not a running daemon.
        // Distinguishing them is a *connection*, not a guess: if something
        // answers, MCF is already up; if nothing does, the file is a leftover
        // and removing it is safe (A27's habit from the other side).
        if places.socket.exists() {
            if UnixStream::connect(&places.socket).is_ok() {
                return Err(Failure::new(
                    Category::ConfigConflict,
                    Attribution::User,
                    Disposition::Refused,
                    WHERE,
                    "another MCF is already listening there",
                )
                .with_context("socket", places.socket.display().to_string())
                .with_context(
                    "what_to_do",
                    "two daemons would share one record, and D20 makes the record the thing \
                     MCF is: ask the running one to stop, or point this one somewhere else",
                ));
            }
            let _leftover = std::fs::remove_file(&places.socket);
        }

        let recovered = recover(&places)?;
        let listener = UnixListener::bind(&places.socket)
            .map_err(|error| unusable("the control socket", &places.socket, &error))?;

        let started = Timestamp::now();
        let daemon = Self {
            places,
            listener,
            started,
            since: SystemClock.now(),
            recovered,
            resident: std::sync::Mutex::new(None),
        };
        // An event, not a tick. *MCF was up between these two moments* is a
        // condition of anything measured in between (§3.4), and a daemon that
        // recorded nothing would leave it unanswerable — while one that
        // recorded on a timer would fail B-031's measurement. A record that
        // cannot be written does not stop the daemon: it is reported and the
        // daemon carries on, because a machine with a full disk still wants
        // MCF up (A4).
        daemon.note(
            EntryKind::DaemonStarted,
            started,
            daemon.recovered_as_value(),
        );
        Ok(daemon)
    }

    /// Writes one line to the record, or says why it could not.
    ///
    /// Deliberately not a `Result`: the caller is a lifecycle event rather than
    /// a request, and a daemon that refused to start because it could not
    /// write down that it had started would be trading a working MCF for a
    /// tidy record (A4, A2 — said, not swallowed).
    fn note(&self, kind: EntryKind, at: Timestamp, body: Value) {
        let Ok(mut journal) = mcf_record::journal::Journal::open(&self.places.journal) else {
            eprintln!(
                "mcf: the record at {} could not be opened, so this event is unrecorded",
                self.places.journal.display()
            );
            return;
        };
        if let Err(failure) = journal.append(&Entry::new(kind, at, body)) {
            eprintln!("mcf: {kind} could not be recorded: {failure}");
        }
    }

    /// What this daemon recovered, in the record's own shape.
    fn recovered_as_value(&self) -> Value {
        Value::map([
            (
                "socket",
                Value::text(self.places.socket.display().to_string()),
            ),
            (
                "record_entries",
                Value::Integer(i64::try_from(self.recovered.entries).unwrap_or(i64::MAX)),
            ),
            (
                "record_unreadable",
                match &self.recovered.unreadable {
                    Some(what) => Value::text(what.clone()),
                    None => Value::Null,
                },
            ),
            (
                "models_held",
                Value::Integer(i64::try_from(self.recovered.held).unwrap_or(i64::MAX)),
            ),
        ])
    }

    /// What this daemon recovered when it started.
    #[must_use]
    pub const fn recovered(&self) -> &Recovered {
        &self.recovered
    }

    /// Where it is listening.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.places.socket
    }

    /// Answers clients until one asks it to stop.
    ///
    /// Blocks in `accept`, which is the whole of the idle discipline: a daemon
    /// with nothing to do is a process the scheduler is not running (§3.13,
    /// D24's zero wakeups).
    ///
    /// One connection at a time, and deliberately: DEC-012 has not settled what
    /// several clients at once means, and a daemon that guessed would be
    /// answering a question nobody has asked yet (§3.13's refusal of
    /// generality).
    ///
    /// The cost of that choice is stated rather than hidden: a client that
    /// connects and says nothing delays every other client by at most
    /// [`PATIENCE`], and then the daemon carries on. It cannot hold MCF for
    /// ever, which is the property B7 asks for; it can make somebody wait, and
    /// what fixes *that* is DEC-012 rather than a smaller number here. The
    /// socket is reachable only by this user (B-036), so the client that could
    /// do it is the operator's own.
    pub fn serve(&mut self) -> Stopped {
        loop {
            let connection = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) => {
                    return Stopped::Broken {
                        failure: Box::new(unusable(
                            "the control socket",
                            &self.places.socket,
                            &error,
                        )),
                    };
                }
            };
            if let Some(stopped) = self.answer_one(&connection) {
                // A26: a stop has an account, and the account is in the record
                // rather than only in what the client was told.
                self.note(
                    EntryKind::DaemonStopped,
                    Timestamp::now(),
                    match &stopped {
                        Stopped::Asked { reason } => Value::map([
                            ("how", Value::text("asked")),
                            (
                                "reason",
                                if reason.is_empty() {
                                    Value::Null
                                } else {
                                    Value::text(reason.clone())
                                },
                            ),
                        ]),
                        Stopped::Broken { failure } => Value::map([
                            ("how", Value::text("broken")),
                            ("why", mcf_record::encode::failure(failure)),
                        ]),
                    },
                );
                return stopped;
            }
        }
    }

    /// Reads one request, answers it, and says whether that was the last.
    fn answer_one(&self, connection: &UnixStream) -> Option<Stopped> {
        // A client that connects and says nothing must not hold the daemon:
        // B7 makes a hang a defined outcome, and this is the one place a
        // stranger could cause one.
        let _deadline = connection.set_read_timeout(Some(PATIENCE));
        let _writing = connection.set_write_timeout(Some(PATIENCE));

        let mut line = String::new();
        // Bounded before it is read, not after: a client that sends a gigabyte
        // without a newline must not become a gigabyte in this process (§3.7).
        let ceiling = u64::try_from(REQUEST_CEILING.saturating_add(1)).unwrap_or(u64::MAX);
        let read = BufReader::new(std::io::Read::take(connection, ceiling)).read_line(&mut line);
        let mut writer = connection;

        let answer = match read {
            Err(_) | Ok(0) => return None,
            Ok(_) => match Request::read(line.trim_end()) {
                Ok(Request::Generate {
                    model,
                    prompt,
                    limit,
                    seed,
                    tokens,
                    engine,
                }) => {
                    // A generation is one request and many lines, so it has
                    // its own path: nothing about it fits in one `Answer`.
                    self.generate(
                        &model,
                        &prompt,
                        limit,
                        seed,
                        tokens.as_deref(),
                        engine.as_deref(),
                        &mut writer,
                    );
                    return None;
                }
                Ok(request) => {
                    let (answer, stop) = self.respond(&request);
                    let _written = writeln!(writer, "{}", answer.to_line());
                    let _flushed = writer.flush();
                    return stop;
                }
                Err(failure) => Answer::refused(&failure),
            },
        };
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
        None
    }

    /// A model answers a prompt, one line per token, then the account (B-034,
    /// PR9).
    ///
    /// **Loaded per request and dropped after.** Whether a served model stays
    /// resident when nobody is looking is DEC-018 and open; until it is
    /// decided, the daemon holds nothing between requests, which is the answer
    /// that costs nothing while idle (§3.13) and hides no choice (§3.15). The
    /// price is paid at the start of every generation and stated in the
    /// account as `loaded: per_request`.
    ///
    /// **What is recorded is the terminating line.** The same object the
    /// client was sent (D20): a client that ignores the conditions still leaves
    /// them behind, and one that hangs up mid-stream leaves the account of what
    /// it got (A4, A26).
    #[allow(
        clippy::too_many_arguments,
        reason = "one request's conditions, each named in the account"
    )]
    fn generate(
        &self,
        named: &str,
        prompt: &str,
        limit: usize,
        seed: u64,
        tokens: Option<&[usize]>,
        engine: Option<&str>,
        writer: &mut &UnixStream,
    ) {
        let at = Timestamp::now();
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        let account = crate::generation::serve_generation(
            &self.places.models,
            &mcf_home,
            &self.resident,
            named,
            prompt,
            limit,
            seed,
            tokens,
            engine,
            writer,
        );
        self.note(EntryKind::Generated, at, account);
    }

    /// What MCF says to each request.
    fn respond(&self, request: &Request) -> (Answer, Option<Stopped>) {
        match request {
            Request::Status => (Answer::served(self.status()), None),
            Request::Holding => (Answer::served(self.holding()), None),
            // Handled before `respond` is reached; here so the match is
            // total and a future request type is a compile error rather than a
            // silent fall-through.
            Request::Generate { .. } => (
                Answer::refused(&crate::control::refused(
                    "a generation reached the one-answer path",
                    "generate",
                )),
                None,
            ),
            Request::Stop { reason } => (
                Answer::served(Value::map([
                    ("stopping", Value::Bool(true)),
                    ("reason", Value::text(reason.clone())),
                ])),
                Some(Stopped::Asked {
                    reason: reason.clone(),
                }),
            ),
        }
    }

    /// What this daemon is.
    ///
    /// Including what it will not do, because a status that listed only
    /// capabilities would leave a reader to infer the rest — and the thing to
    /// infer today is that MCF cannot serve a model (A19, C7).
    fn status(&self) -> Value {
        let identity = BuildIdentity::current();
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("build", mcf_record::encode::build_identity(identity)),
            ("resident", self.resident_value()),
            ("started_at", mcf_record::encode::timestamp(self.started)),
            (
                "up_nanoseconds",
                Value::Integer(
                    i64::try_from(
                        SystemClock
                            .now()
                            .saturating_duration_since(self.since)
                            .as_nanos(),
                    )
                    .unwrap_or(i64::MAX),
                ),
            ),
            (
                "socket",
                Value::text(self.places.socket.display().to_string()),
            ),
            (
                "recovered",
                Value::map([
                    (
                        "record_entries",
                        Value::Integer(i64::try_from(self.recovered.entries).unwrap_or(i64::MAX)),
                    ),
                    (
                        "record_unreadable",
                        match &self.recovered.unreadable {
                            Some(what) => Value::text(what.clone()),
                            None => Value::Null,
                        },
                    ),
                    (
                        "models_held",
                        Value::Integer(i64::try_from(self.recovered.held).unwrap_or(i64::MAX)),
                    ),
                ]),
            ),
            (
                "cannot",
                Value::List(vec![Value::text(
                    "serve a model: no inference engine is vendored yet (B-320, D32)",
                )]),
            ),
        ])
    }

    /// The model held between requests, or `Null` (D41).
    ///
    /// Said in status because memory held is the price of residency, and a
    /// price nobody can see is a hidden choice (§3.15).
    fn resident_value(&self) -> Value {
        let held = self
            .resident
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match held.as_ref() {
            Some(resident) => resident.describe(),
            None => Value::Null,
        }
    }

    /// What this machine is holding, read from the disk rather than remembered.
    fn holding(&self) -> Value {
        match mcf_hub::store::held(&self.places.models) {
            Err(failure) => Value::map([
                ("readable", Value::Bool(false)),
                ("why", mcf_record::encode::failure(&failure)),
            ]),
            Ok(holding) => Value::map([
                ("readable", Value::Bool(true)),
                (
                    "models",
                    Value::List(
                        holding
                            .iter()
                            .map(|held| {
                                Value::map([
                                    ("path", Value::text(held.path.display().to_string())),
                                    (
                                        "bytes",
                                        Value::Integer(
                                            i64::try_from(held.bytes).unwrap_or(i64::MAX),
                                        ),
                                    ),
                                    (
                                        "provenance",
                                        match &held.provenance {
                                            Ok(provenance) => {
                                                mcf_record::encode::provenance(provenance)
                                            }
                                            Err(None) => Value::Null,
                                            Err(Some(failure)) => {
                                                mcf_record::encode::failure(failure)
                                            }
                                        },
                                    ),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
        }
    }
}

impl Drop for Daemon {
    /// The socket goes when the daemon does.
    ///
    /// A27: what MCF created, it removes. A leftover socket is not harmful — the
    /// next start connects to it, finds nothing and replaces it — but leaving
    /// one behind means the next start cannot tell *left over* from *running*
    /// without trying, and doing the tidying here keeps that check rare.
    fn drop(&mut self) {
        let _removed = std::fs::remove_file(&self.places.socket);
    }
}

/// What the disk says, read at start.
fn recover(places: &Places) -> Result<Recovered> {
    let (entries, unreadable) = if places.journal.exists() {
        // Through the index rather than a replay (B-300, D20): a daemon start
        // that parsed the whole history would cost seconds on a record that has
        // been measuring models for a while — 7.9 s at a million entries, where
        // the index takes 72 ms (F14) — and would do it at every start.
        let index = mcf_record::journal::Index::over(
            &places.journal,
            &mcf_record::journal::index::default_path(&places.journal),
        )?;
        (index.entries().len(), index.loss().map(ToString::to_string))
    } else {
        // No record is not a damaged record: a machine that has never run MCF
        // has nothing to recover, and saying so is different from saying it
        // recovered nothing (A7).
        (0, None)
    };

    let held = if places.models.exists() {
        mcf_hub::store::held(&places.models)
            .map(|holding| holding.len())
            .unwrap_or_default()
    } else {
        0
    };

    Ok(Recovered {
        entries,
        unreadable,
        held,
    })
}

fn unusable(what: &str, path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::ResourceDiskReadonly,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        format!("{what} could not be made"),
    )
    .with_context("path", path.display().to_string())
    .with_context("reason", error.to_string())
}

#[cfg(test)]
mod tests;
