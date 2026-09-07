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

/// A count is a reading, and the answer names its reader: on a machine with
/// nothing provisioned MCF's own tokenizer counts, says so, and counts
/// without the beginning marker — a text's cost, not a turn's. A count of a
/// model that is not there is refused with the file named (B-442, A2).
#[test]
fn a_count_names_who_counted() {
    let machine = Machine::new("tokenize");
    let places = machine.places();
    let (handle, socket) = running(places.clone());

    let model = places.models.join("owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, crate::probes::tests::chatml()).expect("a model file");

    let counted = ask(
        &socket,
        &Request::Tokenize {
            model: "owner/model/model.gguf".to_owned(),
            text: "a".to_owned(),
            engine: None,
            beginning: false,
        },
    );
    assert!(counted.served, "{:?}", counted.body);
    assert_eq!(
        counted.body.get("tokens").and_then(Value::as_integer),
        Some(1),
        "`a` is one token of this fixture and the beginning marker is not counted: {:?}",
        counted.body
    );
    assert!(
        counted
            .body
            .get("read_by")
            .and_then(Value::as_text)
            .is_some_and(|reader| reader.contains("MCF's own")),
        "{:?}",
        counted.body
    );
    let read = counted
        .body
        .get("read")
        .and_then(Value::as_list)
        .expect("the reading itself comes back");
    // `▁a`, identifier 1: the unigram convention puts a space before the
    // first word, and the reading shows it rather than tidying it away (A1).
    assert_eq!(
        read.first()
            .and_then(|token| token.get("piece"))
            .and_then(Value::as_text),
        Some(" a"),
        "{read:?}"
    );

    let as_a_turn = ask(
        &socket,
        &Request::Tokenize {
            model: "owner/model/model.gguf".to_owned(),
            text: "a".to_owned(),
            engine: None,
            beginning: true,
        },
    );
    // This fixture names no beginning-of-text token, so a turn's start costs
    // what the text costs: the convention is the file's, and a marker the
    // file does not name is not invented for it (A7).
    assert!(as_a_turn.served, "{:?}", as_a_turn.body);
    assert_eq!(
        as_a_turn.body.get("tokens").and_then(Value::as_integer),
        Some(1),
        "{:?}",
        as_a_turn.body
    );

    let missing = ask(
        &socket,
        &Request::Tokenize {
            model: "not-a-model.gguf".to_owned(),
            text: "a".to_owned(),
            engine: None,
            beginning: false,
        },
    );
    assert!(!missing.served);
    assert!(missing.body.to_line().contains("not-a-model.gguf"));

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: "done".to_owned(),
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
/// A reading as served: its figure and the answer it was read from.
fn a_reading(moved: i64, answer: &str) -> Value {
    Value::map([
        ("moved_parts_per_million", Value::Integer(moved)),
        ("held", Value::Null),
        ("answer", Value::text(answer)),
    ])
}

/// Two forms as served: one read, one not rendered (B-444).
fn a_served_forms() -> Value {
    Value::List(vec![
        Value::map([
            ("form", Value::text("bullets")),
            ("moved_parts_per_million", Value::Integer(410_000)),
            ("held", Value::Null),
            ("answer", Value::text("Here is a list")),
        ]),
        Value::map([
            ("form", Value::text("one line")),
            (
                "not_rendered",
                Value::text("the prompt is written this way"),
            ),
        ]),
    ])
}

/// Two clauses as served: one that moved nothing, one that moved most.
fn a_served_clauses() -> Value {
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
    ])
}

fn a_served_report() -> Value {
    Value::map([
        ("baseline", Value::text("def slugify(title): pass")),
        ("floor_parts_per_million", Value::Integer(120_000)),
        ("floor_held", Value::Null),
        (
            "floors",
            Value::List(vec![Value::map([
                ("position", Value::Integer(0)),
                ("moved_parts_per_million", Value::Integer(140_000)),
                ("held", Value::Null),
            ])]),
        ),
        (
            "floor_spread",
            Value::map([
                ("least_parts_per_million", Value::Integer(120_000)),
                ("middle_parts_per_million", Value::Integer(140_000)),
                ("most_parts_per_million", Value::Integer(140_000)),
            ]),
        ),
        (
            "alone",
            Value::List(vec![a_reading(310_000, "Here is a careful function")]),
        ),
        ("alone_floor", a_reading(980_000, "Here is nothing")),
        (
            "prefixes",
            Value::List(vec![a_reading(640_000, "Here is a start")]),
        ),
        (
            "swaps",
            Value::List(vec![a_reading(90_000, "Here is a careful function")]),
        ),
        ("forms", a_served_forms()),
        ("forced_depth", Value::Integer(60)),
        ("ranked_under", Value::text("chatml")),
        ("clauses", a_served_clauses()),
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
        (
            "expected_by_part",
            Value::map([
                (
                    "parts",
                    Value::List(vec![Value::map([
                        ("tokens", Value::Integer(6)),
                        ("first_choice", Value::Integer(2)),
                        ("past_depth", Value::Integer(1)),
                    ])]),
                ),
                ("unplaced", Value::Integer(1)),
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
        "\"answer\"",
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
    // The floor at every position travels whole: positions and figures,
    // no text (B-434).
    assert_eq!(
        at(&["floor_spread", "most_parts_per_million"]),
        Some(Value::Integer(140_000))
    );
    assert_eq!(
        at(&["floors"]).and_then(|held| held.as_list().map(<[Value]>::len)),
        Some(1)
    );
    // Each part alone, the control alone, the prompt grown from the front
    // and the neighbours swapped travel as their figures; the answers stay
    // behind (B-435, B-436, B-437, A25).
    let first_moved = |key: &str| {
        at(&[key])
            .and_then(|held| held.as_list().and_then(<[Value]>::first).cloned())
            .and_then(|held| held.get("moved_parts_per_million").cloned())
    };
    assert_eq!(first_moved("alone"), Some(Value::Integer(310_000)));
    assert_eq!(first_moved("prefixes"), Some(Value::Integer(640_000)));
    assert_eq!(first_moved("swaps"), Some(Value::Integer(90_000)));
    assert_eq!(
        at(&["alone_floor", "moved_parts_per_million"]),
        Some(Value::Integer(980_000))
    );
    // The reading grouped by part is counts, and travels whole (B-433).
    assert_eq!(
        at(&["expected_by_part", "unplaced"]),
        Some(Value::Integer(1))
    );
    assert_eq!(
        at(&["expected_by_part", "parts"])
            .and_then(|held| held.as_list().and_then(<[Value]>::first).cloned())
            .and_then(|held| held.get("past_depth").cloned()),
        Some(Value::Integer(1))
    );
}

/// A form travels as its name and figures, or its name and why it was not
/// rendered; the answer stays behind (B-444, A25, A7).
#[test]
fn a_form_travels_as_its_figures_or_as_why_it_was_not_rendered() {
    let served = a_served_report();
    let entry = super::prompt_report_entry(
        &served,
        std::path::Path::new("/models/a.gguf"),
        "You are a careful assistant. Reply with only the function.",
        41,
        &std::iter::once("mcf-standin".to_owned()).collect(),
    );
    let form = |at_form: usize, key: &str| {
        entry
            .get("forms")
            .and_then(|held| held.as_list().and_then(|forms| forms.get(at_form)))
            .and_then(|held| held.get(key).cloned())
    };
    assert_eq!(form(0, "form"), Some(Value::text("bullets")));
    assert_eq!(
        form(0, "moved_parts_per_million"),
        Some(Value::Integer(410_000))
    );
    assert_eq!(form(0, "answer"), None);
    assert_eq!(form(1, "form"), Some(Value::text("one line")));
    assert_eq!(
        form(1, "not_rendered"),
        Some(Value::text("the prompt is written this way"))
    );
    // Every key is written, null where the form had nothing for it, so a
    // reader of the record finds the same shape on every row.
    assert_eq!(form(1, "moved_parts_per_million"), Some(Value::Null));
    assert_eq!(form(0, "not_rendered"), Some(Value::Null));
    assert!(!entry.to_line().contains("Here is a list"));
}

/// The served report groups the rank reading by part, and serves null for
/// the grouping where no reading was taken, so that an absent reading is not
/// read as a prompt the model wholly expected (B-433, A7).
#[test]
fn a_served_report_groups_the_rank_reading_by_part_or_says_it_has_none() {
    use crate::prompt::{Report, Taken, Unit};
    let taken = Taken {
        text: "Be terse.\n\nWhat is 2 + 2?",
        by: Some(Unit::Paragraph),
        most: None,
        extras: crate::prompt::Extras::NONE,
    };
    let parts = taken.parts();
    let report = Report {
        baseline_thought: None,
        floor_thought: None,
        floor: 0,
        floor_held: None,
        floors: None,
        alone: None,
        alone_floor: None,
        prefixes: None,
        swaps: None,
        forms: None,
        baseline: "4".to_owned(),
        clauses: Vec::new(),
        clauses_over_the_cap: 0,
        unit: Unit::Paragraph,
        unit_chosen: true,
        most: 8,
        settled: None,
    };
    let row = |text: &str, rank: Option<i64>| {
        Value::map([
            ("text", Value::text(text)),
            ("rank", rank.map_or(Value::Null, Value::Integer)),
            ("engine_said", Value::Null),
        ])
    };
    let ranked = super::RankedPrompt {
        rows: vec![
            row("Be", None),
            row(" terse", Some(9)),
            row(".", Some(1)),
            row("What", Some(1)),
            row(" is", Some(1)),
            row(" 2", Some(4)),
        ],
        refused: None,
        under: Some("chatml".to_owned()),
    };
    let served =
        super::prompt_report_value(&report, &parts, 1, Ok(9), ranked, "a test".to_owned(), None);
    let grouped = served.get("expected_by_part").cloned();
    assert_eq!(
        grouped,
        Some(Value::map([
            (
                "parts",
                Value::List(vec![
                    Value::map([
                        ("tokens", Value::Integer(3)),
                        ("first_choice", Value::Integer(1)),
                        ("past_depth", Value::Integer(1)),
                        ("no_context", Value::Integer(0)),
                    ]),
                    Value::map([
                        ("tokens", Value::Integer(3)),
                        ("first_choice", Value::Integer(2)),
                        ("past_depth", Value::Integer(0)),
                        ("no_context", Value::Integer(0)),
                    ]),
                ]),
            ),
            ("unplaced", Value::Integer(0)),
        ]))
    );
    let none = super::RankedPrompt {
        rows: Vec::new(),
        refused: Some("no template".to_owned()),
        under: None,
    };
    let served = super::prompt_report_value(
        &report,
        &parts,
        1,
        Ok(9),
        none,
        "a test".to_owned(),
        Some("no engine resolves this model: it does not fit".to_owned()),
    );
    assert_eq!(served.get("expected_by_part"), Some(&Value::Null));
    // A forced reading that was not taken says why, apart from a rank (A2).
    assert_eq!(
        served.get("held_refused").and_then(Value::as_text),
        Some("no engine resolves this model: it does not fit")
    );
    // Not asked is null, not an empty list (A7).
    assert_eq!(served.get("alone"), Some(&Value::Null));
    assert_eq!(served.get("alone_floor"), Some(&Value::Null));
    assert_eq!(served.get("prefixes"), Some(&Value::Null));
}

/// A report with every extra taken, served: two parts, each alone, the
/// control alone, one prefix, one swap, one form read and one not rendered.
fn a_report_with_every_extra_served() -> Value {
    use crate::prompt::{Reading, Report, Taken, Unit};
    let taken = Taken {
        text: "Be terse.\n\nWhat is 2 + 2?",
        by: Some(Unit::Paragraph),
        most: None,
        extras: crate::prompt::Extras::NONE.with(crate::prompt::Extra::Alone, true),
    };
    let parts = taken.parts();
    let read = |moved: u64, answer: &str| Reading {
        thought: None,
        moved,
        held: None,
        answer: answer.to_owned(),
    };
    let report = Report {
        baseline_thought: None,
        floor_thought: None,
        floor: 0,
        floor_held: None,
        floors: None,
        alone: Some(vec![read(1_000_000, "Yes."), read(0, "4")]),
        alone_floor: Some(read(1_000_000, "Hello!")),
        prefixes: Some(vec![read(750_000, "Sure.")]),
        swaps: Some(vec![read(125_000, "4.")]),
        forms: Some(vec![
            crate::prompt::Formed {
                form: crate::prompt::Form::Bullets,
                outcome: crate::prompt::Rendering::Read(read(250_000, "Four.")),
            },
            crate::prompt::Formed {
                form: crate::prompt::Form::OneLine,
                outcome: crate::prompt::Rendering::NotRendered(crate::prompt::AS_WRITTEN),
            },
        ]),
        baseline: "4".to_owned(),
        clauses: Vec::new(),
        clauses_over_the_cap: 0,
        unit: Unit::Paragraph,
        unit_chosen: true,
        most: 8,
        settled: None,
    };
    let none = super::RankedPrompt {
        rows: Vec::new(),
        refused: None,
        under: None,
    };
    super::prompt_report_value(&report, &parts, 6, Ok(9), none, "a test".to_owned(), None)
}

/// A report that asked each part alone serves each answer beside its figure,
/// and the control alone with them (B-435).
#[test]
fn a_served_report_carries_each_part_alone_with_its_answer() {
    let served = a_report_with_every_extra_served();
    let alone = served.get("alone").and_then(Value::as_list).unwrap_or(&[]);
    assert_eq!(alone.len(), 2);
    assert_eq!(
        alone.get(1).and_then(|held| held.get("answer")),
        Some(&Value::text("4"))
    );
    assert_eq!(
        alone
            .first()
            .and_then(|held| held.get("moved_parts_per_million")),
        Some(&Value::Integer(1_000_000))
    );
    assert_eq!(
        served
            .get("alone_floor")
            .and_then(|held| held.get("answer")),
        Some(&Value::text("Hello!"))
    );
    // The prompt grown from the front travels the same way (B-436).
    let prefixes = served
        .get("prefixes")
        .and_then(Value::as_list)
        .unwrap_or(&[]);
    assert_eq!(
        prefixes.first().and_then(|held| held.get("answer")),
        Some(&Value::text("Sure."))
    );
    // And the neighbours swapped (B-437).
    let swaps = served.get("swaps").and_then(Value::as_list).unwrap_or(&[]);
    assert_eq!(
        swaps.first().and_then(|held| held.get("answer")),
        Some(&Value::text("4."))
    );
}

/// **The parts in each form are served as what each form came to**
/// (B-444, A7): a form read carries its name and its reading, a form not
/// rendered carries its name and why, and neither borrows the other's
/// fields.
#[test]
fn a_served_report_carries_each_form_read_or_why_it_was_not() {
    let served = a_report_with_every_extra_served();
    let forms = served.get("forms").and_then(Value::as_list).unwrap_or(&[]);
    assert_eq!(forms.len(), 2);
    assert_eq!(
        forms.first().and_then(|held| held.get("form")),
        Some(&Value::text("bullets"))
    );
    assert_eq!(
        forms.first().and_then(|held| held.get("answer")),
        Some(&Value::text("Four."))
    );
    assert_eq!(
        forms.first().and_then(|held| held.get("not_rendered")),
        None
    );
    assert_eq!(
        forms.get(1).and_then(|held| held.get("not_rendered")),
        Some(&Value::text(crate::prompt::AS_WRITTEN))
    );
    assert_eq!(
        forms
            .get(1)
            .and_then(|held| held.get("moved_parts_per_million")),
        None
    );
}

/// **The cut the seeds were drawn under travels with their figures**
/// (B-440, §3.4): the served `settled` names `top_k`, `top_p` and `min_p` as
/// the file declared them or as *off*, and whose they were — so two models'
/// seeded readings are never compared under two cuts nobody was shown.
#[test]
fn a_served_settledness_carries_the_cut_it_was_drawn_under() {
    use crate::prompt::{Settled, Stated, Truncation, Whose};
    use mcf_core::configuration::Thousandths;
    let settled = Settled {
        temperature: Thousandths(700),
        truncation: Truncation {
            top_k: Stated::Declared(20),
            top_p: Stated::Declared(Thousandths(950)),
            min_p: Stated::Off,
            whose: Whose::File,
        },
        asked: 3,
        distinct: 2,
        spread: 120_000,
        from_greedy: 90_000,
    };
    let served = super::settled_value(Some(&settled));
    assert_eq!(served.get("temperature"), Some(&Value::text("0.700")));
    assert_eq!(served.get("top_k"), Some(&Value::text("20")));
    assert_eq!(served.get("top_p"), Some(&Value::text("0.950")));
    assert_eq!(served.get("min_p"), Some(&Value::text("off")));
    assert_eq!(
        served.get("truncation"),
        Some(&Value::text("declared by the file"))
    );
    let off = super::settled_value(Some(&Settled {
        truncation: Truncation::recommended(None),
        ..settled
    }));
    assert_eq!(
        off.get("truncation"),
        Some(&Value::text("none declared by the file"))
    );
    assert_eq!(super::settled_value(None), Value::Null);
}

/// A prompt report says what its generations were addressed as from their
/// own accounts, and a prompt that went bare is said to have gone bare
/// rather than as *one user turn* (A21, F160).
#[test]
fn a_prompt_report_says_what_its_generations_were_addressed_as() {
    let none = std::collections::BTreeSet::new();
    assert!(
        super::addressed_as(&none).starts_with("the prompt alone: no addressing is on file"),
        "{}",
        super::addressed_as(&none)
    );
    let seen: std::collections::BTreeSet<String> =
        ["im_start…im_end as assistant — set by the chat-template probe".to_owned()]
            .into_iter()
            .collect();
    assert_eq!(
        super::addressed_as(&seen),
        "im_start…im_end as assistant — set by the chat-template probe"
    );
}

/// A rung read off one pair has a figure and no spread, and says how many
/// pairs it was read off and what became of the rest; one read off two has a
/// spread between them (F174).
#[test]
fn a_rung_over_one_pair_has_no_spread() {
    let mut pairs = super::Pairs::of(3);
    pairs.separated = 1;
    pairs.not_apart = 2;
    let reading = super::rung_reading(
        512,
        &mut [226_000],
        &mut [380_000_000],
        (Some(4096), None),
        &pairs,
        (None, None),
    );
    assert_eq!(reading.get("measured"), Some(&Value::Bool(true)));
    assert_eq!(reading.get("spread_ns"), Some(&Value::Null));
    assert_eq!(reading.get("spread_ms"), Some(&Value::Null));
    let pairs = reading.get("pairs").expect("the pairs are counted");
    assert_eq!(pairs.get("separated"), Some(&Value::Integer(1)));
    assert_eq!(pairs.get("did_not_separate"), Some(&Value::Integer(2)));

    let mut pairs = super::Pairs::of(3);
    pairs.separated = 2;
    pairs.pin_missed = 1;
    let reading = super::rung_reading(
        512,
        &mut [16_000_000, 16_078_000],
        &mut [1_000, 2_000],
        (None, None),
        &pairs,
        (None, None),
    );
    assert_eq!(reading.get("spread_ns"), Some(&Value::Integer(78_000)));
    assert_eq!(
        reading.get("ns_per_token"),
        Some(&Value::Integer(16_078_000)),
        "the median of two is the upper, as the middle of a sorted list"
    );

    let none = super::rung_reading(
        512,
        &mut Vec::new(),
        &mut Vec::new(),
        (None, None),
        &super::Pairs::of(3),
        (None, None),
    );
    assert_eq!(none.get("measured"), Some(&Value::Bool(false)));
    assert!(
        none.get("pairs").is_some(),
        "an unmeasured rung counts its pairs too"
    );
}

/// What the hub answered is kept for a day and served from there with when
/// it was read; fresh asks again; a refusal is not kept (B-488).
#[test]
fn what_the_hub_answered_is_kept_for_a_day() {
    let machine = Machine::new("hub-kept");
    let home = machine.root.clone();
    let asked = std::cell::Cell::new(0_u32);
    let answer = || {
        asked.set(asked.get() + 1);
        Answer::served(Value::map([("query", Value::text("gemma"))]))
    };
    let first = super::kept_answer(&home, "search:gemma@", false, answer);
    assert!(first.served);
    assert_eq!(first.body.get("kept"), Some(&Value::Bool(false)));
    assert!(
        first.body.get("read_at").is_some(),
        "when it was read is on it"
    );
    assert_eq!(
        first.body.get("done"),
        Some(&Value::Bool(true)),
        "one answer says it is the whole of it, so a reader of a stream does not take the close for a death (F194)"
    );
    let again = super::kept_answer(&home, "search:gemma@", false, answer);
    assert_eq!(
        asked.get(),
        1,
        "the second ask within the day reached the hub"
    );
    assert_eq!(again.body.get("kept"), Some(&Value::Bool(true)));
    assert_eq!(again.body.get("done"), Some(&Value::Bool(true)));
    assert_eq!(again.body.get("query"), Some(&Value::text("gemma")));
    let fresh = super::kept_answer(&home, "search:gemma@", true, answer);
    assert_eq!(asked.get(), 2, "fresh did not reach the hub");
    assert_eq!(fresh.body.get("kept"), Some(&Value::Bool(false)));
    let refused = super::kept_answer(&home, "search:nothing@", false, || {
        Answer::refused(&crate::control::refused("no", "nothing"))
    });
    assert!(!refused.served);
    let asked_again = std::cell::Cell::new(0_u32);
    let _served = super::kept_answer(&home, "search:nothing@", false, || {
        asked_again.set(1);
        Answer::served(Value::map([("query", Value::text("nothing"))]))
    });
    assert_eq!(
        asked_again.get(),
        1,
        "a refusal was kept as if it were an answer"
    );
}

/// A run's body kept by the daemon carries when it was recorded, so that
/// a surface can say when a diagnostic last ran (D53, B-507).
#[test]
fn a_kept_run_body_is_dated() {
    let at = mcf_core::time::Timestamp::now();
    let dated = super::dated(Value::map([("readings", Value::List(Vec::new()))]), at);
    assert_eq!(dated.get("at"), Some(&Value::text(at.to_string())));
    assert!(
        dated.get("readings").is_some(),
        "the body is otherwise as it was"
    );
    assert_eq!(
        super::dated(Value::Null, at),
        Value::Null,
        "only a map is dated"
    );
}

/// A model's readings are answered from the record, newest run first, each
/// dated, and by one method where asked (D54, B-511).
#[test]
fn readings_are_answered_from_the_record_newest_first() {
    let machine = Machine::new("readings");
    let places = machine.places();
    let model = places.models.join("a-model.gguf");
    {
        let mut journal =
            mcf_record::journal::Journal::open(&places.journal).expect("a journal opens");
        for (method, value) in [
            ("prefill-saturation", 405),
            ("prefill-saturation", 254),
            ("cold-start", 7),
        ] {
            let body = mcf_record::readings::run_body(
                &model.display().to_string(),
                method,
                "provisioned",
                vec![("depth", Value::Integer(1024))],
                &[mcf_record::readings::Reading::new(
                    &[("batch", Value::Integer(64))],
                    "read_ns",
                    value,
                    "ns",
                )],
            );
            journal
                .append(&mcf_record::journal::Entry::new(
                    mcf_record::journal::EntryKind::Readings,
                    mcf_core::time::Timestamp::now(),
                    body,
                ))
                .expect("it appends");
        }
    }
    let (handle, socket) = running(places);
    let all = ask(
        &socket,
        &Request::Readings {
            model: model.display().to_string(),
            method: None,
        },
    );
    assert!(all.served, "{:?}", all.body);
    let runs = all.body.get("runs").and_then(Value::as_list).expect("runs");
    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs[0].get("method").and_then(Value::as_text),
        Some("cold-start"),
        "the newest run is not first: {runs:?}"
    );
    assert!(runs[0].get("at").is_some(), "a run is not dated");
    let rows = mcf_record::readings::rows_of(&runs[1]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, 254);
    assert_eq!(rows[0].dim("batch"), "64");
    let one = ask(
        &socket,
        &Request::Readings {
            model: model.display().to_string(),
            method: Some("prefill-saturation".to_owned()),
        },
    );
    let runs = one.body.get("runs").and_then(Value::as_list).expect("runs");
    assert_eq!(runs.len(), 2, "one method's runs only");
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

/// The newest classified failures are answered from the record, newest
/// first, with how many the record holds (B-074).
#[test]
fn the_newest_failures_are_answered_from_the_record_newest_first() {
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
    let machine = Machine::new("failures");
    let places = machine.places();
    {
        let mut journal =
            mcf_record::journal::Journal::open(&places.journal).expect("a journal opens");
        for (category, detail) in [
            (Category::EngineExitImmediate, "the first"),
            (Category::EngineSpawnRefused, "the second"),
        ] {
            let failure = Failure::new(
                category,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::daemon"),
                detail,
            )
            .with_context("looked_for", "/x");
            journal
                .append(&mcf_record::journal::Entry::new(
                    mcf_record::journal::EntryKind::Failure,
                    mcf_core::time::Timestamp::now(),
                    mcf_record::encode::failure(&failure),
                ))
                .expect("the failure is recorded");
        }
    }
    let (handle, socket) = running(places);

    let answer = ask(&socket, &Request::Failures { last: 1 });
    assert!(answer.served, "{:?}", answer.body);
    let listed = answer
        .body
        .get("failures")
        .and_then(Value::as_list)
        .expect("a list");
    assert_eq!(listed.len(), 1);
    let newest = &listed[0];
    assert_eq!(
        newest
            .get("body")
            .and_then(|body| body.get("detail"))
            .and_then(Value::as_text),
        Some("the second")
    );
    assert_eq!(
        newest
            .get("body")
            .and_then(|body| body.get("subsystem"))
            .and_then(Value::as_text),
        Some("mcf-serve::daemon")
    );
    assert!(newest.get("recorded_at").is_some());
    assert_eq!(
        answer.body.get("in_record").and_then(Value::as_integer),
        Some(2)
    );

    let all = ask(&socket, &Request::Failures { last: 20 });
    assert_eq!(
        all.body
            .get("failures")
            .and_then(Value::as_list)
            .map(<[Value]>::len),
        Some(2)
    );

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: "done".to_owned(),
        },
    );
    let _joined = handle.join();
}

/// A refusal the daemon answers is a classified failure in the record, with
/// what was asked beside it; the same refusal within a minute is one row,
/// and a different one is another (B-588).
#[test]
fn a_refusal_answered_is_recorded_once_a_minute_per_refusal() {
    let machine = Machine::new("refusals");
    let (handle, socket) = running(machine.places());

    let refused = ask(
        &socket,
        &Request::Anatomy {
            model: "/nowhere/at/all.gguf".to_owned(),
        },
    );
    assert!(!refused.served, "{:?}", refused.body);
    let again = ask(
        &socket,
        &Request::Anatomy {
            model: "/nowhere/at/all.gguf".to_owned(),
        },
    );
    assert!(!again.served);

    let failures = ask(&socket, &Request::Failures { last: 10 });
    let listed = failures
        .body
        .get("failures")
        .and_then(Value::as_list)
        .expect("a list");
    assert_eq!(listed.len(), 1, "{listed:?}");
    let body = listed[0].get("body").expect("a body");
    assert_eq!(body.get("asked").and_then(Value::as_text), Some("anatomy"));
    assert!(body.get("category").and_then(Value::as_text).is_some());
    assert_eq!(
        body.get("category").and_then(Value::as_text),
        refused.body.get("category").and_then(Value::as_text)
    );
    assert_eq!(
        failures.body.get("in_record").and_then(Value::as_integer),
        Some(1)
    );

    let other = ask(
        &socket,
        &Request::Settings {
            model: "/nowhere/else.gguf".to_owned(),
        },
    );
    assert!(!other.served);
    let failures = ask(&socket, &Request::Failures { last: 10 });
    assert_eq!(
        failures
            .body
            .get("failures")
            .and_then(Value::as_list)
            .map(<[Value]>::len),
        Some(2)
    );

    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: "done".to_owned(),
        },
    );
    let _joined = handle.join();
}
