//! What a source can say, and what MCF does about it.
//!
//! The responses here are the ones the real hub sent on 2026-08-25
//! ([findings.md](../../../../doc/findings.md) F9), and the ones it did not:
//! every case that would be a defect is written out, because a client is judged
//! by what it refuses.

use super::{
    HEADER_CEILING, Next, REDIRECT_CEILING, Request, Response, Url, next, redirect, user_agent,
};
use mcf_core::failure::Category;

fn url(text: &str) -> Url {
    Url::parse(text).expect("a URL")
}

fn response(text: &str) -> Response {
    Response::read(text.as_bytes()).expect("a response").0
}

/// The shapes a URL arrives in.
#[test]
fn a_url_is_read_or_refused_by_name() {
    let plain = url("https://huggingface.co/api/models/owner/model");
    assert_eq!(plain.scheme(), "https");
    assert_eq!(plain.host(), "huggingface.co");
    assert_eq!(plain.port(), 443);
    assert_eq!(plain.target(), "/api/models/owner/model");

    let with_port = url("http://localhost:8080/x?y=1");
    assert_eq!(with_port.port(), 8080);
    assert_eq!(with_port.target(), "/x?y=1");
    assert_eq!(with_port.authority(), "localhost:8080");

    // A host and nothing else is the root.
    assert_eq!(url("https://huggingface.co").target(), "/");

    for refused in [
        "huggingface.co/x",
        "ftp://huggingface.co/x",
        "file:///etc/passwd",
        "https:///x",
        "https://host:not-a-port/x",
    ] {
        let failure = Url::parse(refused).expect_err(refused);
        assert_eq!(
            failure.category(),
            Category::HubMetadataMalformed,
            "{refused}"
        );
    }
}

/// A credential written into a URL is refused: it would end up in a log, a
/// record and a shell history, and B-024 keeps credentials in one place.
#[test]
fn a_url_carrying_a_credential_is_refused() {
    let failure = Url::parse("https://user:token@huggingface.co/x").expect_err("refused");
    assert_eq!(failure.category(), Category::HubMetadataMalformed);
}

/// Case in a host is not a difference. Two spellings of one host that compared
/// unequal would drop a credential MCF should carry.
#[test]
fn a_host_is_the_same_host_however_it_is_spelled() {
    assert!(url("https://HuggingFace.CO/x").same_origin(&url("https://huggingface.co/y")));
    assert_eq!(url("https://HuggingFace.CO/x").host(), "huggingface.co");
}

/// And a host that merely *ends with* another is a different host. This is the
/// check that a `ends_with` would have got wrong.
#[test]
fn a_lookalike_host_is_a_different_origin() {
    let hub = url("https://huggingface.co/x");
    for other in [
        "https://evil-huggingface.co/x",
        "https://huggingface.co.evil.example/x",
        "https://huggingface.co:8443/x",
        "http://huggingface.co/x",
    ] {
        assert!(
            !hub.same_origin(&url(other)),
            "{other} was treated as the same origin as the hub"
        );
    }
}

/// The two shapes of location the hub actually sends (F9 §9.1).
#[test]
fn a_location_resolves_relative_or_absolute() {
    let from = url("https://huggingface.co/owner/model/resolve/main/model.gguf");
    assert_eq!(
        from.resolve("/api/resolve-cache/models/owner/model/abc/model.gguf")
            .expect("relative"),
        url("https://huggingface.co/api/resolve-cache/models/owner/model/abc/model.gguf")
    );
    assert_eq!(
        from.resolve("https://us.aws.cdn.hf.co/xet-bridge-us/abc?Expires=1")
            .expect("absolute"),
        url("https://us.aws.cdn.hf.co/xet-bridge-us/abc?Expires=1")
    );
    // Scheme-relative, which a CDN sometimes sends.
    assert_eq!(
        from.resolve("//cdn.example/x").expect("scheme-relative"),
        url("https://cdn.example/x")
    );
    from.resolve("nonsense").expect_err("not a location");
}

/// A request is the smallest correct thing, and it says what MCF is.
#[test]
fn a_request_is_what_goes_on_the_wire() {
    let request = Request::get(url("https://huggingface.co/api/models/owner/model"));
    let bytes = request.to_bytes();
    let text = String::from_utf8(bytes).expect("a request is text");

    assert!(
        text.starts_with("GET /api/models/owner/model HTTP/1.1\r\n"),
        "{text}"
    );
    assert!(text.contains("Host: huggingface.co\r\n"), "{text}");
    assert!(
        text.contains(&format!("User-Agent: {}\r\n", user_agent())),
        "{text}"
    );
    assert!(text.contains("Accept-Encoding: identity\r\n"), "{text}");
    assert!(text.ends_with("\r\n\r\n"), "{text}");
    assert!(!request.is_authenticated());
}

/// What MCF calls itself names MCF and nothing about this computer: a user
/// agent leaves the machine, and §XIV's habit starts there.
#[test]
fn the_user_agent_names_mcf_and_not_the_machine() {
    let agent = user_agent();
    assert!(agent.starts_with("mcf/"), "{agent}");
    for absent in ["Mozilla", "Linux", "x86_64", "curl"] {
        assert!(!agent.contains(absent), "{agent} says {absent}");
    }
}

/// Resumption asks for the rest, from where the partial file ended.
#[test]
fn a_resumed_request_asks_for_the_rest() {
    let text = String::from_utf8(
        Request::get(url("https://huggingface.co/x"))
            .resuming(1_048_576)
            .to_bytes(),
    )
    .expect("text");
    assert!(text.contains("Range: bytes=1048576-\r\n"), "{text}");
}

/// A credential is offered as a bearer token and nowhere else.
#[test]
fn a_credential_is_offered_in_one_header() {
    let request = Request::get(url("https://huggingface.co/x")).offering("hf_token");
    assert!(request.is_authenticated());
    let text = String::from_utf8(request.to_bytes()).expect("text");
    assert_eq!(
        text.matches("hf_token").count(),
        1,
        "the token appears more than once: {text}"
    );
    assert!(
        text.contains("Authorization: Bearer hf_token\r\n"),
        "{text}"
    );
}

/// The response the hub sends for a listing.
#[test]
fn a_response_is_read_into_its_parts() {
    let (answer, consumed) = Response::read(
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 8941\r\n\r\n{}",
    )
    .expect("a response");
    assert_eq!(answer.status(), 200);
    assert_eq!(answer.header("content-type"), Some("application/json"));
    assert_eq!(answer.content_length().expect("a length"), Some(8941));
    assert_eq!(consumed, 73, "the body begins after the blank line");
}

/// Header names are compared without case, because a source chooses its own
/// spelling and the hub uses both.
#[test]
fn a_header_is_found_however_it_was_capitalized() {
    let answer = response("HTTP/1.1 200 OK\r\nX-Repo-Commit: 50968a44\r\n\r\n");
    assert_eq!(answer.header("x-repo-commit"), Some("50968a44"));
}

/// The headers that make acquisition possible, from the answer the hub actually
/// gave (F9 §9.1): the size, the digest, and the revision to pin.
#[test]
fn the_hubs_own_answer_carries_what_acquisition_needs() {
    let answer = response(
        "HTTP/1.1 302 Found\r\n\
         Location: https://us.aws.cdn.hf.co/xet-bridge-us/abc?Expires=1\r\n\
         X-Repo-Commit: 50968a4468ef4233ed78cd7c3de230dd1d61a56b\r\n\
         Accept-Ranges: bytes\r\n\
         X-Linked-Size: 396705472\r\n\
         X-Linked-ETag: \"ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a\"\r\n\
         \r\n",
    );
    assert_eq!(answer.header("x-linked-size"), Some("396705472"));
    assert_eq!(
        answer.header("x-repo-commit"),
        Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b")
    );
    assert!(
        answer
            .header("x-linked-etag")
            .is_some_and(|etag| etag.contains("ac2d977")),
        "the declared digest is not readable"
    );
}

/// A redirect to another host does not carry the credential. This is the whole
/// reason MCF writes its own client.
#[test]
fn a_redirect_to_another_host_leaves_the_credential_behind() {
    let from = url("https://huggingface.co/owner/model/resolve/main/model.gguf");
    let answer = response(
        "HTTP/1.1 302 Found\r\nLocation: https://us.aws.cdn.hf.co/xet-bridge-us/abc\r\n\r\n",
    );
    match next(&answer, &from).expect("a redirect") {
        Next::Follow {
            to,
            carrying_the_credential,
        } => {
            assert_eq!(to.host(), "us.aws.cdn.hf.co");
            assert!(
                !carrying_the_credential,
                "the credential would have gone to a host the network named"
            );
        }
        Next::Body => panic!("a 302 was read as a body"),
    }
}

/// A redirect within the hub does carry it, because that is the same place and
/// dropping it would turn a private repository into a 401 nobody can explain.
#[test]
fn a_redirect_within_the_hub_keeps_the_credential() {
    let from = url("https://huggingface.co/owner/model/resolve/main/model.gguf");
    let answer =
        response("HTTP/1.1 307 Temporary Redirect\r\nLocation: /api/resolve-cache/x\r\n\r\n");
    assert_eq!(
        redirect(&answer, &from).expect("a redirect"),
        Some((url("https://huggingface.co/api/resolve-cache/x"), true))
    );
}

/// Every redirect status the hub might use is a redirect, and an ordinary
/// answer is not.
#[test]
fn the_redirect_statuses_are_the_ones_that_redirect() {
    let from = url("https://huggingface.co/x");
    for status in [301, 302, 303, 307, 308] {
        let answer = response(&format!(
            "HTTP/1.1 {status} Moved\r\nLocation: /elsewhere\r\n\r\n"
        ));
        assert!(
            redirect(&answer, &from).expect("read").is_some(),
            "{status} did not redirect"
        );
    }
    for status in [200, 206, 401, 403, 404, 429, 500] {
        let answer = response(&format!("HTTP/1.1 {status} Something\r\n\r\n"));
        assert_eq!(
            redirect(&answer, &from).expect("read"),
            None,
            "{status} was read as a redirect"
        );
    }
}

/// A redirect with nowhere to go is a source MCF cannot follow, said rather
/// than guessed at.
#[test]
fn a_redirect_with_no_location_is_refused() {
    let failure = next(
        &response("HTTP/1.1 302 Found\r\n\r\n"),
        &url("https://x.example/y"),
    )
    .expect_err("nowhere to go");
    assert_eq!(failure.category(), Category::HubMetadataMalformed);
}

/// A status MCF has no answer for is not this module's business: it says *the
/// body is the answer* and the caller with the reference and the credential
/// classifies it (B-024).
#[test]
fn an_unauthorized_answer_is_left_for_the_caller_to_classify() {
    let answer = response("HTTP/1.1 401 Unauthorized\r\n\r\n");
    assert_eq!(answer.status(), 401);
    assert_eq!(
        next(&answer, &url("https://x.example/y")).expect("read"),
        Next::Body
    );
}

/// The range the hub returns for a resumed transfer, and the one that would
/// quietly corrupt a file.
#[test]
fn a_content_range_says_where_the_bytes_go() {
    let answer =
        response("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-15/396705472\r\n\r\n");
    let range = answer
        .content_range()
        .expect("a range")
        .expect("one is there");
    assert_eq!(range.first, 0);
    assert_eq!(range.last, 15);
    assert_eq!(range.total, Some(396_705_472));
    assert!(range.continues_from(0));
    assert!(!range.continues_from(16));

    let restarted =
        response("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-99/1000\r\n\r\n")
            .content_range()
            .expect("a range")
            .expect("one is there");
    assert!(
        !restarted.continues_from(900),
        "a source that restarted was read as continuing, which is how a file becomes a mixture"
    );
}

/// A total nobody will state is unknown rather than an error: the source has
/// still said where this piece goes (A7).
#[test]
fn a_range_with_no_total_is_still_a_range() {
    let range = response("HTTP/1.1 206 Partial\r\nContent-Range: bytes 4-9/*\r\n\r\n")
        .content_range()
        .expect("a range")
        .expect("one is there");
    assert_eq!(range.total, None);
    assert!(range.continues_from(4));
}

/// Everything a source could say that is not a range.
#[test]
fn a_range_mcf_cannot_read_is_refused() {
    for value in [
        "items 0-15/100",
        "bytes 0-15",
        "bytes 015/100",
        "bytes a-b/100",
        "bytes 20-10/100",
        // The fuzz tier's find: a span that ends past the file it names, which
        // is a source contradicting itself in the two numbers a fetcher acts on
        // (B-191).
        "bytes 0-15/2",
        "bytes 0-15/many",
    ] {
        let answer = response(&format!(
            "HTTP/1.1 206 Partial\r\nContent-Range: {value}\r\n\r\n"
        ));
        let failure = answer.content_range().expect_err(value);
        assert_eq!(
            failure.category(),
            Category::HubMetadataMalformed,
            "{value}"
        );
    }
}

/// A length that is not a number is refused rather than treated as zero: a zero
/// would make an empty file look like a complete one.
#[test]
fn a_length_that_is_not_a_number_is_refused() {
    let answer = response("HTTP/1.1 200 OK\r\nContent-Length: lots\r\n\r\n");
    assert_eq!(
        answer
            .content_length()
            .expect_err("not a number")
            .category(),
        Category::HubMetadataMalformed
    );
    assert_eq!(
        response("HTTP/1.1 200 OK\r\n\r\n")
            .content_length()
            .expect("absent is a state"),
        None
    );
}

/// Bytes that stop mid-header are an interruption rather than a defect: the
/// caller can read more and try again, and telling it the source was malformed
/// would send it to the wrong conclusion.
#[test]
fn a_response_that_has_not_finished_arriving_says_so() {
    let failure = Response::read(b"HTTP/1.1 200 OK\r\nContent-Len").expect_err("incomplete");
    assert_eq!(failure.category(), Category::TransferInterrupted);
}

/// A source that never ends its headers is stopped at a stated ceiling rather
/// than allowed to exhaust this machine (§3.7, B7).
#[test]
fn headers_without_end_are_stopped_at_the_ceiling() {
    let mut bytes = b"HTTP/1.1 200 OK\r\n".to_vec();
    while bytes.len() <= HEADER_CEILING {
        bytes.extend_from_slice(b"X-Filler: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n");
    }
    let failure = Response::read(&bytes).expect_err("past the ceiling");
    assert_eq!(failure.category(), Category::HubMetadataMalformed);
}

/// Everything else a source could put where a response goes.
#[test]
fn what_is_not_a_response_is_refused_by_name() {
    for rubbish in [
        "HTTP/2 200 OK\r\n\r\n",
        "200 OK\r\n\r\n",
        "HTTP/1.1 twenty OK\r\n\r\n",
        "HTTP/1.1 999 Nonsense\r\n\r\n",
        "HTTP/1.1 200 OK\r\na header with no colon\r\n\r\n",
    ] {
        let failure = Response::read(rubbish.as_bytes()).expect_err(rubbish);
        assert_eq!(
            failure.category(),
            Category::HubMetadataMalformed,
            "{rubbish}"
        );
    }
}

/// A source's own words are kept in the refusal and bounded, because §3.7 says
/// it can send a megabyte where a word belongs and A1 still wants what was
/// seen.
#[test]
fn what_a_source_said_is_kept_but_bounded() {
    let enormous = format!("HTTP/1.1 200 OK\r\n{}\r\n\r\n", "x".repeat(4096));
    let failure = Response::read(enormous.as_bytes()).expect_err("no colon");
    let found = failure
        .context()
        .iter()
        .find(|entry| entry.key == "found")
        .map(|entry| entry.value.clone())
        .unwrap_or_default();
    assert!(found.contains("xxx"), "what was seen is not in the refusal");
    assert!(
        found.chars().count() < 300,
        "a source wrote {} characters into MCF's refusal",
        found.chars().count()
    );
}

/// The ceilings are stated numbers rather than whatever arrives.
#[test]
fn the_ceilings_are_stated() {
    assert_eq!(HEADER_CEILING, 65_536);
    assert_eq!(REDIRECT_CEILING, 5);
}

/// A URL renders as itself, so a record and a message name the same place.
#[test]
fn a_url_renders_as_what_it_is() {
    for text in [
        "https://huggingface.co/api/models/owner/model",
        "http://localhost:8080/x?y=1",
    ] {
        assert_eq!(url(text).to_string(), text);
    }
    assert_eq!(
        url("https://huggingface.co").to_string(),
        "https://huggingface.co/"
    );
}
