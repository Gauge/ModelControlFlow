//! What a client may ask a running MCF, and what it may not (B-030, B-036).
//!
//! **A protocol before a daemon, for the reason the wire came before TLS.**
//! This module turns a request into bytes and bytes into a request and touches
//! no socket, so what a client and a daemon agree on can be exercised without
//! either. Everything hostile about a control plane — a request that is not
//! one, a request larger than anything MCF will read, a version this build does
//! not know — is decided here (§3.7).
//!
//! **It is deliberately tiny, and the reason is A19 rather than taste.** Three
//! requests: *what are you*, *what are you holding*, and *stop*. Serving a
//! model is not among them because MCF cannot yet run one, and a control plane
//! that advertised what it could not do would be the fabricated report C7 is
//! written against. B-040 adds to this list when there is an engine to add for.
//!
//! **One line per message.** The record is line-delimited JSON (D20) and so is
//! this, for the same reason: a torn message is a torn *line*, which a reader
//! can identify and report rather than being unable to find the boundary at
//! all. It also means a person can drive the control plane with `socat` and
//! read what came back, which A22 asks for — a surface only a client can reach
//! is a surface the laboratory cannot test.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-serve::control");

/// The protocol this build speaks.
///
/// Sent in every request and every answer. §7.30's habit applied to a live
/// connection rather than to a record: a client from another version is told so
/// rather than being half-understood, and *half-understood* is the failure mode
/// a version number exists to prevent (C5).
pub const VERSION: i64 = 1;

/// How long a request may be.
///
/// Sixty-four kibibytes. A control request is a verb and a name; anything
/// larger is a client that is broken or trying something, and §3.7 makes the
/// bound a stated number rather than *whatever arrives*.
pub const REQUEST_CEILING: usize = 64 * 1024;

/// What a client is asking for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// What this daemon is: its build, how long it has been up, what it will
    /// and will not do.
    Status,
    /// What models this machine is holding.
    Holding,
    /// Stop: refuse new work, finish what is in hand, and exit.
    Stop {
        /// Why, which is recorded. A26 makes stopping an act with an account
        /// rather than a signal that leaves no trace.
        reason: String,
    },
}

impl Request {
    /// The line a client sends.
    #[must_use]
    pub fn to_line(&self) -> String {
        let body = match self {
            Self::Status => Value::map([("ask", Value::text("status"))]),
            Self::Holding => Value::map([("ask", Value::text("holding"))]),
            Self::Stop { reason } => Value::map([
                ("ask", Value::text("stop")),
                ("reason", Value::text(reason.clone())),
            ]),
        };
        let Value::Map(mut fields) = body else {
            return String::new();
        };
        fields.insert("protocol".to_owned(), Value::Integer(VERSION));
        Value::Map(fields).to_line()
    }

    /// Reads what a client sent.
    ///
    /// # Errors
    ///
    /// `config.invalid` for a line that is not a request, naming what was seen
    /// but bounded — a client is untrusted, and a refusal that quoted a
    /// megabyte back would be a way of writing to MCF's own output (§3.7, A1).
    /// `exchange.schema.unreadable` for a protocol version this build does not
    /// speak, which is a different thing from a request it does not have.
    pub fn read(line: &str) -> Result<Self> {
        if line.len() > REQUEST_CEILING {
            return Err(refused(
                "a request larger than MCF will read",
                &format!("{} bytes", line.len()),
            ));
        }
        let value = json::parse(line)
            .map_err(|error| refused("a request that is not a request", &error.to_string()))?;

        match value.get("protocol").and_then(Value::as_integer) {
            Some(VERSION) => {}
            Some(other) => {
                return Err(Failure::new(
                    Category::ExchangeSchemaUnreadable,
                    Attribution::User,
                    Disposition::Refused,
                    WHERE,
                    "a client speaking a protocol version this build does not",
                )
                .with_context("client_speaks", other.to_string())
                .with_context("this_build_speaks", VERSION.to_string()));
            }
            None => return Err(refused("a request naming no protocol version", line)),
        }

        match value.get("ask").and_then(Value::as_text) {
            Some("status") => Ok(Self::Status),
            Some("holding") => Ok(Self::Holding),
            Some("stop") => Ok(Self::Stop {
                reason: value
                    .get("reason")
                    .and_then(Value::as_text)
                    .unwrap_or_default()
                    .to_owned(),
            }),
            Some(other) => Err(refused("a request MCF does not have", other)),
            None => Err(refused("a request asking for nothing", line)),
        }
    }
}

/// What the daemon says back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// Whether MCF could serve the request.
    pub served: bool,
    /// The answer itself, or what went wrong.
    pub body: Value,
}

impl Answer {
    /// An answer MCF could give.
    #[must_use]
    pub fn served(body: Value) -> Self {
        Self { served: true, body }
    }

    /// A classified refusal, in the record's own shape so that a client reads
    /// the same structure a record holds (A2, C1).
    #[must_use]
    pub fn refused(failure: &Failure) -> Self {
        Self {
            served: false,
            body: mcf_record::encode::failure(failure),
        }
    }

    /// The line the daemon sends.
    #[must_use]
    pub fn to_line(&self) -> String {
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("served", Value::Bool(self.served)),
            ("answer", self.body.clone()),
        ])
        .to_line()
    }

    /// Reads what a daemon sent.
    ///
    /// # Errors
    ///
    /// `config.invalid` for a line that is not an answer.
    pub fn read(line: &str) -> Result<Self> {
        let value = json::parse(line)
            .map_err(|error| refused("an answer that is not one", &error.to_string()))?;
        let served = match value.get("served") {
            Some(Value::Bool(served)) => *served,
            _ => {
                return Err(refused(
                    "an answer that does not say whether it is one",
                    line,
                ));
            }
        };
        let body = value
            .get("answer")
            .cloned()
            .ok_or_else(|| refused("an answer with nothing in it", line))?;
        Ok(Self { served, body })
    }
}

/// What a client said, kept but bounded.
fn refused(wanted: &str, found: &str) -> Failure {
    let kept: String = found.chars().take(200).collect();
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "a client sent something MCF cannot read",
    )
    .with_context("wanted", wanted.to_owned())
    .with_context("found", kept)
}

#[cfg(test)]
mod tests;
