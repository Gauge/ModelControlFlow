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

    /// Where content goes by default: beside the record, and not in it.
    ///
    /// `None` when there is nowhere to put it, for the same reason
    /// `journal::default_path` returns `None`: MCF does not invent a location
    /// to write the user's data into (A7).
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        crate::journal::default_path().map(|record| {
            record
                .parent()
                .map_or_else(|| PathBuf::from("content"), |dir| dir.join("content"))
        })
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
