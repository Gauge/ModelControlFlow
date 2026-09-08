use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

const HEADERS_LIMIT: usize = 64 << 10;

const BODY_LIMIT: usize = 64 << 10;

#[derive(Debug, Clone)]
pub struct Incoming {
    pub method: String,
    pub path: String,
    pub body: String,
}

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
