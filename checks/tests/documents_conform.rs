#![allow(clippy::panic)]

use mcf_checks::document::{Document, all};

fn documents() -> Vec<Document> {
    all().unwrap_or_else(|path| panic!("{} is readable", path.display()))
}

#[test]
fn the_documents_are_found() {
    let documents = documents();
    assert!(
        documents.len() >= 4,
        "only {} documents were found",
        documents.len()
    );
    for expected in ["README.md", "doc/goals.md", "doc/build.md"] {
        assert!(
            documents.iter().any(|d| d.relative_path == expected),
            "{expected} was not found"
        );
    }
}

#[test]
fn every_document_opens_with_a_title() {
    for document in documents() {
        assert!(
            document.title().is_some(),
            "{} does not open with a `# Title`",
            document.relative_path
        );
    }
}

#[test]
fn every_relative_link_resolves() {
    let root = mcf_checks::workspace::root();
    let mut broken = Vec::new();
    for document in documents() {
        let directory = document
            .path
            .parent()
            .unwrap_or(root.as_path())
            .to_path_buf();
        for (line, target) in document.relative_links() {
            let without_anchor = target.split('#').next().unwrap_or(&target);
            if without_anchor.is_empty() {
                continue;
            }
            if !directory.join(without_anchor).exists() {
                broken.push(format!("{}:{line} → {target}", document.relative_path));
            }
        }
    }
    assert!(broken.is_empty(), "broken relative links: {broken:#?}");
}
