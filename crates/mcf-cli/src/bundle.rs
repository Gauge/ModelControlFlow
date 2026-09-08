use std::path::{Path, PathBuf};

use mcf_record::export::{self, Kind};
use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId, EntryKind};
use mcf_record::json::Value;

use crate::Response;

pub(crate) fn run(wanted: &str, into: Option<&str>) -> Response {
    let Some(journal) = mcf_record::journal::default_path() else {
        return Response {
            text: "mcf: there is no record to read — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let index = match Index::over(&journal, &index::default_path(&journal)) {
        Ok(index) => index,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the record could not be read", &failure),
                served: false,
            };
        }
    };

    let Some(claim) = find(&index, wanted) else {
        return Response {
            text: format!(
                "mcf: no entry called {wanted} in {}\n  `mcf log` lists what is there, and an \
                 identifier is the first field of each line",
                journal.display()
            ),
            served: false,
        };
    };
    if claim.kind() != EntryKind::Comparison {
        return Response {
            text: format!(
                "mcf: {wanted} is a {} and a bundle reproduces a *claim*\n  `mcf log --kind \
                 comparison` lists the entries that are one (PR2)",
                claim.kind()
            ),
            served: false,
        };
    }

    let rests_on = rests_on(&claim);
    let machine = machine_when(&index, &claim);
    let to = destination(into, wanted);
    let carried = std::cell::RefCell::new(Vec::new());
    let written = export::write_selected(&journal, &to, Kind::ReproBundle, |entry| {
        let keep = keeps(entry, wanted, &rests_on, machine.as_deref());
        if keep && let Some(id) = entry.get("id").and_then(Value::as_text) {
            carried.borrow_mut().push(id.to_owned());
        }
        keep
    });
    match written {
        Ok(manifest) => Response {
            text: report(
                &to,
                &claim,
                &carried.borrow(),
                manifest.entries,
                &manifest.digest,
                &prompt_beside(&journal, &to, wanted),
            ),
            served: true,
        },
        Err(failure) => Response {
            text: crate::say::refusal("the bundle could not be written", &failure),
            served: false,
        },
    }
}

fn find(index: &Index, wanted: &str) -> Option<Entry> {
    index
        .latest(None, index.count_matching(None))
        .into_iter()
        .filter_map(|located| index.read(&located).ok())
        .find(|entry| entry.id().map(EntryId::as_str) == Some(wanted))
}

fn rests_on(claim: &Entry) -> Vec<String> {
    ["left", "right"]
        .into_iter()
        .filter_map(|side| {
            claim
                .body()
                .get(side)
                .and_then(|arm| arm.get("arm"))
                .and_then(Value::as_text)
                .map(str::to_owned)
        })
        .collect()
}

fn keeps(entry: &Value, claim: &str, arms: &[String], machine: Option<&str>) -> bool {
    let id = entry.get("id").and_then(Value::as_text).unwrap_or_default();
    if id == claim {
        return true;
    }
    let Some(kind) = entry.get("kind").and_then(Value::as_text) else {
        return false;
    };
    let body = entry.get("body");
    match kind {
        "artifact_acquired" => body
            .and_then(|held| held.get("path"))
            .and_then(Value::as_text)
            .is_some_and(|path| arms.iter().any(|arm| arm == path)),
        "component_provisioned" => true,
        "machine_profile" => machine.is_some_and(|held| held == id),
        _ => false,
    }
}

fn machine_when(index: &Index, claim: &Entry) -> Option<String> {
    let taken = claim.recorded_at().utc_nanos();
    index
        .latest(
            Some(EntryKind::MachineProfile),
            index.count_matching(Some(EntryKind::MachineProfile)),
        )
        .into_iter()
        .filter_map(|located| index.read(&located).ok())
        .filter(|entry| entry.recorded_at().utc_nanos() <= taken)
        .max_by_key(|entry| entry.recorded_at().utc_nanos())
        .and_then(|entry| entry.id().map(|id| id.as_str().to_owned()))
}

fn destination(into: Option<&str>, wanted: &str) -> PathBuf {
    match into {
        Some(named) => {
            let held = PathBuf::from(named);
            if held.is_dir() {
                held.join(format!("{wanted}.mcf-bundle"))
            } else {
                held
            }
        }
        None => PathBuf::from(format!("{wanted}.mcf-bundle")),
    }
}

fn prompt_beside(journal: &Path, bundle: &Path, wanted: &str) -> String {
    let beside = bundle.with_extension("mcf-bundle.prompt");
    let store = match mcf_record::content::ContentStore::open(
        &mcf_record::content::ContentStore::beside(journal),
    ) {
        Ok(store) => store,
        Err(failure) => return format!("  the prompt could not be reached: {failure}"),
    };
    match store.disclose_kept(wanted) {
        Ok(Some(prompt)) => match std::fs::write(&beside, prompt.disclose()) {
            Ok(()) => format!(
                "  the prompt is beside it in {} — {} byte(s) of your own text, in\n  a file of \
                 its own so that sending one is not sending the other (A25)",
                beside.display(),
                prompt.length_bytes()
            ),
            Err(error) => format!("  the prompt could not be written beside it: {error}"),
        },
        Ok(None) => "  the prompt is NOT here: this claim was recorded before MCF filed what \
                     was\n  asked, so the bundle carries its digest and not its text (F105)"
            .to_owned(),
        Err(failure) => format!("  the prompt is filed and would not be read: {failure}"),
    }
}

fn report(
    to: &Path,
    claim: &Entry,
    carried: &[String],
    entries: usize,
    digest: &str,
    prompt: &str,
) -> String {
    let mut lines = vec![
        format!("wrote {}", to.display()),
        format!("  {entries} entr(ies), sha256 {digest}"),
        String::new(),
        "── what is in it ────────────────────────────────────────────".to_owned(),
        format!("  the claim   {}", crate::log::summarize(claim)),
    ];
    for id in carried {
        if claim.id().map(EntryId::as_str) != Some(id.as_str()) {
            lines.push(format!("  rests on    {id}"));
        }
    }
    if !carried.iter().any(|id| id.starts_with("machine_profile")) {
        lines
            .push("  NOT in it   what this machine is: no machine profile was recorded".to_owned());
        lines.push(
            "              before this claim. `mcf doctor` writes one, and a bundle".to_owned(),
        );
        lines.push(
            "              without it is checkable against a machine nobody described".to_owned(),
        );
    }
    lines.push(String::new());
    lines.push("── what leaves with it, if you send it ──────────────────────".to_owned());
    lines.push(prompt.to_owned());
    lines.push(String::new());
    for said in [
        "  the prompt both arms were asked, which is text you wrote",
        "  this machine's full hardware identity, which a contribution would",
        "  strip and a bundle deliberately keeps — it is what makes the claim",
        "  checkable rather than aggregable",
        "  every trial's raw duration, not a summary of them",
    ] {
        lines.push(said.to_owned());
    }
    lines.push(String::new());
    lines
        .push("  Writing this file is not sending it. Sending it is your act, and this".to_owned());
    lines.push("  is the list you should read before you do.".to_owned());
    lines.join("\n")
}

#[cfg(test)]
mod tests;
