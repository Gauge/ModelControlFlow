//! Prompt and completion content, in a store that is not the record.
//!
//! A25 is absolute and its check is `compiler`: *a contributable record
//! contains no user content, by construction.* §6.8 is the reasoning — the
//! guarantee has to be **structural**, because a filter can be misconfigured
//! and a store that never held the data cannot leak it. B9's violation is one
//! database with an `is_user_content` column and an export query that excludes
//! it.
//!
//! So there are two stores and no path between them. [`ContentStore`] takes
//! [`Content`] and nothing else; `journal::Journal` takes `Entry` and nothing
//! else; neither type converts to the other, and B-161's check reads both
//! modules to say so.
//!
//! **The store had no way to hold anything for its first three months** (F105).
//! It could be opened and asked where it was, and that is all — so nothing was
//! ever put in it, and the model's completions went into the *record* as
//! ordinary strings, where a type that never appeared could not stop them. A25
//! is absolute and it was not held: 4 019 completions and 29 prompts were in
//! the record of the machine this was found on, and `mcf export` printed *no
//! prompt or completion content, by construction* over the file it had just
//! written them into. The guarantee was structural and unused, which is the
//! most expensive kind: it reads like a defence in every review.
//!
//! **Keyed by the record's own identifier, and the pointer only goes one way.**
//! Content is filed under the entry it belongs to, as an opaque string — this
//! module cannot name an `EntryId` without acquiring a path to the record, so it
//! takes a `&str` and validates its shape. Nothing in the record points here:
//! an entry says how many bytes were said and never where they are, so a reader
//! who has the record has no route to the content, which is what makes an
//! export that walks the record unable to reach it.
//!
//! **What is content and what is not** (B9). Metrics, timings, resource states,
//! configurations and error conditions are *system* record and are recorded in
//! full. What a user typed and what a model generated are content. Benchmark
//! suite content is fixture data rather than user data and may be kept in full
//! — which is exactly why the two must be separated structurally rather than by
//! a flag, since a flag would have to be right about which is which every time.
//!
//! **What this module does not decide.** How long content is kept, and what
//! happens when its budget is exhausted, is §7.5 and DEC-005. What it does
//! decide is that the question can be answered independently for the two
//! stores, which is what having two of them is for.

use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-record::content");

/// Something a user wrote or a model generated.
///
/// A newtype rather than a `String`, so that content cannot be passed where a
/// record field is expected by having the same type. The `Debug` implementation
/// deliberately does not print it: a debug log is a surface, and A17 holds that
/// nothing leaves the machine unchosen — including into a terminal somebody is
/// screen-sharing.
#[derive(Clone, PartialEq, Eq)]
pub struct Content {
    text: String,
}

impl Content {
    /// Holds some content.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// The content itself.
    ///
    /// Named `disclose` rather than `text` or `as_str` because that is what
    /// calling it does. A25's guarantee is structural, and the one place the
    /// structure cannot help is a caller that has decided to look — so the call
    /// site says what it is doing.
    #[must_use]
    pub fn disclose(&self) -> &str {
        &self.text
    }

    /// How much of it there is.
    ///
    /// Length is *not* content: it is a measurement about content, it belongs
    /// in the system record, and it is what lets the record say "a 4 096-byte
    /// prompt" without holding one.
    #[must_use]
    pub fn length_bytes(&self) -> usize {
        self.text.len()
    }
}

impl core::fmt::Debug for Content {
    /// Says how much there is and never what it says.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Content({} bytes, undisclosed)", self.text.len())
    }
}

/// Where content lives, which is not where the record lives.
///
/// A separate file in a separate place, opened by a separate type. The
/// separation is the feature: an export that walks the record cannot reach this
/// store, because nothing in the record points at it and no function here takes
/// or returns a record type.
#[derive(Debug)]
pub struct ContentStore {
    path: PathBuf,
}

impl ContentStore {
    /// Opens the content store at a path.
    ///
    /// # Errors
    ///
    /// `record.unwritable` when the directory cannot be created.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Failure::new(
                    Category::RecordUnwritable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "the content store could not be created",
                )
                .with_context("path", path.display().to_string())
                .with_context("os_error", error.to_string())
            })?;
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Where it lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Files one piece of content under the record entry it belongs to.
    ///
    /// **The key is the entry's identifier and this module cannot say so in a
    /// type.** Taking an `EntryId` would mean naming one of the record's types,
    /// which is the path B-161 exists to prevent, so the key arrives as a `&str`
    /// and its shape is checked here instead.
    ///
    /// One file per key, holding the bytes and nothing else: no format, no
    /// escaping, no parser, and forgetting one piece of content is removing one
    /// file — which is what §7.5's retention question will need when it is
    /// answered (DEC-005).
    ///
    /// # Errors
    ///
    /// `internal.invariant_violated` when the key is not one this store will
    /// make a filename of; `record.unwritable` when the write fails.
    pub fn keep(&self, key: &str, content: &Content) -> Result<()> {
        let path = self.file(key)?;
        std::fs::create_dir_all(&self.path).map_err(|error| Self::unwritable(&path, &error))?;
        std::fs::write(&path, content.text.as_bytes())
            .map_err(|error| Self::unwritable(&path, &error))
    }

    /// Hands back content that was filed, if any was.
    ///
    /// Named for what calling it does, exactly as [`Content::disclose`] is: a
    /// caller that reads content has decided to, and the call site says so.
    ///
    /// `Ok(None)` is the ordinary answer for an entry whose content was never
    /// kept or has been forgotten — which is a state rather than a failure
    /// (A7), and is what a reader meets for every entry written before this
    /// store could hold anything.
    ///
    /// # Errors
    ///
    /// `internal.invariant_violated` for a key this store would not have
    /// written; `record.unreadable` when the file is there and will not be
    /// read, because *there and unreadable* is not the same answer as *absent*.
    pub fn disclose_kept(&self, key: &str) -> Result<Option<Content>> {
        let path = self.file(key)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(Content::new(String::from_utf8_lossy(&bytes)))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Failure::new(
                Category::RecordContentUnreadable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "content is filed here and would not be read",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string())),
        }
    }

    /// Where one key's content lives, refusing a key that is not a name.
    ///
    /// A key with a separator or a parent component in it would write outside
    /// the store, which is the traversal §3.7 is about pointed inward. The
    /// permitted shape is what the record's own identifiers are made of.
    fn file(&self, key: &str) -> Result<PathBuf> {
        let usable = !key.is_empty()
            && key.len() <= 160
            && key
                .chars()
                .all(|held| held.is_ascii_alphanumeric() || matches!(held, '-' | '_' | '.'))
            && !key.starts_with('.');
        if !usable {
            return Err(Failure::new(
                Category::InternalInvariantViolated,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "a content key must be a plain name: letters, digits, dash, underscore and dot",
            )
            .with_context("key_bytes", key.len().to_string()));
        }
        Ok(self.path.join(key))
    }

    /// The failure for a write that would not happen.
    fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "content could not be written to the content store",
        )
        .with_context("path", path.display().to_string())
        .with_context("os_error", error.to_string())
    }

    /// Where content goes by default: beside the record, and not in it.
    ///
    /// `None` when there is nowhere to put it, for the same reason
    /// `journal::default_path` returns `None`: MCF does not invent a location
    /// to write the user's data into (A7).
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        crate::journal::default_path().map(|record| Self::beside(&record))
    }

    /// Where content lives beside a record at a stated path.
    ///
    /// The *stated* path, because a daemon is told where its record is rather
    /// than looking one up, and a process that asked the environment where to
    /// put content while writing its record somewhere else would put the two
    /// halves of one machine's history in two places (F46's shape).
    #[must_use]
    pub fn beside(record: &Path) -> PathBuf {
        record
            .parent()
            .map_or_else(|| PathBuf::from("content"), |dir| dir.join("content"))
    }
}

#[cfg(test)]
mod tests {
    use super::{Content, ContentStore};

    /// A25's guarantee is structural, and the structure is that these are
    /// different types with no conversion. The compiler enforces it; what is
    /// recorded here is that content does not render itself by accident.
    #[test]
    fn content_does_not_disclose_itself_in_a_debug_rendering() {
        let secret = Content::new("the operator's private prompt");
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains("private"), "{rendered}");
        assert!(rendered.contains("undisclosed"), "{rendered}");
        assert!(rendered.contains("29 bytes"), "{rendered}");
    }

    /// Disclosure is a call somebody wrote. The method is named for what it
    /// does so that the call site says it.
    #[test]
    fn disclosure_returns_what_was_held() {
        let content = Content::new("hello");
        assert_eq!(content.disclose(), "hello");
        assert_eq!(content.length_bytes(), 5);
    }

    /// Length is a measurement *about* content and belongs in the system
    /// record; it is what lets a record say "a 4096-byte prompt" without
    /// holding one.
    #[test]
    fn length_is_available_without_disclosure() {
        let content = Content::new("a".repeat(4096));
        assert_eq!(content.length_bytes(), 4096);
    }

    /// The two stores are in different places, so an export that walks the
    /// record's directory does not walk into content by accident.
    #[test]
    fn the_content_store_is_not_the_record() {
        let (Some(record), Some(content)) =
            (crate::journal::default_path(), ContentStore::default_path())
        else {
            return;
        };
        assert_ne!(record, content);
        assert!(!content.ends_with("record.jsonl"));
    }

    /// A7: MCF does not invent a place to write the user's content.
    #[test]
    fn a_store_can_be_opened_where_it_is_told_to_be() {
        let path = std::env::temp_dir().join(format!("mcf-content-{}", std::process::id()));
        let store = ContentStore::open(&path.join("content")).expect("a temporary path opens");
        assert!(store.path().ends_with("content"));
        let _removed = std::fs::remove_dir_all(&path);
    }
}
