use mcf_core::failure::Category;
use mcf_serve::daemon::{Daemon, Places};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const TWO_DAEMONS: Scenario = Scenario {
    id: "serve/two-daemons",
    produces: Category::ConfigConflict,
    summary: "a second daemon on one socket is refused, because two would share one record",
    run: two_daemons,
};

fn two_daemons(world: &World) -> Outcome {
    let places = Places {
        socket: world.path("control.sock"),
        journal: world.path("record.jsonl"),
        models: world.path("models"),
    };
    let Ok(first) = Daemon::start(places.clone()) else {
        return Outcome::Unexpected("the first daemon did not start".to_owned());
    };

    let outcome = match Daemon::start(places) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("two daemons started on one socket".to_owned()),
    };
    drop(first);
    outcome
}
