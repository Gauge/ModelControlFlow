//! Reads `doc/taxonomy.md`, so the document and the types can be compared.
//!
//! A13 holds that the failure taxonomy and the laboratory's fault catalogue
//! are the same list. That check is B-010's and waits on the laboratory
//! (DEC-021). The half available now is the one between the *document* and the
//! *code*, and it is worth having on its own: the taxonomy is a public
//! interface the moment §XIV ships, its codes are stable for life (C5), and a
//! generated file that has drifted from the document it was generated from is
//! a public interface nobody is reading.
//!
//! The reader claims exactly the two shapes the taxonomy uses — the domain
//! headings and the code tables — and refuses anything else rather than
//! guessing (A7's habit, applied to a parser).

use std::path::PathBuf;

/// One domain heading: `### 3 · `artifact.*` — a local model artifact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainEntry {
    /// The domain's position in the document, one-based.
    pub number: usize,
    /// The prefix its codes carry.
    pub prefix: String,
    /// What the domain covers.
    pub subject: String,
}

/// One row of a code table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeEntry {
    /// The dotted code.
    pub code: String,
    /// What the document says it means.
    pub meaning: String,
    /// The prefix of the domain whose table it appeared in.
    pub domain_prefix: String,
}

/// The taxonomy document, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    /// The domains, in document order.
    pub domains: Vec<DomainEntry>,
    /// The codes, in document order.
    pub codes: Vec<CodeEntry>,
    /// The values the axis table gives for `Attribution`.
    pub attributions: Vec<String>,
    /// The values the axis table gives for `Disposition`.
    pub dispositions: Vec<String>,
}

/// What could not be read.
#[derive(Debug)]
pub enum TaxonomyError {
    /// The document could not be read.
    Io(PathBuf, std::io::Error),
    /// The document was read and something it must contain was absent.
    Absent(&'static str),
}

impl std::fmt::Display for TaxonomyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
            Self::Absent(what) => write!(f, "the taxonomy states no {what}"),
        }
    }
}

impl std::error::Error for TaxonomyError {}

impl Taxonomy {
    /// Reads `doc/taxonomy.md` from the workspace root.
    ///
    /// # Errors
    ///
    /// Returns a [`TaxonomyError`] when the file cannot be read, or when it
    /// contains no domains, no codes or no axis values — each of which would
    /// make an agreement check pass by having nothing to compare, which is the
    /// vacuous green A2 forbids aimed at the suite.
    pub fn read() -> Result<Self, TaxonomyError> {
        let path = crate::workspace::root().join("doc/taxonomy.md");
        let source =
            std::fs::read_to_string(&path).map_err(|error| TaxonomyError::Io(path, error))?;
        Self::parse(&source)
    }

    /// Reads a taxonomy from its text.
    ///
    /// # Errors
    ///
    /// As [`Taxonomy::read`].
    pub fn parse(source: &str) -> Result<Self, TaxonomyError> {
        let mut domains = Vec::new();
        let mut codes = Vec::new();
        let mut attributions = Vec::new();
        let mut dispositions = Vec::new();
        let mut current_prefix = String::new();

        for line in source.lines() {
            let line = line.trim();
            if let Some(entry) = parse_domain_heading(line) {
                current_prefix.clone_from(&entry.prefix);
                domains.push(entry);
                continue;
            }
            if let Some((code, meaning)) = parse_code_row(line) {
                codes.push(CodeEntry {
                    code,
                    meaning,
                    domain_prefix: current_prefix.clone(),
                });
                continue;
            }
            if let Some(values) = parse_axis_row(line, "Attribution") {
                attributions = values;
                continue;
            }
            if let Some(values) = parse_axis_row(line, "Disposition") {
                dispositions = values;
            }
        }

        if domains.is_empty() {
            return Err(TaxonomyError::Absent("domains"));
        }
        if codes.is_empty() {
            return Err(TaxonomyError::Absent("codes"));
        }
        if attributions.is_empty() {
            return Err(TaxonomyError::Absent("attributions"));
        }
        if dispositions.is_empty() {
            return Err(TaxonomyError::Absent("dispositions"));
        }
        Ok(Self {
            domains,
            codes,
            attributions,
            dispositions,
        })
    }
}

/// `### 3 · `artifact.*` — a local model artifact`
fn parse_domain_heading(line: &str) -> Option<DomainEntry> {
    let rest = line.strip_prefix("### ")?;
    let (number, rest) = rest.split_once(" · ")?;
    let number = number.trim().parse().ok()?;
    let (marker, subject) = rest.split_once(" — ")?;
    let prefix = marker.trim().strip_prefix('`')?.strip_suffix(".*`")?;
    Some(DomainEntry {
        number,
        prefix: prefix.to_owned(),
        subject: subject.trim().to_owned(),
    })
}

/// `| `hub.auth.required` | Credentials absent |`
fn parse_code_row(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    let (code, meaning) = inner.split_once('|')?;
    let code = code.trim().strip_prefix('`')?.strip_suffix('`')?;
    if code.is_empty() || !code.contains('.') {
        return None;
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
    {
        return None;
    }
    Some((code.to_owned(), meaning.trim().to_owned()))
}

/// `| **Attribution** | *Whose failure it is* | `mcf` · `managed` · … |`
fn parse_axis_row(line: &str, axis: &str) -> Option<Vec<String>> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    let mut cells = inner.split('|');
    let name = cells.next()?.trim();
    if name != format!("**{axis}**") {
        return None;
    }
    let values = cells.nth(1)?;
    Some(
        values
            .split('·')
            .map(|value| value.trim().trim_matches('`').to_owned())
            .filter(|value| !value.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{Taxonomy, parse_axis_row, parse_code_row, parse_domain_heading};

    #[test]
    fn reads_a_domain_heading() {
        let entry = parse_domain_heading("### 3 · `artifact.*` — a local model artifact")
            .expect("the heading is the shape the reader claims");
        assert_eq!(entry.number, 3);
        assert_eq!(entry.prefix, "artifact");
        assert_eq!(entry.subject, "a local model artifact");
    }

    #[test]
    fn reads_a_code_row() {
        assert_eq!(
            parse_code_row("| `hub.auth.required` | Credentials absent |"),
            Some((
                "hub.auth.required".to_owned(),
                "Credentials absent".to_owned()
            ))
        );
    }

    /// A table header, a prose row and a row whose first cell is not a code are
    /// all not codes. A reader that accepted them would compare the types
    /// against noise.
    #[test]
    fn refuses_what_is_not_a_code_row() {
        assert_eq!(parse_code_row("| Code | Meaning |"), None);
        assert_eq!(parse_code_row("|---|---|"), None);
        assert_eq!(parse_code_row("| `C5` | stable for life |"), None);
        assert_eq!(parse_code_row("not a row at all"), None);
    }

    #[test]
    fn reads_an_axis_row() {
        let line = "| **Disposition** | *What MCF did* | `refused` · `degraded` · `partial` |";
        assert_eq!(
            parse_axis_row(line, "Disposition"),
            Some(vec![
                "refused".to_owned(),
                "degraded".to_owned(),
                "partial".to_owned()
            ])
        );
        assert_eq!(parse_axis_row(line, "Attribution"), None);
    }

    /// A2: a document with nothing in it is refused, not read as an empty
    /// taxonomy that every agreement check would pass against.
    #[test]
    fn an_empty_document_is_refused() {
        let error = Taxonomy::parse("# Failure Taxonomy\n")
            .expect_err("a document with no domains must be refused");
        assert!(error.to_string().contains("domains"), "{error}");
    }

    /// The real document is readable, which is the precondition every check in
    /// `tests/taxonomy_agreement.rs` rests on.
    #[test]
    fn the_committed_taxonomy_is_readable() {
        let taxonomy = Taxonomy::read().expect("doc/taxonomy.md is readable");
        assert_eq!(taxonomy.domains.len(), 16);
        assert_eq!(taxonomy.codes.len(), 112);
    }
}
