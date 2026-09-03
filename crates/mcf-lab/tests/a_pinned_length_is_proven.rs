//! A timing's request pins how many tokens, and the account proves it (B-396).
//!
//! The defect this holds shut: a duration divided by a count the request
//! asked for, when the engine stopped at the model's end of text well short
//! of it. The pin travels with the request, the engine is told to run past
//! its ending, and the count that comes back is the engine's own — so a reader
//! dividing by it is dividing by what happened (A21, A7).
//!
//! Hermetic (B19): the model is the laboratory's own one-hot fixture, and the
//! engine is MCF's stand-in — which is the same generation path a client's
//! request runs, and the one whose ending was never honoured under a pin.

// A test says what went wrong by failing.
#![allow(clippy::expect_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use mcf_record::json::Value;
use mcf_serve::control::{Request, Streamed};
use mcf_serve::daemon::{Daemon, Places};

/// A machine of this test's own, removed when it is dropped.
struct Machine(PathBuf);

impl Machine {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "mcf-pinned-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch directory");
        Self(root)
    }

    fn places(&self) -> Places {
        Places {
            socket: self.0.join("control.sock"),
            journal: self.0.join("record.jsonl"),
            models: self.0.join("models"),
        }
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// One generation of `<s>` from the model that ends on it, and its account.
fn generated(socket: &Path, model: &Path, limit: usize, pinned: bool) -> Value {
    let mut connection = UnixStream::connect(socket).expect("the daemon is listening");
    connection
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a deadline");
    let request = Request::Generate {
        model: model.display().to_string(),
        prompt: String::new(),
        limit: Some(limit),
        seed: 0,
        // The token the model answers with itself, and its end of text.
        tokens: Some(vec![0]),
        engine: Some("stand-in".to_owned()),
        whose: mcf_record::content::Whose::Fixture,
        pinned,
        turn: None,
    };
    writeln!(connection, "{}", request.to_line()).expect("the request goes out");
    connection.flush().expect("it is sent");
    for line in BufReader::new(&connection).lines() {
        let line = line.expect("a line");
        if let Ok(Streamed::Done(account)) = Streamed::read(line.trim_end()) {
            return account;
        }
    }
    panic!("the stream ended before its account");
}

fn count(account: &Value, key: &str) -> i64 {
    account
        .get(key)
        .and_then(Value::as_integer)
        .unwrap_or_else(|| panic!("no {key} in {}", account.to_line()))
}

fn said(account: &Value, key: &str) -> String {
    account
        .get(key)
        .and_then(Value::as_text)
        .unwrap_or_else(|| panic!("no {key} in {}", account.to_line()))
        .to_owned()
}

fn condition(account: &Value, key: &str) -> String {
    said(account.get("conditions").expect("conditions"), key)
}

/// Unpinned, the model's end of text ends the turn; pinned, the engine runs
/// to the limit — and either way the account says which, and counts.
#[test]
fn a_ceiling_ends_at_the_models_ending_and_a_pin_runs_past_it() {
    let machine = Machine::new();
    let places = machine.places();
    let model = places.models.join("lab/fixture/ends-its-turn.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_ends_its_turn()).expect("a model");
    let socket = places.socket.clone();
    let mut daemon = Daemon::start(places).expect("it starts");
    let serving = thread::spawn(move || daemon.serve());

    // A ceiling: the first token the model produces is its ending, so the
    // turn ends there with nothing produced, and the account says so.
    let ceiling = generated(&socket, &model, 3, false);
    assert_eq!(count(&ceiling, "tokens"), 0, "{}", ceiling.to_line());
    assert_eq!(said(&ceiling, "stopped"), "stop_token");
    assert_eq!(condition(&ceiling, "length"), "at_most");

    // A pin: the ending is not an ending, and the count is the one asked for.
    let pinned = generated(&socket, &model, 3, true);
    assert_eq!(count(&pinned, "tokens"), 3, "{}", pinned.to_line());
    assert_eq!(said(&pinned, "stopped"), "limit");
    assert_eq!(condition(&pinned, "length"), "exactly");

    // And the pin is a condition of the *account*, not of the model: the
    // same model, asked again without it, ends where it ends.
    let again = generated(&socket, &model, 3, false);
    assert_eq!(count(&again, "tokens"), 0, "{}", again.to_line());

    let mut connection = UnixStream::connect(&socket).expect("still listening");
    let stop = Request::Stop {
        reason: "the test is done".to_owned(),
    };
    writeln!(connection, "{}", stop.to_line()).expect("the stop goes out");
    let _ended = serving.join();
}
