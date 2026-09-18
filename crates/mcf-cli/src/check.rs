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
use crate::pull::{DEFAULT_HUB, Offered, credential};
use mcf_hub::wire::for_url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    Everything,
    HereOnly,
}

pub(crate) fn run(
    only: Option<&str>,
    reach: Reach,
    from: Option<&str>,
    offered: Offered<'_>,
) -> Response {
    let stores = models::stores();
    if let Some(response) = nothing_to_check(&stores) {
        return response;
    }
    let holding: Vec<Held> = models::held_everywhere()
        .into_iter()
        .flat_map(|(_, holding)| holding)
        .collect();

    let wanted: Vec<&Held> = holding
        .iter()
        .filter(|held| only.is_none_or(|name| held.path.to_string_lossy().contains(name)))
        .collect();
    if wanted.is_empty() {
        return Response {
            text: match only {
                Some(name) => format!("mcf: this machine holds nothing called {name}"),
                None => format!(
                    "no models: nothing is held in any of the {} store(s) MCF knows about",
                    stores.len()
                ),
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

    let hashed = bytes_here_for_each(&wanted);

    for (index, held) in wanted.iter().enumerate() {
        let Ok(provenance) = &held.provenance else {
            lines.push(format!(
                "  {} — nothing beside it says where it came from, so there is nothing to \
                 check it against (A7)",
                held.path.display()
            ));
            continue;
        };

        let (said, matched) = match hashed.iter().find(|(at, _)| *at == index) {
            Some((_, hashed)) => hashed.clone(),
            None => bytes_here(&held.path, provenance),
        };
        lines.extend(said);
        if matched == Some(false) {
            corrupt = corrupt.saturating_add(1);
        }

        let observed = hub
            .as_ref()
            .and_then(|hub| upstream(hub, provenance, &held.path, at, &mut lines));
        if let Some(observed) = &observed {
            if matches!(observed.found, Decay::Unreachable { .. }) {
                unanswered = unanswered.saturating_add(1);
            } else {
                looked = looked.saturating_add(1);
            }
            if observed.found.is_a_change() {
                changes = changes.saturating_add(1);
            }
        }

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

fn nothing_to_check(stores: &[std::path::PathBuf]) -> Option<Response> {
    if stores.is_empty() {
        return Some(Response {
            text: format!(
                "mcf: there is nowhere to look — none of {}, XDG_DATA_HOME or HOME says where \
                 models go",
                models::STORES
            ),
            served: false,
        });
    }
    if stores.iter().all(|root| !root.exists()) {
        return Some(Response {
            text: format!(
                "no models: {} does not exist yet\n\
                 \x20 nothing has been acquired on this machine, so there is nothing to check",
                stores
                    .first()
                    .map(|root| root.display().to_string())
                    .unwrap_or_default()
            ),
            served: true,
        });
    }
    None
}

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

/// How many artifacts MCF hashes at once.
///
/// Eight threads ask for about three gigabytes a second, which is as much as a fast disk
/// will give and enough to keep the cores that matter busy. Every core at once would only
/// queue deeper on the disk, and on a spinning one it would turn one sequential read into
/// a scramble of seeks and come out slower than doing them one at a time.
const AT_ONCE: usize = 8;

/// What hashing one artifact said, and whether its bytes still match.
type Hashed = (Vec<String>, Option<bool>);

/// Hash every artifact that has a digest recorded, several at a time.
///
/// Hashing is the whole cost of a check — a library of large models is hundreds of
/// gigabytes, and MCF reads every byte of it at a few hundred megabytes a second on one
/// core. One artifact at a time made that a wait of many minutes while thirty-one cores
/// sat idle. Each thread takes the next artifact not yet claimed, so one enormous file
/// does not hold up the rest, and the findings are put back in the order they were asked
/// for rather than the order they finished.
fn bytes_here_for_each(wanted: &[&Held]) -> Vec<(usize, Hashed)> {
    let at_once = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(AT_ONCE)
        .min(wanted.len())
        .max(1);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: std::sync::Mutex<Vec<(usize, Hashed)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..at_once {
            let _hashing = scope.spawn(|| {
                loop {
                    let mine = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(held) = wanted.get(mine) else {
                        return;
                    };
                    let Ok(provenance) = &held.provenance else {
                        continue;
                    };
                    let said = bytes_here(&held.path, provenance);
                    done.lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push((mine, said));
                }
            });
        }
    });
    let mut hashed = done
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    hashed.sort_by_key(|(at, _)| *at);
    hashed
}

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

fn hub_for(from: Option<&str>, offered: Offered<'_>) -> std::result::Result<Hub, Response> {
    let refuse = |failure: &mcf_core::failure::Failure| Response {
        text: crate::say::refusal("nothing was checked", failure),
        served: false,
    };
    let base = Url::parse(from.unwrap_or(DEFAULT_HUB)).map_err(|failure| refuse(&failure))?;
    let wire = for_url(&base).map_err(|failure| refuse(&failure))?;
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

fn file_of(path: &Path) -> Option<&str> {
    path.file_name().and_then(std::ffi::OsStr::to_str)
}

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
