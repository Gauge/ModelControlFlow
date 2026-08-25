//! Tests for the restoration ledger.
//!
//! A27's test has one right answer, and it is about the worst moment rather
//! than the ordinary one: *if this run were interrupted at the worst possible
//! moment, could the machine be returned to how it was found?* So most of what
//! is tested here is the interrupted case — a ledger left on disk by a process
//! that never ran a destructor.

use std::path::PathBuf;

use super::{Change, Ledger};
use mcf_core::failure::Category;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("mcf-restore-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        let _made = std::fs::create_dir_all(&path);
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// The ordinary path: MCF changes something, finishes, and puts it back.
#[test]
fn what_was_changed_is_put_back() {
    let scratch = Scratch::new("ordinary");
    let subject = scratch.path("governor");
    std::fs::write(&subject, "powersave").expect("writable");

    {
        let (mut ledger, recovery) =
            Ledger::open(&scratch.path("ledger")).expect("the ledger opens");
        assert!(!recovery.found_anything());
        ledger
            .record(Change::about_to_replace(&subject).expect("readable"))
            .expect("the ledger records");
        std::fs::write(&subject, "performance").expect("writable");
        assert_eq!(ledger.pending().len(), 1);
    }

    assert_eq!(
        std::fs::read_to_string(&subject).expect("readable"),
        "powersave"
    );
}

/// A27's actual test: the process is killed, no destructor runs, and the next
/// one puts the machine back. Simulated by leaking the ledger, which is what a
/// killed process leaves behind.
#[test]
fn a_killed_process_leaves_a_machine_the_next_one_restores() {
    let scratch = Scratch::new("killed");
    let subject = scratch.path("governor");
    let ledger_path = scratch.path("ledger");
    std::fs::write(&subject, "powersave").expect("writable");

    {
        let (mut ledger, _) = Ledger::open(&ledger_path).expect("the ledger opens");
        ledger
            .record(Change::about_to_replace(&subject).expect("readable"))
            .expect("the ledger records");
        std::fs::write(&subject, "performance").expect("writable");
        // What a `kill -9` does: no destructor, no restoration, the ledger
        // still on disk.
        core::mem::forget(ledger);
    }

    assert_eq!(
        std::fs::read_to_string(&subject).expect("readable"),
        "performance",
        "the change should still be in place before recovery"
    );

    let (_ledger, recovery) = Ledger::open(&ledger_path).expect("the ledger reopens");
    assert!(recovery.found_anything());
    assert_eq!(recovery.restored.len(), 1);
    assert!(recovery.failed.is_empty());
    assert_eq!(
        std::fs::read_to_string(&subject).expect("readable"),
        "powersave"
    );
    assert!(
        recovery.statement().contains("1 restored"),
        "{}",
        recovery.statement()
    );
}

/// A file that did not exist is restored by being removed again, which is a
/// different restoration from writing an empty one.
#[test]
fn a_file_that_did_not_exist_is_removed_again() {
    let scratch = Scratch::new("created");
    let subject = scratch.path("new-file");
    let ledger_path = scratch.path("ledger");

    {
        let (mut ledger, _) = Ledger::open(&ledger_path).expect("the ledger opens");
        ledger
            .record(Change::about_to_replace(&subject).expect("an absent file is readable"))
            .expect("the ledger records");
        std::fs::write(&subject, "something MCF put here").expect("writable");
        core::mem::forget(ledger);
    }

    assert!(subject.exists());
    let (_ledger, recovery) = Ledger::open(&ledger_path).expect("the ledger reopens");
    assert_eq!(recovery.restored.len(), 1);
    assert!(!subject.exists(), "the file MCF created is still there");
}

/// Overlapping changes are undone newest first, so a file replaced twice comes
/// back to what it was before the first replacement.
#[test]
fn overlapping_changes_are_undone_newest_first() {
    let scratch = Scratch::new("overlapping");
    let subject = scratch.path("governor");
    let ledger_path = scratch.path("ledger");
    std::fs::write(&subject, "original").expect("writable");

    {
        let (mut ledger, _) = Ledger::open(&ledger_path).expect("the ledger opens");
        ledger
            .record(Change::about_to_replace(&subject).expect("readable"))
            .expect("records");
        std::fs::write(&subject, "first change").expect("writable");
        ledger
            .record(Change::about_to_replace(&subject).expect("readable"))
            .expect("records");
        std::fs::write(&subject, "second change").expect("writable");
        core::mem::forget(ledger);
    }

    let (_ledger, recovery) = Ledger::open(&ledger_path).expect("reopens");
    assert_eq!(recovery.restored.len(), 2);
    assert_eq!(
        std::fs::read_to_string(&subject).expect("readable"),
        "original"
    );
}

/// A27: what MCF cannot restore, it does not touch. A file it could not read
/// is refused *before* the change, not discovered afterwards.
#[test]
fn a_file_that_cannot_be_read_is_refused_before_it_is_changed() {
    let scratch = Scratch::new("unreadable");
    // A directory is present and cannot be read as a file.
    let failure = Change::about_to_replace(&scratch.0).expect_err("a directory is not readable");
    assert_eq!(failure.category(), Category::ArtifactUnreadable);
    assert!(failure.detail().contains("could not then put it back"));
}

/// A ledger entry this version does not understand is neither restored nor
/// discarded, and the ledger is not cleared — so the work is still there for a
/// version that does understand it (A1, §7.30).
#[test]
fn an_entry_from_another_version_is_kept_rather_than_discarded() {
    let scratch = Scratch::new("unknown");
    let ledger_path = scratch.path("ledger");
    std::fs::write(
        &ledger_path,
        "{\"change\":\"governor_set\",\"cpu\":0,\"previous\":\"powersave\"}\n",
    )
    .expect("writable");

    let (_ledger, recovery) = Ledger::open(&ledger_path).expect("the ledger opens");
    assert_eq!(recovery.unreadable, 1);
    assert!(recovery.restored.is_empty());
    assert!(
        ledger_path.exists(),
        "the ledger was cleared with work still in it"
    );
    assert!(
        recovery.statement().contains("cannot read"),
        "{}",
        recovery.statement()
    );
}

/// Nothing left behind is the ordinary case, and it says so rather than saying
/// nothing.
#[test]
fn a_clean_start_says_it_found_nothing() {
    let scratch = Scratch::new("clean");
    let (_ledger, recovery) = Ledger::open(&scratch.path("ledger")).expect("opens");
    assert!(!recovery.found_anything());
    assert_eq!(recovery.statement(), "nothing was left to restore");
}
