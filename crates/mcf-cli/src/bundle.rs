//! `mcf bundle <entry-id>`: one file that reproduces one claim (B-211, PR2,
//! §II, A6, A24).
//!
//! **§II is the intent that makes MCF worth trusting**: *every measurement
//! carries the obligations of a measurement — a stated method, stated
//! conditions, stated uncertainty, and the ability for someone else to repeat
//! it.* Three of those four were built. The fourth was a property of the design
//! rather than a thing anybody could hand over: there was no artifact a user
//! could attach to a bug report that says *here is the claim, and here is
//! everything required to check it*.
//!
//! **What goes in.** The claim, and everything it rests on, selected out of the
//! record rather than assembled beside it:
//!
//! * the comparison itself — its verdict, its method, both arms' full
//!   condition floors, every pair's two raw durations and what each drew;
//! * every artifact either arm named, with the provenance recorded when it was
//!   acquired (§3.6);
//! * the engine, if one was provisioned — the image by digest, the source by
//!   commit, the packages by exact version;
//! * what this machine was, as the last machine profile read it.
//!
//! **It is one mechanism, not a fourth.** `mcf_record::export` already writes
//! exports and will write contributions; B-302 requires the three be one thing,
//! because three serializations of the same evidence eventually disagree about
//! what the evidence was. A bundle is that mechanism with a selector.
//!
//! **Producing is not sending** (A24). Writing a file to a path the operator
//! named is not publication and is not gated. What the surface owes is the
//! other half of A24: **it shows what the file contains before it goes
//! anywhere**, and it names the two things a reader would not expect — the
//! prompt, which is the operator's own text, and the full hardware identity,
//! which a *contribution* would strip and a bundle deliberately keeps.

use std::path::{Path, PathBuf};

use mcf_record::export::{self, Kind};
use mcf_record::journal::index::{self, Index};
use mcf_record::journal::{Entry, EntryId, EntryKind};
use mcf_record::json::Value;

use crate::Response;

/// Writes the bundle for one recorded claim.
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
    // The machine as it was *when the claim was taken*, which is the newest
    // profile not later than the claim. Carrying every profile the record
    // holds would carry the machine on other days, which is not what this
    // claim rests on.
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
            ),
            served: true,
        },
        Err(failure) => Response {
            text: crate::say::refusal("the bundle could not be written", &failure),
            served: false,
        },
    }
}

/// The entry with that identifier, if the record holds one.
fn find(index: &Index, wanted: &str) -> Option<Entry> {
    index
        .latest(None, index.count_matching(None))
        .into_iter()
        .filter_map(|located| index.read(&located).ok())
        .find(|entry| entry.id().map(EntryId::as_str) == Some(wanted))
}

/// What a claim rests on, named from the claim itself.
///
/// The arms' paths, which is how an acquisition and a comparison are joined:
/// the record has no foreign keys, and inventing one would be inventing a
/// relation the record does not hold (D20). What is here is what the claim
/// itself says.
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

/// Whether an entry belongs in the bundle.
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
        // The provenance of what was measured (§3.6). Matched on the path each
        // arm names, because that is the only join the record holds.
        "artifact_acquired" => body
            .and_then(|held| held.get("path"))
            .and_then(Value::as_text)
            .is_some_and(|path| arms.iter().any(|arm| arm == path)),
        // The engine, by image digest, source commit and exact packages — a
        // measurement through a provisioned engine is a measurement of that
        // environment (D39, §3.4).
        "component_provisioned" => true,
        // What this machine was when the claim was taken, and not what it was
        // on other days.
        "machine_profile" => machine.is_some_and(|held| held == id),
        _ => false,
    }
}

/// The newest machine profile not later than the claim, if the record holds
/// one.
///
/// `None` where it holds none — which is a real state and is said in the
/// report rather than papered over: a bundle whose conditions include *what
/// the machine was* is a stronger artifact than one whose do not, and the
/// difference is the reader's to weigh (A7).
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

/// Where the bundle goes.
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

/// What the operator is told, before the file goes anywhere (A24).
fn report(to: &Path, claim: &Entry, carried: &[String], entries: usize, digest: &str) -> String {
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
    for said in [
        "  the prompt both arms were asked, which is text you wrote",
        "  this machine's full hardware identity, which a contribution would",
        "  strip and a bundle deliberately keeps — it is what makes the claim",
        "  checkable rather than aggregable (PR2, §XIV)",
        "  every trial's raw duration, not a summary of them (D16)",
    ] {
        lines.push(said.to_owned());
    }
    lines.push(String::new());
    lines
        .push("  Writing this file is not sending it. Sending it is your act, and this".to_owned());
    lines.push("  is the list you should read before you do (A24, §3.20).".to_owned());
    lines.join("\n")
}

#[cfg(test)]
mod tests;
