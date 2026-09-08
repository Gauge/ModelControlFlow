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
