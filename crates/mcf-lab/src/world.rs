//! The world a scenario runs in.
//!
//! Two things a scenario needs and must not reach for itself: a clock, and
//! somewhere to build the conditions it is reproducing.
//!
//! **The clock is supplied** (D26). A scenario about a deadline states that
//! time passed; it does not wait for it. The type is [`SimulatedClock`], whose
//! durations are a different type from the monotonic clock's, so a timing taken
//! here cannot become a performance number (A11, B37).
//!
//! **The scratch directory is unique per run, and elided from the outcome.**
//! Both halves are needed and the first draft had only one. A directory shared
//! between runs collides the moment two of them overlap — which the laboratory's
//! own suite does, running scenarios on several threads — and a directory that
//! differs between runs puts a different path into every failure's context, so
//! a hundred identical failures compare as a hundred different ones. The
//! resolution is that **the location is environment and not part of the
//! failure**: each run gets its own directory, and [`World::elide`] replaces it
//! with a placeholder before outcomes are compared. What remains is the failure
//! itself, which is what §3.17 asks to reproduce.
//!
//! **A scenario leaves nothing behind** (B58, A27). The directory is cleared on
//! the way in as well as on the way out, so a scenario killed mid-run does not
//! contaminate the next one — which is the case a teardown-only discipline
//! misses.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mcf_core::time::SimulatedClock;

/// Distinguishes worlds within one process, so two runs of one scenario never
/// share a directory.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// The placeholder a scratch path is replaced with before outcomes are
/// compared.
pub const SCRATCH_PLACEHOLDER: &str = "<scratch>";

/// Where a scenario builds what it is reproducing.
#[derive(Debug)]
pub struct World {
    scratch: PathBuf,
    clock: SimulatedClock,
}

impl World {
    /// A world for one run of one scenario.
    ///
    /// The directory names the scenario, the process and this run, so two runs
    /// never share one — including two on different threads, which is the case
    /// the laboratory's own suite produces.
    #[must_use]
    pub fn for_scenario(id: &str) -> Self {
        let safe: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let run = NEXT.fetch_add(1, Ordering::Relaxed);
        let scratch =
            std::env::temp_dir().join(format!("mcf-lab-{}-{safe}-{run}", std::process::id()));
        // Cleared on the way in: a scenario killed mid-run left residue, and
        // the next run must not see it (B58).
        let _cleared = std::fs::remove_dir_all(&scratch);
        let _made = std::fs::create_dir_all(&scratch);
        Self {
            scratch,
            clock: SimulatedClock::new(),
        }
    }

    /// The directory this scenario may build in.
    #[must_use]
    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    /// A path inside it.
    #[must_use]
    pub fn path(&self, name: &str) -> PathBuf {
        self.scratch.join(name)
    }

    /// The same text with this world's scratch path replaced by
    /// [`SCRATCH_PLACEHOLDER`].
    ///
    /// Where a run happened is environment; *what happened* is the failure.
    /// Eliding the first is what lets two runs of one scenario be compared for
    /// the second (§3.17).
    #[must_use]
    pub fn elide(&self, text: &str) -> String {
        text.replace(self.scratch.to_string_lossy().as_ref(), SCRATCH_PLACEHOLDER)
    }

    /// The scenario's clock.
    ///
    /// It moves only when a scenario moves it, which is what makes an outcome
    /// that depends on time reproducible (§3.17, B27).
    #[must_use]
    pub const fn clock(&self) -> &SimulatedClock {
        &self.clock
    }
}

impl Drop for World {
    /// A27: what the laboratory changed, it restores.
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.scratch);
    }
}
