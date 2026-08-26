//! `mcf check`: is what this machine holds still what the hub says it is?
//! (B-331, D37, §7.38).
//!
//! **What it does, in two halves.** *The bytes here*: re-read the artifact and
//! compare its digest with the one recorded when it arrived (B-301, §7.49) —
//! which catches the silent disk corruption that would otherwise be discovered
//! as a garbage measurement rather than as a bad file. *The upstream*: ask the
//! repository it came from what it says now, and compare with what was written
//! down at acquisition (B-331, D37). Neither fetches weights; the second
//! fetches a listing and a model card, and `--here` does not touch the network
//! at all.
//!
//! **Why the two belong in one command.** They are the same question — *is what
//! I hold still what it should be* — asked of the two things that can change
//! independently. Keeping them apart would mean an operator has to know which
//! kind of rot they are looking for before they look.
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

use mcf_core::attested::Attested;
use mcf_core::integrity;
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

/// How much of the question to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    /// Both halves: the bytes here and the repository they came from.
    Everything,
    /// The bytes on this disk, and no network at all.
    HereOnly,
}

/// Checks what this machine holds: the bytes, and where they came from.
pub(crate) fn run(
    only: Option<&str>,
    reach: Reach,
    from: Option<&str>,
    offered: Offered<'_>,
) -> Response {
    let Some(root) = models::default_root() else {
        return Response {
            text: "mcf: there is nowhere to look — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    // A machine that has never acquired anything has no store, and that is not
    // an unreadable one: `mcf list` has always said so and this said *the model
    // store could not be read*, which is two answers to one situation (A6) and
    // the wrong one of the two.
    if !root.exists() {
        return Response {
            text: format!(
                "no models: {} does not exist yet\n\
                 \x20 nothing has been acquired on this machine, so there is nothing to check",
                root.display()
            ),
            served: true,
        };
    }
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

    let hub = match reach {
        Reach::HereOnly => None,
        Reach::Everything => match hub_for(from, offered) {
            Ok(hub) => Some(hub),
            Err(response) => return response,
        },
    };

    let at = Timestamp::now();
    let mut lines = vec![format!(
        "checking {} artifact(s){}",
        wanted.len(),
        match reach {
            Reach::HereOnly => " against what was recorded here, and nothing else",
            Reach::Everything => " here and upstream",
        }
    )];
    let mut changes = 0_usize;
    let mut looked = 0_usize;
    let mut unanswered = 0_usize;
    let mut corrupt = 0_usize;

    for held in wanted {
        let Ok(provenance) = &held.provenance else {
            lines.push(format!(
                "  {} — nothing beside it says where it came from, so there is nothing to \
                 check it against (A7)",
                held.path.display()
            ));
            continue;
        };

        // The bytes first, because it is the half that needs no network and the
        // half a bad answer would come from: a measurement taken against a
        // corrupted file is worse than one not taken (§7.49, B-301).
        let (said, matched) = bytes_here(&held.path, provenance);
        lines.extend(said);
        if matched == Some(false) {
            corrupt = corrupt.saturating_add(1);
        }

        let observed = hub
            .as_ref()
            .and_then(|hub| upstream(hub, provenance, &held.path, at, &mut lines));
        if let Some(observed) = &observed {
            // *Asked and answered* rather than *asked*: a hub that refused or
            // could not be reached told MCF nothing about the artifact, and
            // counting it among the checked would let a run of failures read as
            // a clean bill of health (A7, F17).
            if matches!(observed.found, Decay::Unreachable { .. }) {
                unanswered = unanswered.saturating_add(1);
            } else {
                looked = looked.saturating_add(1);
            }
            if observed.found.is_a_change() {
                changes = changes.saturating_add(1);
            }
        }

        // Written down whatever was found, including *nothing was wrong*: a
        // check that left no account could not answer *when was this last known
        // to be fine*, which is the question D37 exists for. An upstream
        // finding is also appended beside the artifact, where a reader of the
        // file finds it; the bytes half is an event rather than a property of
        // the provenance, so it lives only in the record (D20).
        match record(&held.path, provenance, matched, observed.as_ref(), at) {
            Ok(()) => {}
            Err(failure) => lines.push(format!(
                "    the finding could not be written down: {failure}"
            )),
        }
    }

    lines.push(String::new());
    lines.extend(verdict(corrupt, hub.is_some(), looked, changes, unanswered));

    Response {
        text: lines.join("\n"),
        served: true,
    }
}

/// The two sentences a check ends with, which are about different things.
///
/// Kept apart deliberately: corruption is a fact about this disk and a decay is
/// a fact about somebody else's server, and running them together would invite
/// a reader to think one caused the other.
fn verdict(
    corrupt: usize,
    asked_upstream: bool,
    looked: usize,
    changes: usize,
    unanswered: usize,
) -> Vec<String> {
    let mut said = vec![match corrupt {
        0 => "every artifact with a recorded digest still matches it.".to_owned(),
        _ => format!(
            "{corrupt} artifact(s) no longer match the digest recorded for them. That is a \
             fact about this disk rather than about the hub, and a measurement taken against \
             one of them is a measurement of the corruption (§7.49, §3.8)."
        ),
    }];
    if asked_upstream {
        said.push(match changes {
            0 if looked == 0 => format!(
                "nothing was checked upstream: {unanswered} repository question(s) got no \
                 answer, which says nothing about whether anything there has changed."
            ),
            0 => format!("{looked} checked upstream, and nothing there has changed."),
            _ => format!(
                "{changes} of {looked} changed upstream. Nothing is invalidated by that: the \
                 artifacts are here and their digests are what they were (D37). What a change \
                 costs is somebody else's ability to reproduce from the same reference."
            ),
        });
        if unanswered > 0 && looked > 0 {
            said.push(format!(
                "{unanswered} more got no answer at all, and are neither checked nor changed."
            ));
        }
    }
    said
}

/// Asks the repository an artifact came from what it says now.
///
/// `None` when there is no upstream to ask about, which is a state rather than
/// a failure: an artifact converted on this machine has an origin and no
/// repository (A7). Either way the reader is told which.
fn upstream(
    hub: &Hub,
    provenance: &Provenance,
    path: &Path,
    at: Timestamp,
    lines: &mut Vec<String>,
) -> Option<Observation> {
    let Some(observed) = decay::look(hub, provenance, file_of(path), at) else {
        lines.push(
            "      its origin is not a repository, so there is no upstream to look at".to_owned(),
        );
        return None;
    };
    lines.push(format!("      upstream: {}", observed.found));
    Some(observed)
}

/// Re-reads an artifact and compares it with the digest recorded for it.
///
/// The second half of the answer is `Some(false)` when the bytes have changed,
/// `Some(true)` when they have not, and `None` when there is nothing to compare
/// against — which is a third state rather than a pass (A7).
fn bytes_here(path: &Path, provenance: &Provenance) -> (Vec<String>, Option<bool>) {
    match provenance.integrity() {
        Attested::Known(recorded) => match integrity::verify(path, recorded) {
            Ok(()) => (
                vec![format!(
                    "  {} — the bytes here are the bytes that arrived",
                    path.display()
                )],
                Some(true),
            ),
            Err(failure) => {
                let mut said = vec![format!("  {} — {failure}", path.display())];
                for entry in failure.context() {
                    said.push(format!("      {}: {}", entry.key, entry.value));
                }
                (said, Some(false))
            }
        },
        Attested::Unknown => (
            vec![format!(
                "  {} — no digest was recorded for it, so the bytes cannot be checked against \
                 anything (A7)",
                path.display()
            )],
            None,
        ),
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

/// Writes what was found into the record, and an upstream finding beside the
/// artifact as well.
///
/// `matched` is the bytes half: `Some(true)` when the artifact still matches
/// the digest recorded for it, `Some(false)` when it does not, and `None` when
/// nothing was recorded to compare against — three states rather than a pass
/// and a fail (A7). `observed` is the upstream half, absent when nobody asked
/// for it.
fn record(
    path: &Path,
    provenance: &Provenance,
    matched: Option<bool>,
    observed: Option<&Observation>,
    at: Timestamp,
) -> mcf_core::failure::Result<()> {
    if let Some(observed) = observed {
        store::record_provenance(path, &provenance.clone().observed(observed.clone()))?;
    }

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
            (
                "bytes",
                Value::text(match matched {
                    Some(true) => "matched",
                    Some(false) => "changed",
                    None => "no digest was recorded to compare against",
                }),
            ),
            (
                "upstream",
                match observed {
                    Some(observed) => Value::text(observed.found.as_str()),
                    None => Value::Null,
                },
            ),
            (
                "detail",
                match observed {
                    Some(observed) => Value::text(observed.found.to_string()),
                    None => Value::text("the upstream was not asked"),
                },
            ),
            (
                "is_a_change",
                Value::Bool(
                    matched == Some(false)
                        || observed.is_some_and(|observed| observed.found.is_a_change()),
                ),
            ),
        ]),
    ))?;
    Ok(())
}
