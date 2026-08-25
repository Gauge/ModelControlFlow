//! A directory a tier may build in, and that it takes with it when it goes.
//!
//! The property, fuzz, load and soak tiers all need somewhere to put a real
//! journal, because none of them is testing a mock: B62's loss reporting is a
//! statement about a file on a disk, and a fake file system would test the
//! fake. What they must not do is leave anything behind — A27 applies to MCF's
//! own suite as much as to a measurement run, and B58's lesson from the
//! laboratory is that clearing on the way *in* matters as much as on the way
//! out, because a run that was killed had no way out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes scratch directories within one process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A directory that exists for as long as this value does.
#[derive(Debug)]
pub struct Scratch(PathBuf);

impl Scratch {
    /// A fresh directory, named for the tier that asked for it.
    ///
    /// Cleared on the way in as well as out, so a previous run that was killed
    /// cannot contaminate this one (B58).
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

    /// The directory itself.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// A path inside it.
    #[must_use]
    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// The conventional journal path inside it.
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
