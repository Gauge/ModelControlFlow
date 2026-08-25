//! Every document in `doc/` holds to the format contract.
//!
//! B-041. The contract is in the repository README; this is the command that
//! fails when a document leaves it. It exists because this project has already
//! caught three of its own documents disagreeing with themselves — a front
//! matter two versions behind its own changelog, header counts three revisions
//! stale, and a register entry still reporting the rule count of the version
//! that closed it. None of those were noticed by reading; all three are
//! mechanical.
//!
//! What is checked here is structure and citation. Prose is left to review,
//! and the one clause of the contract that resists a machine — present tense
//! outside changelogs — is checked only for the constructions the README names
//! outright, with the rest recorded as a `review` obligation rather than
//! claimed.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use mcf_checks::document::{Document, Identifiers, all};

fn documents() -> Vec<Document> {
    all().unwrap_or_else(|path| panic!("{} is readable", path.display()))
}

/// There is something to check. A reader that found no documents would make
/// every test below pass by having nothing to disagree with (A19).
#[test]
fn the_documents_are_found() {
    let documents = documents();
    assert!(
        documents.len() >= 10,
        "only {} documents were found",
        documents.len()
    );
    for expected in [
        "README.md",
        "doc/document-of-intent.md",
        "doc/rules.md",
        "doc/backlog.md",
    ] {
        assert!(
            documents.iter().any(|d| d.relative_path == expected),
            "{expected} was not found"
        );
    }
}

/// Every document opens with its own title, on the first line.
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

/// Every document carries front matter naming its type, version and status.
#[test]
fn every_document_carries_front_matter() {
    for document in documents() {
        for key in ["Type", "Version", "Status"] {
            assert!(
                document.front_matter(key).is_some(),
                "{} states no **{key}** in its front matter",
                document.relative_path
            );
        }
    }
}

/// The version is a number that counts up, not a date or a name.
#[test]
fn every_version_is_a_number() {
    for document in documents() {
        let version = document
            .front_matter("Version")
            .unwrap_or_else(|| panic!("{} states a version", document.relative_path));
        assert!(
            version.parse::<u32>().is_ok_and(|n| n >= 1),
            "{} states version {version:?}, which is not a number",
            document.relative_path
        );
    }
}

/// Every document ends with a changelog, and it is the last section.
///
/// Last because the contract says so and because the reason is legible: the
/// current state is reached in one screen, and history is what the reader
/// scrolls *past* the document to find.
#[test]
fn every_document_ends_with_its_changelog() {
    for document in documents() {
        let start = document
            .changelog_start()
            .unwrap_or_else(|| panic!("{} has no changelog section", document.relative_path));
        let later: Vec<&str> = document
            .sections()
            .into_iter()
            .filter(|(line, _)| *line > start)
            .map(|(_, heading)| heading)
            .collect();
        assert!(
            later.is_empty(),
            "{} has sections after its changelog: {later:?}",
            document.relative_path
        );
    }
}

/// The changelog accounts for the version the front matter claims.
///
/// This is the check that would have caught the backlog's front matter sitting
/// at version 4 while its changelog had reached 21.
#[test]
fn the_changelog_accounts_for_the_current_version() {
    for document in documents() {
        let version = document
            .front_matter("Version")
            .unwrap_or_else(|| panic!("{} states a version", document.relative_path));
        let start = document
            .changelog_start()
            .unwrap_or_else(|| panic!("{} has a changelog", document.relative_path));
        let changelog = document
            .lines
            .iter()
            .skip(start)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let entry_heading = format!("Version {version}");
        let entry_row = format!("| {version} |");
        assert!(
            changelog.contains(&entry_heading) || changelog.contains(&entry_row),
            "{} claims version {version} and its changelog does not account for it",
            document.relative_path
        );
    }
}

/// No document claims to derive from a version of the intent document that
/// does not exist yet.
///
/// A document may lag its source — that is normal, and the lag is exactly what
/// the front matter is for. What it may not do is claim to have absorbed a
/// version that was never written, which is how a derived document acquires
/// authority it does not have.
#[test]
fn no_document_derives_from_a_version_that_does_not_exist() {
    let documents = documents();
    let intent = documents
        .iter()
        .find(|d| d.relative_path == "doc/document-of-intent.md")
        .unwrap_or_else(|| panic!("the intent document is present"));
    let current: u32 = intent
        .front_matter("Version")
        .and_then(|version| version.parse().ok())
        .unwrap_or_else(|| panic!("the intent document states a numeric version"));

    for document in &documents {
        let Some(authority) = document.front_matter("Authority") else {
            continue;
        };
        if !authority.contains("document-of-intent.md") {
            continue;
        }
        for token in authority.split_whitespace() {
            let token = token.trim_end_matches(|c: char| !c.is_ascii_digit());
            let Some(digits) = token.strip_prefix('v') else {
                continue;
            };
            let Ok(claimed) = digits.parse::<u32>() else {
                continue;
            };
            assert!(
                claimed <= current,
                "{} claims to derive from intent v{claimed}, and the intent document is at v{current}",
                document.relative_path
            );
        }
    }
}

/// Every relative link resolves to a file that exists.
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

/// Every `B-###` and `DEC-###` cited anywhere resolves to a row in the
/// register.
///
/// C5 makes identifiers stable for life so a citation made once stays valid; a
/// citation to something that never existed is the other half of that promise.
#[test]
fn every_register_citation_resolves() {
    let documents = documents();
    let identifiers = Identifiers::gather(&documents)
        .unwrap_or_else(|path| panic!("{} is readable", path.display()));
    assert!(
        identifiers.build_items.len() > 100,
        "only {} build items were found, so this check is reading the register wrong",
        identifiers.build_items.len()
    );

    let mut dangling = Vec::new();
    for document in &documents {
        for (line, text) in document.prose() {
            for token in tokens(text) {
                let resolved = if let Some(digits) = token.strip_prefix("DEC-") {
                    numeric(digits).then(|| identifiers.decisions.contains(&token))
                } else if let Some(digits) = token.strip_prefix("B-") {
                    numeric(digits).then(|| identifiers.build_items.contains(&token))
                } else {
                    None
                };
                if resolved == Some(false) {
                    dangling.push(format!("{}:{line} → {token}", document.relative_path));
                }
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "dangling register citations: {dangling:#?}"
    );
}

/// Every `§` citation resolves to a clause of the intent document.
#[test]
fn every_clause_citation_resolves() {
    let documents = documents();
    let identifiers = Identifiers::gather(&documents)
        .unwrap_or_else(|path| panic!("{} is readable", path.display()));
    assert!(
        identifiers.clauses.len() > 50,
        "only {} clauses were found, so this check is reading the intent document wrong",
        identifiers.clauses.len()
    );

    let mut dangling = Vec::new();
    for document in &documents {
        for (line, text) in document.prose() {
            for citation in clause_citations(text) {
                if !identifiers.clauses.contains(&citation) {
                    dangling.push(format!("{}:{line} → §{citation}", document.relative_path));
                }
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "dangling clause citations: {dangling:#?}"
    );
}

/// Every rule, resolution, laboratory and milestone identifier cited resolves
/// to the document that defines it.
#[test]
fn every_identifier_citation_resolves() {
    let documents = documents();
    let identifiers = Identifiers::gather(&documents)
        .unwrap_or_else(|path| panic!("{} is readable", path.display()));
    assert!(
        identifiers.rules.len() >= 99,
        "only {} rules were found, so this check is reading rules.md wrong",
        identifiers.rules.len()
    );

    let mut dangling = Vec::new();
    for document in &documents {
        for (line, text) in document.prose() {
            for token in tokens(text) {
                let Some((letter, digits)) = split_identifier(&token) else {
                    continue;
                };
                let known = match letter {
                    'A' | 'B' | 'C' => identifiers.rules.contains(&token),
                    // `P` is two namespaces: the precedence rules and the
                    // proposals. C5 forbids renumbering either, so a citation
                    // resolves if it names one of them (B-353).
                    'P' => {
                        identifiers.rules.contains(&token) || identifiers.proposals.contains(&token)
                    }
                    'D' => identifiers.resolutions.contains(&token),
                    'L' => identifiers.laboratories.contains(&token),
                    'M' => identifiers.milestones.contains(&token),
                    _ => continue,
                };
                if !known && !digits.is_empty() {
                    dangling.push(format!("{}:{line} → {token}", document.relative_path));
                }
            }
        }
    }
    assert!(
        dangling.is_empty(),
        "dangling identifier citations: {dangling:#?}"
    );
}

/// The past tense stays in the changelog.
///
/// Only the constructions the contract names outright are checked — "this was
/// resolved in version 4" belongs in a changelog entry, and the clause it
/// resolved simply states the resolution. The rest of the tense contract
/// remains a `review` obligation, stated here rather than silently dropped.
#[test]
fn the_past_tense_stays_in_the_changelog() {
    const PAST: [&str; 6] = [
        "was resolved in version",
        "were resolved in version",
        "was added in version",
        "were added in version",
        "was removed in version",
        "were removed in version",
    ];
    let mut offenders = Vec::new();
    for document in documents() {
        let changelog = document.changelog_start().unwrap_or(usize::MAX);
        for (line, text) in document.prose() {
            if line >= changelog {
                continue;
            }
            let lowered = without_quotations(text).to_lowercase();
            for phrase in PAST {
                if lowered.contains(phrase) {
                    offenders.push(format!("{}:{line} → {phrase:?}", document.relative_path));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the past tense outside a changelog: {offenders:#?}"
    );
}

/// A line with anything inside double quotation marks removed.
///
/// The format contract quotes its own counter-example — *"this was resolved in
/// version 4" belongs in a changelog entry* — and a check that read the
/// quotation as prose would fail on the sentence that states the rule.
fn without_quotations(text: &str) -> String {
    let mut kept = String::new();
    let mut inside = false;
    for character in text.chars() {
        if character == '"' {
            inside = !inside;
            continue;
        }
        if !inside {
            kept.push(character);
        }
    }
    kept
}

/// Whether every character is a digit, and there is at least one.
fn numeric(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

/// Words a citation could be, with the punctuation that surrounds prose
/// stripped off.
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .map(|token| token.trim_matches('-'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `A6` → `('A', "6")`, and nothing for a word or a version string.
fn split_identifier(token: &str) -> Option<(char, &str)> {
    let mut characters = token.chars();
    let letter = characters.next()?;
    if !letter.is_ascii_uppercase() {
        return None;
    }
    let digits = token.get(1..)?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((letter, digits))
}

/// Every `§…` citation in a line.
fn clause_citations(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for piece in text.split('§').skip(1) {
        let citation: String = piece
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || "IVXL".contains(*c))
            .collect();
        let citation = citation.trim_end_matches('.');
        if !citation.is_empty() {
            found.push(citation.to_owned());
        }
    }
    found
}
