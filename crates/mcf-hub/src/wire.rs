use std::io::Write as _;
use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::http::{HEADER_CEILING, Next, REDIRECT_CEILING, Request, Response, Url, next};

const WHERE: Subsystem = Subsystem::new("mcf-hub::wire");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadlines {
    pub connect: Duration,
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

pub trait Duplex: std::io::Read + std::io::Write {}

impl<T: std::io::Read + std::io::Write> Duplex for T {}

pub trait Wire {
    fn describe(&self) -> String;

    fn carries_secrets(&self) -> bool;

    fn dial(&self, host: &str, port: u16) -> Result<Box<dyn Duplex>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tcp {
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
                    "a connection that cannot be given a deadline is one that can hang",
                )
                .with_context("reason", error.to_string())
            })?;
        }
        Ok(Box::new(stream))
    }
}

pub struct Tls {
    pub deadlines: Deadlines,
    configuration: std::sync::Arc<rustls::ClientConfig>,
}

impl core::fmt::Debug for Tls {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tls")
            .field("deadlines", &self.deadlines)
            .field("roots", &webpki_roots::TLS_SERVER_ROOTS.len())
            .field("configuration", &"<the vendored provider's>")
            .finish()
    }
}

impl Tls {
    pub fn new() -> Result<Self> {
        Self::with(Deadlines::default())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchanged {
    pub response: Response,
    pub bytes: u64,
    pub served_by: Url,
    pub redirects: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handling {
    Take,
    Skip,
}

pub fn fetch(
    wire: &dyn Wire,
    request: &Request,
    into: &mut dyn std::io::Write,
) -> Result<Exchanged> {
    vetted(wire, request, into, &|_| Ok(()))
}

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

    if decide(&response)? == Handling::Skip {
        return Ok((response, 0));
    }

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

pub fn for_url(base: &crate::http::Url) -> Result<Box<dyn Wire>> {
    if base.scheme() == "https" {
        Ok(Box::new(Tls::new()?))
    } else {
        Ok(Box::new(Tcp::default()))
    }
}
