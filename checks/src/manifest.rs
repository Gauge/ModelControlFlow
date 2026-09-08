use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    tables: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
    pub line: usize,
    pub reason: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for ManifestError {}

impl Manifest {
    pub fn parse(source: &str) -> Result<Self, ManifestError> {
        let mut tables: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut table = String::new();
        let mut lines = source.lines().enumerate();

        while let Some((index, raw)) = lines.next() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                header.trim().clone_into(&mut table);
                tables.entry(table.clone()).or_default();
                continue;
            }
            let Some((key, first)) = line.split_once('=') else {
                return Err(ManifestError {
                    line: index + 1,
                    reason: format!("neither a table header nor a key: {line:?}"),
                });
            };

            let mut value = first.trim().to_owned();
            while unclosed(&value) {
                let Some((_, continuation)) = lines.next() else {
                    return Err(ManifestError {
                        line: index + 1,
                        reason: format!("value is never closed: {value:?}"),
                    });
                };
                value.push(' ');
                value.push_str(strip_comment(continuation).trim());
            }

            tables
                .entry(table.clone())
                .or_default()
                .insert(key.trim().to_owned(), value);
        }
        Ok(Self { tables })
    }

    #[must_use]
    pub fn has_table(&self, table: &str) -> bool {
        self.tables.contains_key(table)
    }

    #[must_use]
    pub fn get(&self, table: &str, key: &str) -> Option<&str> {
        self.tables
            .get(table)
            .and_then(|t| t.get(key))
            .map(String::as_str)
    }

    #[must_use]
    pub fn keys(&self, table: &str) -> Vec<&str> {
        self.tables
            .get(table)
            .map(|t| t.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn list(&self, table: &str, key: &str) -> Option<Vec<String>> {
        let value = self.get(table, key)?;
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        Some(
            inner
                .split(',')
                .map(|element| element.trim().trim_matches('"').to_owned())
                .filter(|element| !element.is_empty())
                .collect(),
        )
    }
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(at) => line.split_at(at).0,
        None => line,
    }
}

fn unclosed(value: &str) -> bool {
    let opens = value.matches('[').count();
    let closes = value.matches(']').count();
    opens > closes
}

#[cfg(test)]
mod tests {
    use super::Manifest;

    #[test]
    fn reads_tables_and_keys() {
        let manifest = Manifest::parse(
            "[package]\nname = \"mcf-core\"\n\n[dependencies]\nmcf-record = { path = \"..\" }\n",
        )
        .expect("a manifest of two plain tables is inside what the reader claims");
        assert_eq!(manifest.get("package", "name"), Some("\"mcf-core\""));
        assert_eq!(manifest.keys("dependencies"), vec!["mcf-record"]);
        assert!(manifest.has_table("dependencies"));
        assert!(!manifest.has_table("build-dependencies"));
    }

    #[test]
    fn joins_a_bracketed_value_across_lines() {
        let manifest = Manifest::parse("[workspace]\nmembers = [\n  \"a\",\n  \"b\",\n]\n")
            .expect("a multi-line array is what the reader exists to read");
        assert_eq!(
            manifest.list("workspace", "members"),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn an_absent_list_is_not_an_empty_list() {
        let manifest = Manifest::parse("[workspace]\nmembers = []\n")
            .expect("an empty array is inside what the reader claims");
        assert_eq!(manifest.list("workspace", "members"), Some(vec![]));
        assert_eq!(manifest.list("workspace", "absent"), None);
    }

    #[test]
    fn comments_and_blank_lines_are_not_content() {
        let manifest = Manifest::parse("# a comment\n\n[package]\nname = \"x\" # trailing\n")
            .expect("comments are inside what the reader claims");
        assert_eq!(manifest.get("package", "name"), Some("\"x\""));
        assert_eq!(manifest.keys("package"), vec!["name"]);
    }

    #[test]
    fn an_unreadable_line_is_refused_with_its_number() {
        let error = Manifest::parse("[package]\nthis is not a manifest line\n")
            .expect_err("a line that is neither a header nor a pair must be refused");
        assert_eq!(error.line, 2);
        assert!(error.to_string().contains("line 2"), "{error}");
    }

    #[test]
    fn an_unclosed_value_is_refused() {
        let error = Manifest::parse("[workspace]\nmembers = [\n  \"a\",\n")
            .expect_err("an array that never closes must be refused");
        assert!(error.to_string().contains("never closed"), "{error}");
    }

    #[test]
    fn a_key_before_the_first_header_is_in_the_root_table() {
        let manifest = Manifest::parse("cargo-features = [\"x\"]\n[package]\n")
            .expect("a root key is inside what the reader claims");
        assert_eq!(
            manifest.list("", "cargo-features"),
            Some(vec!["x".to_owned()])
        );
    }
}
