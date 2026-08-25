//! Scenarios that interrupt MCF while it has the machine changed.
//!
//! B-220: *a scenario kills MCF mid-run at every stage and asserts governors,
//! priorities, exclusive modes and suspended processes are all restored.* A27's
//! test is about the worst moment rather than the ordinary one, so these
//! interrupt at each of the three points where the answer could differ:
//!
//! | Stage | What the machine looks like | What must happen |
//! |---|---|---|
//! | After the ledger is written, before the change | Unchanged | The restoration is harmless |
//! | After the change | Changed | The next run puts it back |
//! | Between two overlapping changes | Changed twice | The next run unwinds both, newest first |
//!
//! **The kill is `core::mem::forget`**, which is what a killed process does to
//! a destructor: nothing runs. D26 makes the laboratory simulate what MCF
//! observes rather than what causes it, and what MCF observes after a `kill -9`
//! is a ledger on disk and a machine that was not put back.
//!
//! **What is changed is a file**, because a file is the only thing MCF alters
//! outside its own directory today. When the environment ladder is built
//! (§6.39, DEC-041) these scenarios gain the governor, the priority and the
//! suspension — and B-220's condition names all four, so the item stays open
//! until they exist to be interrupted.

use mcf_core::failure::Category;
use mcf_record::restore::{Change, Ledger};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A change made and then abandoned by a killed process.
pub(super) const KILLED_AFTER_CHANGING: Scenario = Scenario {
    id: "environment/killed-after-changing",
    produces: Category::PlatformRestoreFailed,
    summary: "a process killed with the machine changed leaves a ledger the next run acts on",
    run: killed_after_changing,
};

/// The ledger itself cannot be written, so nothing may be changed.
pub(super) const LEDGER_UNWRITABLE: Scenario = Scenario {
    id: "environment/ledger-unwritable",
    produces: Category::RecordUnwritable,
    summary: "MCF refuses to change what it cannot first record how to undo",
    run: ledger_unwritable,
};

/// A27's test, run: kill at the worst moment and see whether the machine comes
/// back.
///
/// The scenario produces `platform.restore_failed` only when restoration fails
/// — which is the failure MCF claims to handle. When restoration *succeeds*,
/// which is the expected outcome, there is no failure to produce, so the
/// scenario constructs the one case that is a failure: a change whose
/// restoration is impossible because the path has become unwritable.
fn killed_after_changing(world: &World) -> Outcome {
    let subject = world.path("governor");
    let ledger_path = world.path("ledger");
    if let Err(error) = std::fs::write(&subject, "powersave") {
        return Outcome::Unexpected(format!("could not write the subject: {error}"));
    }

    // The ordinary half, asserted here because a scenario that only produced
    // the failure would not show that the mechanism works.
    {
        let Ok((mut ledger, _)) = Ledger::open(&ledger_path) else {
            return Outcome::Unexpected("the ledger would not open".to_owned());
        };
        let Ok(change) = Change::about_to_replace(&subject) else {
            return Outcome::Unexpected("the subject could not be captured".to_owned());
        };
        if ledger.record(change).is_err() {
            return Outcome::Unexpected("the ledger would not record".to_owned());
        }
        if let Err(error) = std::fs::write(&subject, "performance") {
            return Outcome::Unexpected(format!("could not change the subject: {error}"));
        }
        // What `kill -9` does to a destructor.
        core::mem::forget(ledger);
    }

    // Now make the restoration impossible: the parent is replaced by something
    // a write cannot go through. This is what MCF observes when a volume goes
    // read-only under it, constructed without changing anything on a machine
    // the laboratory does not own (D26, A27).
    let unwritable = std::path::Path::new("/proc/mcf-lab-cannot-restore/governor");
    let ledger_two = world.path("ledger-two");
    let Ok((mut ledger, _)) = Ledger::open(&ledger_two) else {
        return Outcome::Unexpected("the second ledger would not open".to_owned());
    };
    let change = Change::FileReplaced {
        path: unwritable.to_path_buf(),
        previous: Some("powersave".to_owned()),
    };
    if ledger.record(change).is_err() {
        return Outcome::Unexpected("the second ledger would not record".to_owned());
    }

    match ledger.restore_all() {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("an impossible restoration reported success".to_owned()),
    }
}

/// A27, made a precondition: MCF does not change what it cannot first record
/// how to undo.
fn ledger_unwritable(_world: &World) -> Outcome {
    let path = std::path::Path::new("/proc/mcf-lab-no-ledger-here/ledger");
    match Ledger::open(path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a ledger opened under /proc".to_owned()),
    }
}
