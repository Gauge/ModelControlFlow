use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::{Daemon, Places, Stopped};
use crate::control::{Answer, Request};
use mcf_core::failure::Category;
use mcf_record::json::Value;

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

fn ask(socket: &std::path::Path, request: &Request) -> Answer {
    ask_within(socket, request, Duration::from_secs(5))
}

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

fn running(places: Places) -> (thread::JoinHandle<Stopped>, PathBuf) {
    let socket = places.socket.clone();
    let mut daemon = Daemon::start(places).expect("it starts");
    let handle = thread::spawn(move || daemon.serve());
    (handle, socket)
}

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

#[test]
fn a_restart_recovers_what_the_record_holds() {
    let machine = Machine::new("recovery");
    let places = machine.places();

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

#[test]
fn a_socket_left_by_a_dead_daemon_is_taken_over() {
    let machine = Machine::new("leftover");
    let places = machine.places();
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

    assert!(ask(&socket, &Request::Status).served);
    let _stopped = ask(
        &socket,
        &Request::Stop {
            reason: String::new(),
        },
    );
    let _ended = handle.join();
}

#[test]
fn a_client_that_says_nothing_does_not_wedge_it() {
    let machine = Machine::new("silent-client");
    let (handle, socket) = running(machine.places());

    let silent = UnixStream::connect(&socket).expect("it is listening");
    thread::sleep(Duration::from_millis(50));

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

#[test]
fn what_it_is_holding_is_read_from_the_disk() {
    let machine = Machine::new("holding");
    let places = machine.places();
    let (handle, socket) = running(places.clone());

    let empty = ask(&socket, &Request::Holding);
    assert!(empty.served, "{:?}", empty.body);

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

#[test]
fn a_files_name_shortens_toward_its_repositorys() {
    let mut name = "Vega3.8-Flash-Next-UD-Q3_K_XL-00001-of-00003.gguf".to_owned();
    let mut tried = Vec::new();
    while let Some(shorter) = super::shorter_name(&name) {
        tried.push(shorter.clone());
        name = shorter;
    }
    assert_eq!(
        tried,
        [
            "Vega3.8-Flash-Next-UD-Q3_K_XL-00001-of-00003",
            "Vega3.8-Flash-Next-UD-Q3_K_XL",
            "Vega3.8-Flash-Next-UD",
            "Vega3.8-Flash-Next",
            "Vega3.8-Flash",
            "Vega3.8",
        ]
    );
    assert_eq!(super::shorter_name("Vega3.8"), None);
    assert_eq!(super::shorter_name(""), None);
    assert_eq!(super::shorter_name("-"), None);
}

/// A rate is a difference between two readings, and the span between them has to be long
/// enough for a whole token to have arrived in it.
///
/// The window used to ask what the hold was doing twice in a row, a millisecond apart. The
/// second ask measured against the first, no token had arrived in between, and so every
/// rate read zero for as long as the model was working — which is exactly when a rate is
/// worth reading. Readings are kept in a short history now and the one to measure against
/// is chosen from it, so how often somebody asks cannot decide what the answer is.
mod rates {
    use super::super::{RATE_OVER_AT_LEAST_NS, READINGS_KEPT, readings_for};
    use crate::served::Reach;
    use mcf_core::time::{Instant, Monotonic};

    fn a_reach(port: u16) -> Reach {
        Reach::Port { port, key: None }
    }

    /// A clock this test holds the hands of. Readings are chosen by how far apart they
    /// are, so the test has to be able to say how far apart they are rather than sleep
    /// and hope.
    fn at(nanos: u64) -> Instant<Monotonic> {
        Instant::from_nanos(nanos.saturating_add(1_000_000_000_000))
    }

    #[test]
    fn the_first_reading_measures_against_nothing_and_states_no_rate() {
        let (against, over) = readings_for(&a_reach(19_001), at(0), Some(0), Some(0));
        assert!(
            against.is_none(),
            "a rate was stated against a reading that was never taken"
        );
        assert_eq!(over, 0);
    }

    #[test]
    fn two_readings_a_moment_apart_state_no_rate_rather_than_a_rate_of_zero() {
        let reach = a_reach(19_002);
        let _first = readings_for(&reach, at(0), Some(100), Some(10));
        let (against, over) = readings_for(&reach, at(1_000_000), Some(100), Some(10));
        assert!(
            against.is_none(),
            "a rate measured over no time at all reads as a model doing nothing when it is \
             working: {over}ns"
        );
    }

    #[test]
    fn a_reading_far_enough_back_is_what_a_rate_is_measured_against() {
        let reach = a_reach(19_003);
        let _first = readings_for(&reach, at(0), Some(100), Some(10));
        let later = at(RATE_OVER_AT_LEAST_NS.saturating_mul(2));
        let (against, over) = readings_for(&reach, later, Some(300), Some(40));
        let against = against.expect("the reading taken a moment ago is there to measure against");
        assert_eq!(against.generated, 100);
        assert_eq!(against.prompted, 10);
        assert!(
            over >= RATE_OVER_AT_LEAST_NS,
            "a rate was stated over a span shorter than MCF states rates over: {over}ns"
        );
    }

    #[test]
    fn asking_far_more_often_than_once_a_second_still_measures_over_a_usable_span() {
        let reach = a_reach(19_004);
        let step = RATE_OVER_AT_LEAST_NS.saturating_div(8);
        let mut generated = 0;
        let mut last = None;
        for tick in 0..24_u64 {
            generated += 5;
            last = Some(readings_for(
                &reach,
                at(step.saturating_mul(tick)),
                Some(generated),
                Some(0),
            ));
        }
        let (against, over) = last.expect("there were readings");
        assert!(
            against.is_some(),
            "a caller asking eight times a second was told no rate at all"
        );
        assert!(
            over >= RATE_OVER_AT_LEAST_NS,
            "the span was decided by how often the caller asked rather than by MCF: {over}ns"
        );
    }

    #[test]
    fn the_history_of_one_engine_does_not_grow_without_end() {
        let reach = a_reach(19_005);
        for tick in 0..(READINGS_KEPT.saturating_mul(3) as u64) {
            let _read = readings_for(
                &reach,
                at(tick.saturating_mul(1_000_000)),
                Some(tick),
                Some(0),
            );
        }
        let held = super::super::COUNTED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let kept = held
            .get(&super::super::where_it_answers(&reach))
            .expect("kept");
        assert!(
            kept.len() <= READINGS_KEPT,
            "a daemon left up for a week would hold every reading it ever took: {}",
            kept.len()
        );
    }
}

/// The queue of what MCF is bringing here, over the control socket.
///
/// A transfer is not a hold: it writes bytes to a disk and takes nothing from whatever is
/// running, so several are asked for at once and none of them waits on the one-model rule.
/// Everything about the queue is answered in one shape, because every client reads it with
/// the same code.
mod the_queue {
    use super::{Machine, Request, ask, running};
    use mcf_record::json::Value;

    fn rows(body: &Value) -> Vec<Value> {
        body.get("transfers")
            .and_then(Value::as_list)
            .unwrap_or_else(|| panic!("the queue's rows are at the top of every answer: {body:?}"))
            .to_vec()
    }

    fn stop(socket: &std::path::Path) {
        let _stopped = ask(
            socket,
            &Request::Stop {
                reason: "the test is done with it".to_owned(),
            },
        );
    }

    fn asking_for(file: &str) -> Request {
        Request::Queue {
            reference: "owner/model".to_owned(),
            file: file.to_owned(),
            from: Some("http://127.0.0.1:1/".to_owned()),
        }
    }

    #[test]
    fn more_than_one_file_is_asked_for_and_none_of_them_waits_on_the_others() {
        let machine = Machine::new("queue-several");
        let (handle, socket) = running(machine.places());

        let mut ids = Vec::new();
        for file in ["one.gguf", "two.gguf", "three.gguf"] {
            let answered = ask(&socket, &asking_for(file));
            assert!(answered.served, "{:?}", answered.body);
            let id = answered
                .body
                .get("id")
                .and_then(Value::as_integer)
                .expect("a transfer is numbered so it can be spoken about");
            assert!(!ids.contains(&id), "two transfers were given one number");
            ids.push(id);
        }
        let listed = ask(&socket, &Request::Transfers);
        assert!(listed.served, "{:?}", listed.body);
        assert_eq!(
            rows(&listed.body).len(),
            3,
            "a file asked for while others were arriving was refused or dropped"
        );

        stop(&socket);
        let _ended = handle.join();
    }

    #[test]
    fn every_answer_about_the_queue_has_the_same_shape() {
        let machine = Machine::new("queue-shape");
        let (handle, socket) = running(machine.places());

        // Asking for one, listing them, and dropping the finished ones are the three
        // answers that always come back served, whatever state a transfer has reached.
        // Pausing is not: by the time this runs the transfer may already have failed, and
        // a pause of something that has finished is rightly refused.
        let queued = ask(&socket, &asking_for("one.gguf"));
        let _read = rows(&queued.body);
        for asked in [Request::Transfers, Request::ForgetTransfers] {
            let answered = ask(&socket, &asked);
            assert!(
                answered.served,
                "{asked:?} was refused: {:?}",
                answered.body
            );
            let _read = rows(&answered.body);
        }

        stop(&socket);
        let _ended = handle.join();
    }

    #[test]
    fn nothing_the_queue_never_had_is_answered_about() {
        let machine = Machine::new("queue-unknown");
        let (handle, socket) = running(machine.places());

        for asked in [
            Request::PauseTransfer { id: 404 },
            Request::ResumeTransfer { id: 404 },
            Request::GiveUpTransfer { id: 404 },
        ] {
            let answered = ask(&socket, &asked);
            assert!(
                !answered.served,
                "{asked:?} was answered about a transfer that does not exist: {:?}",
                answered.body
            );
        }

        stop(&socket);
        let _ended = handle.join();
    }

    #[test]
    fn a_file_the_repository_does_not_publish_is_refused_and_says_so_in_the_queue() {
        let machine = Machine::new("queue-refused");
        let (handle, socket) = running(machine.places());

        let queued = ask(&socket, &asking_for("nothing.gguf"));
        assert!(queued.served, "the asking itself is answered at once");

        // Nothing is listening on port 1, so the transfer fails. What matters is that it
        // fails in the queue with a refusal to read, rather than vanishing.
        let mut settled = None;
        for _ in 0..80 {
            let listed = ask(&socket, &Request::Transfers);
            let held = rows(&listed.body);
            let first = held.first().cloned().expect("it is still in the queue");
            let state = first
                .get("state")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned();
            if state == "failed" {
                settled = Some(first);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let failed = settled.expect("a transfer that cannot be made must say so, not hang");
        assert!(
            failed
                .get("why")
                .is_some_and(|why| !matches!(why, Value::Null)),
            "a failure the operator cannot read is one they cannot act on: {failed:?}"
        );

        stop(&socket);
        let _ended = handle.join();
    }

    #[test]
    fn the_finished_are_forgotten_when_asked_and_not_before() {
        let machine = Machine::new("queue-forget");
        let (handle, socket) = running(machine.places());

        let queued = ask(&socket, &asking_for("one.gguf"));
        let id = u64::try_from(
            queued
                .body
                .get("id")
                .and_then(Value::as_integer)
                .expect("numbered"),
        )
        .expect("a number");
        let _given = ask(&socket, &Request::GiveUpTransfer { id });

        let forgotten = ask(&socket, &Request::ForgetTransfers);
        assert!(forgotten.served, "{:?}", forgotten.body);
        assert!(
            forgotten
                .body
                .get("forgotten")
                .and_then(Value::as_integer)
                .is_some(),
            "what was dropped is not said: {:?}",
            forgotten.body
        );

        stop(&socket);
        let _ended = handle.join();
    }
}

/// What MCF reads off an engine's own counters.
///
/// The names are the engine's, and they change between builds. Reading only the names one
/// build happened to use leaves a figure reading "not measured yet" for the life of every
/// hold — a measurement that looks like it failed rather than one that was never offered.
mod engine_counters {
    use super::super::counters;

    /// Trimmed from what a current llama.cpp actually served on this machine, counters and
    /// all. The two `kv_cache_*` gauges are absent from it: they were dropped when the
    /// cache was unified, and what replaces them is the deepest sequence seen and how much
    /// of a prompt was reused.
    const AS_SERVED: &str = "\
# HELP llamacpp:prompt_tokens_total Number of prompt tokens processed
# TYPE llamacpp:prompt_tokens_total counter
llamacpp:prompt_tokens_total 96641
llamacpp:prompt_tokens_cached_total 2.16578e+06
llamacpp:prompt_seconds_total 227.44
llamacpp:tokens_predicted_total 23210
llamacpp:tokens_predicted_seconds_total 507.005
llamacpp:n_decode_total 23381
llamacpp:n_tokens_max 42094
llamacpp:spec_decode_num_draft_tokens_total 0
llamacpp:spec_decode_num_accepted_tokens_total 0
llamacpp:prompt_tokens_seconds 424.925
llamacpp:predicted_tokens_seconds 45.4769
llamacpp:requests_processing 0
llamacpp:requests_deferred 0
llamacpp:n_busy_slots_per_decode 1
";

    /// The older spelling, which MCF must go on reading: an engine built before the cache
    /// was unified is still a provisioned engine somebody is holding a model on.
    const AS_ONCE_SERVED: &str = "\
llamacpp:prompt_tokens_total 10
llamacpp:tokens_predicted_total 20
llamacpp:kv_cache_usage_ratio 0.5
llamacpp:kv_cache_tokens 1234
llamacpp:n_decode_total 30
";

    fn read(metrics: &str, key: &str) -> Option<String> {
        counters(metrics)
            .into_iter()
            .find(|(name, _)| *name == key)
            .and_then(|(_, value)| value.as_text().map(str::to_owned))
    }

    #[test]
    fn what_a_current_engine_serves_is_all_read() {
        for (key, expected) in [
            ("prompted_tokens", "96641"),
            ("generated_tokens", "23210"),
            ("decodes", "23381"),
            ("requests_processing", "0"),
            ("requests_queued", "0"),
        ] {
            assert_eq!(read(AS_SERVED, key).as_deref(), Some(expected), "{key}");
        }
    }

    #[test]
    fn the_figures_that_replaced_the_cache_gauges_are_read() {
        assert_eq!(
            read(AS_SERVED, "deepest_tokens").as_deref(),
            Some("42094"),
            "the deepest sequence the engine has seen went unread, and the tile that would \
             have shown it read as a measurement that failed"
        );
        assert_eq!(
            read(AS_SERVED, "prompt_tokens_reused").as_deref(),
            Some("2.16578e+06"),
            "what the prompt cache saved went unread — on a conversation that keeps its \
             prefix this is most of the prompt"
        );
    }

    #[test]
    fn the_engines_own_average_throughput_is_read() {
        assert_eq!(
            read(AS_SERVED, "engine_said_tokens_per_second").as_deref(),
            Some("45.4769")
        );
        assert_eq!(
            read(AS_SERVED, "engine_said_prompt_tokens_per_second").as_deref(),
            Some("424.925")
        );
    }

    #[test]
    fn an_older_engines_cache_gauges_are_still_read() {
        assert_eq!(
            read(AS_ONCE_SERVED, "cache_used_ratio").as_deref(),
            Some("0.5")
        );
        assert_eq!(
            read(AS_ONCE_SERVED, "cache_tokens").as_deref(),
            Some("1234")
        );
        assert_eq!(
            read(AS_ONCE_SERVED, "generated_tokens").as_deref(),
            Some("20")
        );
    }

    #[test]
    fn a_body_that_is_not_metrics_at_all_yields_nothing() {
        // What a 401 looks like. It has a space in it, so it splits like a metric line;
        // nothing it splits into is a name MCF knows, and nothing must come of it.
        let refused = "{\"error\":{\"message\":\"Invalid API Key\",\"code\":401}}";
        assert!(counters(refused).is_empty(), "{:?}", counters(refused));
        assert!(counters("").is_empty());
    }

    #[test]
    fn comments_are_not_counters() {
        let only_help = "# HELP llamacpp:prompt_tokens_total Number of prompt tokens\n\
                         # TYPE llamacpp:prompt_tokens_total counter\n";
        assert!(counters(only_help).is_empty());
    }

    #[test]
    fn every_name_read_is_one_the_window_asks_for() {
        // The daemon and the window agree on these names by nothing but spelling, so the
        // spelling is what this pins.
        let read: Vec<&str> = counters(AS_SERVED)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        for wanted in [
            "prompted_tokens",
            "generated_tokens",
            "decodes",
            "deepest_tokens",
            "prompt_tokens_reused",
            "engine_said_tokens_per_second",
        ] {
            assert!(read.contains(&wanted), "{wanted} is not among {read:?}");
        }
    }

    #[test]
    fn a_count_past_what_a_single_holds_exactly_survives_the_reading() {
        // Sixteen million is where an f32 stops holding whole numbers exactly. A token
        // counter passes it on a long session, which is when it is worth reading.
        let big = "llamacpp:tokens_predicted_total 99000001\n";
        let said = read(big, "generated_tokens").expect("read");
        let held = mcf_record::json::Value::text(said.clone());
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the cast the window makes, which is what this is about"
        )]
        let as_read = held
            .as_text()
            .and_then(|text| text.trim().parse::<f64>().ok())
            .map(|held| held as u64);
        assert_eq!(
            as_read,
            Some(99_000_001),
            "the count came back rounded: {said}"
        );
    }
}
