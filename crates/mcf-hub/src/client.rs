use std::path::Path;

use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

use crate::credentials::{self, Credential, Identity};
use crate::http::{Request, Response, Url};
use crate::reference::Reference;
use crate::source::{Entry, Fetched, Lineage, Listing, Source};
use crate::wire::{self, Wire};

const WHERE: Subsystem = Subsystem::new("mcf-hub::client");

pub const METADATA_CEILING: u64 = 1024 * 1024;

pub struct Hub {
    base: Url,
    wire: Box<dyn Wire>,
    credential: Option<Credential>,
    stopping: crate::stopping::Stopping,
}

impl core::fmt::Debug for Hub {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Hub")
            .field("base", &self.base.to_string())
            .field("wire", &self.wire.describe())
            .field("authenticated", &self.credential.is_some())
            .field("stopping", &self.stopping)
            .finish()
    }
}

impl Hub {
    #[must_use]
    pub fn at(base: Url, wire: Box<dyn Wire>) -> Self {
        Self {
            base,
            wire,
            credential: None,
            stopping: crate::stopping::Stopping::never(),
        }
    }

    /// Let whoever asked for this transfer stop it part way through. Without this a
    /// transfer runs until the file is whole or the hub gives up.
    #[must_use]
    pub fn stopped_by(mut self, stopping: crate::stopping::Stopping) -> Self {
        self.stopping = stopping;
        self
    }

    #[must_use]
    pub fn offering(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    pub fn hugging_face(wire: Box<dyn Wire>) -> Result<Self> {
        Ok(Self::at(Url::parse("https://huggingface.co/")?, wire))
    }

    fn url(&self, target: &str) -> Result<Url> {
        self.base.resolve(target)
    }

    fn asking(&self, url: Url) -> Request {
        let request = Request::get(url);
        match &self.credential {
            Some(credential) => request.offering(credential.secret().reveal()),
            None => request,
        }
    }

    pub fn search(&self, query: &str) -> Result<Vec<Found>> {
        use core::fmt::Write as _;
        let mut encoded = String::new();
        for byte in query.trim().bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(char::from(byte));
                }
                other => {
                    let _written = write!(encoded, "%{other:02X}");
                }
            }
        }
        let url = self.url(&format!(
            "/api/models?search={encoded}&filter=gguf&sort=downloads&direction=-1&limit=20"
        ))?;
        let mut body = Bounded::new(METADATA_CEILING);
        let exchanged = wire::fetch(self.wire.as_ref(), &self.asking(url.clone()), &mut body)?;
        if !(200..=299).contains(&exchanged.response.status()) {
            return Err(Failure::new(
                Category::HubMetadataMalformed,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the hub did not answer the search",
            )
            .with_context("asked", url.to_string())
            .with_context("status", exchanged.response.status().to_string()));
        }
        let listed = read_json(&body.held)?;
        let Some(repositories) = listed.as_list() else {
            return Err(malformed("a list of repositories", "not a list"));
        };
        Ok(repositories
            .iter()
            .filter_map(|repository| {
                let count = |key: &str| {
                    repository
                        .get(key)
                        .and_then(Value::as_integer)
                        .and_then(|held| u64::try_from(held).ok())
                };
                Some(Found {
                    id: repository.get("id").and_then(Value::as_text)?.to_owned(),
                    downloads: count("downloads"),
                    likes: count("likes"),
                    updated: repository
                        .get("lastModified")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                })
            })
            .collect())
    }

    fn read_metadata(&self, reference: &Reference, target: &str) -> Result<(Response, Vec<u8>)> {
        let url = self.url(target)?;
        let mut body = Bounded::new(METADATA_CEILING);
        let exchanged = wire::fetch(self.wire.as_ref(), &self.asking(url.clone()), &mut body)?;
        if body.over {
            return Err(Failure::new(
                Category::HubMetadataMalformed,
                Attribution::Artifact,
                Disposition::Refused,
                WHERE,
                "a source answered a question about metadata with more than MCF will read",
            )
            .with_context("asked", url.to_string())
            .with_context("ceiling_bytes", METADATA_CEILING.to_string()));
        }
        self.status_is_an_answer(&exchanged.response, &url, reference)?;
        Ok((exchanged.response, body.held))
    }

    fn status_is_an_answer(
        &self,
        response: &Response,
        url: &Url,
        reference: &Reference,
    ) -> Result<()> {
        match response.status() {
            200..=299 => Ok(()),
            401 => Err(match &self.credential {
                None => credentials::missing(reference, &self.describe()),
                Some(_) => credentials::rejected(reference, &self.describe(), &self.identity()),
            }),
            403 => Err(credentials::gated(
                reference,
                &self.describe(),
                &self.identity(),
            )),
            404 => Err(Failure::new(
                Category::HubRefNotFound,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "the hub publishes nothing at that name",
            )
            .with_context("asked", url.to_string())),
            429 => {
                let hint = response.header("retry-after").unwrap_or("unstated");
                Err(Failure::new(
                    Category::HubRateLimited,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "this account is throttled",
                )
                .with_context("asked", url.to_string())
                .with_context("retry_after_seconds", hint.to_owned()))
            }
            other => Err(Failure::new(
                Category::HubUnreachable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the hub answered with a status MCF has no meaning for",
            )
            .with_context("asked", url.to_string())
            .with_context("status", other.to_string())),
        }
    }

    pub fn configuration(&self, listing: &Listing) -> Result<Option<Value>> {
        self.metadata_file(listing, "config.json")
    }

    pub fn metadata_file(&self, listing: &Listing, named: &str) -> Result<Option<Value>> {
        if listing.entry(named).is_none() {
            return Ok(None);
        }
        let revision = listing
            .revision
            .clone()
            .unwrap_or_else(|| "main".to_owned());
        let target = format!(
            "/{}/resolve/{revision}/{named}",
            listing.reference.repository()
        );
        match self.read_metadata(&listing.reference, &target) {
            Ok((_, body)) => read_json(&body).map(Some),
            Err(failure) if failure.category() == Category::HubRefNotFound => Ok(None),
            Err(failure) => Err(failure),
        }
    }

    fn resolve_url(&self, reference: &Reference, revision: &str, path: &str) -> Result<Url> {
        self.url(&format!(
            "/{}/resolve/{revision}/{path}",
            reference.repository()
        ))
    }

    pub fn prefix_of(&self, reference: &Reference, entry: &Entry, bytes: u64) -> Result<Vec<u8>> {
        let revision = reference
            .revision
            .clone()
            .unwrap_or_else(|| "main".to_owned());
        let url = self.resolve_url(reference, &revision, &entry.path)?;
        let request = self.asking(url.clone()).first(bytes);
        let mut held = Bounded::new(bytes);
        let _exchanged = wire::vetted(self.wire.as_ref(), &request, &mut held, &|response| {
            self.status_is_an_answer(response, &url, reference)
        })?;
        Ok(held.held)
    }

    fn transfer(
        &self,
        reference: &Reference,
        entry: &Entry,
        into: &Path,
        from: Option<u64>,
    ) -> Result<Fetched> {
        let revision = reference
            .revision
            .clone()
            .unwrap_or_else(|| "main".to_owned());
        let url = self.resolve_url(reference, &revision, &entry.path)?;
        let request = match from {
            Some(offset) => self.asking(url.clone()).resuming(offset),
            None => self.asking(url.clone()),
        };

        let file = match from {
            Some(_) => std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(into),
            None => std::fs::File::create(into),
        }
        .map_err(|error| {
            Failure::new(
                Category::ResourceDiskReadonly,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the destination could not be opened",
            )
            .with_context("path", into.display().to_string())
            .with_context("reason", error.to_string())
        })?;

        let mut sink = Digesting {
            into: std::io::BufWriter::new(file),
            digest: Sha256::default(),
            stopping: self.stopping.clone(),
        };
        let exchanged = wire::vetted(self.wire.as_ref(), &request, &mut sink, &|response| {
            self.status_is_an_answer(response, &url, reference)?;
            let Some(offset) = from else {
                return Ok(());
            };
            let range = response.content_range()?.ok_or_else(|| {
                Failure::new(
                    Category::HubUnreachable,
                    Attribution::Artifact,
                    Disposition::Refused,
                    WHERE,
                    "the hub answered a resumption with the whole file",
                )
                .with_context("asked_from", offset.to_string())
                .with_context("status", response.status().to_string())
            })?;
            if range.continues_from(offset) {
                Ok(())
            } else {
                Err(Failure::new(
                    Category::TransferMutated,
                    Attribution::Artifact,
                    Disposition::Refused,
                    WHERE,
                    "the hub continued a transfer from somewhere else than it was asked to",
                )
                .with_context("asked_from", offset.to_string())
                .with_context("answered_from", range.first.to_string()))
            }
        })?;

        Ok(Fetched {
            bytes: exchanged.bytes,
            digest: sink.digest.finish().hex(),
        })
    }
}

impl Source for Hub {
    fn describe(&self) -> String {
        format!("{} over {}", self.base, self.wire.describe())
    }

    fn identity(&self) -> Identity {
        match &self.credential {
            None => Identity::Anonymous,
            Some(credential) => Identity::Offered {
                fingerprint: credential.secret().fingerprint().to_owned(),
            },
        }
    }

    fn list(&self, reference: &Reference) -> Result<Listing> {
        let (_, card) = self.read_metadata(
            reference,
            &format!("/api/models/{}", reference.repository()),
        )?;
        let card = read_json(&card)?;

        let revision = reference
            .revision
            .clone()
            .or_else(|| card.get("sha").and_then(Value::as_text).map(str::to_owned));
        let asked_for = revision.clone().unwrap_or_else(|| "main".to_owned());

        let (_, tree) = self.read_metadata(
            reference,
            &format!(
                "/api/models/{}/tree/{asked_for}?recursive=true",
                reference.repository()
            ),
        )?;
        let entries = read_tree(&read_json(&tree)?)?;

        Ok(Listing {
            reference: Reference {
                revision: revision.clone(),
                ..reference.clone()
            },
            revision,
            entries,
            gated: card.get("gated").and_then(gate_of),
            declared_licence: declared_licence(&card),
            lineage: lineage(&card),
        })
    }

    fn fetch(&self, reference: &Reference, entry: &Entry, into: &Path) -> Result<Fetched> {
        self.transfer(reference, entry, into, None)
    }

    fn fetch_from(
        &self,
        reference: &Reference,
        entry: &Entry,
        from: u64,
        into: &Path,
    ) -> Result<Fetched> {
        self.transfer(reference, entry, into, Some(from))
    }
}

fn read_tree(tree: &Value) -> Result<Vec<Entry>> {
    let rows = tree
        .as_list()
        .ok_or_else(|| malformed("a tree is a list of files", &tree.to_line()))?;
    let mut entries = Vec::new();
    for row in rows {
        if row.get("type").and_then(Value::as_text) == Some("directory") {
            continue;
        }
        let path = row
            .get("path")
            .and_then(Value::as_text)
            .ok_or_else(|| malformed("every file in a tree has a path", &row.to_line()))?;
        let size = row
            .get("size")
            .and_then(Value::as_integer)
            .and_then(|size| u64::try_from(size).ok())
            .ok_or_else(|| malformed("every file in a tree has a size", &row.to_line()))?;
        let entry = Entry::new(path, size);
        entries.push(
            match row
                .get("lfs")
                .and_then(|lfs| lfs.get("oid"))
                .and_then(Value::as_text)
            {
                Some(digest) => entry.declaring(digest),
                None => entry,
            },
        );
    }
    Ok(entries)
}

fn gate_of(value: &Value) -> Option<String> {
    match value {
        Value::Bool(true) => Some("gated".to_owned()),
        Value::Text(how) => Some(how.clone()),
        Value::Bool(false)
        | Value::Null
        | Value::ForeignNumber(_)
        | Value::Integer(_)
        | Value::List(_)
        | Value::Map(_) => None,
    }
}

fn declared_licence(card: &Value) -> Option<String> {
    if let Some(stated) = card
        .get("cardData")
        .and_then(|data| data.get("license"))
        .and_then(Value::as_text)
    {
        return Some(stated.to_owned());
    }
    card.get("tags")
        .and_then(Value::as_list)
        .and_then(|tags| {
            tags.iter()
                .filter_map(Value::as_text)
                .find_map(|tag| tag.strip_prefix("license:"))
        })
        .map(str::to_owned)
}

fn lineage(card: &Value) -> Option<Lineage> {
    let tags: Vec<&str> = card
        .get("tags")
        .and_then(Value::as_list)?
        .iter()
        .filter_map(Value::as_text)
        .filter_map(|tag| tag.strip_prefix("base_model:"))
        .collect();

    let mut base = None;
    let mut relation = None;
    for tag in tags {
        match tag.split_once(':') {
            Some((what, from)) if from.contains('/') => {
                relation = Some(what.to_owned());
                base = Some(from.to_owned());
            }
            _ if tag.contains('/') => base = base.or_else(|| Some(tag.to_owned())),
            _ => {}
        }
    }
    base.map(|base| Lineage { base, relation })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub updated: Option<String>,
}

impl Found {
    #[must_use]
    pub fn to_value(&self) -> Value {
        let count = |held: Option<u64>| {
            held.map_or(Value::Null, |held| {
                Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
            })
        };
        Value::map([
            ("id", Value::text(self.id.clone())),
            ("downloads", count(self.downloads)),
            ("likes", count(self.likes)),
            (
                "updated",
                self.updated.clone().map_or(Value::Null, Value::text),
            ),
        ])
    }
}

fn read_json(bytes: &[u8]) -> Result<Value> {
    let text = core::str::from_utf8(bytes)
        .map_err(|_| malformed("an answer that is text", "not UTF-8"))?;
    json::parse(text).map_err(|error| malformed("an answer that is JSON", &error.to_string()))
}

fn malformed(wanted: &str, found: &str) -> Failure {
    let kept: String = found.chars().take(200).collect();
    Failure::new(
        Category::HubMetadataMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "the hub answered with something MCF cannot read",
    )
    .with_context("wanted", wanted.to_owned())
    .with_context("found", kept)
}

struct Bounded {
    held: Vec<u8>,
    ceiling: u64,
    over: bool,
}

impl Bounded {
    const fn new(ceiling: u64) -> Self {
        Self {
            held: Vec::new(),
            ceiling,
            over: false,
        }
    }
}

impl std::io::Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let held = u64::try_from(self.held.len()).unwrap_or(u64::MAX);
        let arriving = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if held.saturating_add(arriving) > self.ceiling {
            self.over = true;
        } else {
            self.held.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Digesting<W: std::io::Write> {
    into: W,
    digest: Sha256,
    stopping: crate::stopping::Stopping,
}

/// What a transfer that was asked to stop says to the wire underneath it. The bytes
/// already written stay on disk; this only ends the writing.
pub const STOPPED: &str = "the transfer was asked to stop";

impl<W: std::io::Write> std::io::Write for Digesting<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.stopping.asked() {
            self.into.flush()?;
            return Err(std::io::Error::other(STOPPED));
        }
        let written = self.into.write(bytes)?;
        self.digest.update(bytes.get(..written).unwrap_or_default());
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.into.flush()
    }
}

#[cfg(test)]
mod tests;
