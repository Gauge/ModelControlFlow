//! What the daemon costs to talk to (B-035, D24, §VII, §3.8).
//!
//! **The figure this is about.** D24 budgets *added latency, request to the
//! engine's first token* at 5 ms at the 99th percentile, and gives the reason
//! for the percentile: the tail is what a user feels. B-035 asks that the
//! overhead MCF interposes be measured under stated conditions and defended in
//! CI rather than asserted in prose.
//!
//! **What is measurable today, and what is not.** There is no engine (B-320),
//! so *the engine's first token* does not exist to be waited for. What does
//! exist is everything MCF does around that hand-off: accept a connection, read
//! a request, parse it, route it, compose an answer and write it back. That is
//! the control plane's half of the figure, and it is the half MCF is
//! responsible for — an engine's own latency is the engine's.
//!
//! **So the reading is named for what it is.** [`Interposed`] carries what was
//! measured *and what was not*, because A21 keeps a partial reading from being
//! read as a whole one and §3.4 makes the conditions part of the number. When
//! B-032 gives the daemon an engine to dispatch to, the missing half arrives
//! here rather than in a new figure.
//!
//! **Nothing here starts a daemon.** A measurement that spawned the thing it
//! measures would be measuring a cold start (D24 budgets that separately). The
//! caller starts one, and this asks it questions.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_core::measurement::{Conditions, Measurement};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock};

use crate::control::{Answer, Request, Streamed};

/// A reading of what MCF interposes, with what it does not cover.
#[derive(Debug)]
pub struct Interposed {
    /// The round trips, from just before the request is written to just after
    /// the answer is read.
    pub round_trip: Measurement<Duration<Monotonic>>,
    /// What this reading does **not** include, in one sentence each.
    ///
    /// Not a footnote: a figure compared against D24's ceiling without them is
    /// a figure claiming to be the whole of the thing D24 named (A21, §3.4).
    pub excludes: Vec<&'static str>,
}

/// What the daemon at `socket` costs for one question and one answer.
///
/// A fresh connection per trial, because that is what a client does: the
/// control plane answers one request per connection, and a measurement that
/// reused one would be measuring something no caller can ask for.
///
/// A trial that could not complete is excluded rather than counted as a slow
/// one — it is not a slow trial, it is not a trial (B24) — and the sample count
/// says how many there were. `None` when fewer than two completed, because
/// §3.4 makes a single timing an anecdote and [`Measurement`] will not hold
/// one.
#[must_use]
pub fn interposed(socket: &Path, trials: usize, conditions: Conditions) -> Option<Interposed> {
    let clock = SystemClock;
    let mut samples = Vec::with_capacity(trials);
    for _ in 0..trials {
        let Some(elapsed) = one_round_trip(socket, clock) else {
            continue;
        };
        samples.push(elapsed);
    }
    Some(Interposed {
        round_trip: Measurement::from_samples(samples, conditions)?,
        excludes: vec![
            "the engine's own latency, and the hand-off to it: there is no engine (B-320, B-032)",
            "model resolution and residency, which D24 budgets separately (§7.18, B-037)",
            "a network hop: the control plane is a Unix socket, and §XI's remote surface is later",
        ],
    })
}

/// One question, one answer, timed the way a caller experiences it.
/// The other half of D24's figure: hand a request to the engine and wait for
/// its first token (B-035, B-034).
///
/// Measured across the real process boundary on a running daemon, from the
/// write of a `generate` request to the read of the first token line. It
/// **includes** everything MCF interposes on that path — accept, parse, route,
/// resolve the model, load it (per request, which is DEC-018's price and is
/// named as such), tokenize the prompt, the engine's first forward pass, and
/// the write back — because that is what a person waiting for a first token
/// waits for. The model is the caller's: on the laboratory's fixture the
/// engine's own share is microseconds and the figure is MCF's overhead; on a
/// real model the engine's share dominates and the figure is the engine's,
/// which is why the budget tier measures it on the fixture and says so.
///
/// Every trial reads the stream to its end before the next begins, so that no
/// trial is timed against a daemon still finishing the last.
#[must_use]
pub fn to_first_token(
    socket: &Path,
    model: &Path,
    trials: usize,
    conditions: Conditions,
) -> Option<Interposed> {
    let clock = SystemClock;
    let mut samples = Vec::with_capacity(trials);
    for _ in 0..trials {
        let Some(elapsed) = one_first_token(socket, model, clock) else {
            continue;
        };
        samples.push(elapsed);
    }
    Some(Interposed {
        round_trip: Measurement::from_samples(samples, conditions)?,
        excludes: vec![
            "residency: the model is loaded per request and that load is *included* here, \
             which is DEC-018's price until it is decided (§7.18)",
            "a network hop: the control plane is a Unix socket, and §XI's remote surface is later",
            "any engine but MCF's own: a provisioned engine's hand-off arrives with B-032",
        ],
    })
}

fn one_first_token(socket: &Path, model: &Path, clock: SystemClock) -> Option<Duration<Monotonic>> {
    let mut connection = UnixStream::connect(socket).ok()?;
    let line = Request::Generate {
        model: model.display().to_string(),
        prompt: "yes".to_owned(),
        limit: 2,
        seed: 0,
        tokens: None,
        // D24's figure is about what MCF interposes around its own engine; a
        // provisioned one would put its own load inside the number.
        engine: Some("stand-in".to_owned()),
    }
    .to_line();
    let started = clock.now();
    writeln!(connection, "{line}").ok()?;
    connection.flush().ok()?;
    let mut reader = BufReader::new(&connection);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let elapsed = clock.now().saturating_duration_since(started);
    // The first line must be a token, not a refusal wearing `done`: a trial
    // that timed a refusal would be timing the wrong thing.
    match Streamed::read(first.trim_end()).ok()? {
        Streamed::Token { .. } => {}
        Streamed::Done(_) => return None,
    }
    // Drain to the end so the next trial starts on a quiet daemon.
    let mut rest = String::new();
    while reader.read_line(&mut rest).ok()? > 0 {
        if let Ok(Streamed::Done(_)) = Streamed::read(rest.trim_end()) {
            break;
        }
        rest.clear();
    }
    Some(elapsed)
}

fn one_round_trip(socket: &Path, clock: SystemClock) -> Option<Duration<Monotonic>> {
    let mut connection = UnixStream::connect(socket).ok()?;
    let line = Request::Status.to_line();

    // The clock starts at the write rather than at the connect: what B-035 is
    // about is what MCF does with a request, and a connect is the client
    // arriving rather than MCF answering. The connect is not free and it is not
    // hidden — it is one of the things `excludes` names.
    let started = clock.now();
    writeln!(connection, "{line}").ok()?;
    connection.flush().ok()?;
    let mut answer = String::new();
    BufReader::new(&connection).read_line(&mut answer).ok()?;
    let elapsed = clock.now().saturating_duration_since(started);

    // A refusal is a completed round trip and a wrong answer is not: a reading
    // taken from a daemon that could not answer would be timing an error path
    // (A2 — the outcome is checked, never assumed).
    Answer::read(answer.trim_end()).ok()?;
    Some(elapsed)
}

#[cfg(test)]
mod tests;
