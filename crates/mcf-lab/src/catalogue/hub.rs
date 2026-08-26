//! Scenarios about the model source.
//!
//! The hub is unbounded, heterogeneous and changes without notice (§6.3), and
//! §III commits MCF to a defined, actionable outcome for every reference to it.
//! These are the failures that can be produced without a hub: a reference is a
//! string, and what MCF does with a string it cannot use is decidable here and
//! now.
//!
//! What is *not* here yet is the rest of B7's check — the hostile-hub fixtures
//! that need a simulated hub to serve them (B-028). D26 draws that line: this
//! scenario simulates what MCF observes, which is a reference that names
//! nothing, rather than a hub that behaves badly.

use std::time::Duration;

use mcf_core::failure::Category;
use mcf_hub::http::Request;
use mcf_hub::reference::Reference;
use mcf_hub::source::{Entry, Source as _};
use mcf_hub::wire::{Deadlines, Tcp as Wire};

use crate::hub::{Behaviour, FakeHub, Repository};
use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The card says one thing and the weights say another.
pub(super) const DECEPTIVE_METADATA: Scenario = Scenario {
    id: "hub/deceptive-metadata",
    produces: Category::HubMetadataDeceptive,
    summary: "a repository declaring an architecture its weights are not is caught by reading them",
    run: deceptive_metadata,
};

/// The repository declares no terms at all.
pub(super) const NO_LICENCE: Scenario = Scenario {
    id: "hub/no-licence",
    produces: Category::HubMetadataAbsent,
    summary: "a repository whose terms nobody can read is a state to report, not one to fill in",
    run: no_licence,
};

/// The transfer ends early.
pub(super) const TRUNCATED_TRANSFER: Scenario = Scenario {
    id: "hub/truncated-transfer",
    produces: Category::ArtifactIncomplete,
    summary: "fewer bytes than the listing promised is a partial artifact, on the disk and in the record",
    run: truncated_transfer,
};

/// The source cannot continue a transfer from where it stopped.
pub(super) const CANNOT_RESUME: Scenario = Scenario {
    id: "hub/cannot-resume",
    produces: Category::HubUnreachable,
    summary: "a hub without ranges says so, and a fetch starts again rather than pretending",
    run: cannot_resume,
};

/// A reference that is not one.
pub(super) const REFERENCE_IS_NOT_ONE: Scenario = Scenario {
    id: "hub/reference-is-not-one",
    produces: Category::HubRefNotFound,
    summary: "a string that does not name a repository is refused, saying what it saw",
    run: reference_is_not_one,
};

/// The repository is readable and this caller is not.
pub(super) const NEEDS_CREDENTIALS: Scenario = Scenario {
    id: "hub/needs-credentials",
    produces: Category::HubAuthRequired,
    summary: "a private repository asked for without credentials says which is missing",
    run: needs_credentials,
};

/// Something was offered and the hub would not have it.
pub(super) const CREDENTIAL_REFUSED: Scenario = Scenario {
    id: "hub/credential-refused",
    produces: Category::HubAuthRejected,
    summary: "a credential the hub refuses is a different answer from no credential, and says so",
    run: credential_refused,
};

/// The credentials are fine and the terms are not accepted.
pub(super) const GATED: Scenario = Scenario {
    id: "hub/gated",
    produces: Category::HubAccessGated,
    summary: "a gated repository is a different answer from an unauthenticated one",
    run: gated,
};

/// The account is throttled.
pub(super) const RATE_LIMITED: Scenario = Scenario {
    id: "hub/rate-limited",
    produces: Category::HubRateLimited,
    summary: "throttling carries the hint the hub gave, so waiting is a decision",
    run: rate_limited,
};

/// The answer is not an answer.
pub(super) const ANSWER_IS_NOT_A_RESPONSE: Scenario = Scenario {
    id: "hub/answer-is-not-a-response",
    produces: Category::HubMetadataMalformed,
    summary: "a source that answers with something that is not HTTP is refused, saying what it saw",
    run: answer_is_not_a_response,
};

/// The answer stops before it has finished arriving.
pub(super) const ANSWER_CUT_SHORT: Scenario = Scenario {
    id: "hub/answer-cut-short",
    produces: Category::TransferInterrupted,
    summary: "an answer that ends mid-header is an interruption rather than a malformed source",
    run: answer_cut_short,
};

/// The host accepts a connection and then says nothing.
pub(super) const ANSWER_NEVER_COMES: Scenario = Scenario {
    id: "hub/answer-never-comes",
    produces: Category::TransferStalled,
    summary: "a host that accepts a connection and says nothing ends at MCF's deadline, not never",
    run: answer_never_comes,
};

/// The hub answers a resumption by starting again.
pub(super) const RESUMPTION_RESTARTED: Scenario = Scenario {
    id: "hub/resumption-restarted",
    produces: Category::TransferMutated,
    summary: "a source that answers a resumption from somewhere else is refused, not appended to",
    run: resumption_restarted,
};

/// A hub asked to continue from an offset that sends the file from the start.
///
/// The observation is the answer, not why it was sent: a cache that lost the
/// object, a mirror that never supported ranges and a hostile source splicing
/// two files together all look the same from here (D26). What must not happen
/// is the append — a partial file plus a whole one is a model that is its own
/// first megabyte twice, and B-021 exists to make that impossible.
fn resumption_restarted(world: &World) -> Outcome {
    let restarted = "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-5/6\r\n\
                     Content-Length: 6\r\n\r\nGGUFxx";
    let Some((port, serving)) = answering_once(restarted) else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };

    let into = world.path("model.gguf");
    if let Err(error) = std::fs::write(&into, b"GGUF") {
        return Outcome::Unexpected(format!("could not write the partial file: {error}"));
    }
    let Ok(base) = mcf_hub::http::Url::parse(&format!("http://127.0.0.1:{port}/")) else {
        return Outcome::Unexpected("the loopback address is a URL".to_owned());
    };
    let hub = mcf_hub::client::Hub::at(base, Box::new(quick_wire()));
    let reference = Reference {
        owner: "owner".to_owned(),
        name: "model".to_owned(),
        revision: Some("abc123".to_owned()),
        file: None,
    };

    let outcome = match hub.fetch_from(&reference, &Entry::new("model.gguf", 6), 4, &into) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a source that restarted was treated as resuming".to_owned()),
    };
    // The partial file must be untouched: a refused resumption that had already
    // appended would have destroyed what it refused to add to.
    let held = std::fs::read(&into).unwrap_or_default();
    let _joined = serving.join();
    if held != b"GGUF" {
        return Outcome::Unexpected(format!(
            "the partial file was changed by a refused resumption: {} bytes",
            held.len()
        ));
    }
    outcome
}

/// A listener that answers one request with this and then stops.
fn answering_once(answer: &'static str) -> Option<(u16, std::thread::JoinHandle<()>)> {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let serving = std::thread::spawn(move || {
        if let Ok((mut held, _)) = listener.accept() {
            let _deadline = held.set_read_timeout(Some(Duration::from_secs(5)));
            let mut seen = Vec::new();
            let mut byte = [0_u8; 1];
            while held.read(&mut byte).unwrap_or(0) == 1 {
                seen.push(byte[0]);
                if seen.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let _written = held.write_all(answer.as_bytes());
            let _flushed = held.flush();
            let _closed = held.shutdown(std::net::Shutdown::Write);
        }
    });
    Some((port, serving))
}

/// A wire with deadlines short enough to run a hundred times in §3.17's check.
fn quick_wire() -> Wire {
    Wire {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            idle: Duration::from_millis(500),
        },
    }
}

/// A host that takes the connection and never answers.
///
/// The observation is *silence after an accept*, which is what a hung server, a
/// dropped route and a middlebox holding a connection open all look like from
/// here; the cause stays out of it (D26). The listener is on the loopback
/// address and answers nothing, so the scenario needs no network and no hub —
/// what it exercises is that B7's *a hang is a defined outcome* is true of the
/// real socket path rather than of a stand-in for it.
fn answer_never_comes(_world: &World) -> Outcome {
    use std::net::TcpListener;
    use std::sync::mpsc;

    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };
    let Ok(address) = listener.local_addr() else {
        return Outcome::Unexpected("the listener has no address".to_owned());
    };
    let port = address.port();

    // The connection is held open until this scenario says otherwise, so the
    // client's deadline is the only thing that ends the wait — which is the
    // claim being made.
    let (stop, wait) = mpsc::channel::<()>();
    let holding = std::thread::spawn(move || {
        if let Ok((held, _)) = listener.accept() {
            let _told = wait.recv();
            drop(held);
        }
    });

    // Short deadlines on purpose: the scenario reproduces a hundred times over
    // in §3.17's check, and what is being demonstrated is *that* a deadline
    // ends the wait rather than how long MCF's is.
    let wire = Wire {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            idle: Duration::from_millis(20),
        },
    };
    let Ok(url) = mcf_hub::http::Url::parse(&format!("http://127.0.0.1:{port}/model.gguf")) else {
        return Outcome::Unexpected("the loopback address is a URL".to_owned());
    };

    let mut nothing = Vec::new();
    let outcome = match mcf_hub::wire::fetch(&wire, &Request::get(url), &mut nothing) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a host that said nothing answered something".to_owned()),
    };

    // Let the held connection go, and wake the listener in case the client
    // never reached it, so the scenario leaves no thread behind (A27, B58).
    let _told = stop.send(());
    if let Ok(waker) = std::net::TcpStream::connect(("127.0.0.1", port)) {
        drop(waker);
    }
    let _joined = holding.join();
    outcome
}

/// What a hostile or broken source puts where a response goes.
///
/// The observation is the bytes, not the cause: a proxy's error page, a captive
/// portal's login form and a server that has lost its mind all arrive the same
/// way, and what MCF has to get right is that none of them is read as a model
/// (D26, §3.7).
fn answer_is_not_a_response(_world: &World) -> Outcome {
    // A complete head — it ends where a head ends — that is not a response. A
    // page rather than a protocol, which is what a captive portal and a
    // misconfigured proxy both put on the wire.
    let captive_portal = b"<html><head><title>Sign in to continue</title></head>\r\n\r\n<body>";
    match mcf_hub::http::Response::read(captive_portal) {
        Err(failure) => Outcome::Produced(failure),
        Ok((response, _)) => Outcome::Unexpected(format!(
            "a web page was read as a response with status {}",
            response.status()
        )),
    }
}

/// A connection that closed while the headers were still arriving.
///
/// Distinct from the scenario above on purpose: *incomplete* and *wrong* lead a
/// caller to different places — one reads more and tries again, the other stops
/// — and a client that confused them would either hang on a broken source or
/// give up on a slow one.
fn answer_cut_short(_world: &World) -> Outcome {
    let cut = b"HTTP/1.1 200 OK\r\nContent-Length: 3967";
    match mcf_hub::http::Response::read(cut) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("half a header block was read as a whole one".to_owned()),
    }
}

/// Asks the simulated hub for a repository that behaves in a stated way.
///
/// The scenario supplies the *observation* — a hub that answers this way — and
/// not the cause of it (D26). What is being reproduced is what MCF does with
/// the answer.
fn ask(behaviour: Behaviour, authenticated: bool) -> Outcome {
    let repository = Repository::holding("model.gguf", b"weights").behaving(behaviour);
    let hub = FakeHub::new().with("owner/model", repository);
    let hub = if authenticated {
        hub.authenticated()
    } else {
        hub
    };
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    match hub.list(&reference) {
        Err(failure) => Outcome::Produced(failure),
        Ok(listing) => Outcome::Unexpected(format!(
            "the hub answered with {} entries",
            listing.entries.len()
        )),
    }
}

fn needs_credentials(_world: &World) -> Outcome {
    ask(Behaviour::NeedsCredentials, false)
}

fn credential_refused(_world: &World) -> Outcome {
    // Offered on purpose: *refused* is a thing that can only happen to a
    // credential that exists, and the unauthenticated run is the other
    // scenario.
    ask(Behaviour::RejectsCredentials, true)
}

fn gated(_world: &World) -> Outcome {
    // Authenticated on purpose: gated is *credentials accepted, terms not*, and
    // running it unauthenticated would reproduce the other failure.
    ask(Behaviour::Gated, true)
}

fn rate_limited(_world: &World) -> Outcome {
    ask(Behaviour::RateLimited { retry_after: 30 }, true)
}

/// A GGUF that declares an architecture, with no tensors — enough for a reader
/// to say what it is, which is all this scenario needs.
fn model_declaring(architecture: &str) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    push_string(&mut bytes, "general.architecture");
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    push_string(&mut bytes, architecture);
    bytes
}

fn deceptive_metadata(world: &World) -> Outcome {
    // The repository's card says llama; the weights it serves say mamba. The
    // whole path runs: list, fetch, read the weights, compare.
    let hub = FakeHub::new().with(
        "owner/mislabelled",
        Repository::holding("model.gguf", &model_declaring("mamba")),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/mislabelled") else {
        return Outcome::Unexpected("owner/mislabelled is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };

    let into = world.path("model.gguf");
    if let Err(failure) = hub.fetch(&reference, entry, &into) {
        return Outcome::Unexpected(format!("the fetch failed: {failure}"));
    }
    let file = match mcf_standin::gguf::read(&into) {
        Ok(file) => file,
        Err(failure) => {
            return Outcome::Unexpected(format!("the weights would not read: {failure}"));
        }
    };

    // The card is what the repository says; the architecture is what the
    // weights say. A21's divergence, and the most useful thing MCF can report
    // about a repository like this.
    let compared = mcf_hub::inspect::Architecture::compare(Some("llama"), file.architecture());
    match compared.divergence() {
        Some(failure) => Outcome::Produced(failure),
        None => Outcome::Unexpected(format!("the mislabelling was not noticed: {compared:?}")),
    }
}

fn cannot_resume(world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789").behaving(Behaviour::NeverResumes),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };
    match hub.fetch_from(&reference, entry, 4, &world.path("model.gguf.partial")) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a hub with no ranges continued a transfer".to_owned()),
    }
}

fn no_licence(_world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/quiet",
        Repository::holding("model.gguf", b"weights").without_licence(),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/quiet") else {
        return Outcome::Unexpected("owner/quiet is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    match mcf_hub::inspect::terms_are_legible(&listing) {
        Err(failure) => Outcome::Produced(failure),
        Ok(licence) => Outcome::Unexpected(format!("a licence appeared from nowhere: {licence}")),
    }
}

fn truncated_transfer(world: &World) -> Outcome {
    let hub = FakeHub::new().with(
        "owner/model",
        Repository::holding("model.gguf", b"0123456789")
            .behaving(Behaviour::Truncates { after: 4 }),
    );
    let Ok(reference) = mcf_hub::reference::parse("owner/model") else {
        return Outcome::Unexpected("owner/model is a reference".to_owned());
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => return Outcome::Unexpected(format!("the listing failed: {failure}")),
    };
    let Some(entry) = listing.entry("model.gguf") else {
        return Outcome::Unexpected("the repository lists no weights".to_owned());
    };

    let into = world.path("model.gguf");
    let fetched = match hub.fetch(&reference, entry, &into) {
        Ok(fetched) => fetched,
        Err(failure) => return Outcome::Unexpected(format!("the fetch failed: {failure}")),
    };
    match mcf_hub::inspect::arrived_as_promised(entry, fetched.bytes) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("a short transfer passed as whole".to_owned()),
    }
}

/// A length-prefixed string, as GGUF writes them.
fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&u64::try_from(value.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn reference_is_not_one(_world: &World) -> Outcome {
    // A path traversal dressed as a reference: the exact input §3.7 exists for,
    // and the one that would reach the filesystem if the syntax were not where
    // it stopped.
    match mcf_hub::reference::parse("../../etc/passwd") {
        Err(failure) => Outcome::Produced(failure),
        Ok(reference) => Outcome::Unexpected(format!("a path traversal parsed as {reference}")),
    }
}
