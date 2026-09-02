//! A daemon, started and stopped and started again.
//!
//! Real sockets and real files: what is being tested is a process that stays
//! up, and a daemon simulated in memory would be a simulation of the part that
//! cannot go wrong.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::{Daemon, Places, Stopped};
use crate::control::{Answer, Request};
use mcf_core::failure::Category;
use mcf_record::json::Value;

/// A machine of this test's own.
struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "mcf-daemon-{name}-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch directory");
        Self { root }
    }

    fn places(&self) -> Places {
        Places {
            socket: self.root.join("control.sock"),
            journal: self.root.join("record.jsonl"),
            models: self.root.join("models"),
        }
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.root);
    }
}

/// Asks a running daemon one thing and reads the answer.
fn ask(socket: &std::path::Path, request: &Request) -> Answer {
    ask_within(socket, request, Duration::from_secs(5))
}

/// The same, waiting no longer than this.
fn ask_within(socket: &std::path::Path, request: &Request, patience: Duration) -> Answer {
    let mut connection = UnixStream::connect(socket).expect("the daemon is listening");
    connection
        .set_read_timeout(Some(patience))
        .expect("a deadline");
    writeln!(connection, "{}", request.to_line()).expect("the request goes out");
    connection.flush().expect("it is sent");

    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .expect("an answer comes back");
    Answer::read(line.trim_end()).expect("it is an answer")
}

/// Runs a daemon in a thread until something stops it.
fn running(places: Places) -> (thread::JoinHandle<Stopped>, PathBuf) {
    let socket = places.socket.clone();
    let mut daemon = Daemon::start(places).expect("it starts");
    let handle = thread::spawn(move || daemon.serve());
    // The socket exists before `start` returns, so a client can connect at
    // once: there is no window in which the daemon is up and unreachable.
    (handle, socket)
}

/// The ordinary life of a daemon: it starts, it answers, it stops when asked,
/// and it says why it stopped.
#[test]
fn it_starts_answers_and_stops_when_asked() {
    let machine = Machine::new("lifecycle");
    let (handle, socket) = running(machine.places());

    let status = ask(&socket, &Request::Status);
    assert!(status.served, "{:?}", status.body);
    assert!(status.body.get("build").is_some(), "{:?}", status.body);
    assert!(status.body.get("up_nanoseconds").is_some());

    let stopped = ask(
        &socket,
        &Request::Stop {
            reason: "the test is done with it".to_owned(),
        },
    );
    assert!(stopped.served);

    match handle.join().expect("the daemon thread ends") {
        Stopped::Asked { reason } => assert_eq!(reason, "the test is done with it"),
        Stopped::Broken { failure } => panic!("it broke rather than stopping: {failure}"),
    }
    assert!(!socket.exists(), "the socket outlived the daemon (A27)");
}

/// It says what it cannot do. A status that listed only capabilities would
/// leave a reader to infer the rest, and what there is to infer today is that
/// MCF cannot serve a model (A19, C7).
#[test]
fn its_status_says_what_it_cannot_do() {
    let machine = Machine::new("cannot");
    let (handle, socket) = running(machine.places());

    let status = ask(&socket, &Request::Status);
    let cannot = status
        .body
        .get("cannot")
        .and_then(Value::as_list)
        .expect("a status says what it cannot do")
        .iter()
        .filter_map(Value::as_text)
        .collect::<Vec<_>>()
        .join(" ");
    // This machine has nothing provisioned, so the daemon says so and says
    // what would fix it. It used to say the same sentence whatever was on the
    // disk, with two rule identifiers in it — which is how two provisioned
    // engines sat on the operator's machine while the daemon reported none.
    assert!(cannot.contains("no engine is installed"), "{cannot}");
    assert!(cannot.contains("builds one"), "{cannot}");
    for cited in ["B-320", "D32"] {
        assert!(
            !cannot.contains(cited),
            "a citation reached a person: {cannot}"
        );
    }

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// What it knows on starting is what the disk says, and a restart re-reads it
/// rather than remembering: a record written by one daemon is recovered by the
/// next (B-030, D20).
#[test]
fn a_restart_recovers_what_the_record_holds() {
    let machine = Machine::new("recovery");
    let places = machine.places();

    // A record written by something else, which is what a restart finds.
    {
        let mut journal =
            mcf_record::journal::Journal::open(&places.journal).expect("a journal opens");
        for _entry in 0..3 {
            journal
                .append(&mcf_record::journal::Entry::new(
                    mcf_record::journal::EntryKind::MachineProfile,
                    mcf_core::time::Timestamp::now(),
                    Value::map([("nothing", Value::Bool(true))]),
                ))
                .expect("it appends");
        }
    }

    let (handle, socket) = running(places.clone());
    let status = ask(&socket, &Request::Status);
    let recovered = status.body.get("recovered").expect("a status recovers");
    assert_eq!(
        recovered.get("record_entries").and_then(Value::as_integer),
        Some(3),
        "the daemon did not recover the three entries that were there before it: {recovered:?}"
    );
    assert_eq!(
        recovered.get("record_unreadable"),
        Some(&Value::Null),
        "a whole record was reported as damaged"
    );
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();

    // And again: the second daemon recovers what the first *left* — the three
    // entries plus the two the first wrote about starting and stopping. The
    // relation matters more than the number: a daemon that recovered a stale
    // count would be reading its own memory rather than the disk (D20).
    let left = mcf_record::journal::replay(&places.journal)
        .expect("the record replays")
        .entries
        .len();
    assert!(
        left > 3,
        "the first daemon recorded neither its start nor its stop"
    );

    let (handle, socket) = running(places);
    let status = ask(&socket, &Request::Status);
    assert_eq!(
        status
            .body
            .get("recovered")
            .and_then(|recovered| recovered.get("record_entries"))
            .and_then(Value::as_integer),
        Some(i64::try_from(left).expect("a count that fits")),
        "the second daemon did not recover what the first left"
    );
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// A damaged record is recovered *and said*: a daemon that started quietly on a
/// torn journal would be the silent failure A2 calls worse than a crash (B62).
#[test]
fn a_damaged_record_is_recovered_and_reported() {
    let machine = Machine::new("damaged");
    let places = machine.places();
    {
        let mut journal =
            mcf_record::journal::Journal::open(&places.journal).expect("a journal opens");
        journal
            .append(&mcf_record::journal::Entry::new(
                mcf_record::journal::EntryKind::MachineProfile,
                mcf_core::time::Timestamp::now(),
                Value::map([("nothing", Value::Bool(true))]),
            ))
            .expect("it appends");
    }
    // A line that stops in the middle, which is what a crash mid-append leaves.
    let mut torn = std::fs::OpenOptions::new()
        .append(true)
        .open(&places.journal)
        .expect("the journal opens");
    torn.write_all(b"{\"id\":\"half a line").expect("it writes");
    drop(torn);

    let (handle, socket) = running(places);
    let status = ask(&socket, &Request::Status);
    let recovered = status.body.get("recovered").expect("a status recovers");
    assert_eq!(
        recovered.get("record_entries").and_then(Value::as_integer),
        Some(1),
        "what was whole was not recovered"
    );
    assert!(
        recovered
            .get("record_unreadable")
            .and_then(Value::as_text)
            .is_some(),
        "the daemon started on a torn record and said nothing"
    );
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// Two daemons would share one record, and D20 makes the record what MCF is.
/// The second refuses rather than joining.
#[test]
fn a_second_daemon_on_one_socket_is_refused() {
    let machine = Machine::new("two");
    let (handle, socket) = running(machine.places());

    let failure = Daemon::start(machine.places()).expect_err("one is already there");
    assert_eq!(failure.category(), Category::ConfigConflict);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("record")),
        "the refusal does not say why two is wrong"
    );

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// A socket left by a process that died is not a running daemon, and the next
/// start takes it over rather than refusing for ever.
#[test]
fn a_socket_left_by_a_dead_daemon_is_taken_over() {
    let machine = Machine::new("leftover");
    let places = machine.places();
    // What a killed daemon leaves: a socket file with nothing behind it.
    std::fs::create_dir_all(places.socket.parent().expect("a parent")).expect("a directory");
    std::fs::write(&places.socket, b"").expect("a leftover file");

    let (handle, socket) = running(places);
    let status = ask(&socket, &Request::Status);
    assert!(status.served);
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// A client that says something that is not a request gets a classified answer
/// and the daemon stays up: A3's rule, at the smallest scale — nothing a client
/// does may take MCF down.
#[test]
fn a_stranger_cannot_stop_it_by_talking_nonsense() {
    let machine = Machine::new("nonsense");
    let (handle, socket) = running(machine.places());

    for rubbish in ["", "not a request", "{\"protocol\":99}"] {
        let mut connection = UnixStream::connect(&socket).expect("it is listening");
        writeln!(connection, "{rubbish}").expect("it goes out");
        connection.flush().expect("sent");
        let mut line = String::new();
        let _read = BufReader::new(&connection).read_line(&mut line);
        if !line.trim().is_empty() {
            let answer = Answer::read(line.trim_end()).expect("an answer");
            assert!(!answer.served, "nonsense was served: {rubbish}");
        }
    }

    // Still up, and still answering.
    assert!(ask(&socket, &Request::Status).served);
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// A client that connects and says nothing does not hold the daemon: B7 makes a
/// hang a defined outcome, and this is the one place a stranger could cause
/// one.
#[test]
fn a_client_that_says_nothing_does_not_wedge_it() {
    let machine = Machine::new("silent-client");
    let (handle, socket) = running(machine.places());

    let silent = UnixStream::connect(&socket).expect("it is listening");
    // Held open and never written to. The daemon's read deadline ends it.
    thread::sleep(Duration::from_millis(50));

    // Another client is answered while the first is still holding its
    // connection open, which is what "does not wedge" means.
    let status = ask(&socket, &Request::Status);
    assert!(status.served);
    drop(silent);

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// What it is holding is read from the store rather than remembered, so a model
/// acquired while it was running is one it reports (D20's habit).
#[test]
fn what_it_is_holding_is_read_from_the_disk() {
    let machine = Machine::new("holding");
    let places = machine.places();
    let (handle, socket) = running(places.clone());

    let empty = ask(&socket, &Request::Holding);
    assert!(empty.served, "{:?}", empty.body);

    // A model appears while the daemon is up.
    let model = places.models.join("owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, b"GGUF").expect("a model file");

    let holding = ask(&socket, &Request::Holding);
    let models = holding
        .body
        .get("models")
        .and_then(Value::as_list)
        .expect("a list of models");
    assert_eq!(models.len(), 1, "{models:?}");
    assert!(
        models
            .first()
            .and_then(|held| held.get("path"))
            .and_then(Value::as_text)
            .is_some_and(|path| path.ends_with("model.gguf")),
        "{models:?}"
    );

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// A cross-check of a model that is not there is refused in one line, with
/// the file named — before any engine is asked for anything (A2, B-424).
#[test]
fn a_cross_check_of_nothing_is_refused_with_the_path() {
    let machine = Machine::new("cross-check-nothing");
    let (handle, socket) = running(machine.places());

    let answer = ask(
        &socket,
        &Request::CrossCheck {
            model: "not-a-model.gguf".to_owned(),
        },
    );
    assert!(!answer.served, "{:?}", answer.body);
    let said = answer.body.to_line();
    assert!(
        said.contains("not-a-model.gguf"),
        "the refusal does not name the file: {said}"
    );

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: "done".to_owned(),
        },
    );
    let _ended = handle.join();
}

/// The cross-check's estimate is a range, grows with the model, and is never
/// nothing (A6, A20).
#[test]
fn a_cross_check_estimate_is_a_range_that_grows_with_the_model() {
    let small = super::cross_check_seconds(Some(1 << 30), Some(8 << 30), 9);
    let large = super::cross_check_seconds(Some(8 << 30), Some(64 << 30), 9);
    let unknown = super::cross_check_seconds(None, None, 9);
    assert!(small.0 < small.1, "{small:?}");
    assert!(small.0 >= 1);
    assert!(
        large.0 > small.0 && large.1 > small.1,
        "{small:?} {large:?}"
    );
    assert!(unknown.0 >= 1 && unknown.0 < unknown.1, "{unknown:?}");
    // The one run the figures come from: Seed-Coder-8B, a 5.2 GB file that
    // dequantizes to 33 GB, took 162 s end to end on this machine's processor
    // (7 s generating, 155 s reading). An estimate that does not hold the run
    // it was fitted to is not an estimate.
    let fitted = super::cross_check_seconds(Some(5_200_000_000), Some(33_000_000_000), 9);
    assert!(
        fitted.0 <= 162 && 162 <= fitted.1,
        "{fitted:?} does not hold 162 s"
    );
}

/// A run is a sample only if it produced exactly what it was pinned to, and
/// one that did not says how far short and why (B-396, A21).
#[test]
fn a_timed_run_holds_its_pin_or_says_how_it_fell_short() {
    let timed = |produced: Option<u64>, stopped: Option<&str>| super::Timed {
        ns: 1_000,
        engine: None,
        peak_resident: None,
        window: None,
        produced,
        stopped: stopped.map(str::to_owned),
    };
    assert!(timed(Some(17), Some("limit")).held_the_pin(17));
    assert!(!timed(Some(16), Some("limit")).held_the_pin(17));
    // A count the engine did not say is not a count that matched.
    assert!(!timed(None, Some("limit")).held_the_pin(17));
    // An engine that stopped on the model's ending five tokens in.
    let short = timed(Some(5), Some("stop_token")).short_of(17);
    assert!(short.contains("5 of the 17 tokens pinned"), "{short}");
    assert!(short.contains("stop_token"), "{short}");
    let unsaid = timed(None, None).short_of(1);
    assert!(unsaid.contains("an unsaid number"), "{unsaid}");
    assert!(unsaid.contains("did not name"), "{unsaid}");
}

/// The record's entry for a prompt report is the figures and the conditions,
/// with the prompt as a length and a digest and the answer as a length —
/// never a word of either (A25, B-432).
/// A served prompt report with text in every place a report carries it.
fn a_served_report() -> Value {
    Value::map([
        ("baseline", Value::text("def slugify(title): pass")),
        ("floor_parts_per_million", Value::Integer(120_000)),
        ("floor_held", Value::Null),
        ("forced_depth", Value::Integer(60)),
        ("ranked_under", Value::text("chatml")),
        (
            "clauses",
            Value::List(vec![
                Value::map([
                    ("text", Value::text("You are a careful assistant.")),
                    ("changed", Value::Bool(false)),
                    ("moved_parts_per_million", Value::Integer(0)),
                    ("without", Value::text("def slugify(title): pass")),
                    ("held", Value::map([("first_rank", Value::Integer(1))])),
                ]),
                Value::map([
                    ("text", Value::text("Reply with only the function.")),
                    ("changed", Value::Bool(true)),
                    ("moved_parts_per_million", Value::Integer(900_000)),
                    ("without", Value::text("Here is a function that ...")),
                    ("held", Value::Null),
                ]),
            ]),
        ),
        ("clauses_over_the_cap", Value::Integer(3)),
        ("unit", Value::text("sentence")),
        ("unit_chosen_by", Value::text("the text")),
        ("most", Value::Integer(2)),
        ("addressed_as", Value::text("one user turn")),
        ("settled", Value::Null),
        ("generations", Value::Integer(4)),
        ("token_limit", Value::Integer(600)),
        ("sampler", Value::text("greedy")),
        ("prompt_tokens", Value::Integer(13)),
        (
            "expected",
            Value::List(vec![
                Value::map([
                    ("text", Value::text(" careful")),
                    ("rank", Value::Integer(1)),
                ]),
                Value::map([
                    ("text", Value::text(" slugify")),
                    ("rank", Value::Integer(41)),
                ]),
                Value::map([("text", Value::text(" only")), ("rank", Value::Null)]),
            ]),
        ),
        ("expected_refused", Value::Null),
    ])
}

#[test]
fn a_prompt_report_entry_holds_figures_and_no_text() {
    let prompt = "You are a careful assistant. Reply with only the function.";
    let served = a_served_report();
    let engines = std::iter::once("mcf-standin".to_owned()).collect();
    let entry = super::prompt_report_entry(
        &served,
        std::path::Path::new("/models/a.gguf"),
        prompt,
        41,
        &engines,
    );
    let line = entry.to_line();
    for word in [
        "careful",
        "assistant",
        "slugify",
        "function",
        "Here is",
        "\"text\"",
        "\"without\"",
        "\"baseline\"",
    ] {
        assert!(!line.contains(word), "{word} reached the record: {line}");
    }
    let at = |path: &[&str]| {
        path.iter()
            .try_fold(&entry, |held, key| held.get(key))
            .cloned()
    };
    assert_eq!(at(&["prompt", "characters"]), Some(Value::Integer(58)));
    assert_eq!(at(&["prompt", "parts"]), Some(Value::Integer(5)));
    assert_eq!(at(&["prompt", "tokens"]), Some(Value::Integer(13)));
    assert_eq!(
        at(&["prompt", "sha256"]),
        Some(Value::text(
            mcf_core::digest::sha256(prompt.as_bytes()).hex()
        ))
    );
    assert_eq!(at(&["answer_characters"]), Some(Value::Integer(24)));
    assert_eq!(
        at(&["conditions", "model"]),
        Some(Value::text("/models/a.gguf"))
    );
    assert_eq!(at(&["conditions", "seed"]), Some(Value::Integer(41)));
    assert_eq!(
        at(&["conditions", "engines"]),
        Some(Value::List(vec![Value::text("mcf-standin")]))
    );
    assert_eq!(at(&["conditions", "unit"]), Some(Value::text("sentence")));
    assert_eq!(
        at(&["floor_parts_per_million"]),
        Some(Value::Integer(120_000))
    );
    let clauses = entry.get("clauses").and_then(Value::as_list).unwrap_or(&[]);
    assert_eq!(clauses.len(), 2);
    assert_eq!(
        clauses
            .get(1)
            .and_then(|held| held.get("moved_parts_per_million")),
        Some(&Value::Integer(900_000))
    );
    assert_eq!(
        clauses.get(1).and_then(|held| held.get("characters")),
        Some(&Value::Integer(29))
    );
    assert_eq!(at(&["expected_read"]), Some(Value::Integer(3)));
    assert_eq!(at(&["expected_first_choice"]), Some(Value::Integer(1)));
}
