//! What the site holds: contributions, exactly as they arrived.
//!
//! **Stored verbatim.** A contribution is the file a person chose to publish,
//! and the site keeps those bytes rather than a parsed version of them. Two
//! reasons, and either would do. Nothing is lost, so a reader can always be
//! shown what was actually submitted rather than what the site understood. And
//! the site does not reimplement MCF's format, so the two cannot drift: it
//! reads only the few fields it needs to list a submission, and displays the
//! rest as it stands.
//!
//! **Publication cannot be undone.** MCF's own terms say so, and the site is
//! where that becomes true: there is no delete here, because offering one would
//! be offering a retraction that the terms already said does not exist.

use std::path::{Path, PathBuf};

/// One submission.
#[derive(Debug, Clone)]
pub struct Held {
    /// What names it: the digest of its bytes, so the same file submitted twice
    /// is one entry rather than two.
    pub digest: String,
    /// When it arrived, in seconds since the epoch.
    pub at: u64,
    /// The bytes, as they arrived.
    pub body: String,
}

impl Held {
    /// The rows: the lines that are ones.
    ///
    /// **Recognised, not merely non-empty.** A line has to open the way MCF
    /// writes a row — `comparison ·` or `absolute ·`. Accepting anything that
    /// was not blank meant a file of arbitrary text was a contribution with as
    /// many rows as it had lines, which is how a public archive fills up with
    /// things nobody measured.
    ///
    /// MCF's trailing count is dropped by the same rule, without needing a rule
    /// of its own: a summary of the rows is not a row, and the rows are there
    /// to be counted.
    #[must_use]
    pub fn rows(&self) -> Vec<&str> {
        self.body.lines().filter(|line| is_a_row(line)).collect()
    }

    /// Every arm named in it, in the order they appear.
    #[must_use]
    pub fn arms(&self) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for row in self.rows() {
            for name in arms_in(row) {
                if !found.contains(&name) {
                    found.push(name);
                }
            }
        }
        found
    }
}

/// Whether a line is a row MCF wrote.
///
/// By the shape MCF writes rather than by what it is not: a kind, a separator,
/// and something after it.
#[must_use]
pub fn is_a_row(line: &str) -> bool {
    let Some((kind, rest)) = line.split_once('·') else {
        return false;
    };
    matches!(kind.trim(), "comparison" | "absolute") && !rest.trim().is_empty()
}

/// The arms a row names, from the shape MCF writes.
///
/// `comparison · A quicker than B by …` and `absolute · A took …`. Read by the
/// words MCF puts between them rather than by position, so a row with an
/// unexpected shape yields nothing instead of yielding rubbish.
#[must_use]
pub fn arms_in(row: &str) -> Vec<String> {
    let Some((kind, rest)) = row.split_once('·') else {
        return Vec::new();
    };
    let rest = rest.trim();
    match kind.trim() {
        "comparison" => {
            let Some((left, after)) = rest.split_once(" quicker than ") else {
                return Vec::new();
            };
            let right = after
                .split_once(" by ")
                .map_or(after, |(name, _)| name)
                .trim();
            vec![left.trim().to_owned(), right.to_owned()]
        }
        "absolute" => rest
            .split_once(" took ")
            .map(|(arm, _)| vec![arm.trim().to_owned()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Everything the site is holding, newest first.
#[derive(Debug, Default)]
pub struct Archive {
    root: PathBuf,
}

impl Archive {
    /// Opens, or creates, an archive under a directory.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let _made = std::fs::create_dir_all(&root);
        Self { root }
    }

    /// Files a submission, and says what it is called.
    ///
    /// # Errors
    ///
    /// What the filesystem said, or that the body is not a contribution.
    pub fn keep(&self, body: &str) -> Result<String, String> {
        if body.trim().is_empty() {
            return Err("that file is empty".to_owned());
        }
        let held = Held {
            digest: digest_of(body),
            at: now(),
            body: body.to_owned(),
        };
        if held.rows().is_empty() {
            return Err(
                "MCF could not find a row in that file — `mcf share` writes the ones it can \
                 contribute, and a run with nothing to say produces none"
                    .to_owned(),
            );
        }
        let path = self.root.join(format!("{}.txt", held.digest));
        // The same file submitted twice is one entry. Publication cannot be
        // undone, so it also cannot be done twice.
        if path.exists() {
            return Ok(held.digest);
        }
        std::fs::write(&path, format!("{}\n{}", held.at, body))
            .map_err(|error| format!("it could not be filed: {error}"))?;
        Ok(held.digest)
    }

    /// One submission, by name.
    #[must_use]
    pub fn one(&self, digest: &str) -> Option<Held> {
        if !digest.chars().all(|c| c.is_ascii_hexdigit()) || digest.len() != 32 {
            // A name that is not a name never becomes a path.
            return None;
        }
        let text = std::fs::read_to_string(self.root.join(format!("{digest}.txt"))).ok()?;
        let (at, body) = text.split_once('\n')?;
        Some(Held {
            digest: digest.to_owned(),
            at: at.trim().parse().unwrap_or(0),
            body: body.to_owned(),
        })
    }

    /// Everything held, newest first.
    #[must_use]
    pub fn all(&self) -> Vec<Held> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut found: Vec<Held> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_str()?.strip_suffix(".txt")?;
                self.one(name)
            })
            .collect();
        found.sort_by(|a, b| b.at.cmp(&a.at));
        found
    }
}

/// Seconds since the epoch, or nought where the clock will not say.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |held| held.as_secs())
}

/// A name for a submission, from its bytes.
///
/// Not a cryptographic digest and not claimed to be: it names a file so the
/// same one submitted twice is one entry, and nothing here depends on it being
/// hard to collide. Saying which it is, is the point (A21).
#[must_use]
pub fn digest_of(body: &str) -> String {
    let mut low: u64 = 0xcbf2_9ce4_8422_2325;
    let mut high: u64 = 0x9e37_79b9_7f4a_7c15;
    for byte in body.as_bytes() {
        low ^= u64::from(*byte);
        low = low.wrapping_mul(0x0000_0100_0000_01b3);
        high = high.rotate_left(7) ^ low;
        high = high.wrapping_mul(0xff51_afd7_ed55_8ccd);
    }
    format!("{low:016x}{high:016x}")
}

/// Where a submission is kept, for a caller that wants to look.
///
/// Used by the tests to clear up after themselves; a running site never asks.
#[cfg(test)]
#[must_use]
pub fn root_of(archive: &Archive) -> &Path {
    &archive.root
}

#[cfg(test)]
mod tests;
