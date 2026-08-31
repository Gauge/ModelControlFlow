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
pub(crate) fn summarize(entry: &Entry) -> String {
    let said = described(entry);
    // Any instrument defect that applies to this entry, beside it (F93). A
    // measurement whose instrument was later found wrong must say so where it
    // is read, not in a document the reader has no reason to open.
    let errata = errata_for(entry.recorded_at());
    if errata.is_empty() {
        said
    } else {
        format!("{said}\n{}", errata.join("\n"))
    }
}

/// The entry, as one line, before any erratum is attached.
/// A timing entry, in one line.
///
/// A9: the rungs that would not separate are results too, so the count says
/// both rather than only the ones that worked.
fn timed(body: &Value) -> String {
    let readings = body.get("readings").and_then(Value::as_list).unwrap_or(&[]);
    let measured = readings
        .iter()
        .filter(|reading| matches!(reading.get("measured"), Some(Value::Bool(true))))
        .count();
    format!(
        "timed at {} depth(s), {measured} measured, on {}",
        readings.len(),
        body.get("conditions")
            .and_then(|conditions| conditions.get("engine_ran"))
            .and_then(Value::as_text)
            .unwrap_or("an engine MCF did not name")
    )
}

fn described(entry: &Entry) -> String {
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
        EntryKind::ModelTimed => timed(body),
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
        EntryKind::Generated => format!(
            "generated {} token(s) from {} — {}{}",
            integer(body, "tokens"),
            body.get("conditions")
                .and_then(|conditions| conditions.get("model"))
                .and_then(Value::as_text)
                .unwrap_or("an unnamed model"),
            text(body, "stopped").unwrap_or_else(|| "stopped for no stated reason".to_owned()),
            match text(body, "degraded") {
                Some(_) => ", MARKED degraded",
                None => "",
            }
        ),
        EntryKind::ArtifactRemoved => format!(
            "removed {} file(s), because: {}",
            body.get("removed")
                .and_then(Value::as_list)
                .map_or(0, <[Value]>::len),
            text(body, "reason").unwrap_or_else(|| "no reason recorded".to_owned())
        ),
        // A9: a comparison that found nothing is a result, and it reads as
        // one here.
        EntryKind::Comparison => comparison(body),
        // B24 with a name attached: a measurement that could not be
        // attributed, and what else was here when it happened (PR5, B-216).
        EntryKind::ContentionSnapshot => contention(body),
        // The other half of A9, and the one §6.3 already calls a complete
        // success: *this will not run here, because it needs 131 GiB and you
        // have 24.*
        EntryKind::FitmentPlanned => fitment(body),
        // `EntryKind` is non-exhaustive: an entry from a newer build is shown as
        // what it is rather than hidden, because a log that skipped what it did
        // not understand would be a log that lies by omission (§7.30, A1).
        other => format!("{other}: {}", body.to_line()),
    }
}

/// A comparison, as a reader meets it in the log (A9, B-086).
///
/// Three of the four outcomes are things a reader will call *it didn't work*,
/// and none of them is a failure. The line says which it was rather than
/// leaving anyone to infer it from a missing number.
/// Any instrument defect that applies to something recorded at this moment
/// (F93).
///
/// **Rendered beside the entry rather than left in a findings document.** A
/// reader meeting a measurement is the person who needs to know the instrument
/// that took it was later found wrong, and they will not go looking. A2: no
/// silent failure, and an uncorrected reading rendered as though nothing were
/// known about it is exactly that.
fn errata_for(at: mcf_core::time::Timestamp) -> Vec<String> {
    let nanos = i64::try_from(at.utc_nanos()).unwrap_or(i64::MAX);
    mcf_core::errata::affecting(nanos)
        .iter()
        .map(|held| format!("  ⚠ ERRATUM {held}"))
        .collect()
}

/// The interval on the size, recomputed from the pairs the entry carries.
///
/// **Derived on read, never stored** (B55, B56, F92). The trials are kept, so
/// every comparison in the record — including one written before the interval
/// existed — renders with the range its own pairs always supported. Nothing is
/// rewritten: the entry on disk is what it was, and the summary is computed
/// each time it is asked for, which is the rule that made this possible.
fn recomputed_spread(body: &Value) -> Option<mcf_bench::enough::Spread> {
    let pairs = body.get("pairs").and_then(Value::as_list)?;
    let differences: Vec<i64> = pairs
        .iter()
        .filter_map(|pair| {
            let left = pair.get("left_ns").and_then(Value::as_integer)?;
            let right = pair.get("right_ns").and_then(Value::as_integer)?;
            let smaller = left.min(right);
            (smaller > 0).then(|| {
                right
                    .saturating_sub(left)
                    .saturating_mul(1_000_000)
                    .wrapping_div(smaller)
            })
        })
        .collect();
    mcf_bench::enough::spread_of(&differences)
}

fn comparison(body: &Value) -> String {
    let arm = |side: &str| {
        body.get(side)
            .and_then(|held| held.get("arm"))
            .and_then(Value::as_text)
            .unwrap_or("an unnamed arm")
            .to_owned()
    };
    let outcome = body.get("outcome");
    let of = |key: &str| {
        outcome
            .and_then(|held| held.get(key))
            .and_then(Value::as_integer)
            .unwrap_or(0)
    };
    let kind = outcome
        .and_then(|held| held.get("kind"))
        .and_then(Value::as_text)
        .unwrap_or("an unrecorded outcome");
    let quicker = outcome
        .and_then(|held| held.get("quicker"))
        .and_then(Value::as_text);
    let said = match kind {
        // A size without a direction is not a comparison (F67), and the log's
        // one line is where most readers meet the verdict.
        // The size as a range, recomputed from the pairs (F92). A record
        // written before the interval existed renders with one anyway,
        // because the trials it kept are what the interval is made of.
        "differ" | "ordered" | "apart" => {
            let sized = recomputed_spread(body)
                .map_or_else(|| per_cent(of("difference")), |held| format!("{held}"));
            let unsettled = if kind == "ordered" {
                format!(
                    " — which does not settle the size at the {} asked about",
                    per_cent(of("resolution"))
                )
            } else {
                String::new()
            };
            match quicker {
                Some(side) => format!(
                    "the {} arm ({}) is quicker by {sized}{unsettled}",
                    side,
                    arm(side)
                ),
                None => format!("they differ by {sized}{unsettled}"),
            }
        }
        "same" => format!(
            "no difference as large as {} — which is a result, not a failure to find one",
            per_cent(of("resolution"))
        ),
        "not_yet" => "not decided: the arms have not separated".to_owned(),
        "not_comparable" => format!("not comparable: {} differed", differing(body)),
        other => other.to_owned(),
    };
    format!(
        "compared {} with {}: {said}, after {} paired trial(s)",
        arm("left"),
        arm("right"),
        of("pairs")
    )
}

/// The conditions a comparison found differing, as one phrase.
fn differing(body: &Value) -> String {
    let Some(held) = body
        .get("isolation")
        .and_then(|held| held.get("differ"))
        .and_then(Value::as_list)
    else {
        return "more than one condition".to_owned();
    };
    held.iter()
        .filter_map(Value::as_text)
        .collect::<Vec<&str>>()
        .join(", ")
}

/// A contention snapshot, as a reader meets it in the log (B-216, PR5).
fn contention(body: &Value) -> String {
    let competitors = body.get("competitors").and_then(Value::as_list);
    let busiest = competitors
        .and_then(<[Value]>::first)
        .and_then(|one| one.get("command"))
        .and_then(Value::as_text)
        .unwrap_or("nothing it could name");
    let taken = body
        .get("cores_taken_thousandths")
        .and_then(Value::as_integer)
        .unwrap_or(0);
    format!(
        "what was competing: {}.{} core(s) across {} process(es), busiest {busiest}",
        taken.wrapping_div(1_000),
        taken.wrapping_div(100).wrapping_rem(10),
        competitors.map_or(0, <[Value]>::len)
    )
}

/// A plan, as a reader meets it in the log (A9, §6.3, B-213).
fn fitment(body: &Value) -> String {
    let variants = body
        .get("plan")
        .and_then(|plan| plan.get("variants"))
        .and_then(Value::as_list);
    let counted = |wanted: &str| {
        variants.map_or(0, |held| {
            held.iter()
                .filter(|variant| variant.get("outcome").and_then(Value::as_text) == Some(wanted))
                .count()
        })
    };
    format!(
        "planned {} — {} of {} variant(s) fit here, {} at a shorter context, {} do not",
        text(body, "repository").unwrap_or_else(|| "a repository".to_owned()),
        counted("fits"),
        variants.map_or(0, <[Value]>::len),
        counted("fits_at_a_shorter_context"),
        counted("does_not_fit")
    )
}

/// A ratio in parts per million, as a reader wants it.
///
/// Integer arithmetic: this crate renders what the record holds and does not
/// introduce a float to do it (A6).
fn per_cent(held: i64) -> String {
    let whole = held.wrapping_div(10_000);
    let tenths = held.wrapping_div(1_000).wrapping_rem(10).abs();
    format!("{whole}.{tenths}%")
}

fn text(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(Value::as_text).map(str::to_owned)
}

fn integer(body: &Value, key: &str) -> i64 {
    body.get(key).and_then(Value::as_integer).unwrap_or(0)
}

#[cfg(test)]
mod tests;

/// Writes down that somebody changed how MCF addresses a model (D43, B-059).
///
/// The pair to the configuration file the way `ComponentProvisioned` pairs
/// with a prefix: the file says what MCF does now, and this says who changed
/// it, when, and on what evidence. A configuration whose file is edited by
/// hand still has this line to be compared against.
///
/// # Errors
///
/// `record.unwritable` where there is nowhere to write.
pub(crate) fn record_configured(
    model: &std::path::Path,
    addressing: &mcf_serve::configured::Addressing,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-cli::log"),
            "there is nowhere to record the configuration",
        ));
    };
    let body = Value::map([
        ("model", Value::text(model.display().to_string())),
        ("addressing", addressing.to_value()),
        // What MCF did before, so that the entry says what *changed* and not
        // only what is now true. A record of the new state alone cannot answer
        // whether anything happened (A1).
        ("was", Value::text("raw text, MCF's default (§3.8)")),
    ]);
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&Entry::new(
        EntryKind::ModelConfigured,
        mcf_core::time::Timestamp::now(),
        body,
    ))?;
    Ok(path)
}

/// Writes what a probe observed about a model's usable context (B-386, B-055).
///
/// **Why this exists at all.** A probe printed its findings and wrote none of
/// them down, so a figure this machine established lived on a terminal until
/// the terminal scrolled. A1: a measurement nobody can find later is the same
/// as one not taken. B-382 is where it surfaced — a prompt's cost could only
/// be stated against the file's claim, because MCF's own measurement of what
/// the engine actually takes existed nowhere readable.
///
/// **What travels with it.** The conditions, because a context measured
/// through one engine on one machine is not a fact about the model (§3.4); and
/// the engine's own words where it refused, because a refusal for an unrelated
/// reason would otherwise be read back as a short context (A1).
///
/// # Errors
///
/// `record.unwritable` where there is nowhere to write, or the journal refuses
/// the append. A probe whose result could not be kept says so rather than
/// reading as kept (A2).
pub(crate) fn record_probed_context(
    model: &std::path::Path,
    context: &mcf_serve::probes::Context,
    engine: &str,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    let mut body = vec![
        (
            "declared_tokens",
            Value::Integer(as_integer(context.declared)),
        ),
        (
            "accepted_tokens",
            Value::Integer(as_integer(context.accepted)),
        ),
    ];
    if let Some(because) = &context.because {
        body.push(("because", Value::text(because.clone())));
    }
    record_probed(model, mcf_serve::probes::USABLE_CONTEXT.name, engine, body)
}

/// Writes what any probe observed (B-386, B-054, D42, A1).
///
/// **One writer, because two would eventually disagree about what a probe
/// result is** (F79). The context probe had the only one, written for it, and
/// the probes built afterwards printed their results and kept nothing — so
/// B-386's *a probe run yesterday can be read back today* held for one probe of
/// four while the row said it held (F106). The fields differ per probe; the
/// model, the method, the engine and the shape of the entry do not, and those
/// are here.
///
/// **Whichever way it came out.** *Agrees* is as much a measurement as
/// *diverges* (A9), and a caller that recorded only the interesting half would
/// leave a record that cannot answer *what did this model do* — only *when was
/// it surprising*.
///
/// # Errors
///
/// `record.unwritable` where there is nowhere to write, or the journal refuses
/// the append. A probe whose result could not be kept says so rather than
/// reading as kept (A2).
pub(crate) fn record_probed(
    model: &std::path::Path,
    method: &str,
    engine: &str,
    fields: Vec<(&'static str, Value)>,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-cli::log"),
            "there is nowhere to record what the probe observed",
        ));
    };
    let mut body = vec![
        ("model", Value::text(model.display().to_string())),
        ("method", Value::text(method.to_owned())),
        ("engine", Value::text(engine.to_owned())),
    ];
    body.extend(fields);
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&Entry::new(
        EntryKind::ModelProbed,
        mcf_core::time::Timestamp::now(),
        Value::map(body),
    ))?;
    Ok(path)
}

/// A count as the record's integer, saturating rather than wrapping.
fn as_integer(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}
