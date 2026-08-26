//! `mcf check`: is what this machine holds still what the hub says it is?
//! (B-331, D37, §7.38).
//!
//! **What it does.** For every artifact this machine holds whose provenance
//! names a repository, it asks that repository what it says now and compares
//! with what was written down at acquisition. It fetches no weights — a listing
//! and a model card — and it changes nothing about the artifact.
//!
//! **When it runs: when somebody runs it.** D37 forbids the timer a watcher
//! would need (B4, §3.13). MCF does not notice a decay overnight, and saying so
//! is better than a background poll nobody asked for.
//!
//! **What a finding costs, which is nothing.** A decay is written down beside
//! the provenance and into the record, and no measurement is withdrawn. The
//! artifact is here, its digest still verifies, and a tool that retracted its
//! own results because somebody else deleted something would be destroying
//! evidence for a reason that is not scientific (D37). What is really lost is
//! *somebody else's* ability to reproduce, and that belongs in a repro bundle
//! as a stated condition (PR2).
//!
//! **The one thing MCF will not say** is which of three things a silent hub
//! means. [findings.md](../../../doc/findings.md) F17 measured that a
//! repository that is private, one that was withdrawn and one that never
//! existed all answer the same way, so an unreachable repository is reported as
//! *unreachable* and is not counted as a change (A7).

use std::path::Path;

use mcf_core::provenance::{Decay, Observation, Origin, Provenance};
use mcf_core::time::Timestamp;
use mcf_hub::client::Hub;
use mcf_hub::decay;
use mcf_hub::http::Url;
use mcf_hub::store::{self, Held};
use mcf_record::journal::{Entry, EntryKind, Journal};
use mcf_record::json::Value;

use crate::Response;
use crate::models;
use crate::pull::{DEFAULT_HUB, Offered, credential, wire_for};

/// Looks upstream at everything held, or at one artifact.
pub(crate) fn run(only: Option<&str>, from: Option<&str>, offered: Offered<'_>) -> Response {
    let Some(root) = models::default_root() else {
        return Response {
            text: "mcf: there is nowhere to look — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let holding = match store::held(&root) {
        Ok(holding) => holding,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the model store could not be read", &failure),
                served: false,
            };
        }
    };

    let wanted: Vec<&Held> = holding
        .iter()
        .filter(|held| only.is_none_or(|name| held.path.to_string_lossy().contains(name)))
        .collect();
    if wanted.is_empty() {
        return Response {
            text: match only {
                Some(name) => format!("mcf: this machine holds nothing called {name}"),
                None => format!("no models: {} holds nothing to check", root.display()),
            },
            served: only.is_none(),
        };
    }

    let hub = match hub_for(from, offered) {
        Ok(hub) => hub,
        Err(response) => return response,
    };

    let at = Timestamp::now();
    let mut lines = vec![format!("looking upstream at {} artifact(s)", wanted.len())];
    let mut changes = 0_usize;
    let mut looked = 0_usize;

    for held in wanted {
        let Ok(provenance) = &held.provenance else {
            lines.push(format!(
                "  {} — nothing beside it says where it came from, so there is nothing to \
                 check (A7)",
                held.path.display()
            ));
            continue;
        };
        let Some(observed) = decay::look(&hub, provenance, file_of(&held.path), at) else {
            lines.push(format!(
                "  {} — its origin is not a repository, so there is no upstream to look at",
                held.path.display()
            ));
            continue;
        };
        looked = looked.saturating_add(1);
        if observed.found.is_a_change() {
            changes = changes.saturating_add(1);
        }
        lines.push(format!("  {} — {}", held.path.display(), observed.found));

        // Written down twice, and neither is a correction: beside the artifact,
        // where a reader of the file finds it, and in the record, where *what
        // happened on this machine* lives (D20, D37).
        match record(&held.path, provenance, &observed, at) {
            Ok(()) => {}
            Err(failure) => lines.push(format!(
                "    the finding could not be written down: {failure}"
            )),
        }
    }

    lines.push(String::new());
    lines.push(match changes {
        0 => format!(
            "{looked} checked, nothing changed upstream. Nothing here was verified against the \
             bytes on this disk — `mcf list` is where an artifact's own integrity is (B-301)."
        ),
        _ => format!(
            "{changes} of {looked} changed upstream. Nothing is invalidated by that: the \
             artifacts are here and their digests are what they were (D37). What a change \
             costs is somebody else's ability to reproduce from the same reference."
        ),
    });

    Response {
        text: lines.join("\n"),
        served: true,
    }
}

/// The hub to ask, with whatever credential the operator named.
///
/// The same construction `mcf pull` makes, for the same reasons: TLS is chosen
/// from the URL rather than configured, and a credential comes from where the
/// operator said and nowhere else (B-024, B-322).
fn hub_for(from: Option<&str>, offered: Offered<'_>) -> std::result::Result<Hub, Response> {
    let refuse = |failure: &mcf_core::failure::Failure| Response {
        text: crate::say::refusal("nothing was checked", failure),
        served: false,
    };
    let base = Url::parse(from.unwrap_or(DEFAULT_HUB)).map_err(|failure| refuse(&failure))?;
    let wire = wire_for(&base).map_err(|failure| refuse(&failure))?;
    let look_up = |name: &str| std::env::var(name).ok();
    match credential(offered, &look_up) {
        Ok(None) => Ok(Hub::at(base, wire)),
        Ok(Some(credential)) => Ok(Hub::at(base, wire).offering(credential)),
        Err(text) => Err(Response {
            text,
            served: false,
        }),
    }
}

/// The artifact's own name in the repository it came from.
fn file_of(path: &Path) -> Option<&str> {
    path.file_name().and_then(std::ffi::OsStr::to_str)
}

/// Writes the finding beside the artifact and into the record.
fn record(
    path: &Path,
    provenance: &Provenance,
    observed: &Observation,
    at: Timestamp,
) -> mcf_core::failure::Result<()> {
    store::record_provenance(path, &provenance.clone().observed(observed.clone()))?;

    let Some(journal) = mcf_record::journal::default_path() else {
        return Ok(());
    };
    let mut open = Journal::open(&journal)?;
    open.append(&Entry::new(
        EntryKind::ArtifactChecked,
        at,
        Value::map([
            ("artifact", Value::text(path.display().to_string())),
            (
                "repository",
                match provenance.origin() {
                    Origin::Hub { repository, .. } => Value::text(repository.as_str()),
                    _ => Value::Null,
                },
            ),
            ("found", Value::text(observed.found.as_str())),
            ("detail", Value::text(observed.found.to_string())),
            ("is_a_change", Value::Bool(observed.found.is_a_change())),
        ]),
    ))?;
    Ok(())
}

/// Whether a finding is one an operator should be told about loudly.
///
/// Kept beside the surface rather than on the type: `Decay::is_a_change` is
/// about the *record*, and this is about a person reading a terminal.
#[allow(dead_code, reason = "the window (M4) is the second reader of this")]
pub(crate) const fn is_loud(found: &Decay) -> bool {
    found.is_a_change()
}
