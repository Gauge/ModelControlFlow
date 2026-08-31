//! HTTP/1.1, as far as MCF needs it and no further (B-322, §III).
//!
//! **Why MCF writes this and vendors the cryptography.**
//! [findings.md](../../../doc/findings.md) F9 measured the boundary: the hub
//! answers HTTP/1.1, so nothing here needs HTTP/2's framing, header compression
//! or flow control; and it answers only over TLS, which is an implementation
//! nobody at MCF is going to write. So the split is the one D32 chose for
//! inference — delegate what specialists maintain, own the wrapper that has to
//! be correct. This is the wrapper.
//!
//! **It touches no socket.** Everything here turns a request into bytes and
//! bytes into an answer, which is what makes the two behaviours that actually
//! matter testable without a network:
//!
//! - **A redirect must not carry a credential to another host.** The hub sends
//!   a `302` to a signed URL on a CDN (F9 §9.1). A client that forwarded the
//!   `Authorization` header would hand the operator's token to whatever host
//!   the answer named — and the answer comes from the network, which §3.7 says
//!   is untrusted. [`redirect`] returns *where to go and whether the credential
//!   goes with it*, and the second half is a decision rather than an oversight.
//! - **A response's own claims are checked before they are believed.** A
//!   `Content-Length` that is not a number, a `Content-Range` that starts
//!   somewhere else than where the transfer resumed, a header block that never
//!   ends — each is a refusal here rather than a surprise later.
//!
//! **Everything is total.** No input reaches a panic, an unbounded allocation
//! or a hang: the header block has a stated ceiling, the status line has a
//! stated shape, and anything else is `hub.metadata.malformed` naming what was
//! seen. That is what makes it a fuzz target (B-191) rather than a hope.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-hub::http");

/// How much header a response may have before MCF stops reading it.
///
/// Sixty-four kibibytes, which is more than twice what the hub sends and less
/// than a hostile source could use to exhaust this machine. §3.7 makes the far
/// end untrusted, so the bound is a stated number rather than *whatever
/// arrives*: an unbounded read is how a source with no model at all turns into
/// a memory exhaustion (B7).
pub const HEADER_CEILING: usize = 64 * 1024;

/// How many redirects a request will follow.
///
/// Five. The hub uses two (repository → cache → CDN), so five is room for the
/// shape to change without being room for a loop. A source that wants more is
/// either broken or trying something, and both end here.
pub const REDIRECT_CEILING: usize = 5;

/// Where a request is going.
///
/// Parsed rather than passed around as a string, because the security question
/// — *is this the same host I was talking to?* — is a comparison a string
/// invites getting wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    /// `https` or `http`.
    scheme: String,
    /// The host, lowercased: `Example.com` and `example.com` are one host, and
    /// a comparison that said otherwise would drop a credential MCF should
    /// carry or carry one it should not.
    host: String,
    /// The port, which is the scheme's default where none was written.
    port: u16,
    /// Everything from the path onwards, query included.
    target: String,
}

impl Url {
    /// Reads a URL.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` for anything that is not one, naming what was
    /// seen. Only `http` and `https` are URLs as far as MCF is concerned: a
    /// `file:` or `ftp:` redirect is a source pointing at this machine or at
    /// something MCF does not speak, and §3.7 makes that a refusal rather than
    /// an experiment.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        let (scheme, rest) = match text.split_once("://") {
            Some((scheme, rest)) => (scheme.to_ascii_lowercase(), rest),
            None => return Err(malformed("a URL has a scheme", text)),
        };
        let default_port = match scheme.as_str() {
            "https" => 443_u16,
            "http" => 80_u16,
            _ => return Err(malformed("MCF speaks http and https", text)),
        };

        let (authority, target) = match rest.find('/') {
            Some(at) => (
                rest.get(..at).unwrap_or_default(),
                rest.get(at..).unwrap_or("/"),
            ),
            None => (rest, "/"),
        };
        if authority.contains('@') {
            // A credential in a URL is a credential in a log, a record and a
            // shell history. B-024 keeps them in one place and this is not it.
            return Err(malformed("a URL carrying a credential is refused", text));
        }

        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => match port.parse::<u16>() {
                Ok(port) => (host, port),
                Err(_) => return Err(malformed("a port is a number", authority)),
            },
            None => (authority, default_port),
        };
        if host.is_empty() {
            return Err(malformed("a URL has a host", text));
        }

        Ok(Self {
            scheme,
            host: host.to_ascii_lowercase(),
            port,
            target: target.to_owned(),
        })
    }

    /// The scheme.
    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The host, lowercased.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// The port, which is the scheme's default where none was written.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// The path and query, as the request line carries them.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Whether two URLs are the same place to send a credential.
    ///
    /// Scheme, host and port together. Not the path: a credential is for a
    /// host, and a host that gave MCF a token for one path has it for the
    /// others. Not a suffix match either — `evil-huggingface.co` ends with
    /// nothing `huggingface.co` does, and a check written with `ends_with`
    /// would have said otherwise.
    #[must_use]
    pub fn same_origin(&self, other: &Self) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }

    /// Resolves a `Location` against the URL it was answered from.
    ///
    /// The hub answers a relative location for its own cache and an absolute
    /// one for the CDN (F9 §9.1), so both shapes are real.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` when the location is neither.
    pub fn resolve(&self, location: &str) -> Result<Self> {
        let location = location.trim();
        if location.contains("://") {
            return Self::parse(location);
        }
        if location.starts_with("//") {
            return Self::parse(&format!("{}:{location}", self.scheme));
        }
        if location.starts_with('/') {
            return Ok(Self {
                scheme: self.scheme.clone(),
                host: self.host.clone(),
                port: self.port,
                target: location.to_owned(),
            });
        }
        Err(malformed("a location MCF cannot resolve", location))
    }

    /// How the host is written in a `Host` header: with the port only when it
    /// is not the scheme's default, which is what every server expects.
    #[must_use]
    pub fn authority(&self) -> String {
        let default = match self.scheme.as_str() {
            "https" => 443_u16,
            _ => 80_u16,
        };
        if self.port == default {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

impl core::fmt::Display for Url {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}://{}{}", self.scheme, self.authority(), self.target)
    }
}

/// A request MCF is about to make.
///
/// `GET` only, because that is the whole of what acquisition does: read a
/// listing, read a file, read part of a file. A method MCF does not need is a
/// method the laboratory cannot exercise (A19), and adding one is a decision
/// rather than a convenience.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    url: Url,
    from: Option<u64>,
    /// The last byte wanted, where the caller wants a bounded range.
    ///
    /// A resumption is open-ended — *everything from here* — and reading a
    /// header is not: what is wanted is a prefix, and asking for the whole of
    /// a forty-gigabyte file to read its first four megabytes would be
    /// acquiring a model to answer a question about it.
    to: Option<u64>,
    credential: Option<String>,
}

impl Request {
    /// A request for the whole of something.
    #[must_use]
    pub fn get(url: Url) -> Self {
        Self {
            url,
            from: None,
            to: None,
            credential: None,
        }
    }

    /// The same, continuing from a byte offset — which the hub serves (F9).
    #[must_use]
    pub fn resuming(mut self, offset: u64) -> Self {
        self.from = Some(offset);
        self
    }

    /// The first `bytes` of the file and no more.
    ///
    /// For reading a header without acquiring what is behind it. A hub that
    /// ignores the range answers with the whole file, which the caller has to
    /// be ready for — so the bound is enforced again on the reading side
    /// rather than trusted here (§3.7).
    #[must_use]
    pub fn first(mut self, bytes: u64) -> Self {
        self.from = Some(0);
        self.to = Some(bytes.saturating_sub(1));
        self
    }

    /// The same, offering a credential.
    ///
    /// Takes the token itself rather than a [`crate::credentials::Credential`]
    /// so that the one place a secret leaves its box is a call the reader can
    /// see (B-024). What arrives here is already a decision somebody made.
    #[must_use]
    pub fn offering(mut self, token: &str) -> Self {
        self.credential = Some(token.to_owned());
        self
    }

    /// Where it is going.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// The same request, sent somewhere a redirect named.
    ///
    /// The credential is kept only when the caller says so, which
    /// [`crate::http::next`] decides by comparing origins. Written as a
    /// constructor rather than a mutation so that *dropping the token* is a
    /// visible act in the one place redirects are followed, rather than a field
    /// somebody forgets to clear.
    ///
    /// The range is kept: a redirect is the same transfer, and a resumption
    /// that lost its offset on the way to a CDN would start again from zero and
    /// report progress that did not happen.
    #[must_use]
    pub fn redirected(&self, to: Url, carrying_the_credential: bool) -> Self {
        Self {
            url: to,
            from: self.from,
            // The bound travels with the redirect for the same reason the
            // offset does: a request that lost it on the way to a CDN would
            // ask for the whole file.
            to: self.to,
            credential: if carrying_the_credential {
                self.credential.clone()
            } else {
                None
            },
        }
    }

    /// Whether it carries a credential.
    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.credential.is_some()
    }

    /// The bytes to put on the wire.
    ///
    /// `Connection: close` because MCF makes one request per connection: reuse
    /// is an optimization, and an optimization that shares state between two
    /// transfers is one the laboratory would have to simulate to test either.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let range = match (self.from, self.to) {
            (Some(from), Some(to)) => format!("Range: bytes={from}-{to}\r\n"),
            (Some(from), None) => format!("Range: bytes={from}-\r\n"),
            (None, _) => String::new(),
        };
        let authorization = match &self.credential {
            Some(token) => format!("Authorization: Bearer {token}\r\n"),
            None => String::new(),
        };
        format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {}\r\nAccept: */*\r\nAccept-Encoding: \
             identity\r\nConnection: close\r\n{range}{authorization}\r\n",
            self.url.target(),
            self.url.authority(),
            user_agent()
        )
        .into_bytes()
    }
}

/// What MCF calls itself to a hub.
///
/// Its own name and version, because a hub's operator reading their logs should
/// be able to tell what this is, and because a client that impersonated a
/// browser would be lying to somebody who has to run a service.
///
/// Deliberately *not* the machine, the operating system or anything else about
/// this computer: a user agent is a thing that leaves this machine, and §XIV's
/// habit — say what is needed and nothing more — starts here rather than at the
/// contribution surface.
#[must_use]
pub fn user_agent() -> String {
    format!("mcf/{}", env!("CARGO_PKG_VERSION"))
}

/// What a source answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
}

impl Response {
    /// Reads the head of a response.
    ///
    /// Returns the response and how many bytes it consumed, so the caller knows
    /// where the body begins in what it has already read.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` when the status line is not one, a header line
    /// has no colon, or the header block passes [`HEADER_CEILING`] without
    /// ending. `transfer.interrupted` when the bytes simply stop — which is a
    /// different thing from being wrong, and the caller can read more and try
    /// again.
    pub fn read(bytes: &[u8]) -> Result<(Self, usize)> {
        let end = match find_head_end(bytes) {
            Some(end) => end,
            None if bytes.len() > HEADER_CEILING => {
                return Err(malformed(
                    "a header block that does not end within the ceiling",
                    &format!("{} bytes and counting", bytes.len()),
                ));
            }
            None => {
                return Err(Failure::new(
                    Category::TransferInterrupted,
                    Attribution::Machine,
                    Disposition::Partial,
                    WHERE,
                    "the response ended before its headers did",
                )
                .with_context("bytes_read", bytes.len().to_string()));
            }
        };

        let head = bytes.get(..end).unwrap_or_default();
        let text = core::str::from_utf8(head)
            .map_err(|_| malformed("a header block that is not text", "not UTF-8"))?;
        let mut lines = text.split("\r\n");
        let status_line = lines
            .next()
            .ok_or_else(|| malformed("a response has a status line", text))?;
        let status = read_status(status_line)?;

        let mut headers = Vec::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let (name, value) = line
                .split_once(':')
                .ok_or_else(|| malformed("a header line has a colon", line))?;
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
        }

        Ok((Self { status, headers }, end))
    }

    /// The status code.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// A header, by its lower-case name.
    ///
    /// The first of them where a source sent several: a duplicate header is a
    /// source disagreeing with itself, and taking the last would let a second
    /// value overwrite one MCF has already acted on.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every header, in the order they arrived.
    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// How many bytes the source says the body is.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` when the header is present and is not a number.
    /// `None` when it is absent, which is a state rather than a zero (A7).
    pub fn content_length(&self) -> Result<Option<u64>> {
        match self.header("content-length") {
            None => Ok(None),
            Some(value) => value
                .parse::<u64>()
                .map(Some)
                .map_err(|_| malformed("a content-length is a number", value)),
        }
    }

    /// Where in the file a partial response starts, and how long the whole file
    /// is: `bytes 0-15/396705472`.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` for a range MCF cannot read. A `*` total is
    /// read as unknown rather than as an error: a source that will not say how
    /// long a file is has still told MCF where this piece goes.
    pub fn content_range(&self) -> Result<Option<ContentRange>> {
        let Some(value) = self.header("content-range") else {
            return Ok(None);
        };
        let rest = value
            .strip_prefix("bytes ")
            .ok_or_else(|| malformed("a content-range in bytes", value))?;
        let (span, total) = rest
            .split_once('/')
            .ok_or_else(|| malformed("a content-range states a total", value))?;
        let (first, last) = span
            .split_once('-')
            .ok_or_else(|| malformed("a content-range states a span", value))?;
        let first = first
            .trim()
            .parse::<u64>()
            .map_err(|_| malformed("a range start is a number", value))?;
        let last = last
            .trim()
            .parse::<u64>()
            .map_err(|_| malformed("a range end is a number", value))?;
        if last < first {
            return Err(malformed("a range that ends before it starts", value));
        }
        let total = match total.trim() {
            "*" => None,
            number => Some(
                number
                    .parse::<u64>()
                    .map_err(|_| malformed("a range total is a number or *", value))?,
            ),
        };
        // A source that says *bytes 0-15 of a 2-byte file* is contradicting
        // itself, and both halves of that sentence are things a fetcher acts
        // on: the total is what it plans against and the span is what it
        // appends. Found by the fuzz tier, which fed a real answer's own
        // numbers back in mutated (B-191).
        if total.is_some_and(|total| last >= total) {
            return Err(malformed(
                "a range that ends inside the file it names",
                value,
            ));
        }
        Ok(Some(ContentRange { first, last, total }))
    }
}

/// Which part of a file a `206` carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    /// The offset of the first byte in this response.
    pub first: u64,
    /// The offset of the last byte in it.
    pub last: u64,
    /// How long the whole file is, where the source said.
    pub total: Option<u64>,
}

impl ContentRange {
    /// Whether this is the continuation the caller asked for.
    ///
    /// A source that answers a request for byte 900 with byte 0 and a `206` is
    /// not resuming, and a fetcher that appended it would build a file that is
    /// the first megabyte twice. B-021 deletes such a mixture; this is where it
    /// is noticed.
    #[must_use]
    pub const fn continues_from(&self, offset: u64) -> bool {
        self.first == offset
    }
}

/// What to do about an answer that is not the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The body is the answer.
    Body,
    /// Go here instead.
    Follow {
        /// Where.
        to: Url,
        /// Whether the credential goes too.
        ///
        /// False whenever the destination is a different origin, which is the
        /// hub's ordinary case: it redirects to a CDN whose URL is already
        /// signed, and a token sent there would be a token given to a host MCF
        /// was told about by the network (§3.7, B-024).
        carrying_the_credential: bool,
    },
}

/// Reads what a response means for the request that produced it.
///
/// # Errors
///
/// `hub.metadata.malformed` when a redirect names no location or names one that
/// cannot be resolved. Status codes that are *outcomes* — unauthorized,
/// forbidden, not found, throttled — are not failures here: this function says
/// what to do next, and the caller with the reference and the credential is the
/// one that can classify them (`credentials::missing` and its neighbours).
pub fn next(response: &Response, from: &Url) -> Result<Next> {
    match response.status() {
        301 | 302 | 303 | 307 | 308 => {
            let location = response
                .header("location")
                .ok_or_else(|| malformed("a redirect names a location", "no location header"))?;
            let to = from.resolve(location)?;
            let carrying_the_credential = from.same_origin(&to);
            Ok(Next::Follow {
                to,
                carrying_the_credential,
            })
        }
        _ => Ok(Next::Body),
    }
}

/// Where a redirect leads, and whether the credential goes with it.
///
/// The short form of [`next`] for a caller that only wants the redirect.
///
/// # Errors
///
/// As [`next`].
pub fn redirect(response: &Response, from: &Url) -> Result<Option<(Url, bool)>> {
    match next(response, from)? {
        Next::Body => Ok(None),
        Next::Follow {
            to,
            carrying_the_credential,
        } => Ok(Some((to, carrying_the_credential))),
    }
}

/// Reads `HTTP/1.1 206 Partial Content`.
fn read_status(line: &str) -> Result<u16> {
    let rest = line
        .strip_prefix("HTTP/1.1 ")
        .or_else(|| line.strip_prefix("HTTP/1.0 "))
        .ok_or_else(|| malformed("a status line begins with HTTP/1.1", line))?;
    let code = rest.split_whitespace().next().unwrap_or_default();
    let status = code
        .parse::<u16>()
        .map_err(|_| malformed("a status is a number", line))?;
    if (100..600).contains(&status) {
        Ok(status)
    } else {
        Err(malformed("a status is between 100 and 599", line))
    }
}

/// Where the header block ends, as an offset past the blank line.
///
/// `\r\n\r\n` only. A bare `\n\n` is what a hand-written server sends and MCF
/// is talking to a hub: accepting both would mean accepting a body that begins
/// with a newline as the end of the headers.
fn find_head_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .and_then(|at| at.checked_add(4))
}

fn malformed(wanted: &str, found: &str) -> Failure {
    Failure::new(
        Category::HubMetadataMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "a source answered with something MCF cannot read",
    )
    .with_context("wanted", wanted.to_owned())
    .with_context("found", elide(found))
}

/// A source's own words, kept but bounded.
///
/// A1 wants what was seen recorded; §3.7 says the far end is untrusted and can
/// send a megabyte where a word belongs. Two hundred characters is enough to
/// recognize what happened and not enough to be a way of writing to MCF's
/// record.
fn elide(found: &str) -> String {
    if found.chars().count() <= 200 {
        return found.to_owned();
    }
    let kept: String = found.chars().take(200).collect();
    format!("{kept}… ({} characters in all)", found.chars().count())
}

#[cfg(test)]
mod tests;
