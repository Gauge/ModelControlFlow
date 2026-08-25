//! The durable record: the journal, and the index derived from it.
//!
//! §3.3 makes the record obligatory and not reducible, and D20 settles its
//! shape — trials and events are appended to a journal, and the queryable
//! SQLite database (D6) is built from that journal and may be discarded and
//! rebuilt (B62).
//!
//! This crate depends on `mcf-core` and on nothing else in the workspace, and
//! nothing in the workspace below it depends on this crate. The direction is
//! the point: A25 requires that the store which can be contributed contain no
//! user content *by construction*, and a structural guarantee survives a
//! misconfiguration in a way a filter does not.
//!
//! Empty at M0 beyond this statement of what it is for. B-004 brings the
//! append-only store, B-042 the schema, B-300 the journal-and-index rebuild,
//! and B-161 the type-level separation from the content store.
