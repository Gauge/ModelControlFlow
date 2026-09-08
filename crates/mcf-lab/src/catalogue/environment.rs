use mcf_core::failure::Category;
use mcf_record::restore::{Change, Ledger};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const KILLED_AFTER_CHANGING: Scenario = Scenario {
    id: "environment/killed-after-changing",
    produces: Category::PlatformRestoreFailed,
    summary: "a process killed with the machine changed leaves a ledger the next run acts on",
    run: killed_after_changing,
};

pub(super) const LEDGER_UNWRITABLE: Scenario = Scenario {
    id: "environment/ledger-unwritable",
    produces: Category::RecordUnwritable,
    summary: "MCF refuses to change what it cannot first record how to undo",
    run: ledger_unwritable,
};

fn killed_after_changing(world: &World) -> Outcome {
    let subject = world.path("governor");
    let ledger_path = world.path("ledger");
    if let Err(error) = std::fs::write(&subject, "powersave") {
        return Outcome::Unexpected(format!("could not write the subject: {error}"));
    }

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
        core::mem::forget(ledger);
    }

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

fn ledger_unwritable(_world: &World) -> Outcome {
    let path = std::path::Path::new("/proc/mcf-lab-no-ledger-here/ledger");
    match Ledger::open(path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a ledger opened under /proc".to_owned()),
    }
}
