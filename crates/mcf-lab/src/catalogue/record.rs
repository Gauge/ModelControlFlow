use mcf_core::failure::Category;
use mcf_core::time::Timestamp;
use mcf_record::content::{Content, ContentStore};
use mcf_record::journal::{Entry, EntryKind, FORMAT_VERSION, Journal, Writer, replay};
use mcf_record::json::Value;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const UNWRITABLE_PATH: Scenario = Scenario {
    id: "record/unwritable-path",
    produces: Category::RecordUnwritable,
    summary: "the record's directory cannot be created, so nothing can be written",
    run: unwritable_path,
};

pub(super) const UNKNOWN_FORMAT: Scenario = Scenario {
    id: "record/unknown-format",
    produces: Category::RecordSchemaUnknown,
    summary: "a journal from a later format version is refused rather than appended to",
    run: unknown_format,
};

pub(super) const TORN_LAST_LINE: Scenario = Scenario {
    id: "record/torn-last-line",
    produces: Category::RecordReplayIncomplete,
    summary: "the journal ends mid-line; everything before it is kept and the loss is stated",
    run: torn_last_line,
};

pub(super) const CORRUPT_LINE: Scenario = Scenario {
    id: "record/corrupt-line",
    produces: Category::RecordCorruptJournal,
    summary: "a line is not JSON; the replay stops there and names the line and the offset",
    run: corrupt_line,
};

pub(super) const HEADERLESS: Scenario = Scenario {
    id: "record/headerless",
    produces: Category::RecordCorruptJournal,
    summary: "the journal's first line is not a header, so the whole file is refused",
    run: headerless,
};

pub(super) const CONTENT_UNREADABLE: Scenario = Scenario {
    id: "record/content-unreadable",
    produces: Category::RecordContentUnreadable,
    summary: "content was filed and the file will not open; the absence is a failure rather \
              than an empty answer",
    run: content_unreadable,
};

pub(super) const CONTENT_KEY_REFUSED: Scenario = Scenario {
    id: "record/content-key-refused",
    produces: Category::InternalInvariantViolated,
    summary: "a content key with a path in it is refused rather than writing outside the store",
    run: content_key_refused,
};

fn entry(sequence: u64) -> Entry {
    const AT: Timestamp = Timestamp::from_utc_nanos(
        1_756_058_651_442_000_000,
        mcf_core::attested::Attested::Unknown,
    );
    Entry::new(
        EntryKind::MachineProfile,
        AT,
        Value::map([(
            "scenario",
            Value::Integer(i64::try_from(sequence).unwrap_or(0)),
        )]),
    )
}

fn a_journal_of(world: &World, count: u64) -> Result<String, Outcome> {
    let path = world.path("record.jsonl");
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal.writing_as(Writer::stated("labbed01")),
        Err(failure) => {
            return Err(Outcome::Unexpected(format!(
                "could not build one: {failure}"
            )));
        }
    };
    for sequence in 0..count {
        if let Err(failure) = journal.append(&entry(sequence)) {
            return Err(Outcome::Unexpected(format!("could not append: {failure}")));
        }
    }
    drop(journal);
    std::fs::read_to_string(&path)
        .map_err(|error| Outcome::Unexpected(format!("could not read it back: {error}")))
}

fn unwritable_path(_world: &World) -> Outcome {
    let path = std::path::Path::new("/proc/mcf-lab-cannot-write-here/record.jsonl");
    match Journal::open(path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("the journal opened under /proc".to_owned()),
    }
}

fn unknown_format(world: &World) -> Outcome {
    let path = world.path("record.jsonl");
    let later = Value::map([("format", Value::Integer(FORMAT_VERSION + 1))]);
    if let Err(error) = std::fs::write(&path, later.to_line() + "\n") {
        return Outcome::Unexpected(format!("could not write the journal: {error}"));
    }
    match Journal::open(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a later format was accepted".to_owned()),
    }
}

fn torn_last_line(world: &World) -> Outcome {
    let text = match a_journal_of(world, 3) {
        Ok(text) => text,
        Err(outcome) => return outcome,
    };
    let path = world.path("record.jsonl");
    let cut = text.len().saturating_sub(20);
    if let Err(error) = std::fs::write(&path, text.split_at(cut).0) {
        return Outcome::Unexpected(format!("could not tear it: {error}"));
    }

    match replay(&path) {
        Err(failure) => Outcome::Unexpected(format!("the replay refused the file: {failure}")),
        Ok(replayed) => match replayed.loss {
            Some(loss) if !replayed.entries.is_empty() => Outcome::Produced(loss.failure),
            Some(_) => Outcome::Unexpected("the loss was reported and nothing was kept".to_owned()),
            None => Outcome::Unexpected("a torn journal replayed as complete".to_owned()),
        },
    }
}

fn corrupt_line(world: &World) -> Outcome {
    let text = match a_journal_of(world, 4) {
        Ok(text) => text,
        Err(outcome) => return outcome,
    };
    let path = world.path("record.jsonl");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    match lines.get_mut(3) {
        Some(line) => "\u{0}\u{0}not json\u{0}".clone_into(line),
        None => return Outcome::Unexpected("the journal is shorter than expected".to_owned()),
    }
    if let Err(error) = std::fs::write(&path, lines.join("\n") + "\n") {
        return Outcome::Unexpected(format!("could not corrupt it: {error}"));
    }

    match replay(&path) {
        Err(failure) => Outcome::Unexpected(format!("the replay refused the file: {failure}")),
        Ok(replayed) => match replayed.loss {
            Some(loss) if !replayed.entries.is_empty() => Outcome::Produced(loss.failure),
            Some(_) => Outcome::Unexpected("the loss was reported and nothing was kept".to_owned()),
            None => Outcome::Unexpected("a corrupt journal replayed as complete".to_owned()),
        },
    }
}

fn headerless(world: &World) -> Outcome {
    let path = world.path("record.jsonl");
    if let Err(error) = std::fs::write(&path, "{\"not\":\"a header\"}\n") {
        return Outcome::Unexpected(format!("could not write the journal: {error}"));
    }
    match Journal::open(&path) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a journal with no header was accepted".to_owned()),
    }
}

fn content_unreadable(world: &World) -> Outcome {
    let store = match ContentStore::open(&world.path("content")) {
        Ok(store) => store,
        Err(failure) => {
            return Outcome::Unexpected(format!("the content store would not open: {failure}"));
        }
    };
    let key = "generated_lab_0000";
    if let Err(failure) = store.keep(key, &Content::new("what a model said")) {
        return Outcome::Unexpected(format!("content would not be kept: {failure}"));
    }
    let path = world.path("content").join(key);
    if let Err(error) = std::fs::remove_file(&path) {
        return Outcome::Unexpected(format!("the kept file was not there: {error}"));
    }
    if let Err(error) = std::fs::create_dir(&path) {
        return Outcome::Unexpected(format!("the directory would not be made: {error}"));
    }
    match store.disclose_kept(key) {
        Err(failure) => Outcome::Produced(failure),
        Ok(None) => Outcome::Unexpected(
            "content that is there and unreadable came back as absent, which is what an entry \
             whose content was never kept says (A1)"
                .to_owned(),
        ),
        Ok(Some(_)) => Outcome::Unexpected("a directory was read as content".to_owned()),
    }
}

fn content_key_refused(world: &World) -> Outcome {
    let store = match ContentStore::open(&world.path("content")) {
        Ok(store) => store,
        Err(failure) => {
            return Outcome::Unexpected(format!("the content store would not open: {failure}"));
        }
    };
    match store.keep("../escaped", &Content::new("somewhere else")) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected(
            "a key with a parent component was written, so content can leave its store".to_owned(),
        ),
    }
}
