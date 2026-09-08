#![allow(clippy::panic)]

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
                !names(&source, forbidden),
                "`{forbidden}` appears in {file}: the record has acquired a way to hold \
                 user content (A25, §6.8)"
            );
        }
    }
}

fn names(source: &str, wanted: &str) -> bool {
    let mut rest = source;
    while let Some(at) = rest.find(wanted) {
        let after = rest
            .get(at.saturating_add(wanted.len())..)
            .and_then(|held| held.chars().next());
        if !after.is_some_and(|held| held.is_ascii_lowercase()) {
            return true;
        }
        let Some(next) = rest.get(at.saturating_add(wanted.len())..) else {
            return false;
        };
        rest = next;
    }
    false
}

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

#[test]
fn the_check_tells_a_type_from_a_prefix() {
    assert!(names("pub struct Content {", "Content"));
    assert!(names("Content,", "Content"));
    assert!(names("ContentStore", "ContentStore"));
    assert!(
        names("fn holds(held: Content) {", "Content"),
        "the type in a signature is still the type"
    );
    assert!(
        !names("ContentionSnapshot,", "Content"),
        "a longer word that begins with the letters is not the name"
    );
    assert!(
        !names("contention_snapshot", "content::"),
        "nor is a field whose name merely starts alike"
    );
}
