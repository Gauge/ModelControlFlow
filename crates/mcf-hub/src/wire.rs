//! The socket, behind a boundary the cryptography will slot into (B-322).
//!
//! **What this is for.** [`crate::http`] turns requests into bytes and bytes
//! into answers; something has to carry them. That something is a socket today
//! and a TLS session once a stack is vendored ([findings.md](../../../doc/findings.md)
//! F9), and the difference between the two is one implementation of [`Wire`] —
//! which is the point of writing the boundary before the cryptography rather
//! than after. Everything above this line is finished and tested; what is
//! missing is one struct.
//!
//! **Nothing here waits forever.** B7 makes a hang a defined outcome rather
//! than an exception, and a socket is where hangs come from: a host that
//! accepts a connection and says nothing will hold a thread until the process
//! dies. So every wire carries deadlines — to connect, to read, to finish — and
//! passing one is `transfer.stalled` naming which deadline and how long it was.
//!
//! **A wire moves bytes and decides nothing.** It does not know what a hub is,
//! it never sees a credential, and it cannot choose where anything on this
//! machine goes: the caller supplies the destination, as [`crate::source`]
//! already requires of a source. That is what makes the laboratory's version of
//! it — a listener on the loopback address, answering from a script — a
//! substitution rather than a simulation of MCF's own code (D26).

use std::io::Write as _;
use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::http::{HEADER_CEILING, Next, REDIRECT_CEILING, Request, Response, Url, next};

const WHERE: Subsystem = Subsystem::new("mcf-hub::wire");

/// How long a wire waits, at each place waiting happens.
///
/// Stated numbers rather than *whatever the kernel does*, because the kernel's
/// answer is minutes and a person watching a command wants to be told sooner
/// than that. They are generous enough that a slow link is not an error and
/// short enough that a dead one is noticed: a hub that has not accepted a
/// connection in fifteen seconds is not going to, and one that has sent no byte
/// for sixty seconds has stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadlines {
    /// How long to wait for a connection to be accepted.
    pub connect: Duration,
    /// How long to wait for the next byte once one is expected.
    pub idle: Duration,
}

impl Default for Deadlines {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(15),
            idle: Duration::from_secs(60),
        }
    }
}

/// Something that carries bytes to a host and back.
pub trait Duplex: std::io::Read + std::io::Write {}

impl<T: std::io::Read + std::io::Write> Duplex for T {}

/// How MCF reaches a host.
///
/// One implementation per way of carrying bytes: [`Tcp`] here, TLS when a stack
/// is admitted, and the laboratory's own for scenarios. A caller written
/// against this is a caller the laboratory can drive through every transfer
/// failure without a network (B19).
pub trait Wire {
    /// What this is, for a record and for a message (§3.4).
    fn describe(&self) -> String;

    /// Whether this wire is safe to send a credential over.
    ///
    /// Plain TCP is not. A token on an unencrypted connection is a token given
    /// to everything between here and the host, and B-024 makes credentials the
    /// operator's rather than something MCF spends on their behalf. [`fetch`]
    /// refuses rather than downgrading, because a silent downgrade is how a
    /// secret leaks in a way nobody can see afterwards (A2).
    fn carries_secrets(&self) -> bool;

    /// Opens a connection.
    ///
    /// # Errors
    ///
    /// `hub.unreachable` when the host cannot be resolved or will not accept a
    /// connection; `transfer.stalled` when it does not accept one in time.
    fn dial(&self, host: &str, port: u16) -> Result<Box<dyn Duplex>>;
}

/// A plain TCP connection.
///
/// Enough for `http://`, which is what the laboratory's own hub speaks and what
/// a mirror on a trusted network can speak. Not enough for the real hub, which
/// answers only over TLS — F9 measured that boundary and B-322 is where it is
/// crossed.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tcp {
    /// How long it waits.
    pub deadlines: Deadlines,
}

impl Wire for Tcp {
    fn describe(&self) -> String {
        "a plain TCP connection".to_owned()
    }

    fn carries_secrets(&self) -> bool {
        false
    }

    fn dial(&self, host: &str, port: u16) -> Result<Box<dyn Duplex>> {
        let mut addresses = (host, port).to_socket_addrs().map_err(|error| {
            // What MCF observed is that this machine could not turn a name into
            // an address. Which of the three causes it was — no resolver, no
            // network, no such name — is not visible from here: the platform
            // reports one thing for all of them (F10), and A7 forbids picking
            // the likely one. D33 is why that is stated rather than guessed.
            Failure::new(
                Category::HubUnreachable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "this machine could not turn that name into an address",
            )
            .with_context("host", format!("{host}:{port}"))
            .with_context("reason", error.to_string())
            .with_context(
                "what_this_does_not_say",
                "whether there is no network, no resolver, or no such name: one observation, \
                 three causes, and MCF does not choose between them (A7, D33)",
            )
            .with_context(
                "what_to_do",
                "everything but acquisition works with no network at all; where a mirror is \
                 reachable, --from points at one",
            )
        })?;
        let address = addresses.next().ok_or_else(|| {
            Failure::new(
                Category::HubUnreachable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the host resolved to no address at all",
            )
            .with_context("host", format!("{host}:{port}"))
        })?;

        let stream =
            TcpStream::connect_timeout(&address, self.deadlines.connect).map_err(|error| {
                // Three observations, kept apart, because they are three
                // different things for an operator to do something about (D33,
                // F10). Refused means something is there and said no; a route
                // that does not exist means this machine cannot get there at
                // all; and silence means MCF's own deadline ended the wait
                // rather than the far end.
                let (category, detail, says) = match error.kind() {
                    std::io::ErrorKind::TimedOut => (
                        Category::TransferStalled,
                        "nothing answered within the deadline MCF set",
                        "a path may exist and nothing on it answered in time; this is MCF's                          deadline rather than the network's",
                    ),
                    std::io::ErrorKind::ConnectionRefused => (
                        Category::HubUnreachable,
                        "something is at that address and refused the connection",
                        "there is a working path to that host: what refused is the host, or                          something answering for it",
                    ),
                    _ => (
                        Category::HubUnreachable,
                        "this machine has no way to reach that address",
                        "the platform reports no route rather than a refusal, which is what                          a machine with no network looks like from here",
                    ),
                };
                Failure::new(
                    category,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    detail,
                )
                .with_context("host", format!("{host}:{port}"))
                .with_context("waited", format!("{:?}", self.deadlines.connect))
                .with_context("what_this_says", says.to_owned())
                .with_context("reason", error.to_string())
            })?;
        for set in [
            stream.set_read_timeout(Some(self.deadlines.idle)),
            stream.set_write_timeout(Some(self.deadlines.idle)),
        ] {
            set.map_err(|error| {
                Failure::new(
                    Category::HubUnreachable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "a connection that cannot be given a deadline is one that can hang (B7)",
                )
                .with_context("reason", error.to_string())
            })?;
        }
        Ok(Box::new(stream))
    }
}

/// A TLS 1.3 connection (B-322).
///
/// The one thing MCF vendors rather than writes, and the reason is in
/// [findings.md](../../../doc/findings.md) F9: nobody here is going to write a
/// TLS implementation, and A19 forbids claiming what is not tested. What is
/// vendored is the smallest tree that does it — fourteen crates, no C, and a
/// provider that builds for every target MCF builds, which F9.5 and F9.6 chose
/// by measurement rather than by default.
///
/// **The certificate authorities are the ones compiled in.** `webpki-roots` is
/// a pinned set rather than the machine's store: §3.12 makes what MCF built
/// with a condition of what it did, and a trust store that differs between two
/// machines is two different verifications wearing one name. It also means the
/// set expires — a root store is a thing to re-pin, which
/// [vendored.md](../../../doc/vendored.md) records as a row rather than
/// leaving to be discovered.
pub struct Tls {
    /// How long it waits.
    pub deadlines: Deadlines,
    configuration: std::sync::Arc<rustls::ClientConfig>,
}

impl core::fmt::Debug for Tls {
    /// The deadlines and how many roots MCF ships. The configuration itself is
    /// summarized rather than printed: it is a certificate store, and a `Debug`
    /// that emitted it would bury whatever a reader was actually looking at.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tls")
            .field("deadlines", &self.deadlines)
            .field("roots", &webpki_roots::TLS_SERVER_ROOTS.len())
            .field("configuration", &"<the vendored provider's>")
            .finish()
    }
}

impl Tls {
    /// A wire that speaks TLS, with the roots MCF ships.
    ///
    /// # Errors
    ///
    /// `transfer.tls` when the vendored provider will not make a configuration,
    /// which is a build that is wrong rather than a network that is — and a
    /// thing to say rather than to panic about (A2).
    pub fn new() -> Result<Self> {
        Self::with(Deadlines::default())
    }

    /// The same, waiting for as long as this says.
    ///
    /// # Errors
    ///
    /// As [`Self::new`].
    pub fn with(deadlines: Deadlines) -> Result<Self> {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let configuration = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
            rustls_graviola::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|error| {
            Failure::new(
                Category::TransferTls,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "the vendored TLS provider would not agree on protocol versions",
            )
            .with_context("reason", error.to_string())
        })?
        .with_root_certificates(roots)
        .with_no_client_auth();

        Ok(Self {
            deadlines,
            configuration: std::sync::Arc::new(configuration),
        })
    }
}

impl Wire for Tls {
    fn describe(&self) -> String {
        format!(
            "TLS over TCP, rustls with the graviola provider and {} compiled-in roots",
            webpki_roots::TLS_SERVER_ROOTS.len()
        )
    }

    fn carries_secrets(&self) -> bool {
        true
    }

    fn dial(&self, host: &str, port: u16) -> Result<Box<dyn Duplex>> {
        let name = rustls::pki_types::ServerName::try_from(host.to_owned()).map_err(|error| {
            Failure::new(
                Category::TransferTls,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "that is not a name a certificate can be checked against",
            )
            .with_context("host", host.to_owned())
            .with_context("reason", error.to_string())
        })?;
        let mut session =
            rustls::ClientConnection::new(std::sync::Arc::clone(&self.configuration), name)
                .map_err(|error| tls_failed("a TLS session could not be started", host, &error))?;

        let plain = Tcp {
            deadlines: self.deadlines,
        };
        let mut socket = plain.dial(host, port)?;

        // The handshake happens here rather than on the first read, so that a
        // certificate MCF will not accept is an outcome of *dialling* — which
        // is where a caller looks for it, and what keeps a TLS failure from
        // arriving wearing a transfer's clothes (A2).
        session
            .complete_io(&mut socket)
            .map_err(|error| tls_failed("the TLS handshake did not complete", host, &error))?;

        Ok(Box::new(rustls::StreamOwned::new(session, socket)))
    }
}

fn tls_failed(what: &str, host: &str, error: &dyn core::fmt::Display) -> Failure {
    Failure::new(
        Category::TransferTls,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        what.to_owned(),
    )
    .with_context("host", host.to_owned())
    .with_context("reason", error.to_string())
}

/// What a completed exchange produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchanged {
    /// The final answer, after any redirects.
    pub response: Response,
    /// How many bytes of body arrived.
    pub bytes: u64,
    /// Where the answer finally came from, which is not always where the
    /// request went: the hub redirects downloads to a CDN, and *which host
    /// served these bytes* is a condition of the artifact (§3.4).
    pub served_by: Url,
    /// How many redirects were followed, which is a fact about the hub worth
    /// recording rather than an implementation detail.
    pub redirects: usize,
}

/// What to do with a body once its head has been read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handling {
    /// Write it out.
    Take,
    /// Read no further: the head was enough to know this is not the answer.
    Skip,
}

/// Makes a request, follows what it is told to, and writes the body out.
///
/// The credential travels only as far as the origin it was given for: a
/// redirect to another host is followed *without* it (see [`crate::http`]), and
/// a wire that cannot keep a secret refuses to carry one at all.
///
/// # Errors
///
/// `hub.unreachable`, `transfer.stalled` and `transfer.interrupted` from the
/// wire; `hub.metadata.malformed` from the answer; `config.invalid` when a
/// credential is offered over a wire that cannot keep it. A redirect loop ends
/// at [`REDIRECT_CEILING`] with `hub.unreachable`, naming every host it was
/// sent to — going round for ever is the failure B7 exists to prevent.
pub fn fetch(
    wire: &dyn Wire,
    request: &Request,
    into: &mut dyn std::io::Write,
) -> Result<Exchanged> {
    vetted(wire, request, into, &|_| Ok(()))
}

/// The same, with a chance to refuse the answer before a byte of it is written.
///
/// `vet` sees the head of the answer that is *not* a redirect, and nothing has
/// been written when it is called. That ordering is the whole point: a source
/// that answers a resumption by starting again, or answers a request for
/// weights with an error page, must be refused **before** its bytes reach the
/// file — an append that is undone afterwards is a file that was wrong in
/// between, and a crash in between leaves it wrong for good (B-021, A1).
///
/// A redirect's body is never read at all: the head named somewhere else to go,
/// and reading the courtesy page underneath it would be bytes nobody wanted.
///
/// # Errors
///
/// As [`fetch`], plus whatever `vet` returns.
pub fn vetted(
    wire: &dyn Wire,
    request: &Request,
    into: &mut dyn std::io::Write,
    vet: &dyn Fn(&Response) -> Result<()>,
) -> Result<Exchanged> {
    if request.url().scheme() == "https" && !wire.carries_secrets() {
        return Err(Failure::new(
            Category::ConfigUnsatisfiable,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "MCF cannot open an encrypted connection: no TLS stack is vendored",
        )
        .with_context("asked", request.url().to_string())
        .with_context("wire", wire.describe())
        .with_context(
            "what_to_do",
            "this is stated rather than attempted and failed obscurely. A stack is admitted in \
             doc/vendored.md and B-322 is the item; until then an http source — a mirror, or \
             the laboratory's own hub — is what MCF can reach",
        ));
    }
    if request.is_authenticated() && !wire.carries_secrets() {
        return Err(Failure::new(
            Category::ConfigInvalid,
            Attribution::Mcf,
            Disposition::Refused,
            WHERE,
            "a credential was offered over a connection that cannot keep it",
        )
        .with_context("wire", wire.describe())
        .with_context(
            "what_to_do",
            "this is refused rather than downgraded: a token sent in the clear is a token \
             given to everything between here and the host (B-024)",
        ));
    }

    let mut attempt = request.clone();
    let mut visited: Vec<String> = Vec::new();
    for redirects in 0..=REDIRECT_CEILING {
        visited.push(attempt.url().to_string());
        // A cell rather than a plain binding: the decision is made inside a
        // closure that only borrows, and where a redirect leads is the one
        // thing the caller needs back out of it.
        let going: std::cell::RefCell<Option<(Url, bool)>> = std::cell::RefCell::new(None);
        let (response, bytes) = once(wire, &attempt, into, &|response| match next(
            response,
            attempt.url(),
        )? {
            Next::Body => {
                vet(response)?;
                Ok(Handling::Take)
            }
            Next::Follow {
                to,
                carrying_the_credential,
            } => {
                *going.borrow_mut() = Some((to, carrying_the_credential));
                Ok(Handling::Skip)
            }
        })?;
        let going = going.into_inner();
        match going {
            None => {
                return Ok(Exchanged {
                    response,
                    bytes,
                    served_by: attempt.url().clone(),
                    redirects,
                });
            }
            Some((to, carrying_the_credential)) => {
                attempt = attempt.redirected(to, carrying_the_credential);
            }
        }
    }

    Err(Failure::new(
        Category::HubUnreachable,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "a source redirected further than MCF will follow",
    )
    .with_context("ceiling", REDIRECT_CEILING.to_string())
    .with_context("visited", visited.join(" → ")))
}

/// One request, one answer.
///
/// Reads the head into a bounded buffer and then streams the body straight out,
/// so a file larger than this machine's memory is a file MCF can still fetch —
/// which is the ordinary case rather than the exotic one (§6.3).
fn once(
    wire: &dyn Wire,
    request: &Request,
    into: &mut dyn std::io::Write,
    decide: &dyn Fn(&Response) -> Result<Handling>,
) -> Result<(Response, u64)> {
    let url = request.url();
    let mut connection = wire.dial(url.host(), url.port())?;
    connection
        .write_all(&request.to_bytes())
        .and_then(|()| connection.flush())
        .map_err(|error| stalled("the request could not be sent", &error))?;

    let mut head = Vec::new();
    let mut buffer = [0_u8; 8192];
    let (response, consumed) = loop {
        match Response::read(&head) {
            Ok(read) => break read,
            Err(failure) if failure.category() != Category::TransferInterrupted => {
                return Err(failure);
            }
            Err(_) => {}
        }
        if head.len() > HEADER_CEILING {
            return Err(Failure::new(
                Category::HubMetadataMalformed,
                Attribution::Artifact,
                Disposition::Refused,
                WHERE,
                "a source sent more header than MCF will read",
            )
            .with_context("ceiling", HEADER_CEILING.to_string()));
        }
        let read = patiently(&mut connection, &mut buffer)
            .map_err(|error| stalled("the answer stopped arriving", &error))?;
        if read == 0 {
            // The connection closed before the headers ended. Whatever this
            // was, it was not an answer.
            return Err(Failure::new(
                Category::TransferInterrupted,
                Attribution::Machine,
                Disposition::Partial,
                WHERE,
                "the connection closed before the answer's headers ended",
            )
            .with_context("bytes_read", head.len().to_string()));
        }
        head.extend_from_slice(buffer.get(..read).unwrap_or_default());
    };

    // Nothing has been written yet, which is what lets a caller refuse an
    // answer whose head is enough to know it is wrong.
    if decide(&response)? == Handling::Skip {
        return Ok((response, 0));
    }

    // Whatever arrived after the head is the beginning of the body.
    let mut bytes = 0_u64;
    let leftover = head.get(consumed..).unwrap_or_default().to_vec();
    if !leftover.is_empty() {
        into.write_all(&leftover)
            .map_err(|error| unwritable(&error))?;
        bytes = bytes.saturating_add(leftover.len().try_into().unwrap_or(u64::MAX));
    }
    loop {
        let read = patiently(&mut connection, &mut buffer)
            .map_err(|error| stalled("the body stopped arriving", &error))?;
        if read == 0 {
            break;
        }
        into.write_all(buffer.get(..read).unwrap_or_default())
            .map_err(|error| unwritable(&error))?;
        bytes = bytes.saturating_add(read.try_into().unwrap_or(u64::MAX));
    }
    into.flush().map_err(|error| unwritable(&error))?;

    Ok((response, bytes))
}

/// One read, retrying the one error kind that means *ask again*.
///
/// `ErrorKind::Interrupted` is EINTR: a signal arrived while the thread was
/// blocked in the kernel, and the read did not happen. It is the one io error
/// whose contract is *retry* — `std` says so, and every other reader in the
/// standard library does exactly this.
///
/// **MCF was classifying it as `transfer.interrupted`**, which is a different
/// sentence entirely: *the far end stopped sending*. An operator on a busy
/// machine would have been told their download was cut off by something that
/// was not there. A2 forbids the wrong answer stated confidently as firmly as
/// it forbids silence, and a misclassification is exactly that.
///
/// Found while characterizing a rare divergence in the stall scenario under
/// load ([findings.md](../../../doc/findings.md) F61). It is not established
/// that this was the cause — the divergence was seen once and not reproduced
/// in three and a half thousand attempts — and it is a defect either way, by
/// reading the contract of `read` rather than by measurement.
fn patiently(
    connection: &mut impl std::io::Read,
    buffer: &mut [u8],
) -> std::result::Result<usize, std::io::Error> {
    loop {
        match connection.read(buffer) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

fn stalled(what: &str, error: &std::io::Error) -> Failure {
    let waiting = matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    );
    Failure::new(
        if waiting {
            Category::TransferStalled
        } else {
            Category::TransferInterrupted
        },
        Attribution::Machine,
        Disposition::Partial,
        WHERE,
        what.to_owned(),
    )
    .with_context("reason", error.to_string())
}

/// A write that did not happen, classified by why.
///
/// A full filesystem is its own outcome rather than a general write failure:
/// §3.11 makes disk exhaustion a decision, and an operator who is told *could
/// not write* when the answer is *there is no room* looks at permissions.
///
/// [findings.md](../../../doc/findings.md) F11 records where it surfaces: a
/// buffered write succeeds and the **flush** fails, so a fetcher that ignored
/// the flush would believe it had written the file.
fn unwritable(error: &std::io::Error) -> Failure {
    let full = error.kind() == std::io::ErrorKind::StorageFull;
    Failure::new(
        if full {
            Category::ResourceDiskExhausted
        } else {
            Category::ResourceDiskReadonly
        },
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        if full {
            "the filesystem filled while the transfer was being written"
        } else {
            "what arrived could not be written"
        },
    )
    .with_context("reason", error.to_string())
}

#[cfg(test)]
mod tests;

/// The transport a URL calls for.
///
/// One line of logic, in one place, because both surfaces that reach a hub
/// need it and a surface that chose differently would be a surface reaching
/// the network on terms nobody else's did.
///
/// # Errors
///
/// What building the TLS client said.
pub fn for_url(base: &crate::http::Url) -> Result<Box<dyn Wire>> {
    if base.scheme() == "https" {
        Ok(Box::new(Tls::new()?))
    } else {
        Ok(Box::new(Tcp::default()))
    }
}
