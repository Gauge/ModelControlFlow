//! A hub, answered by a server this test is holding.
//!
//! The answers below are the shapes the real hub sends
//! ([findings.md](../../../../doc/findings.md) F9 and its `/api/models` and
//! `/tree` calls), served from the loopback address so the whole path — two
//! metadata calls, a redirect, a range, a digest — runs in the gating tier with
//! no network (B19, B38).

use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::{Hub, METADATA_CEILING};
use crate::credentials::{Credential, Identity, Origin, Secret};
use crate::http::Url;
use crate::reference::{self, Reference};
use crate::source::{Entry, Source as _};
use crate::wire::{Deadlines, Duplex, Tcp, Wire};
use mcf_core::digest::sha256;
use mcf_core::failure::Category;

/// A wire that carries what TCP carries and claims it can keep a secret.
///
/// The loopback address is not a network, so a credential on it goes nowhere —
/// but the *code* under test is the one that decides whether a credential may
/// travel, and testing it needs a wire that says yes. Deliberately not a
/// capability MCF offers an operator: the only honest yes is TLS.
struct Trusted(Tcp);

impl Wire for Trusted {
    fn describe(&self) -> String {
        format!("{} the laboratory trusts", self.0.describe())
    }

    fn carries_secrets(&self) -> bool {
        true
    }

    fn dial(&self, host: &str, port: u16) -> mcf_core::failure::Result<Box<dyn Duplex>> {
        self.0.dial(host, port)
    }
}

/// A hub that answers by path.
struct Server {
    port: u16,
    /// What was asked for, in order.
    asked: Arc<Mutex<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn answering(answers: BTreeMap<String, String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let port = listener.local_addr().expect("an address").port();
        let asked = Arc::new(Mutex::new(Vec::new()));
        let recording = Arc::clone(&asked);

        let handle = thread::spawn(move || {
            for connection in listener.incoming() {
                let Ok(stream) = connection else { break };
                let request = read_request(&stream);
                let Some(line) = request.lines().next().map(str::to_owned) else {
                    // The connection this test's own `Drop` makes to wake the
                    // listener: nothing was asked, so nothing more is coming.
                    break;
                };
                if let Ok(mut seen) = recording.lock() {
                    seen.push(request.clone());
                }
                let target = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                let answer = answers.get(&target).cloned().unwrap_or_else(|| {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_owned()
                });
                let mut stream = stream;
                let _written = stream.write_all(answer.as_bytes());
                let _flushed = stream.flush();
                let _closed = stream.shutdown(std::net::Shutdown::Write);
            }
        });

        Self {
            port,
            asked,
            handle: Some(handle),
        }
    }

    fn base(&self) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}/", self.port)).expect("a URL")
    }

    fn hub(&self) -> Hub {
        Hub::at(self.base(), Box::new(tcp()))
    }

    fn trusting_hub(&self) -> Hub {
        Hub::at(self.base(), Box::new(Trusted(tcp())))
    }

    fn asked(&self) -> Vec<String> {
        self.asked
            .lock()
            .map(|seen| seen.clone())
            .unwrap_or_default()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Ok(waker) = TcpStream::connect(("127.0.0.1", self.port)) {
            drop(waker);
        }
        if let Some(handle) = self.handle.take() {
            let _joined = handle.join();
        }
    }
}

fn tcp() -> Tcp {
    Tcp {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            idle: Duration::from_millis(500),
        },
    }
}

fn read_request(stream: &TcpStream) -> String {
    let _deadline = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = stream;
    let mut seen = Vec::new();
    let mut byte = [0_u8; 1];
    while reader.read(&mut byte).unwrap_or(0) == 1 {
        seen.push(byte[0]);
        if seen.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&seen).into_owned()
}

fn answer(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// The card the hub sends, in its own shape — including the tags §XII's hard
/// case lives in.
fn card() -> String {
    answer(
        r#"{"id":"owner/model","sha":"50968a4468ef4233ed78cd7c3de230dd1d61a56b","gated":false,
            "tags":["gguf","license:apache-2.0","base_model:somebody/original",
            "base_model:quantized:somebody/original"],
            "cardData":{"license":"apache-2.0"}}"#,
    )
}

/// The tree the hub sends, with an LFS digest on the weights and none on the
/// small files.
fn tree() -> String {
    answer(
        r#"[{"type":"file","oid":"c4d8","size":3135,"path":".gitattributes"},
            {"type":"directory","oid":"aaaa","size":0,"path":"subfolder"},
            {"type":"file","oid":"79c9","size":6,"lfs":{"oid":"1111111111111111111111111111111111111111111111111111111111111111","size":6,"pointerSize":134},"path":"model.gguf"}]"#,
    )
}

fn a_hub_with(extra: &[(&str, String)]) -> Server {
    Server::answering(answers_with(extra))
}

/// The two answers every listing needs, plus whatever a test replaces or adds.
fn answers_with(extra: &[(&str, String)]) -> BTreeMap<String, String> {
    let mut answers = BTreeMap::new();
    answers.insert("/api/models/owner/model".to_owned(), card());
    answers.insert(
        "/api/models/owner/model/tree/50968a4468ef4233ed78cd7c3de230dd1d61a56b?recursive=true"
            .to_owned(),
        tree(),
    );
    for (target, answer) in extra {
        answers.insert((*target).to_owned(), answer.clone());
    }
    answers
}

fn a_reference() -> Reference {
    reference::parse("owner/model").expect("a reference")
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mcf-client-{name}-{}-{:?}",
        std::process::id(),
        thread::current().id()
    ));
    let _fresh = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a scratch directory");
    path
}

/// The listing is what B-213 plans against and what B-021 verifies: every file,
/// its size, and the digest the hub declares for the ones that have one.
#[test]
fn a_listing_carries_the_sizes_the_digests_and_the_revision() {
    let server = a_hub_with(&[]);
    let listing = server.hub().list(&a_reference()).expect("a listing");

    assert_eq!(
        listing.revision.as_deref(),
        Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b"),
        "the revision to pin is the hub's, not the branch that was asked for"
    );
    assert_eq!(listing.declared_licence.as_deref(), Some("apache-2.0"));
    assert_eq!(listing.entries.len(), 2, "{:?}", listing.entries);

    let weights = listing.entry("model.gguf").expect("the weights are listed");
    assert_eq!(weights.size, 6);
    assert_eq!(
        weights.digest.as_deref(),
        Some("1111111111111111111111111111111111111111111111111111111111111111")
    );
    let small = listing.entry(".gitattributes").expect("listed");
    assert_eq!(
        small.digest, None,
        "a git blob hash was recorded as though it were the file's digest"
    );
}

/// A repository with no `cardData` still has terms, in the tag the hub
/// publishes them under.
#[test]
fn terms_are_read_from_the_tag_when_the_card_has_no_field() {
    let server = Server::answering(answers_with(&[
        (
            "/api/models/owner/model",
            answer(r#"{"sha":"abc","tags":["gguf","license:llama3.1"]}"#),
        ),
        ("/api/models/owner/model/tree/abc?recursive=true", tree()),
    ]));

    let listing = server.hub().list(&a_reference()).expect("a listing");
    assert_eq!(listing.declared_licence.as_deref(), Some("llama3.1"));
}

/// The two questions MCF asks before a byte of weights moves, and no others.
#[test]
fn a_listing_costs_two_cheap_questions() {
    let server = a_hub_with(&[]);
    let _listing = server.hub().list(&a_reference()).expect("a listing");

    let asked = server.asked();
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(
        asked[0].contains("GET /api/models/owner/model HTTP/1.1"),
        "{asked:?}"
    );
    assert!(asked[1].contains("/tree/50968a44"), "{asked:?}");
    assert!(asked[1].contains("recursive=true"), "{asked:?}");
}

/// A name the hub publishes nothing at is that, and not a mystery.
#[test]
fn a_repository_that_is_not_there_is_not_found() {
    let server = Server::answering(BTreeMap::new());
    let failure = server
        .hub()
        .list(&a_reference())
        .expect_err("nothing there");
    assert_eq!(failure.category(), Category::HubRefNotFound);
}

/// A private repository asked for without a credential says which is missing,
/// in `mcf_hub::credentials`' own words (B-024).
#[test]
fn a_private_repository_asks_for_a_credential() {
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n".to_owned(),
    )]);
    let failure = server.hub().list(&a_reference()).expect_err("unauthorized");
    assert_eq!(failure.category(), Category::HubAuthRequired);
    assert_eq!(
        failure.context_value("repository"),
        Some("owner/model"),
        "the refusal does not name the repository an operator has to act on"
    );
    assert!(
        failure.context().iter().any(|entry| entry
            .value
            .contains("does not take one from the environment")),
        "the refusal is not the one B-024 writes"
    );
}

/// And with a credential the hub refuses, it is the other answer.
#[test]
fn a_refused_credential_is_a_different_answer() {
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n".to_owned(),
    )]);
    let hub = server
        .trusting_hub()
        .offering(Credential::new(Secret::new("hf_expired"), Origin::Supplied));

    let failure = hub.list(&a_reference()).expect_err("refused");
    assert_eq!(failure.category(), Category::HubAuthRejected);
    assert!(!format!("{failure:?}").contains("hf_expired"));
    assert!(matches!(hub.identity(), Identity::Offered { .. }));
}

/// Terms not accepted is its own answer, and no better token will help.
#[test]
fn a_gated_repository_says_the_terms_are_the_problem() {
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".to_owned(),
    )]);
    let failure = server.hub().list(&a_reference()).expect_err("gated");
    assert_eq!(failure.category(), Category::HubAccessGated);
}

/// Throttling carries the hub's own hint, so waiting is a decision rather than
/// a guess (B7).
#[test]
fn throttling_carries_the_hint_the_hub_gave() {
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 30\r\nContent-Length: 0\r\n\r\n".to_owned(),
    )]);
    let failure = server.hub().list(&a_reference()).expect_err("throttled");
    assert_eq!(failure.category(), Category::HubRateLimited);
    assert_eq!(
        failure.context_value("retry_after_seconds"),
        Some("30"),
        "the hint the hub gave was not kept"
    );
}

/// A credential is not sent to a hub over a wire that cannot keep it: refused
/// rather than downgraded (B-024).
#[test]
fn a_credential_is_not_offered_over_a_wire_that_cannot_keep_it() {
    let server = a_hub_with(&[]);
    let hub = server
        .hub()
        .offering(Credential::new(Secret::new("hf_token"), Origin::Supplied));
    let failure = hub.list(&a_reference()).expect_err("refused");
    assert_eq!(failure.category(), Category::ConfigInvalid);
    assert!(server.asked().is_empty(), "the request went out anyway");
}

/// The file arrives, and the digest is of what arrived rather than of what the
/// source said it sent (§3.7, B-021).
#[test]
fn a_fetch_writes_the_file_and_digests_what_arrived() {
    let server = a_hub_with(&[(
        "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf",
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nGGUFxx".to_owned(),
    )]);
    let listing = server.hub().list(&a_reference()).expect("a listing");
    let entry = listing.entry("model.gguf").expect("listed").clone();

    let into = scratch("fetch").join("model.gguf");
    let fetched = server
        .hub()
        .fetch(&listing.reference, &entry, &into)
        .expect("the fetch runs");

    assert_eq!(std::fs::read(&into).expect("readable"), b"GGUFxx");
    assert_eq!(fetched.bytes, 6);
    assert_eq!(fetched.digest, sha256(b"GGUFxx").hex());
    let _cleared = std::fs::remove_dir_all(into.parent().unwrap_or(&into));
}

/// A download that redirects to another host is followed, which is what the
/// real hub does with every file (F9 §9.1).
#[test]
fn a_download_follows_the_redirect_the_hub_sends() {
    let cdn = Server::answering(BTreeMap::from([(
        "/xet-bridge-us/abc?Expires=1".to_owned(),
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nGGUFxx".to_owned(),
    )]));
    let server = Server::answering(answers_with(&[(
        "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf",
        format!(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{}/xet-bridge-us/abc?Expires=1\r\nContent-Length: 0\r\n\r\n",
            cdn.port
        ),
    )]));
    let hub = Hub::at(server.base(), Box::new(tcp()));

    let listing = hub.list(&a_reference()).expect("a listing");
    let entry = listing.entry("model.gguf").expect("listed").clone();
    let into = scratch("redirect").join("model.gguf");
    let fetched = hub
        .fetch(&listing.reference, &entry, &into)
        .expect("the fetch runs");

    assert_eq!(fetched.bytes, 6);
    assert_eq!(fetched.digest, sha256(b"GGUFxx").hex());
    assert!(
        cdn.asked().len() == 1,
        "the CDN was not asked for the file: {:?}",
        cdn.asked()
    );
    let _cleared = std::fs::remove_dir_all(into.parent().unwrap_or(&into));
}

/// A resumption asks for the rest and appends it.
#[test]
fn a_resumption_appends_what_it_asked_for() {
    let target = "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf";
    let server = a_hub_with(&[(
        target,
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 4-5/6\r\nContent-Length: 2\r\n\r\nxx"
            .to_owned(),
    )]);
    let directory = scratch("resume");
    let into = directory.join("model.gguf");
    std::fs::write(&into, b"GGUF").expect("a partial file");

    let entry = Entry::new("model.gguf", 6);
    let reference = Reference {
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        ..a_reference()
    };
    let fetched = server
        .hub()
        .fetch_from(&reference, &entry, 4, &into)
        .expect("the resumption runs");

    assert_eq!(std::fs::read(&into).expect("readable"), b"GGUFxx");
    assert_eq!(
        fetched.bytes, 2,
        "the digest is of what arrived, not the file"
    );
    let asked = server.asked();
    assert!(
        asked
            .last()
            .is_some_and(|last| last.contains("Range: bytes=4-")),
        "{asked:?}"
    );
    let _cleared = std::fs::remove_dir_all(&directory);
}

/// A source that answers a resumption by starting again is refused: appending
/// what it sent would build a file that is the first part twice (B-021).
#[test]
fn a_source_that_restarts_a_resumption_is_refused() {
    let target = "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf";
    let server = a_hub_with(&[(
        target,
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-5/6\r\nContent-Length: 6\r\n\r\nGGUFxx"
            .to_owned(),
    )]);
    let directory = scratch("restart");
    let into = directory.join("model.gguf");
    std::fs::write(&into, b"GGUF").expect("a partial file");

    let reference = Reference {
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        ..a_reference()
    };
    let failure = server
        .hub()
        .fetch_from(&reference, &Entry::new("model.gguf", 6), 4, &into)
        .expect_err("it did not resume");
    assert_eq!(failure.category(), Category::TransferMutated);
    let _cleared = std::fs::remove_dir_all(&directory);
}

/// And one that answers a resumption with the whole file, saying nothing about
/// ranges, is refused for the same reason.
#[test]
fn a_source_that_ignores_the_range_is_refused() {
    let target = "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf";
    let server = a_hub_with(&[(
        target,
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nGGUFxx".to_owned(),
    )]);
    let directory = scratch("ignored-range");
    let into = directory.join("model.gguf");
    std::fs::write(&into, b"GGUF").expect("a partial file");

    let reference = Reference {
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        ..a_reference()
    };
    let failure = server
        .hub()
        .fetch_from(&reference, &Entry::new("model.gguf", 6), 4, &into)
        .expect_err("no range");
    assert_eq!(failure.category(), Category::HubUnreachable);
    let _cleared = std::fs::remove_dir_all(&directory);
}

/// A source that answers a small question with an enormous answer is noticed
/// rather than allowed to fill this machine (§3.7).
#[test]
fn an_answer_larger_than_the_ceiling_is_refused() {
    let enormous = "x".repeat(usize::try_from(METADATA_CEILING).unwrap_or(usize::MAX) + 1);
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{enormous}",
            enormous.len()
        ),
    )]);
    let failure = server.hub().list(&a_reference()).expect_err("too much");
    assert_eq!(failure.category(), Category::HubMetadataMalformed);
}

/// An answer that is not JSON is refused saying what it saw, bounded so a
/// source cannot write into MCF's record (A1, §3.7).
#[test]
fn an_answer_that_is_not_json_is_refused() {
    let server = a_hub_with(&[(
        "/api/models/owner/model",
        answer("<html>not a listing at all</html>"),
    )]);
    let failure = server.hub().list(&a_reference()).expect_err("not JSON");
    assert_eq!(failure.category(), Category::HubMetadataMalformed);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("JSON")),
        "the refusal does not say what it wanted"
    );
}

/// A hub says what it is and what it is to the hub, because both are conditions
/// of anything acquired through it (§3.4).
#[test]
fn a_hub_describes_itself_without_naming_the_credential() {
    let server = a_hub_with(&[]);
    let hub = server
        .trusting_hub()
        .offering(Credential::new(Secret::new("hf_secret"), Origin::Supplied));

    let described = hub.describe();
    assert!(described.contains("127.0.0.1"), "{described}");
    assert!(described.contains("TCP"), "{described}");
    assert!(!format!("{hub:?}").contains("hf_secret"), "{hub:?}");
    assert!(
        format!("{hub:?}").contains("authenticated: true"),
        "{hub:?}"
    );
}

/// An answer that is not the file never reaches the file. A hub's error page
/// written into `model.gguf` would be a corrupt artifact MCF put there itself,
/// and the check that stops it happens before a byte is written (B-021, A1).
#[test]
fn an_error_page_is_never_written_into_the_artifact() {
    let target = "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf";
    let page = "<html>the model has moved</html>";
    let server = a_hub_with(&[(
        target,
        format!(
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{page}",
            page.len()
        ),
    )]);
    let directory = scratch("error-page");
    let into = directory.join("model.gguf");

    let reference = Reference {
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        ..a_reference()
    };
    let failure = server
        .hub()
        .fetch(&reference, &Entry::new("model.gguf", 6), &into)
        .expect_err("not found");

    assert_eq!(failure.category(), Category::HubRefNotFound);
    let held = std::fs::read(&into).unwrap_or_default();
    assert!(
        held.is_empty(),
        "the hub's error page was written into the artifact: {} bytes",
        held.len()
    );
    let _cleared = std::fs::remove_dir_all(&directory);
}

/// And a redirect's own body is never read: the head named somewhere else to
/// go, and what is underneath it is bytes nobody asked for.
#[test]
fn a_redirects_body_is_not_read() {
    let cdn = Server::answering(BTreeMap::from([(
        "/real".to_owned(),
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nGGUFxx".to_owned(),
    )]));
    let courtesy = "you are being redirected, here is a page about it";
    let server = Server::answering(answers_with(&[(
        "/owner/model/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/model.gguf",
        format!(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{}/real\r\nContent-Length: {}\r\n\r\n{courtesy}",
            cdn.port,
            courtesy.len()
        ),
    )]));
    let directory = scratch("redirect-body");
    let into = directory.join("model.gguf");
    let reference = Reference {
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        ..a_reference()
    };

    let fetched = server
        .hub()
        .fetch(&reference, &Entry::new("model.gguf", 6), &into)
        .expect("the fetch runs");

    assert_eq!(std::fs::read(&into).expect("readable"), b"GGUFxx");
    assert_eq!(fetched.bytes, 6, "the redirect's own body was counted");
    let _cleared = std::fs::remove_dir_all(&directory);
}

/// §XII's hard case, as the hub actually publishes it: the repository that
/// made these weights says which weights it made them from, and what it did.
#[test]
fn the_lineage_a_publisher_states_is_read() {
    let server = a_hub_with(&[]);
    let listing = server.hub().list(&a_reference()).expect("a listing");
    let lineage = listing.lineage.expect("the card names a base");

    assert_eq!(lineage.base, "somebody/original");
    assert_eq!(lineage.relation.as_deref(), Some("quantized"));
}

/// A repository that says nothing about where its weights came from leaves the
/// link absent rather than unlinked-and-assumed-original (A7).
#[test]
fn a_repository_that_says_nothing_about_its_base_leaves_it_absent() {
    let server = Server::answering(answers_with(&[(
        "/api/models/owner/model",
        answer(r#"{"sha":"50968a4468ef4233ed78cd7c3de230dd1d61a56b","tags":["gguf"]}"#),
    )]));
    let listing = server.hub().list(&a_reference()).expect("a listing");
    assert_eq!(listing.lineage, None);
}

/// A base named without a relation is still a link: what the publisher said is
/// kept, and what they did not say stays unsaid.
#[test]
fn a_base_with_no_relation_is_still_a_link() {
    let server = Server::answering(answers_with(&[(
        "/api/models/owner/model",
        answer(
            r#"{"sha":"50968a4468ef4233ed78cd7c3de230dd1d61a56b",
                "tags":["base_model:somebody/original"]}"#,
        ),
    )]));
    let lineage = server
        .hub()
        .list(&a_reference())
        .expect("a listing")
        .lineage
        .expect("a base");
    assert_eq!(lineage.base, "somebody/original");
    assert_eq!(lineage.relation, None);
}
