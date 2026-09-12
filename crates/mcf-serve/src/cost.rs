use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_core::measurement::{Conditions, Measurement};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock};

use crate::control::{Answer, Request, Streamed};

#[derive(Debug)]
pub struct Interposed {
    pub round_trip: Measurement<Duration<Monotonic>>,
    pub excludes: Vec<&'static str>,
}

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
            "the engine's own latency, and the hand-off to it: there is no engine",
            "model resolution and residency, which are budgeted separately",
            "a network hop: the control plane is a Unix socket, and a remote surface is later",
        ],
    })
}

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
             which is the price until residency is decided",
            "a network hop: the control plane is a Unix socket, and a remote surface is later",
            "any engine but MCF's own: a provisioned engine's hand-off is not measured here yet",
        ],
    })
}

fn one_first_token(socket: &Path, model: &Path, clock: SystemClock) -> Option<Duration<Monotonic>> {
    let mut connection = UnixStream::connect(socket).ok()?;
    let line = Request::Generate {
        whose: mcf_record::content::Whose::Fixture,
        model: model.display().to_string(),
        prompt: "yes".to_owned(),
        limit: Some(2),
        seed: 0,
        tokens: None,
        pieces: None,
        engine: Some("stand-in".to_owned()),
        pinned: false,
        turn: None,
        image: None,
        started: std::boxed::Box::new(crate::declared::Started::default()),
    }
    .to_line();
    let started = clock.now();
    writeln!(connection, "{line}").ok()?;
    connection.flush().ok()?;
    let mut reader = BufReader::new(&connection);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let elapsed = clock.now().saturating_duration_since(started);
    match Streamed::read(first.trim_end()).ok()? {
        Streamed::Token { .. } => {}
        Streamed::Progress { .. } | Streamed::Done(_) => return None,
    }
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

    let started = clock.now();
    writeln!(connection, "{line}").ok()?;
    connection.flush().ok()?;
    let mut answer = String::new();
    BufReader::new(&connection).read_line(&mut answer).ok()?;
    let elapsed = clock.now().saturating_duration_since(started);

    Answer::read(answer.trim_end()).ok()?;
    Some(elapsed)
}

#[cfg(test)]
mod tests;
