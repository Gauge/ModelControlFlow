use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId, EntryKind};
use mcf_record::json::Value;

use crate::Response;

pub(crate) const SHOWN: usize = 20;

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

pub(crate) fn summarize(entry: &Entry) -> String {
    let said = described(entry);
    let errata = errata_for(entry.recorded_at());
    if errata.is_empty() {
        said
    } else {
        format!("{said}\n{}", errata.join("\n"))
    }
}

fn hosted(body: &Value) -> String {
    let said = |key: &str| body.get(key).and_then(Value::as_text);
    let moved = match body.get("changed").and_then(Value::as_list) {
        Some(changed) if !changed.is_empty() => format!(
            " ({} setting(s) moved off what MCF recommended)",
            changed.len()
        ),
        _ => String::new(),
    };
    format!(
        "hosting {} at {}, reachable from {}{moved}",
        said("model").unwrap_or_default(),
        said("address").unwrap_or_default(),
        said("reachable_from").unwrap_or("MCF did not say"),
    )
}

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

fn cross_checked(body: &Value) -> String {
    let agreement = body.get("agreement");
    let count = |key: &str| {
        agreement
            .and_then(|held| held.get(key))
            .and_then(Value::as_integer)
            .unwrap_or(0)
    };
    let verdict = match agreement.and_then(|held| held.get("within_arithmetic")) {
        Some(Value::Bool(true)) => "the engines agree",
        Some(Value::Bool(false)) => "the engines DIVERGE",
        _ => "no verdict",
    };
    format!(
        "{verdict}: MCF's engine chose the same token at {} of {} positions, furthest rank {}, \
         against {}",
        count("agreed"),
        count("positions"),
        count("furthest_rank"),
        body.get("conditions")
            .and_then(|conditions| conditions.get("engine_ran"))
            .and_then(Value::as_text)
            .unwrap_or("an engine MCF did not name")
    )
}

fn prompt_reported(body: &Value) -> String {
    let conditions = body.get("conditions");
    let of = |held: Option<&Value>, key: &str| {
        held.and_then(|held| held.get(key))
            .and_then(Value::as_integer)
            .unwrap_or(0)
    };
    let floor = integer(body, "floor_parts_per_million");
    let clauses = body.get("clauses").and_then(Value::as_list).unwrap_or(&[]);
    let past_the_floor = clauses
        .iter()
        .filter(|clause| {
            clause
                .get("moved_parts_per_million")
                .and_then(Value::as_integer)
                .is_some_and(|moved| moved > floor)
        })
        .count();
    let settled = match body.get("settled") {
        Some(Value::Map(_)) => format!(
            ", {} seeds at temperature {} gave {} answer(s)",
            of(body.get("settled"), "seeds_asked"),
            body.get("settled")
                .and_then(|held| held.get("temperature"))
                .and_then(Value::as_text)
                .unwrap_or("?"),
            of(body.get("settled"), "distinct_answers")
        ),
        _ => String::new(),
    };
    let spread = match body.get("floor_spread") {
        Some(spread @ Value::Map(_)) => format!(
            " (drawn at every position: {} to {})",
            crate::prompt::percent(integer(spread, "least_parts_per_million")),
            crate::prompt::percent(integer(spread, "most_parts_per_million"))
        ),
        _ => String::new(),
    };
    format!(
        "a prompt of {} {}(s), {} characters, taken apart on {}: floor {}{spread}, {} of {} \
         removed moved the answer past it{settled}",
        of(body.get("prompt"), "parts"),
        conditions
            .and_then(|held| held.get("unit"))
            .and_then(Value::as_text)
            .unwrap_or("part"),
        of(body.get("prompt"), "characters"),
        conditions
            .and_then(|held| held.get("model"))
            .and_then(Value::as_text)
            .unwrap_or("a model MCF did not name"),
        crate::prompt::percent(floor),
        past_the_floor,
        clauses.len(),
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
        EntryKind::CrossChecked => cross_checked(body),
        EntryKind::PromptReported => prompt_reported(body),
        EntryKind::ModelHosted => hosted(body),
        EntryKind::ModelUnhosted => format!(
            "stopped hosting {}: {}",
            text(body, "model").unwrap_or_default(),
            text(body, "reason").unwrap_or_else(|| "MCF did not say".to_owned())
        ),
        EntryKind::Trials => format!(
            "{} trial(s)",
            body.get("trials")
                .and_then(Value::as_list)
                .map_or(0, <[Value]>::len)
        ),
        EntryKind::DaemonStarted => daemon_started_said(body),
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
        EntryKind::Comparison => comparison(body),
        EntryKind::ContentionSnapshot => contention(body),
        EntryKind::FitmentPlanned => fitment(body),
        other => format!("{other}: {}", body.to_line()),
    }
}

fn errata_for(at: mcf_core::time::Timestamp) -> Vec<String> {
    let nanos = i64::try_from(at.utc_nanos()).unwrap_or(i64::MAX);
    mcf_core::errata::affecting(nanos)
        .iter()
        .map(|held| format!("  ⚠ ERRATUM {held}"))
        .collect()
}

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

fn per_cent(held: i64) -> String {
    let whole = held.wrapping_div(10_000);
    let tenths = held.wrapping_div(1_000).wrapping_rem(10).abs();
    format!("{whole}.{tenths}%")
}

fn text(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(Value::as_text).map(str::to_owned)
}

fn daemon_started_said(body: &Value) -> String {
    let stopped = body
        .get("engines_stopped")
        .and_then(Value::as_list)
        .map_or(0, <[Value]>::len);
    format!(
        "the daemon started; recovered {} record entries and {} model files{}",
        integer(body, "record_entries"),
        integer(body, "models_held"),
        match stopped {
            0 => String::new(),
            1 => "; stopped 1 engine server whose daemon was gone".to_owned(),
            n => format!("; stopped {n} engine servers whose daemons were gone"),
        }
    )
}

fn integer(body: &Value, key: &str) -> i64 {
    body.get(key).and_then(Value::as_integer).unwrap_or(0)
}

#[cfg(test)]
mod tests;
