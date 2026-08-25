//! Reads the documents in `doc/`, so the format contract can be checked.
//!
//! The contract is stated in the repository README's *Format contract* section: a
//! title, a front-matter table, a lead, a body, and a changelog last; present
//! tense outside the changelog; identifiers stable for life (C5); citations by
//! clause rather than by paraphrase.
//!
//! B-041 makes it checkable. A contract nothing checks is a convention, and
//! this project has already found three of its own documents disagreeing with
//! themselves — a front matter two versions behind its changelog, header counts
//! three revisions stale, and an item still reporting the rule count of the
//! version that closed it.
//!
//! What this module does *not* attempt is prose. It reads structure —
//! headings, table rows, links, citations — and leaves judgement to review.
//!

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One document, read.
#[derive(Debug, Clone)]
pub struct Document {
    /// Where it lives, relative to the workspace root.
    pub relative_path: String,
    /// Its absolute path.
    pub path: PathBuf,
    /// Every line, in order.
    pub lines: Vec<String>,
}

impl Document {
    /// The document's `# Title`, if the first line is one.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.lines
            .first()
            .and_then(|line| line.strip_prefix("# "))
            .map(str::trim)
    }

    /// The value of a front-matter row, if the document has one.
    ///
    /// Front matter is the two-column table at the top: rows of the form
    /// `| **Version** | 7 |`.
    #[must_use]
    pub fn front_matter(&self, key: &str) -> Option<&str> {
        let wanted = format!("| **{key}** |");
        self.lines
            .iter()
            .take(24)
            .find(|line| line.starts_with(&wanted))
            .and_then(|line| line.strip_prefix(&wanted))
            .and_then(|rest| rest.trim().strip_suffix('|'))
            .map(str::trim)
    }

    /// Every `##`-level heading, with its line number, in order.
    #[must_use]
    pub fn sections(&self) -> Vec<(usize, &str)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                line.strip_prefix("## ")
                    .map(|heading| (index + 1, heading.trim()))
            })
            .collect()
    }

    /// The line number the changelog starts at, if the document has one.
    ///
    /// A heading whose text *ends* with `Changelog`, so that the intent
    /// document's numbered `## 9. Changelog` counts alongside a plain one.
    #[must_use]
    pub fn changelog_start(&self) -> Option<usize> {
        self.sections()
            .into_iter()
            .find(|(_, heading)| heading.ends_with("Changelog"))
            .map(|(line, _)| line)
    }

    /// The document's body with fenced code blocks removed.
    ///
    /// Citations inside a fence are output, a record or a command — C8 makes
    /// them verbatim — so they are not claims this document is making and are
    /// not checked as such.
    #[must_use]
    pub fn prose(&self) -> Vec<(usize, &str)> {
        let mut prose = Vec::new();
        let mut fenced = false;
        for (index, line) in self.lines.iter().enumerate() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if !fenced {
                prose.push((index + 1, line.as_str()));
            }
        }
        prose
    }

    /// Every relative link target in the document, with its line number.
    ///
    /// Absolute URLs are not checked: reaching them would be a network request,
    /// and B19 keeps the suite offline.
    #[must_use]
    pub fn relative_links(&self) -> Vec<(usize, String)> {
        let mut found = Vec::new();
        for (number, line) in self.prose() {
            let mut rest = line;
            while let Some(open) = rest.find("](") {
                let after = rest.split_at(open + 2).1;
                let Some(close) = after.find(')') else {
                    break;
                };
                let (target, remainder) = after.split_at(close);
                rest = remainder;
                if target.starts_with("http://")
                    || target.starts_with("https://")
                    || target.starts_with('#')
                    || target.is_empty()
                {
                    continue;
                }
                found.push((number, target.to_owned()));
            }
        }
        found
    }
}

/// Every Markdown document the contract governs: `README.md` and everything
/// under `doc/`.
///
/// # Errors
///
/// Returns the path that could not be read.
pub fn all() -> Result<Vec<Document>, PathBuf> {
    let root = crate::workspace::root();
    let mut paths = vec![root.join("README.md")];
    collect_markdown(&root.join("doc"), &mut paths);
    paths.sort();

    let mut documents = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|_| path.clone())?;
        let relative_path = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        documents.push(Document {
            relative_path,
            path,
            lines: text.lines().map(str::to_owned).collect(),
        });
    }
    Ok(documents)
}

fn collect_markdown(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, into);
        } else if path.extension().is_some_and(|e| e == "md") {
            into.push(path);
        }
    }
}

/// Every identifier the documents may cite, gathered from the documents that
/// define them.
#[derive(Debug, Clone, Default)]
pub struct Identifiers {
    /// Backlog build items: `B-001`.
    pub build_items: BTreeSet<String>,
    /// Backlog decisions: `DEC-016`.
    pub decisions: BTreeSet<String>,
    /// Rule identifiers from `rules.md`: `A6`, `B12`, `C5`, `P2`.
    pub rules: BTreeSet<String>,
    /// Resolution identifiers from the intent document: `D24`.
    pub resolutions: BTreeSet<String>,
    /// Laboratory identifiers from `labs.md`: `L1`.
    pub laboratories: BTreeSet<String>,
    /// Proposal identifiers from `proposals.md`: `PR6`.
    ///
    /// They used to be `P<n>`, which collided with the precedence rules. C5
    /// permits deprecating an identifier in favour of a named successor, and
    /// B-353 did that: `P` now names one thing.
    pub proposals: BTreeSet<String>,
    /// Milestone identifiers from the roadmap: `M0`.
    pub milestones: BTreeSet<String>,
    /// Clause references from the intent document: `3.4`, `7.16`, `VII`, `5`.
    pub clauses: BTreeSet<String>,
}

impl Identifiers {
    /// Gathers every identifier from the documents that define them.
    ///
    /// # Errors
    ///
    /// Returns the path of a document that could not be read.
    pub fn gather(documents: &[Document]) -> Result<Self, PathBuf> {
        let mut identifiers = Self::default();
        for document in documents {
            for line in &document.lines {
                match document.relative_path.as_str() {
                    "doc/backlog.md" => {
                        if let Some(id) = row_identifier(line, "B-") {
                            identifiers.build_items.insert(id);
                        }
                        if let Some(id) = row_identifier(line, "DEC-") {
                            identifiers.decisions.insert(id);
                        }
                    }
                    "doc/rules.md" => {
                        if let Some(id) = heading_identifier(line) {
                            identifiers.rules.insert(id);
                        }
                        if let Some(id) = precedence_identifier(line) {
                            identifiers.rules.insert(id);
                        }
                    }
                    "doc/labs.md" => {
                        if let Some(id) = heading_identifier(line) {
                            identifiers.laboratories.insert(id);
                        }
                    }
                    "doc/proposals.md" => {
                        if let Some(id) = section_identifier(line) {
                            identifiers.proposals.insert(id);
                        }
                    }
                    "doc/roadmap.md" => {
                        identifiers.milestones.extend(milestone_identifiers(line));
                    }
                    "doc/document-of-intent.md" => {
                        if let Some(id) = heading_identifier(line) {
                            identifiers.resolutions.insert(id);
                        }
                        identifiers.clauses.extend(clause_identifiers(line));
                        identifiers.clauses.extend(retired_void_identifier(line));
                    }
                    _ => {}
                }
            }
        }
        Ok(identifiers)
    }
}

/// `| B-001 | …` → `B-001`
fn row_identifier(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix("| ")?;
    let (first, _) = rest.split_once(" |")?;
    let first = first.trim();
    let digits = first.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(first.to_owned())
}

/// `### A6 — Never lose information` → `A6`
fn heading_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("### ")?;
    let (candidate, _) = rest.split_once(' ')?;
    let (letters, digits) = candidate.split_at(candidate.find(|c: char| c.is_ascii_digit())?);
    if letters.len() != 1 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(candidate.to_owned())
}

/// `## PR2 — The repro bundle` → `PR2`
fn section_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## ")?;
    let (candidate, _) = rest.split_once(' ')?;
    let digits = candidate.strip_prefix("PR")?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(candidate.to_owned())
}

/// `| §7.10 | Failure taxonomy | … |` → `7.10`.
///
/// A void that has been answered keeps its number and loses its heading: §8
/// requires the substance move into §2.1, §3 or §6, and the retired-void index
/// is what keeps the citation valid (C5). Without this the index's own rows
/// would read as dangling citations.
fn retired_void_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("| §")?;
    let (candidate, _) = rest.split_once(" |")?;
    let candidate = candidate.trim();
    if candidate.is_empty() || !candidate.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(candidate.to_owned())
}

/// `| **P1** | **Honesty outranks continuity.** …` → `P1`
fn precedence_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("| **")?;
    let (candidate, _) = rest.split_once("**")?;
    if !candidate.starts_with('P') || candidate.len() < 2 {
        return None;
    }
    if !candidate.get(1..)?.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(candidate.to_owned())
}

/// `| **M0** | **The instrument** …` and `## M0 — The instrument`.
fn milestone_identifiers(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    for candidate in line.split(|c: char| !c.is_ascii_alphanumeric()) {
        let Some(digits) = candidate.strip_prefix('M') else {
            continue;
        };
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            found.push(candidate.to_owned());
        }
    }
    found
}

/// Headings that define a citable clause: `### 3.4 …`, `### 7.16 …`,
/// `### VII. …`, `## 5. Anti-Goals`.
fn clause_identifiers(line: &str) -> Vec<String> {
    let heading = line
        .strip_prefix("### ")
        .or_else(|| line.strip_prefix("## "));
    let Some(heading) = heading else {
        return Vec::new();
    };
    let Some(first) = heading.split_whitespace().next() else {
        return Vec::new();
    };
    let candidate = first.trim_end_matches('.');
    let is_numeric = candidate
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' && candidate.contains('.'));
    let is_roman = !candidate.is_empty() && candidate.chars().all(|c| "IVXL".contains(c));
    if candidate.is_empty() || !(is_numeric || is_roman) {
        return Vec::new();
    }
    let mut found = vec![candidate.to_owned()];
    // `### 3.4 …` also makes `§3` citable, since the section it sits in is
    // `## 3. Principles`.
    if let Some((major, _)) = candidate.split_once('.') {
        found.push(major.to_owned());
    }
    found
}
