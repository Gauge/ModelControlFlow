use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-hub::http");

pub const HEADER_CEILING: usize = 64 * 1024;

pub const REDIRECT_CEILING: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    scheme: String,
    host: String,
    port: u16,
    target: String,
}

impl Url {
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

    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    #[must_use]
    pub fn same_origin(&self, other: &Self) -> bool {
        self.scheme == other.scheme && self.host == other.host && self.port == other.port
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    url: Url,
    from: Option<u64>,
    to: Option<u64>,
    credential: Option<String>,
}

impl Request {
    #[must_use]
    pub fn get(url: Url) -> Self {
        Self {
            url,
            from: None,
            to: None,
            credential: None,
        }
    }

    #[must_use]
    pub fn resuming(mut self, offset: u64) -> Self {
        self.from = Some(offset);
        self
    }

    #[must_use]
    pub fn first(mut self, bytes: u64) -> Self {
        self.from = Some(0);
        self.to = Some(bytes.saturating_sub(1));
        self
    }

    #[must_use]
    pub fn offering(mut self, token: &str) -> Self {
        self.credential = Some(token.to_owned());
        self
    }

    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    #[must_use]
    pub fn redirected(&self, to: Url, carrying_the_credential: bool) -> Self {
        Self {
            url: to,
            from: self.from,
            to: self.to,
            credential: if carrying_the_credential {
                self.credential.clone()
            } else {
                None
            },
        }
    }

    #[must_use]
    pub const fn is_authenticated(&self) -> bool {
        self.credential.is_some()
    }

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

#[must_use]
pub fn user_agent() -> String {
    format!("mcf/{}", env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
}

impl Response {
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

    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }

    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub fn content_length(&self) -> Result<Option<u64>> {
        match self.header("content-length") {
            None => Ok(None),
            Some(value) => value
                .parse::<u64>()
                .map(Some)
                .map_err(|_| malformed("a content-length is a number", value)),
        }
    }

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
        if total.is_some_and(|total| last >= total) {
            return Err(malformed(
                "a range that ends inside the file it names",
                value,
            ));
        }
        Ok(Some(ContentRange { first, last, total }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub first: u64,
    pub last: u64,
    pub total: Option<u64>,
}

impl ContentRange {
    #[must_use]
    pub const fn continues_from(&self, offset: u64) -> bool {
        self.first == offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    Body,
    Follow {
        to: Url,
        carrying_the_credential: bool,
    },
}

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

pub fn redirect(response: &Response, from: &Url) -> Result<Option<(Url, bool)>> {
    match next(response, from)? {
        Next::Body => Ok(None),
        Next::Follow {
            to,
            carrying_the_credential,
        } => Ok(Some((to, carrying_the_credential))),
    }
}

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

fn elide(found: &str) -> String {
    if found.chars().count() <= 200 {
        return found.to_owned();
    }
    let kept: String = found.chars().take(200).collect();
    format!("{kept}… ({} characters in all)", found.chars().count())
}

#[cfg(test)]
mod tests;
