use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Held {
    pub digest: String,
    pub at: u64,
    pub body: String,
}

impl Held {
    #[must_use]
    pub fn rows(&self) -> Vec<&str> {
        self.body.lines().filter(|line| is_a_row(line)).collect()
    }

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

#[must_use]
pub fn is_a_row(line: &str) -> bool {
    let Some((kind, rest)) = line.split_once('·') else {
        return false;
    };
    matches!(kind.trim(), "comparison" | "absolute") && !rest.trim().is_empty()
}

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

#[derive(Debug, Default)]
pub struct Archive {
    root: PathBuf,
}

impl Archive {
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let _made = std::fs::create_dir_all(&root);
        Self { root }
    }

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
        if path.exists() {
            return Ok(held.digest);
        }
        std::fs::write(&path, format!("{}\n{}", held.at, body))
            .map_err(|error| format!("it could not be filed: {error}"))?;
        Ok(held.digest)
    }

    #[must_use]
    pub fn one(&self, digest: &str) -> Option<Held> {
        if !digest.chars().all(|c| c.is_ascii_hexdigit()) || digest.len() != 32 {
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

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |held| held.as_secs())
}

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

#[cfg(test)]
#[must_use]
pub fn root_of(archive: &Archive) -> &Path {
    &archive.root
}

#[cfg(test)]
mod tests;
