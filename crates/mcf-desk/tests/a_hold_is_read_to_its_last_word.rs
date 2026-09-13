#![allow(clippy::expect_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use mcf_record::json::Value;
use mcf_serve::control::Request;

struct Fake {
    path: PathBuf,
}

impl Fake {
    fn answering(name: &str, lines: Vec<String>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-fake-{name}-{}-{:?}.sock",
            std::process::id(),
            std::thread::current().id()
        ));
        let _gone = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("a socket to answer on");
        let _serving = std::thread::spawn(move || {
            let Ok((stream, _who)) = listener.accept() else {
                return;
            };
            let mut asked = String::new();
            let _read = BufReader::new(&stream).read_line(&mut asked);
            let mut writing = &stream;
            for line in lines {
                if writeln!(writing, "{line}").is_err() {
                    return;
                }
                let _flushed = writing.flush();
            }
        });
        Self { path }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _gone = std::fs::remove_file(&self.path);
    }
}

fn answered(body: Value, served: bool) -> String {
    Value::map([
        ("protocol", Value::Integer(1)),
        ("served", Value::Bool(served)),
        ("answer", body),
    ])
    .to_line()
}

fn loading(seconds: i64) -> String {
    answered(
        Value::map([
            ("hosting", Value::text("/m/a-model.gguf")),
            (
                "loading",
                Value::map([("seconds", Value::Integer(seconds))]),
            ),
            ("done", Value::Bool(false)),
        ]),
        true,
    )
}

fn held_at(port: u16) -> String {
    answered(
        Value::map([
            ("hosting", Value::text("/m/a-model.gguf")),
            ("address", Value::text(format!("http://127.0.0.1:{port}"))),
            ("done", Value::Bool(true)),
        ]),
        true,
    )
}

fn asked(fake: &Fake) -> Result<mcf_serve::control::Answer, String> {
    mcf_desk::asked_until_done(
        &fake.path,
        &Request::Host {
            model: "/m/a-model.gguf".to_owned(),
            settings: Value::Null,
        },
        std::time::Duration::from_secs(10),
        |_along| {},
    )
}

#[test]
fn a_hold_that_reports_progress_first_is_read_past_it_to_the_address() {
    let fake = Fake::answering(
        "progress-then-address",
        vec![loading(0), loading(1), loading(2), held_at(17_817)],
    );
    let answer = asked(&fake).expect("an answer");
    assert!(answer.served);
    assert_eq!(
        answer.body.get("address").and_then(Value::as_text),
        Some("http://127.0.0.1:17817"),
        "reading only the first line gets a progress report, which names no port at all"
    );
}

#[test]
fn a_hold_that_answers_at_once_is_read_at_once() {
    let fake = Fake::answering("straight-away", vec![held_at(1234)]);
    let answer = asked(&fake).expect("an answer");
    assert_eq!(
        answer.body.get("address").and_then(Value::as_text),
        Some("http://127.0.0.1:1234")
    );
}

#[test]
fn a_refusal_partway_through_stops_the_reading_there() {
    let fake = Fake::answering(
        "refused",
        vec![
            loading(0),
            answered(
                Value::map([("message", Value::text("the engine would not start"))]),
                false,
            ),
            held_at(9999),
        ],
    );
    let answer = asked(&fake).expect("an answer");
    assert!(
        !answer.served,
        "a refusal is the answer, and nothing after it is read as success"
    );
}

#[test]
fn a_stream_that_stops_without_finishing_gives_back_what_it_did_say() {
    let fake = Fake::answering("cut-short", vec![loading(0), loading(1)]);
    let answer = asked(&fake).expect("the last thing it said");
    assert!(
        answer.body.get("address").is_none(),
        "there is no address in a progress line, and none is invented"
    );
}

#[test]
fn a_port_is_read_off_the_address_the_daemon_gives() {
    assert_eq!(mcf_desk::port_of("http://127.0.0.1:17817"), Some(17_817));
    assert_eq!(mcf_desk::port_of("http://127.0.0.1:17817/"), Some(17_817));
    assert_eq!(mcf_desk::port_of("nothing-like-an-address"), None);
    assert_eq!(mcf_desk::port_of(""), None);
}
