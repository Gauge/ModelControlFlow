use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mcf_core::time::SimulatedClock;

static NEXT: AtomicU64 = AtomicU64::new(0);

pub const SCRATCH_PLACEHOLDER: &str = "<scratch>";

#[derive(Debug)]
pub struct World {
    scratch: PathBuf,
    clock: SimulatedClock,
}

impl World {
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
        let _cleared = std::fs::remove_dir_all(&scratch);
        let _made = std::fs::create_dir_all(&scratch);
        Self {
            scratch,
            clock: SimulatedClock::new(),
        }
    }

    #[must_use]
    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    #[must_use]
    pub fn path(&self, name: &str) -> PathBuf {
        self.scratch.join(name)
    }

    #[must_use]
    pub fn elide(&self, text: &str) -> String {
        text.replace(self.scratch.to_string_lossy().as_ref(), SCRATCH_PLACEHOLDER)
    }

    #[must_use]
    pub const fn clock(&self) -> &SimulatedClock {
        &self.clock
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.scratch);
    }
}

const PREFIX: &str = "mcf-lab-";

fn sweep_dead_worlds() {
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
                continue;
            }
            let _removed = std::fs::remove_dir_all(entry.path());
        }
    });
}

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

    #[test]
    fn a_dead_world_is_swept_and_a_live_one_is_left_alone() {
        if process_exists(std::process::id()) != Some(true) {
            println!("  this platform does not publish its processes, so nothing is swept");
            return;
        }

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

        super::sweep_for_test();

        assert!(!orphan.exists(), "a dead process's residue was left behind");
        assert!(
            mine.exists(),
            "a live process's scratch was deleted, which is a measurement corrupted"
        );
        let _cleaned = std::fs::remove_dir_all(&mine);
    }

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

    #[test]
    fn sweeping_an_empty_machine_does_nothing() {
        sweep_dead_worlds();
        sweep_dead_worlds();
    }
}
