//! There is no path from the content store to the record store.
//!
//! A25's check is `compiler`: *the content store and the record store are
//! distinct types with no path between them* (B-161). §6.8 gives the reason and
//! B9 gives the violation — one database with an `is_user_content` column and an
//! export query that excludes it. The guarantee has to be structural, because a
//! filter can be misconfigured and a store that never held the data cannot leak
//! it.
//!
//! The compiler holds it as long as neither module mentions the other's types.
//! That is what a type cannot check about itself, and the failure mode is a
//! convenience added later — a `fn record_prompt(&mut Journal, Content)` for one
//! call site, after which the guarantee is a filter again.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The record's modules never mention content.
#[test]
fn the_record_does_not_know_what_content_is() {
    for file in [
        "journal.rs",
        "encode.rs",
        "journal/entry.rs",
        "journal/replay.rs",
    ] {
        let source = code_only(&record_source(file));
        for forbidden in ["Content", "ContentStore", "content::"] {
            assert!(
                !source.contains(forbidden),
                "`{forbidden}` appears in {file}: the record has acquired a way to hold \
                 user content (A25, §6.8)"
            );
        }
    }
}

/// And the content store never mentions the record's types.
///
/// It may name `journal::default_path`, and that is deliberate: content lives
/// *beside* the record, so something has to know where the record is. What it
/// must not do is take or return anything the record is made of.
#[test]
fn the_content_store_does_not_know_what_a_record_is() {
    let source = code_only(&record_source("content.rs"));
    for forbidden in ["Entry", "EntryKind", "Journal", "Value", "encode::"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` appears in content.rs: content has acquired a way to reach \
             the record (A25, §6.8)"
        );
    }
}

/// Neither type converts into the other, in either direction.
///
/// The forbidden list names the *two vocabularies* rather than conversion in
/// general: `impl Into<String>` on a constructor is ordinary Rust and says
/// nothing about the boundary, while `From<Content> for Entry` is the boundary
/// dissolving.
#[test]
fn nothing_converts_content_into_a_record_or_back() {
    let content = code_only(&record_source("content.rs"));
    let journal = code_only(&record_source("journal.rs"));
    for (file, source) in [("content.rs", &content), ("journal.rs", &journal)] {
        for forbidden in [
            "From<Content",
            "From<Entry",
            "Into<Content",
            "Into<Entry",
            "as_entry",
            "as_content",
            "to_entry",
            "to_content",
        ] {
            assert!(
                !source.contains(forbidden),
                "`{forbidden}` in {file} would put a path between the two stores (A25)"
            );
        }
    }
}

/// The two stores are separate *places*, not one place with two names. An
/// export that walks the record's directory must not walk into content.
#[test]
fn the_two_stores_are_separate_places() {
    let source = code_only(&record_source("content.rs"));
    assert!(
        source.contains("pub fn default_path"),
        "the content store no longer says where it lives, so this check reads nothing"
    );
    assert!(
        source.contains("join(\"content\")"),
        "the content store's default location is no longer distinct from the record's"
    );
}

/// A file with its documentation removed.
///
/// The documentation names the forbidden constructs in order to say they are
/// absent, so a check that grepped the whole file would fail on the paragraph
/// explaining why it passes.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn record_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-record/src")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}
