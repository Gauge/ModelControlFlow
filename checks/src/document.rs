use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Document {
    pub relative_path: String,
    pub path: PathBuf,
    pub lines: Vec<String>,
}

impl Document {
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.lines
            .first()
            .and_then(|line| line.strip_prefix("# "))
            .map(str::trim)
    }

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

    #[must_use]
    pub fn changelog_start(&self) -> Option<usize> {
        self.sections()
            .into_iter()
            .find(|(_, heading)| heading.ends_with("Changelog"))
            .map(|(line, _)| line)
    }

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

#[derive(Debug, Clone, Default)]
pub struct Identifiers {
    pub build_items: BTreeSet<String>,
    pub decisions: BTreeSet<String>,
    pub rules: BTreeSet<String>,
    pub resolutions: BTreeSet<String>,
    pub laboratories: BTreeSet<String>,
    pub proposals: BTreeSet<String>,
    pub milestones: BTreeSet<String>,
    pub findings: BTreeSet<String>,
    pub clauses: BTreeSet<String>,
}

impl Identifiers {
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
                    "doc/findings.md" => {
                        if let Some(id) = finding_identifier(line) {
                            identifiers.findings.insert(id);
                        }
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

fn finding_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## ")?;
    let (_, after) = rest.split_once(" · ")?;
    let (token, _) = after.split_once(' ')?;
    let digits = token.strip_prefix('F')?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(token.to_owned())
}

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

fn heading_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("### ")?;
    let (candidate, _) = rest.split_once(' ')?;
    let (letters, digits) = candidate.split_at(candidate.find(|c: char| c.is_ascii_digit())?);
    if letters.len() != 1 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(candidate.to_owned())
}

fn section_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("## ")?;
    let (candidate, _) = rest.split_once(' ')?;
    let digits = candidate.strip_prefix("PR")?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(candidate.to_owned())
}

fn retired_void_identifier(line: &str) -> Option<String> {
    let rest = line.strip_prefix("| §")?;
    let (candidate, _) = rest.split_once(" |")?;
    let candidate = candidate.trim();
    if candidate.is_empty() || !candidate.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(candidate.to_owned())
}

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
    if let Some((major, _)) = candidate.split_once('.') {
        found.push(major.to_owned());
    }
    found
}
