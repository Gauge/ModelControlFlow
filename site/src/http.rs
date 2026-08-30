//! Just enough HTTP/1.1 to answer a browser, written rather than depended on.
//!
//! The workspace has no external dependencies and the reason is P3: every crate
//! is a condition of a measurement, and a dependency tree is a set of
//! conditions nobody restates. A server that answers `GET` and `POST` on a
//! loopback socket is a small enough thing to write, and writing it keeps the
//! lockfile the shape §3.12 asks for.
//!
//! Everything hostile is decided here rather than deeper in (§3.7): a request
//! longer than MCF will read, a header block that never ends, a method or a
//! path this build does not know. Nothing read from the socket reaches the
//! control plane without passing a match on a known route first.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// The longest request line and header block MCF will read.
///
/// Sixty-four kibibytes. A browser's request is a few hundred bytes; the limit
/// is here so that a client that never sends a blank line is disconnected
/// rather than read forever.
const HEADERS_LIMIT: usize = 64 << 10;

/// The longest body. Requests carry a reason for stopping and nothing larger.
const BODY_LIMIT: usize = 64 << 10;

/// What a client asked for, once it is known to be something MCF reads.
#[derive(Debug, Clone)]
pub struct Incoming {
    /// The method, uppercase as the client sent it.
    pub method: String,
    /// The path, exactly as sent — matched against a known set, never joined
    /// to a filesystem path.
    pub path: String,
    /// The body, empty where there was none.
    pub body: String,
}

/// Reads one request, or says why it will not.
///
/// # Errors
///
/// A string naming what was wrong, which the caller turns into a status. The
/// text is for MCF's own log, never echoed to the client: a reflected error is
/// a way to make a server repeat an attacker's bytes.
pub fn read(stream: &TcpStream) -> Result<Incoming, String> {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("the request could not be read: {error}"))?;
        if read == 0 {
            return Err("the client closed before sending a request".to_owned());
        }
        head.push_str(&line);
        if line == "\r\n" || line == "\n" {
            break;
        }
        if head.len() > HEADERS_LIMIT {
            return Err("the request head is longer than MCF will read".to_owned());
        }
    }

    let mut lines = head.lines();
    let start = lines.next().unwrap_or_default();
    let mut parts = start.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or_default().to_owned();
    if method.is_empty() || path.is_empty() {
        return Err("a request line that is not one".to_owned());
    }

    // Only the length is read from the headers. Nothing else in them changes
    // what MCF does, so nothing else is parsed.
    let length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if length > BODY_LIMIT {
        return Err("the body is longer than MCF will read".to_owned());
    }
    let mut body = vec![0_u8; length];
    if length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|error| format!("the body could not be read: {error}"))?;
    }

    Ok(Incoming {
        method,
        path,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

/// Writes one response. `kind` is a complete content type.
pub fn respond(mut stream: &TcpStream, status: u16, kind: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        502 => "Bad Gateway",
        _ => "Error",
    };
    // No caching, because every answer is a reading of a live daemon and a
    // stale one would be a number presented as current (A20's habit applied to
    // a screen). The security headers are the small ones that cost nothing: the
    // page loads no third-party anything, so a strict policy is free.
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {kind}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; \
         script-src 'unsafe-inline'; connect-src 'self'\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    let _written = stream.write_all(head.as_bytes()).and_then(|()| {
        if status == 204 {
            Ok(())
        } else {
            stream.write_all(body)
        }
    });
    let _flushed = stream.flush();
}
