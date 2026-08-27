//! Every scenario, in one table.
//!
//! B32 refuses a lab API, a third-party lab, a discovery mechanism and a
//! configuration language. This is what remains: a constant. Adding a scenario
//! is editing this file, and each is admitted by answering B32's question —
//! *what claim can MCF make once this scenario exists that it cannot now?* The
//! answer is always the same one here, and it is A13's: MCF can claim to handle
//! the failure this produces.
//!
//! The catalogue is checked against the code rather than against the taxonomy's
//! full table. D26 binds A13 to what MCF's own code constructs: a category the
//! code can produce needs a scenario, and one nothing produces yet is a
//! classification waiting for its code. `checks/tests/fault_catalogue.rs` is
//! that check (B-010).

use mcf_core::failure::Category;

use super::scenario::Scenario;

mod artifact;
mod engine;
mod environment;
mod exchange;
mod hub;
mod platform;
mod record;
mod resource;
mod serve;
mod store;
mod time;

/// Every scenario the laboratory has.
pub const CATALOGUE: &[Scenario] = &[
    artifact::CORRUPTED,
    artifact::MISSING,
    artifact::UNREADABLE,
    artifact::FORMAT_UNSUPPORTED,
    artifact::FORMAT_MALFORMED,
    artifact::PROVENANCE_INCOMPLETE,
    engine::NO_VENDORED_ENGINE,
    resource::MODEL_LARGER_THAN_MEMORY,
    hub::REFERENCE_IS_NOT_ONE,
    hub::NEEDS_CREDENTIALS,
    hub::CREDENTIAL_REFUSED,
    hub::GATED,
    hub::RATE_LIMITED,
    hub::DECEPTIVE_METADATA,
    hub::NO_LICENCE,
    hub::TRUNCATED_TRANSFER,
    hub::CANNOT_RESUME,
    hub::ANSWER_IS_NOT_A_RESPONSE,
    hub::ANSWER_CUT_SHORT,
    hub::ANSWER_NEVER_COMES,
    hub::RESUMPTION_RESTARTED,
    hub::NO_WAY_TO_ENCRYPT,
    hub::NOT_A_TLS_HOST,
    hub::NO_ROOM_ON_THE_DISK,
    environment::KILLED_AFTER_CHANGING,
    environment::LEDGER_UNWRITABLE,
    exchange::TRUNCATED_BUNDLE,
    exchange::UNREADABLE_BUNDLE,
    platform::PRIVILEGE_DENIED,
    platform::MECHANISM_UNAVAILABLE,
    platform::PRIVILEGE_UNAVAILABLE,
    record::UNWRITABLE_PATH,
    record::UNKNOWN_FORMAT,
    record::TORN_LAST_LINE,
    record::CORRUPT_LINE,
    record::HEADERLESS,
    serve::TWO_DAEMONS,
    store::AUTHORIZATION_IS_STALE,
    store::SHELF_WILL_NOT_EMPTY,
    time::BACKWARD_STEP,
    time::FORWARD_JUMP,
];

/// The scenarios that produce a category.
///
/// A category may have more than one: `record.corrupt.journal` arises from a
/// line that is not JSON and from a file whose first line is not a header, and
/// those are different failures wearing one code. Returning all of them is what
/// lets a reader see which one a record is talking about.
#[must_use]
pub fn find(category: Category) -> Vec<&'static Scenario> {
    CATALOGUE
        .iter()
        .filter(|scenario| scenario.produces == category)
        .collect()
}
