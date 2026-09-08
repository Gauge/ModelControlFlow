use mcf_core::failure::Category;
use mcf_record::export::{FORMAT_VERSION, Kind, read, write};
use mcf_record::journal::{Entry, EntryKind, Journal, Writer};
use mcf_record::json::Value;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const TRUNCATED_BUNDLE: Scenario = Scenario {
    id: "exchange/truncated-bundle",
    produces: Category::ArtifactCorrupt,
    summary: "a bundle with a row missing is refused rather than read as a smaller one",
    run: truncated_bundle,
};

pub(super) const PIN_DIVERGED: Scenario = Scenario {
    id: "exchange/pin-diverged",
    produces: Category::ExchangeReproduceDivergent,
    summary: "a checkout that did not land on its pin is refused with both hashes named, \
              rather than recorded as the pin",
    run: pin_diverged,
};

fn pin_diverged(_world: &World) -> Outcome {
    match mcf_core::provenance::checked_out(
        "925e1179947ea0c0ebfb0032df18af3a729822be",
        "0000000000000000000000000000000000000000\n",
    ) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected("two different hashes were judged the same pin".to_owned()),
    }
}

pub(super) const UNREADABLE_BUNDLE: Scenario = Scenario {
    id: "exchange/unreadable-bundle",
    produces: Category::ExchangeSchemaUnreadable,
    summary: "a bundle written by a later format is refused by name, not partially read",
    run: unreadable_bundle,
};

fn a_bundle_of(world: &World, entries: u64) -> Result<std::path::PathBuf, Outcome> {
    let journal = world.path("record.jsonl");
    {
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
