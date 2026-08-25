//! Scenarios about an artifact leaving the machine.
//!
//! §3.11 makes reclaiming space a decision rather than a surprise, and B-027
//! builds that as four separate acts. These are the two ways the discipline is
//! observed from outside: an authorization that does not match what is in front
//! of it, and a shelf that will not empty.
//!
//! Each constructs the observable and not its cause (D26): the stale
//! authorization is a file that changed size, not a race; the immovable shelf
//! is a directory the kernel refuses to write, not a failing disk.

use std::path::PathBuf;

use mcf_core::failure::Category;
use mcf_core::time::Timestamp;
use mcf_hub::store::{Authorization, preview, purge, remove};
use mcf_record::journal::Journal;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The artifact changed between the looking and the deciding.
pub(super) const AUTHORIZATION_IS_STALE: Scenario = Scenario {
    id: "store/authorization-is-stale",
    produces: Category::ConfigInvalid,
    summary: "a removal authorized against one plan is refused against a different one",
    run: authorization_is_stale,
};

/// What was shelved cannot be deleted.
pub(super) const SHELF_WILL_NOT_EMPTY: Scenario = Scenario {
    id: "store/shelf-will-not-empty",
    produces: Category::ResourceDiskReadonly,
    summary: "a purge that cannot delete says what it freed and what is stuck",
    run: shelf_will_not_empty,
};

fn authorization_is_stale(world: &World) -> Outcome {
    let model = world.path("model.gguf");
    if let Err(error) = std::fs::write(&model, b"weights") {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    let shelf = world.path("shelf");
    let Ok(plan) = preview(std::slice::from_ref(&model), &shelf) else {
        return Outcome::Unexpected("the plan could not be made".to_owned());
    };
    let Ok(authorization) = Authorization::given(&plan, "making room for the Q6") else {
        return Outcome::Unexpected("the authorization was refused".to_owned());
    };

    // The artifact grows under the decision. What would be removed is no longer
    // what somebody looked at.
    if let Err(error) = std::fs::write(&model, vec![b'w'; 4096]) {
        return Outcome::Unexpected(format!("could not grow the artifact: {error}"));
    }
    let Ok(now_a_different_removal) = preview(std::slice::from_ref(&model), &shelf) else {
        return Outcome::Unexpected("the second plan could not be made".to_owned());
    };
    let Ok(mut journal) = Journal::open(&world.path("record.jsonl")) else {
        return Outcome::Unexpected("the journal did not open".to_owned());
    };

    match remove(
        &now_a_different_removal,
        &authorization,
        &mut journal,
        Timestamp::now(),
    ) {
        Err(failure) if model.exists() => Outcome::Produced(failure),
        Err(_) => Outcome::Unexpected("the removal was refused and removed it anyway".to_owned()),
        Ok(_) => Outcome::Unexpected("a stale authorization removed an artifact".to_owned()),
    }
}

fn shelf_will_not_empty(world: &World) -> Outcome {
    // `/proc` refuses every write on every Linux machine, so a shelf under it
    // is one nothing can delete from. The observation is *the artifact is still
    // there after a purge*, which is what MCF has to report honestly; a full or
    // read-only volume is the cause and stays out of here (D26).
    let stuck = PathBuf::from("/proc/mcf-lab-nothing-deletes-here/model.gguf");
    let model = world.path("model.gguf");
    if let Err(error) = std::fs::write(&model, b"weights") {
        return Outcome::Unexpected(format!("could not write the artifact: {error}"));
    }
    let Ok(plan) = preview(std::slice::from_ref(&model), &world.path("shelf")) else {
        return Outcome::Unexpected("the plan could not be made".to_owned());
    };
    let Ok(authorization) = Authorization::given(&plan, "done with it") else {
        return Outcome::Unexpected("the authorization was refused".to_owned());
    };

    // A removal whose shelf is somewhere nothing can be deleted from: the
    // artifact is described as shelved there and the purge has to say it did
    // not go.
    let removed = mcf_hub::store::Removed {
        shelved: vec![stuck],
        refused: Vec::new(),
        bytes: 7,
        reversible: false,
    };
    match purge(&removed, &authorization, &plan) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a file under /proc was deleted".to_owned()),
    }
}
