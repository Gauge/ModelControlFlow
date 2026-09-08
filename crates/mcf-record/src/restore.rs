use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-record::restore");

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    FileReplaced {
        path: PathBuf,
        previous: Option<String>,
    },
}

impl Change {
    pub fn about_to_replace(path: &Path) -> Result<Self> {
        let previous = match std::fs::read_to_string(path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(Failure::new(
                    Category::ArtifactUnreadable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "MCF will not change a file it could not first read, because it \
                     could not then put it back",
                )
                .with_context("path", path.display().to_string())
                .with_context("os_error", error.to_string()));
            }
        };
        Ok(Self::FileReplaced {
            path: path.to_path_buf(),
            previous,
        })
    }

    pub fn restore(&self) -> Result<()> {
        match self {
            Self::FileReplaced { path, previous } => {
                let outcome = match previous {
                    Some(contents) => std::fs::write(path, contents),
                    None => match std::fs::remove_file(path) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        other => other,
                    },
                };
                outcome.map_err(|error| {
                    Failure::new(
                        Category::PlatformRestoreFailed,
                        Attribution::Machine,
                        Disposition::Partial,
                        WHERE,
                        "something MCF changed could not be put back",
                    )
                    .with_context("path", path.display().to_string())
                    .with_context("os_error", error.to_string())
                })
            }
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::FileReplaced { path, previous } => Value::map([
                ("change", Value::text("file_replaced")),
                ("path", Value::text(path.display().to_string())),
                (
                    "previous",
                    match previous {
                        Some(contents) => Value::text(contents.clone()),
                        None => Value::Null,
                    },
                ),
            ]),
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value.get("change")?.as_text()? {
            "file_replaced" => Some(Self::FileReplaced {
                path: PathBuf::from(value.get("path")?.as_text()?),
                previous: match value.get("previous")? {
                    Value::Null => None,
                    Value::Text(contents) => Some(contents.clone()),
                    _ => return None,
                },
            }),
            _ => None,
        }
    }
}

impl core::fmt::Display for Change {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FileReplaced { path, previous } => write!(
                f,
                "{} ({})",
                path.display(),
                match previous {
                    Some(contents) => format!("was {} bytes", contents.len()),
                    None => "did not exist".to_owned(),
                }
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct Recovery {
    pub restored: Vec<Change>,
    pub failed: Vec<Failure>,
    pub unreadable: usize,
}

impl Recovery {
    #[must_use]
    pub fn found_anything(&self) -> bool {
        !self.restored.is_empty() || !self.failed.is_empty() || self.unreadable > 0
    }

    #[must_use]
    pub fn statement(&self) -> String {
        if !self.found_anything() {
            return "nothing was left to restore".to_owned();
        }
        let mut said = vec![format!(
            "a previous run left {} change(s); {} restored",
            self.restored.len() + self.failed.len() + self.unreadable,
            self.restored.len()
        )];
        if !self.failed.is_empty() {
            said.push(format!("{} COULD NOT BE", self.failed.len()));
        }
        if self.unreadable > 0 {
            said.push(format!(
                "{} written by a version this one cannot read",
                self.unreadable
            ));
        }
        said.join(", ")
    }
}

#[derive(Debug)]
pub struct Ledger {
    path: PathBuf,
    pending: Vec<Change>,
}

impl Ledger {
    pub fn open(path: &Path) -> Result<(Self, Recovery)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(path, &error))?;
        }
        let recovery = recover(path);
        let ledger = Self {
            path: path.to_path_buf(),
            pending: Vec::new(),
        };
        if recovery.failed.is_empty() && recovery.unreadable == 0 {
            let _cleared = std::fs::remove_file(path);
        }
        Ok((ledger, recovery))
    }

    pub fn record(&mut self, change: Change) -> Result<()> {
        self.pending.push(change);
        self.flush()
    }

    #[must_use]
    pub fn pending(&self) -> &[Change] {
        &self.pending
    }

    pub fn restore_all(&mut self) -> Result<()> {
        let mut first: Option<Failure> = None;
        while let Some(change) = self.pending.pop() {
            if let (Err(failure), None) = (change.restore(), first.as_ref()) {
                first = Some(failure);
            }
        }
        let _cleared = std::fs::remove_file(&self.path);
        match first {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    fn flush(&self) -> Result<()> {
        let lines: Vec<String> = self
            .pending
            .iter()
            .map(|change| change.to_value().to_line())
            .collect();
        let mut text = lines.join("\n");
        text.push('\n');
        std::fs::write(&self.path, text).map_err(|error| unwritable(&self.path, &error))
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        let _restored = self.restore_all();
    }
}

fn recover(path: &Path) -> Recovery {
    let mut recovery = Recovery::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return recovery;
    };
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = json::parse(line) else {
            recovery.unreadable += 1;
            continue;
        };
        let Some(change) = Change::from_value(&value) else {
            recovery.unreadable += 1;
            continue;
        };
        match change.restore() {
            Ok(()) => recovery.restored.push(change),
            Err(failure) => recovery.failed.push(failure),
        }
    }
    recovery
}

fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the restoration ledger could not be written, so nothing may be changed",
    )
    .with_context("path", path.display().to_string())
    .with_context("os_error", error.to_string())
}

#[cfg(test)]
mod tests;
