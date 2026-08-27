//! `mcf log`: what happened on this machine, read back (B-363, §3.3, A22, B62).
//!
//! **The record has been write-only until now.** MCF has written to it since
//! M0 — machine profiles, self-cost figures, failures, acquisitions, removals,
//! the daemon's own life — and the only way to read it was to open the file.
//! §3.3 ranks machine-readability first and legibility second, but *second is
//! not omitted*, and A22 makes the headless surface the complete one: a record
//! nobody can read from a command is a record only its author can read.
//!
//! **A replay is not a `cat`.** The journal is line-delimited and a crash
//! mid-append leaves a torn last line, so reading it is the record's own job:
//! it reports the line, the offset and the bytes of anything it could not read
//! (B62). This surface shows that report rather than hiding it — a log that
//! quietly stopped at a damaged line would be the silent failure A2 calls worse
//! than a crash.
//!
//! **It reads through the index, and only the entries it prints.** D20's
//! derived index says where each entry is and what kind it is, so *the last
//! twenty acquisitions* costs twenty seeks rather than a parse of the whole
//! history — 196 µs against 7.9 s at a million entries (F14). The index is
//! never the answer: every line printed here is read back out of the journal
//! at the offset the index gave.
//!
//! **One line per event, and the interesting field first.** What a reader wants
//! from an acquisition is what was acquired; from a failure, the category and
//! what it was about; from the daemon, why it stopped. The whole entry is still
//! there — `--full` prints the record's own JSON, which is what a script reads
//! and what `mcf export` sends.

use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId, EntryKind};
use mcf_record::json::Value;

use crate::Response;

/// How many entries are shown when nobody says.
///
/// Twenty, because a record grows for the life of a machine and a command that
/// printed all of it by default would be a command people pipe to `tail` —
/// which is the same as MCF choosing twenty, with less said about it (§3.15).
pub(crate) const SHOWN: usize = 20;

/// Reads the record back.
pub(crate) fn run(kind: Option<&str>, last: Option<usize>, full: bool) -> Response {
    let Some(path) = mcf_record::journal::default_path() else {
        return Response {
            text: "mcf: there is no record to read — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    if !path.exists() {
        return Response {
            text: format!(
                "no record at {}: nothing has been recorded on this machine yet",
                path.display()
            ),
            served: true,
        };
    }

    let wanted = match kind {
        None => None,
        Some(name) => match EntryKind::parse(name) {
            Some(kind) => Some(kind),
            None => {
                return Response {
                    text: format!(
                        "mcf: there is no kind of entry called {name}\n  this build knows: {}",
                        EntryKind::ALL
                            .iter()
                            .map(|kind| kind.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    served: false,
                };
            }
        },
    };

    let index = match Index::over(&path, &index::default_path(&path)) {
        Ok(index) => index,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the record could not be read", &failure),
                served: false,
            };
        }
    };

    let matching = index.count_matching(wanted);
    let shown = last.unwrap_or(SHOWN).min(matching);
    let skipped = matching.saturating_sub(shown);

    let mut lines = vec![format!(
        "{} in {}{}",
        counted(matching, wanted),
        path.display(),
        if skipped == 0 {
            String::new()
        } else {
            format!("; showing the last {shown}, {skipped} earlier not shown")
        }
    )];
    lines.push(String::new());

    for located in index.latest(wanted, shown) {
        // The entry comes from the journal, at the offset the index gave: the
        // index is a pointer and never an answer (D20).
        match index.read(&located) {
            Ok(entry) => lines.push(if full {
                entry.to_value().to_line()
            } else {
                format!(
                    "{}  {}",
                    entry.id().map_or("(unidentified)", EntryId::as_str),
                    summarize(&entry)
                )
            }),
            Err(failure) => lines.push(format!(
                "line {}: THIS ENTRY COULD NOT BE READ: {failure}",
                located.line()
            )),
        }
    }

    // B62: what could not be read is said, at the end where it is the last
    // thing a reader sees rather than the first thing they scroll past.
    if let Some(loss) = index.loss() {
        lines.push(String::new());
        lines.push(format!("PART OF THE RECORD COULD NOT BE READ: {loss}"));
        lines.push(
            "  everything above was read whole; what is missing is what came after that point"
                .to_owned(),
        );
    }

    Response {
        text: lines.join("\n"),
        served: true,
    }
}

fn counted(entries: usize, kind: Option<EntryKind>) -> String {
    match kind {
        Some(kind) => format!(
            "{entries} {kind} entr{}",
            if entries == 1 { "y" } else { "ies" }
        ),
        None => format!("{entries} entr{}", if entries == 1 { "y" } else { "ies" }),
    }
}

/// One line for one event, with the field a reader wants first.
///
/// Every kind gets its own sentence rather than a generic dump: what makes a
/// log readable is that the interesting thing is in the same place every time,
/// and what a reader wants from an acquisition is not what they want from a
/// failure.
fn summarize(entry: &Entry) -> String {
    let body = entry.body();
    match entry.kind() {
        EntryKind::MachineProfile => text(body, "processor")
            .or_else(|| {
                body.get("machine")
                    .and_then(|machine| machine.get("processor"))
                    .and_then(Value::as_text)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "what this machine is".to_owned()),
        EntryKind::Failure => format!(
            "{} [{}] {}",
            text(body, "category").unwrap_or_else(|| "unclassified".to_owned()),
            text(body, "attribution").unwrap_or_else(|| "unattributed".to_owned()),
            text(body, "detail").unwrap_or_default()
        ),
        EntryKind::SelfCost => {
            text(body, "figure").unwrap_or_else(|| "what MCF cost on this machine".to_owned())
        }
        EntryKind::Trials => format!(
            "{} trial(s)",
            body.get("trials")
                .and_then(Value::as_list)
                .map_or(0, <[Value]>::len)
        ),
        EntryKind::DaemonStarted => format!(
            "the daemon started; recovered {} record entries and {} model files",
            integer(body, "record_entries"),
            integer(body, "models_held")
        ),
        EntryKind::DaemonStopped => match text(body, "reason") {
            Some(reason) => format!("the daemon stopped, because: {reason}"),
            None => format!(
                "the daemon stopped ({})",
                text(body, "how").unwrap_or_else(|| "no reason given".to_owned())
            ),
        },
        EntryKind::ArtifactAcquired => format!(
            "acquired {}:{} — {}",
            text(body, "repository").unwrap_or_default(),
            text(body, "file").unwrap_or_default(),
            body.get("verification")
                .and_then(|verification| verification.get("state"))
                .and_then(Value::as_text)
                .unwrap_or("its verification is not recorded")
        ),
        EntryKind::ArtifactChecked => format!(
            "checked {} — the bytes {}; upstream: {}",
            text(body, "repository").unwrap_or_else(|| "an artifact".to_owned()),
            text(body, "bytes").unwrap_or_else(|| "were not compared".to_owned()),
            text(body, "detail").unwrap_or_else(|| "no finding recorded".to_owned())
        ),
        EntryKind::ComponentProvisioned => format!(
            "provisioned {} at {} — image {}, into {}",
            text(body, "component").unwrap_or_else(|| "a component".to_owned()),
            text(body, "commit").unwrap_or_else(|| "an unstated commit".to_owned()),
            text(body, "image").unwrap_or_else(|| "an unstated image".to_owned()),
            text(body, "prefix").unwrap_or_default()
        ),
        EntryKind::ComponentRemoved => format!(
            "removed the provisioned {}, because: {}",
            text(body, "component").unwrap_or_else(|| "component".to_owned()),
            text(body, "reason").unwrap_or_else(|| "no reason recorded".to_owned())
        ),
        EntryKind::ArtifactRemoved => format!(
            "removed {} file(s), because: {}",
            body.get("removed")
                .and_then(Value::as_list)
                .map_or(0, <[Value]>::len),
            text(body, "reason").unwrap_or_else(|| "no reason recorded".to_owned())
        ),
        // `EntryKind` is non-exhaustive: an entry from a newer build is shown as
        // what it is rather than hidden, because a log that skipped what it did
        // not understand would be a log that lies by omission (§7.30, A1).
        other => format!("{other}: {}", body.to_line()),
    }
}

fn text(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(Value::as_text).map(str::to_owned)
}

fn integer(body: &Value, key: &str) -> i64 {
    body.get(key).and_then(Value::as_integer).unwrap_or(0)
}

#[cfg(test)]
mod tests;
