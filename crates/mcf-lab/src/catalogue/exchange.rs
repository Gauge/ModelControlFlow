//! Scenarios in which a bundle does not survive leaving the machine.
//!
//! D20 makes export one portable file, and §XIV will send those files between
//! machines. What MCF claims to handle is that a bundle which arrived damaged,
//! or which was written by a version this one cannot read, is **refused rather
//! than read short** — the failure being that a bundle with rows missing looks
//! exactly like a smaller bundle.
//!
//! What is simulated is the observation (D26): a file with a line removed, and
//! a file whose header names a format nobody here implements. MCF cannot
//! simulate a truncating transfer and does not need to.

use mcf_core::failure::Category;
use mcf_record::export::{FORMAT_VERSION, Kind, read, write};
use mcf_record::journal::{Entry, EntryKind, Journal, Writer};
use mcf_record::json::Value;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A bundle that lost a row on the way.
pub(super) const TRUNCATED_BUNDLE: Scenario = Scenario {
    id: "exchange/truncated-bundle",
    produces: Category::ArtifactCorrupt,
    summary: "a bundle with a row missing is refused rather than read as a smaller one",
    run: truncated_bundle,
};

/// A bundle from a format this build does not implement.
pub(super) const UNREADABLE_BUNDLE: Scenario = Scenario {
    id: "exchange/unreadable-bundle",
    produces: Category::ExchangeSchemaUnreadable,
    summary: "a bundle written by a later format is refused by name, not partially read",
    run: unreadable_bundle,
};

fn a_bundle_of(world: &World, entries: u64) -> Result<std::path::PathBuf, Outcome> {
    let journal = world.path("record.jsonl");
    {
        // A stated writer, because §3.17 wants a scenario to reproduce byte
        // for byte and an identifier carries who wrote it (DEC-037). Everything
        // outside a laboratory takes a distinct writer it did not choose.
        let mut open = Journal::open(&journal)
            .map_err(|failure| Outcome::Unexpected(format!("no journal: {failure}")))?
            .writing_as(Writer::stated("labbed01"));
        for sequence in 0..entries {
            open.append(&Entry::new(
                EntryKind::MachineProfile,
                mcf_core::time::Timestamp::from_utc_nanos(
                    1_756_058_651_442_000_000,
                    mcf_core::attested::Attested::Unknown,
                ),
                Value::map([("n", Value::Integer(i64::try_from(sequence).unwrap_or(0)))]),
            ))
            .map_err(|failure| Outcome::Unexpected(format!("no entry: {failure}")))?;
        }
    }
    let bundle = world.path("bundle.mcf");
    write(&journal, &bundle, Kind::Export)
        .map_err(|failure| Outcome::Unexpected(format!("no bundle: {failure}")))?;
    Ok(bundle)
}

fn truncated_bundle(world: &World) -> Outcome {
    let bundle = match a_bundle_of(world, 4) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };

    let Ok(text) = std::fs::read_to_string(&bundle) else {
        return Outcome::Unexpected("the bundle could not be read back".to_owned());
    };
    let kept: Vec<&str> = text.lines().take(text.lines().count() - 1).collect();
    if std::fs::write(&bundle, kept.join("\n") + "\n").is_err() {
        return Outcome::Unexpected("the bundle could not be truncated".to_owned());
    }

    match read(&bundle) {
        Err(failure) => Outcome::Produced(failure),
        Ok((_, _, entries)) => Outcome::Unexpected(format!(
            "a truncated bundle verified, with {} entries",
            entries.len()
        )),
    }
}

fn unreadable_bundle(world: &World) -> Outcome {
    let bundle = world.path("later.mcf");
    let header = Value::map([
        ("format", Value::Integer(FORMAT_VERSION + 1)),
        ("kind", Value::text("export")),
    ]);
    if std::fs::write(&bundle, header.to_line() + "\n").is_err() {
        return Outcome::Unexpected("the bundle could not be written".to_owned());
    }

    match read(&bundle) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a later format was accepted".to_owned()),
    }
}
