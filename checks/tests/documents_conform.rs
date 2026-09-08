#![allow(clippy::panic)]

use mcf_checks::document::{Document, Identifiers, all};

fn documents() -> Vec<Document> {
    all().unwrap_or_else(|path| panic!("{} is readable", path.display()))
}

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
    assert!(
        identifiers.proposals.len() >= 7,
        "only {} proposals were found, so this check is reading proposals.md wrong",
        identifiers.proposals.len()
    );
    assert!(
        identifiers.findings.len() >= 100,
        "only {} findings were found, so this check is reading findings.md wrong",
        identifiers.findings.len()
    );

    let mut dangling = Vec::new();
    for document in &documents {
        for (line, text) in document.prose() {
            for token in tokens(text) {
                if let Some(proposal) = proposal_identifier(&token) {
                    if !identifiers.proposals.contains(proposal) {
                        dangling.push(format!("{}:{line} → {proposal}", document.relative_path));
                    }
                    continue;
                }
                let Some((letter, digits)) = split_identifier(&token) else {
                    continue;
                };
                let known = match letter {
                    'A' | 'B' | 'C' => identifiers.rules.contains(&token),
                    'P' => {
                        identifiers.rules.contains(&token) || identifiers.proposals.contains(&token)
                    }
                    'D' => identifiers.resolutions.contains(&token),
                    'F' => identifiers.findings.contains(&token),
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

fn numeric(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .map(|token| token.trim_matches('-'))
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn split_identifier(token: &str) -> Option<(char, &str)> {
    if token.starts_with("PR") {
        return None;
    }
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

fn proposal_identifier(token: &str) -> Option<&str> {
    let digits = token.strip_prefix("PR")?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(token)
}

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

#[test]
fn every_register_identifier_is_used_once() {
    let backlog = documents()
        .into_iter()
        .find(|document| document.relative_path.ends_with("backlog.md"))
        .expect("the backlog is one of the documents");

    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut twice = Vec::new();
    for (line, text) in backlog.prose() {
        let trimmed = text.trim_start();
        let Some(identifier) = trimmed.strip_prefix("| ") else {
            continue;
        };
        let Some((identifier, _)) = identifier.split_once(" |") else {
            continue;
        };
        if !(identifier.starts_with("B-") || identifier.starts_with("DEC-")) {
            continue;
        }
        match seen.iter().find(|(already, _)| already == identifier) {
            Some((_, first)) => twice.push(format!(
                "{identifier} is used at line {first} and again at line {line}"
            )),
            None => seen.push((identifier.to_owned(), line)),
        }
    }

    assert!(
        seen.len() > 100,
        "only {} identifiers were found, so this check is reading the register wrong",
        seen.len()
    );
    assert!(
        twice.is_empty(),
        "an identifier names two items, so every citation of it is ambiguous: {twice:#?}"
    );
}
