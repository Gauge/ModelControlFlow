use std::collections::BTreeMap;
use std::time::Duration;

use mcf_core::failure::Category;
use mcf_hub::http::Request;
use mcf_hub::reference::Reference;
use mcf_hub::source::{Entry, Source as _};
use mcf_hub::wire::{Deadlines, Tcp as Wire, Wire as _};

use crate::hub::{Behaviour, FakeHub, Repository};
use crate::scenario::{Outcome, Scenario};
use crate::serving::Serving;
use crate::world::World;

pub(super) const DECEPTIVE_METADATA: Scenario = Scenario {
    id: "hub/deceptive-metadata",
    produces: Category::HubMetadataDeceptive,
    summary: "a repository declaring an architecture its weights are not is caught by reading them",
    run: deceptive_metadata,
};

pub(super) const NO_LICENCE: Scenario = Scenario {
    id: "hub/no-licence",
    produces: Category::HubMetadataAbsent,
    summary: "a repository whose terms nobody can read is a state to report, not one to fill in",
    run: no_licence,
};

pub(super) const TRUNCATED_TRANSFER: Scenario = Scenario {
    id: "hub/truncated-transfer",
    produces: Category::ArtifactIncomplete,
    summary: "fewer bytes than the listing promised is a partial artifact, on the disk and in the record",
    run: truncated_transfer,
};

pub(super) const CANNOT_RESUME: Scenario = Scenario {
    id: "hub/cannot-resume",
    produces: Category::HubUnreachable,
    summary: "a hub without ranges says so, and a fetch starts again rather than pretending",
    run: cannot_resume,
};

pub(super) const REFERENCE_IS_NOT_ONE: Scenario = Scenario {
    id: "hub/reference-is-not-one",
    produces: Category::HubRefNotFound,
    summary: "a string that does not name a repository is refused, saying what it saw",
    run: reference_is_not_one,
};

pub(super) const NEEDS_CREDENTIALS: Scenario = Scenario {
    id: "hub/needs-credentials",
    produces: Category::HubAuthRequired,
    summary: "a private repository asked for without credentials says which is missing",
    run: needs_credentials,
};

pub(super) const CREDENTIAL_REFUSED: Scenario = Scenario {
    id: "hub/credential-refused",
    produces: Category::HubAuthRejected,
    summary: "a credential the hub refuses is a different answer from no credential, and says so",
    run: credential_refused,
};

pub(super) const GATED: Scenario = Scenario {
    id: "hub/gated",
    produces: Category::HubAccessGated,
    summary: "a gated repository is a different answer from an unauthenticated one",
    run: gated,
};

pub(super) const RATE_LIMITED: Scenario = Scenario {
    id: "hub/rate-limited",
    produces: Category::HubRateLimited,
    summary: "throttling carries the hint the hub gave, so waiting is a decision",
    run: rate_limited,
};

pub(super) const ANSWER_IS_NOT_A_RESPONSE: Scenario = Scenario {
    id: "hub/answer-is-not-a-response",
    produces: Category::HubMetadataMalformed,
    summary: "a source that answers with something that is not HTTP is refused, saying what it saw",
    run: answer_is_not_a_response,
};

pub(super) const ANSWER_CUT_SHORT: Scenario = Scenario {
    id: "hub/answer-cut-short",
    produces: Category::TransferInterrupted,
    summary: "an answer that ends mid-header is an interruption rather than a malformed source",
    run: answer_cut_short,
};

pub(super) const ANSWER_NEVER_COMES: Scenario = Scenario {
    id: "hub/answer-never-comes",
    produces: Category::TransferStalled,
    summary: "a host that accepts a connection and says nothing ends at MCF's deadline, not never",
    run: answer_never_comes,
};

pub(super) const NO_ROOM_ON_THE_DISK: Scenario = Scenario {
    id: "hub/no-room-on-the-disk",
    produces: Category::ResourceDiskExhausted,
    summary: "a filesystem that fills mid-transfer is a decision, said as one, not a surprise",
    run: no_room_on_the_disk,
};

fn no_room_on_the_disk(_world: &World) -> Outcome {
    use std::io::Write as _;

    let weights = "GGUF the weights";
    let Some(serving) = Serving::answering(BTreeMap::from([(
        "/owner/model/resolve/abc123/model.gguf".to_owned(),
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
            weights.len()
        ),
    )])) else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else {
        return Outcome::Unexpected("this machine has no /dev/full to write to".to_owned());
    };
    let Ok(url) = mcf_hub::http::Url::parse(&format!(
        "{}owner/model/resolve/abc123/model.gguf",
        serving.base()
    )) else {
        return Outcome::Unexpected("the loopback address is a URL".to_owned());
    };

    let mut sink = std::io::BufWriter::new(full);
    let outcome = match mcf_hub::wire::fetch(&quick_wire(), &Request::get(url), &mut sink) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a filesystem with no room accepted a file".to_owned()),
    };
    let _flushed = sink.flush();
    outcome
}

pub(super) const NO_WAY_TO_ENCRYPT: Scenario = Scenario {
    id: "hub/no-way-to-encrypt",
    produces: Category::ConfigUnsatisfiable,
    summary: "an https hub over a plain socket is refused in as many words, before it is opened",
    run: no_way_to_encrypt,
};

pub(super) const NOT_A_TLS_HOST: Scenario = Scenario {
    id: "hub/not-a-tls-host",
    produces: Category::TransferTls,
    summary: "a host that answers a handshake with something else is a TLS failure, said as one",
    run: not_a_tls_host,
};

fn not_a_tls_host(_world: &World) -> Outcome {
    let Some(serving) = Serving::blurting("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n") else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };
    let Ok(wire) = mcf_hub::wire::Tls::with(Deadlines {
        connect: Duration::from_secs(5),
        idle: Duration::from_millis(500),
    }) else {
        return Outcome::Unexpected("the vendored provider would not configure".to_owned());
    };

    match wire.dial("127.0.0.1", serving.port()) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a plain HTTP server completed a TLS handshake".to_owned()),
    }
}

pub(super) const RESUMPTION_RESTARTED: Scenario = Scenario {
    id: "hub/resumption-restarted",
    produces: Category::TransferMutated,
    summary: "a source that answers a resumption from somewhere else is refused, not appended to",
    run: resumption_restarted,
};

fn resumption_restarted(world: &World) -> Outcome {
    let restarted = "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-5/6\r\n\
                     Content-Length: 6\r\n\r\nGGUFxx";
    let Some(serving) = Serving::answering(BTreeMap::from([(
        "/owner/model/resolve/abc123/model.gguf".to_owned(),
        restarted.to_owned(),
    )])) else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };

    let into = world.path("model.gguf");
    if let Err(error) = std::fs::write(&into, b"GGUF") {
        return Outcome::Unexpected(format!("could not write the partial file: {error}"));
    }
    let Ok(base) = mcf_hub::http::Url::parse(&serving.base()) else {
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
    let held = std::fs::read(&into).unwrap_or_default();
    if held != b"GGUF" {
        return Outcome::Unexpected(format!(
            "the partial file was changed by a refused resumption: {} bytes",
            held.len()
        ));
    }
    outcome
}

fn quick_wire() -> Wire {
    Wire {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            idle: Duration::from_millis(500),
        },
    }
}

fn answer_never_comes(_world: &World) -> Outcome {
    let Some(serving) = Serving::holding_open() else {
        return Outcome::Unexpected("no loopback port is available".to_owned());
    };
    let wire = Wire {
        deadlines: Deadlines {
            connect: Duration::from_secs(5),
            idle: Duration::from_millis(20),
        },
    };
    let Ok(url) = mcf_hub::http::Url::parse(&format!("{}model.gguf", serving.base())) else {
        return Outcome::Unexpected("the loopback address is a URL".to_owned());
    };

    let mut nothing = Vec::new();
    match mcf_hub::wire::fetch(&wire, &Request::get(url), &mut nothing) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a host that said nothing answered something".to_owned()),
    }
}

fn no_way_to_encrypt(_world: &World) -> Outcome {
    let Ok(url) = mcf_hub::http::Url::parse("https://huggingface.co/owner/model") else {
        return Outcome::Unexpected("that is a URL".to_owned());
    };
    let mut nothing = Vec::new();
    match mcf_hub::wire::fetch(&quick_wire(), &Request::get(url), &mut nothing) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("MCF opened an encrypted connection it cannot".to_owned()),
    }
}

fn answer_is_not_a_response(_world: &World) -> Outcome {
    let captive_portal = b"<html><head><title>Sign in to continue</title></head>\r\n\r\n<body>";
    match mcf_hub::http::Response::read(captive_portal) {
        Err(failure) => Outcome::Produced(failure),
        Ok((response, _)) => Outcome::Unexpected(format!(
            "a web page was read as a response with status {}",
            response.status()
        )),
    }
}

fn answer_cut_short(_world: &World) -> Outcome {
    let cut = b"HTTP/1.1 200 OK\r\nContent-Length: 3967";
    match mcf_hub::http::Response::read(cut) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("half a header block was read as a whole one".to_owned()),
    }
}

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
    ask(Behaviour::RejectsCredentials, true)
}

fn gated(_world: &World) -> Outcome {
    ask(Behaviour::Gated, true)
}

fn rate_limited(_world: &World) -> Outcome {
    ask(Behaviour::RateLimited { retry_after: 30 }, true)
}

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

    let compared = mcf_hub::inspect::architecture(Some("llama"), file.architecture());
    match mcf_hub::inspect::deception(&compared) {
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

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&u64::try_from(value.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn reference_is_not_one(_world: &World) -> Outcome {
    match mcf_hub::reference::parse("../../etc/passwd") {
        Err(failure) => Outcome::Produced(failure),
        Ok(reference) => Outcome::Unexpected(format!("a path traversal parsed as {reference}")),
    }
}
