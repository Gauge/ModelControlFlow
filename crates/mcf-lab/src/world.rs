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
//!
//! **And a scenario killed mid-run leaves nothing behind *either*, which took a
//! second look.** Clearing on the way in only clears *this* world's directory,
//! and a world's name carries the process that made it — so a killed process's
//! directories were orphaned, and no later run would ever name them. Forty-five
//! of them were found on the machine this was written on after a morning of
//! killing hung test processes. A27's test has one right answer — *if this run
//! were interrupted at the worst possible moment, could the machine be returned
//! to how it was found?* — and the honest answer was no.
//!
//! So each new world also sweeps the residue of worlds whose process is gone.
//! It sweeps only what it can prove is dead: the owning process id is in the
//! name, and a directory whose process still exists is left alone however old
//! it looks. Where the platform will not say which processes exist, nothing is
//! swept (A7: what MCF cannot determine, it does not act on).

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
        sweep_dead_worlds();
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

/// The prefix every world's directory carries.
const PREFIX: &str = "mcf-lab-";

/// Removes the scratch directories of laboratory processes that are gone.
///
/// Called once per world rather than once per process, which is cheap — a
/// directory listing of the temporary directory — and which means a suite that
/// is killed has its residue cleared by the *next* suite rather than never.
///
/// It removes only what it can prove is dead. A directory whose owning process
/// still exists is left alone however old it looks: two suites run at once on
/// this machine (`heavy` serializes the heavy ones, not the quick ones), and
/// deleting a live run's scratch would be the laboratory corrupting a
/// measurement instead of taking one.
fn sweep_dead_worlds() {
    // Once per process. A hundred scenarios in one run should not list the
    // temporary directory a hundred times, and anything that died after this
    // process started is not this process's residue to clear.
    static SWEPT: std::sync::Once = std::sync::Once::new();
    SWEPT.call_once(|| {
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(rest) = name.strip_prefix(PREFIX) else {
                continue;
            };
            let Some((owner, _)) = rest.split_once('-') else {
                continue;
            };
            let Ok(owner) = owner.parse::<u32>() else {
                continue;
            };
            if owner == std::process::id() {
                continue;
            }
            if process_exists(owner) != Some(false) {
                // Alive, or unknowable. Either way not this run's to remove.
                continue;
            }
            let _removed = std::fs::remove_dir_all(entry.path());
        }
    });
}

/// The sweep itself, without the once-per-process guard.
///
/// Exists for the test that checks the *rule* — a dead process's residue goes,
/// a live one's stays — which cannot use the guarded entry point because this
/// process has already swept.
#[cfg(test)]
fn sweep_for_test() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(rest) = name.strip_prefix(PREFIX) else {
            continue;
        };
        let Some((owner, _)) = rest.split_once('-') else {
            continue;
        };
        let Ok(owner) = owner.parse::<u32>() else {
            continue;
        };
        if owner == std::process::id() || process_exists(owner) != Some(false) {
            continue;
        }
        let _removed = std::fs::remove_dir_all(entry.path());
    }
}

/// Whether a process exists, where the platform publishes that.
///
/// `None` rather than a guess where it does not: A7's habit applied to a
/// question whose wrong answer deletes somebody else's working directory.
fn process_exists(pid: u32) -> Option<bool> {
    let proc = Path::new("/proc");
    if !proc.is_dir() {
        return None;
    }
    Some(proc.join(pid.to_string()).exists())
}

#[cfg(test)]
mod tests {
    use super::{PREFIX, World, process_exists, sweep_dead_worlds};

    /// The residue of a process that is gone is swept; the residue of one that
    /// is alive is not.
    ///
    /// The second half is the one that matters. Two suites can run at once, and
    /// a sweep that deleted a live run's scratch would be the laboratory
    /// corrupting a measurement instead of taking one.
    #[test]
    fn a_dead_world_is_swept_and_a_live_one_is_left_alone() {
        if process_exists(std::process::id()) != Some(true) {
            println!("  this platform does not publish its processes, so nothing is swept");
            return;
        }

        // A process identifier that certainly does not exist: the kernel's
        // maximum plus one, which nothing can be assigned.
        let dead = std::fs::read_to_string("/proc/sys/kernel/pid_max")
            .ok()
            .and_then(|text| text.trim().parse::<u32>().ok())
            .map_or(4_194_303, |maximum| maximum.saturating_add(1));
        assert_eq!(process_exists(dead), Some(false), "{dead} should not exist");

        let temporary = std::env::temp_dir();
        let orphan = temporary.join(format!("{PREFIX}{dead}-sweep-test-0"));
        let mine = temporary.join(format!("{PREFIX}{}-sweep-test-0", std::process::id()));
        std::fs::create_dir_all(&orphan).expect("a scratch directory");
        std::fs::create_dir_all(&mine).expect("a scratch directory");

        // The sweep runs once per process and this process has already made
        // worlds, so it is called directly rather than through `for_scenario`.
        // What is being tested is the rule, not the once-ness.
        super::sweep_for_test();

        assert!(!orphan.exists(), "a dead process's residue was left behind");
        assert!(
            mine.exists(),
            "a live process's scratch was deleted, which is a measurement corrupted"
        );
        let _cleaned = std::fs::remove_dir_all(&mine);
    }

    /// A world still cleans up after itself in the ordinary case, which the
    /// sweep does not replace.
    #[test]
    fn a_world_that_ends_normally_still_removes_itself() {
        let path = {
            let world = World::for_scenario("ends-normally");
            let path = world.scratch().to_path_buf();
            assert!(path.exists());
            path
        };
        assert!(!path.exists(), "the drop did not remove it");
    }

    /// The sweep is safe to call when there is nothing to sweep.
    #[test]
    fn sweeping_an_empty_machine_does_nothing() {
        sweep_dead_worlds();
        sweep_dead_worlds();
    }
}
