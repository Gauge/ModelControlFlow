//! A source that is a hub on the other end of a wire (B-021, B-322).
//!
//! **What it is.** [`crate::source::Source`] asks a hub four questions; this
//! answers them by asking a real one, over [`crate::wire`], in the shapes
//! [findings.md](../../../doc/findings.md) F9 measured. Everything the
//! laboratory's simulated hub has been standing in for since B-028 arrives
//! here.
//!
//! **It is pointed at a base rather than at Hugging Face.** The host is a field
//! and not a constant, which is what lets the whole path — listing, redirect,
//! resume, digest — run against a server this repository's own tests are
//! holding, on the loopback address, in the gating tier (B19). It is also the
//! honest shape for a mirror: an operator on a network that cannot reach the
//! hub has somewhere to point MCF, and *which host served these bytes* is
//! recorded either way (§3.4).
//!
//! **What the hub is asked, and what it says.** Two calls before a byte of
//! weights moves, both of them cheap:
//!
//! | Asked | Answers |
//! |---|---|
//! | `/api/models/{repository}` | the revision to pin (`sha`), whether the repository is gated, and the licence it declares |
//! | `/api/models/{repository}/tree/{revision}?recursive=true` | every file, its size, and — for anything stored in LFS — the SHA-256 the hub declares for it |
//!
//! That second answer is what makes B-213's arithmetic possible before
//! anything is downloaded, and what makes B-021's verification possible at all:
//! a digest that arrives *with the listing* is a digest MCF can check the bytes
//! against, rather than one the same connection could have made up to match
//! what it sent.
//!
//! **A status is an outcome, never an exception.** 401, 403, 404 and 429 each
//! become the failure `mcf_hub::credentials` and its neighbours already write,
//! so the words an operator reads are the same whether the refusal came from a
//! simulated hub or a real one (A6's habit, B-024).

use std::path::Path;

use mcf_core::digest::Sha256;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

use crate::credentials::{self, Credential, Identity};
use crate::http::{Request, Response, Url};
use crate::reference::Reference;
use crate::source::{Entry, Fetched, Listing, Source};
use crate::wire::{self, Wire};

const WHERE: Subsystem = Subsystem::new("mcf-hub::client");

/// How much of an answer to metadata MCF will read.
///
/// A repository's file list is kilobytes; a megabyte is room for the largest
/// repository anybody publishes and a bound on a source that would otherwise
/// answer for ever (§3.7, B7). The weights are not read into memory at all —
/// they go straight to the disk — so this bounds metadata alone.
pub const METADATA_CEILING: u64 = 1024 * 1024;

/// A hub on the other end of a wire.
pub struct Hub {
    base: Url,
    wire: Box<dyn Wire>,
    credential: Option<Credential>,
}

impl core::fmt::Debug for Hub {
    /// Names the hub and the wire, never the credential.
    ///
    /// `Credential`'s own `Debug` redacts (B-024); this one does not print it at
    /// all, because *which token* is a question a record answers through
    /// [`Source::identity`] rather than through a struct dump.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Hub")
            .field("base", &self.base.to_string())
            .field("wire", &self.wire.describe())
            .field("authenticated", &self.credential.is_some())
            .finish()
    }
}

impl Hub {
    /// A hub at this base, reached over this wire.
    #[must_use]
    pub fn at(base: Url, wire: Box<dyn Wire>) -> Self {
        Self {
            base,
            wire,
            credential: None,
        }
    }

    /// The same, offering a credential.
    ///
    /// Held rather than sent: [`crate::wire::fetch`] refuses to carry it over a
    /// connection that cannot keep it, and a redirect to another host leaves it
    /// behind (B-024).
    #[must_use]
    pub fn offering(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Where MCF looks for a model hub by default.
    ///
    /// # Errors
    ///
    /// `hub.metadata.malformed` if this constant ever stops being a URL, which
    /// is a thing a test asserts rather than a thing that happens.
    pub fn hugging_face(wire: Box<dyn Wire>) -> Result<Self> {
        Ok(Self::at(Url::parse("https://huggingface.co/")?, wire))
    }

    /// The URL for something under this hub's base.
    fn url(&self, target: &str) -> Result<Url> {
        self.base.resolve(target)
    }

    /// Makes a request this hub's credential is attached to.
    fn asking(&self, url: Url) -> Request {
        let request = Request::get(url);
        match &self.credential {
            // The one place a secret leaves its box on this path. `wire::fetch`
            // decides whether it may travel at all.
            Some(credential) => request.offering(credential.secret().reveal()),
            None => request,
        }
    }

    /// Reads something small — a listing, a card — into memory.
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

    /// Turns a status that is not an answer into the failure it means.
    ///
    /// The words are `mcf_hub::credentials`' own, so an operator reads the same
    /// sentence whether the refusal came from a real hub or a simulated one.
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

    /// The model's own configuration, where the repository publishes one.
    ///
    /// `config.json` is small, so this is a third cheap question rather than a
    /// download: it is what [`crate::fitment`] needs to say whether a variant
    /// will run here *before* twenty gigabytes are fetched (B-213, PR3).
    ///
    /// `Ok(None)` when the repository publishes none — a GGUF-only repository
    /// often does — which is a state to report rather than a shape to guess
    /// (A7).
    ///
    /// # Errors
    ///
    /// As [`Self::list`], for anything that is not a plain absence.
    pub fn configuration(&self, listing: &Listing) -> Result<Option<Value>> {
        if listing.entry("config.json").is_none() {
            return Ok(None);
        }
        let revision = listing
            .revision
            .clone()
            .unwrap_or_else(|| "main".to_owned());
        let target = format!(
            "/{}/resolve/{revision}/config.json",
            listing.reference.repository()
        );
        match self.read_metadata(&listing.reference, &target) {
            Ok((_, body)) => read_json(&body).map(Some),
            Err(failure) if failure.category() == Category::HubRefNotFound => Ok(None),
            Err(failure) => Err(failure),
        }
    }

    /// Where a file lives, at a stated revision.
    fn resolve_url(&self, reference: &Reference, revision: &str, path: &str) -> Result<Url> {
        self.url(&format!(
            "/{}/resolve/{revision}/{path}",
            reference.repository()
        ))
    }

    /// Fetches a file, appending or truncating, and digests what arrives.
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
        };
        // Everything that could make this answer the wrong one is decided here,
        // before a byte of it is written: a status that is not an answer, and a
        // source that continues a transfer from somewhere other than where it
        // was asked to. Appending first and refusing afterwards would leave a
        // file that is its own first part twice — for a moment if the refusal
        // is handled, and for good if the process dies in between (B-021, A1).
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
        // *Offered*, never *confirmed*: MCF has not asked the hub who this is,
        // and naming an account it was never told would be inventing the one
        // thing identity establishes. `/api/whoami-v2` is where a confirmation
        // would come from, and asking for one is a decision about what MCF
        // sends rather than a detail (§XIV).
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

        // The revision the hub says this is, rather than the branch that was
        // asked for: "main" is not a thing anybody can pin (A7, B-019).
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
            declared_licence: declared_licence(&card),
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

/// The files a repository publishes, from the hub's own tree.
///
/// The digest is the LFS object's, which is the SHA-256 of the file. A file
/// that is not in LFS — a config, a card — has no declared digest here, and
/// that is recorded as absent rather than filled in with the git blob hash,
/// which is a hash of something else (A7, A21).
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

/// What the repository says its terms are.
///
/// From the card's own field where there is one, and from the `license:` tag
/// otherwise — the hub publishes both and a repository sometimes has only the
/// second. `None` when neither is there, which `inspect::terms_are_legible`
/// turns into `hub.metadata.absent` (B-023).
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

/// A sink that keeps what it is given, up to a stated ceiling.
///
/// Past the ceiling it keeps counting and stops keeping, so a source that
/// answers a small question with a large answer is *noticed* rather than
/// allowed to fill this machine (§3.7).
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

/// A sink that digests what passes through it.
///
/// The digest is of what actually arrived on this disk, computed here rather
/// than taken from the source: a checksum a hostile source supplies is a
/// checksum of what it wishes it had sent (§3.7, B-021).
struct Digesting<W: std::io::Write> {
    into: W,
    digest: Sha256,
}

impl<W: std::io::Write> std::io::Write for Digesting<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
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
