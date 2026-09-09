use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainEntry {
    pub number: usize,
    pub prefix: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeEntry {
    pub code: String,
    pub meaning: String,
    pub domain_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    pub domains: Vec<DomainEntry>,
    pub codes: Vec<CodeEntry>,
    pub attributions: Vec<String>,
    pub dispositions: Vec<String>,
}

#[derive(Debug)]
pub enum TaxonomyError {
    Io(PathBuf, std::io::Error),
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
    pub fn read() -> Result<Self, TaxonomyError> {
        let path = crate::workspace::root().join("doc/taxonomy.md");
        let source =
            std::fs::read_to_string(&path).map_err(|error| TaxonomyError::Io(path, error))?;
        Self::parse(&source)
    }

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

    #[test]
    fn an_empty_document_is_refused() {
        let error = Taxonomy::parse("# Failure Taxonomy\n")
            .expect_err("a document with no domains must be refused");
        assert!(error.to_string().contains("domains"), "{error}");
    }

    #[test]
    fn the_committed_taxonomy_is_readable() {
        let taxonomy = Taxonomy::read().expect("doc/taxonomy.md is readable");
        assert_eq!(taxonomy.domains.len(), 14);
        assert_eq!(taxonomy.codes.len(), 99);
    }
}
