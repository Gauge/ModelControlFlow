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

pub const CATALOGUE: &[Scenario] = &[
    artifact::CORRUPTED,
    artifact::MISSING,
    artifact::UNREADABLE,
    artifact::FORMAT_UNSUPPORTED,
    artifact::FORMAT_MALFORMED,
    artifact::PROVENANCE_INCOMPLETE,
    engine::NO_VENDORED_ENGINE,
    engine::ENGINE_NOT_FOUND,
    engine::ENGINE_EXIT_IMMEDIATE,
    engine::ENGINE_EXIT_MIDSTREAM,
    engine::ENGINE_EXIT_SIGNAL,
    engine::ENGINE_SPAWN_REFUSED,
    engine::TWO_ENGINES_PROVISIONED,
    engine::SERVER_NEVER_LISTENS,
    engine::SERVER_ANSWER_UNREADABLE,
    engine::NOTHING_TO_CROSS_CHECK,
    engine::CLIENT_LEFT_MIDSTREAM,
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
    exchange::PIN_DIVERGED,
    exchange::UNREADABLE_BUNDLE,
    platform::PRIVILEGE_DENIED,
    platform::MECHANISM_UNAVAILABLE,
    platform::PRIVILEGE_UNAVAILABLE,
    record::UNWRITABLE_PATH,
    record::UNKNOWN_FORMAT,
    record::TORN_LAST_LINE,
    record::CORRUPT_LINE,
    record::HEADERLESS,
    record::CONTENT_UNREADABLE,
    record::CONTENT_KEY_REFUSED,
    serve::TWO_DAEMONS,
    store::AUTHORIZATION_IS_STALE,
    store::SHELF_WILL_NOT_EMPTY,
    time::BACKWARD_STEP,
    time::FORWARD_JUMP,
];

#[must_use]
pub fn find(category: Category) -> Vec<&'static Scenario> {
    CATALOGUE
        .iter()
        .filter(|scenario| scenario.produces == category)
        .collect()
}
