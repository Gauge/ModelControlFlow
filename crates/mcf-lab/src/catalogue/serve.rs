//! Scenarios about the daemon.
//!
//! D1 makes MCF a process with clients attached, and §3.1's reliability rules
//! are mostly about that process: it survives, it recovers, and nothing a
//! client does takes it down. These are the failures it produces on the way to
//! being one.
//!
//! Each builds the observable and not its cause (D26): *something is already
//! listening* is a socket with a daemon behind it, not a stale PID file or a
//! misread lock.

use mcf_core::failure::Category;
use mcf_serve::daemon::{Daemon, Places};

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A second daemon, where one is already running.
pub(super) const TWO_DAEMONS: Scenario = Scenario {
    id: "serve/two-daemons",
    produces: Category::ConfigConflict,
    summary: "a second daemon on one socket is refused, because two would share one record",
    run: two_daemons,
};

/// Starting MCF where MCF is already running.
///
/// The observation is *something answers on that socket*, which is what a
/// daemon started twice looks like from the second one's side. What makes it
/// worth refusing rather than joining is D20: the journal is the record, and
/// two processes appending to one record are two processes whose account of
/// what happened is neither's.
fn two_daemons(world: &World) -> Outcome {
    let places = Places {
        socket: world.path("control.sock"),
        journal: world.path("record.jsonl"),
        models: world.path("models"),
    };
    // The first is not served: binding is what makes something answer, and a
    // scenario that ran an accept loop would be testing a thread rather than
    // the refusal.
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
