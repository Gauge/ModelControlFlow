use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct Scratch(PathBuf);

impl Scratch {
    #[must_use]
    pub fn new(tier: &str) -> Self {
        let safe: String = tier
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let run = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("mcf-tier-{}-{safe}-{run}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        let _made = std::fs::create_dir_all(&path);
        Self(path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    #[must_use]
    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    #[must_use]
    pub fn journal(&self) -> PathBuf {
        self.join("record.jsonl")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}
