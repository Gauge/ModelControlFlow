use std::path::{Path, PathBuf};

use mcf_core::failure::Failure;
use mcf_core::provenance::Provenance;
use mcf_core::time::Timestamp;
use mcf_hub::client::Hub;
use mcf_hub::credentials::{self, Credential, Origin as Held, Secret};
use mcf_hub::fetch::{Acquired, Verification};
use mcf_hub::fitment::{self, Verdict};
use mcf_hub::http::Url;
use mcf_hub::offer::{PLANNING_CONTEXT, Plan, plan_for};
use mcf_hub::reference::{self};
use mcf_hub::source::{Entry, Listing, Source as _};
use mcf_hub::wire::for_url;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

use crate::Response;
use crate::models;

pub(crate) const DEFAULT_HUB: &str = "https://huggingface.co/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Offered<'a> {
    Nothing,
    File(&'a str),
    Variable(&'a str),
}

pub(crate) fn run(
    asked_for: &str,
    from: Option<&str>,
    into: Option<&str>,
    offered: Offered<'_>,
    fresh: bool,
) -> Response {
    let reference = match reference::parse(asked_for) {
        Ok(reference) => reference,
        Err(_) if !asked_for.contains(['/', ':', '@', ' ']) && !asked_for.trim().is_empty() => {
            return crate::acquire::searched(asked_for, from, fresh);
        }
        Err(failure) => return refused("that is not a reference MCF can resolve", &failure),
    };
    let base = match Url::parse(from.unwrap_or(DEFAULT_HUB)) {
        Ok(base) => base,
        Err(failure) => return refused("that is not a hub MCF can reach", &failure),
    };
    let root = match into {
        Some(named) => {
            let named = std::path::Path::new(named);
            if !named.is_absolute() {
                return Response {
                    text: format!(
                        "mcf: --into needs an absolute path, and {} is not one\n  MCF will not \
                         resolve a store against whatever directory it was started in (A7)",
                        named.display()
                    ),
                    served: false,
                };
            }
            named.to_path_buf()
        }
        None => match models::default_root() {
            Some(root) => root,
            None => {
                return Response {
                    text: format!(
                        "mcf: there is nowhere to keep a model — none of {}, XDG_DATA_HOME or \
                         HOME says where models go, and --into names no store either",
                        models::STORES
                    ),
                    served: false,
                };
            }
        },
    };

    let wire = match for_url(&base) {
        Ok(wire) => wire,
        Err(failure) => return refused("nothing was acquired", &failure),
    };
    let hub = match credential(offered, &environment) {
        Ok(None) => Hub::at(base, wire),
        Ok(Some(credential)) => Hub::at(base, wire).offering(credential),
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let listing = match hub.list(&reference) {
        Ok(listing) => listing,
        Err(failure) => {
            let mut response = refused("nothing was acquired", &failure);
            if failure.category() == mcf_core::failure::Category::HubAuthRequired {
                response.text.push('\n');
                response.text.push_str(&what_is_lying_around(&environment));
            }
            return response;
        }
    };

    let Some(wanted) = reference.file.clone() else {
        let planned = free_memory().and_then(|free| plan_for(&hub, &listing, free));
        if let Ok(plan) = &planned {
            record_plan(&listing, plan, mcf_core::time::Timestamp::now());
        }
        return Response {
            text: offer(&listing, &planned),
            served: true,
        };
    };
    let Some(entry) = listing.entry(&wanted).cloned() else {
        return Response {
            text: format!(
                "mcf: {} publishes no file called {wanted}\n{}",
                listing.reference.repository(),
                offer(
                    &listing,
                    &Err("no plan was made: the file named is not one of these".to_owned())
                )
            ),
            served: false,
        };
    };

    acquire_set(&hub, &listing, &wanted, entry, &root)
}

fn acquire_set(hub: &Hub, listing: &Listing, wanted: &str, entry: Entry, root: &Path) -> Response {
    let parts: Vec<Entry> = match listing.parts_of(wanted) {
        Some(set) if !set.is_whole() => {
            return Response {
                text: format!(
                    "mcf: {wanted} is one of {} parts of a model, and {} publishes only {} of \
                     them: nothing was acquired, because an engine loads the set or nothing",
                    set.of,
                    listing.reference.repository(),
                    set.parts.len()
                ),
                served: false,
            };
        }
        Some(set) => set.parts.into_iter().cloned().collect(),
        None => vec![entry],
    };
    let mut said = Vec::new();
    let count = parts.len();
    let mut about_the_first = None;
    for (index, part) in parts.iter().enumerate() {
        if count > 1 {
            said.push(format!(
                "part {} of {count}: {} ({} bytes)",
                index.saturating_add(1),
                part.path,
                part.size
            ));
        }
        let response = acquire_one(hub, listing, part, root);
        if !response.served {
            said.push(response.text);
            return Response {
                text: said.join("\n"),
                served: false,
            };
        }
        if about_the_first.is_none() {
            about_the_first = Some(response.text);
        }
    }
    if count > 1 {
        said.push(format!(
            "{count} parts, {} bytes in all",
            parts
                .iter()
                .map(|part| part.size)
                .fold(0_u64, u64::saturating_add)
        ));
    }
    said.extend(about_the_first);
    Response {
        text: said.join("\n"),
        served: true,
    }
}

fn free_memory() -> Result<mcf_core::measurement::Bytes, String> {
    match mcf_core::hardware::Machine::read().memory.available {
        mcf_core::attested::Attested::Known(available) => Ok(available),
        mcf_core::attested::Attested::Unknown => {
            Err("this machine will not say how much memory is free".to_owned())
        }
    }
}

fn acquire_one(hub: &Hub, listing: &Listing, entry: &Entry, root: &Path) -> Response {
    let done = match mcf_hub::acquisition::one(
        hub,
        listing,
        entry,
        root,
        &mcf_hub::stopping::Stopping::never(),
    ) {
        Ok(done) => done,
        Err(failure) => return refused("nothing was acquired", &failure),
    };
    let again = match free_memory().and_then(|free| plan_for(hub, listing, free)) {
        Ok(plan) => plan_lines(&plan)
            .into_iter()
            .find(|line| line.contains(&entry.path))
            .ok_or_else(|| {
                format!(
                    "the plan does not mention {}, which is a plan about another repository",
                    entry.path
                )
            }),
        Err(why) => Err(why),
    };
    Response {
        text: render(
            &done.acquired,
            listing,
            &done.sidecar,
            &done.provenance,
            done.recorded.as_ref(),
            &again,
        ),
        served: true,
    }
}

fn environment(variable: &str) -> Option<String> {
    std::env::var(variable).ok()
}

pub(crate) fn credential(
    offered: Offered<'_>,
    look_up: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<Credential>, String> {
    let (token, origin) = match offered {
        Offered::Nothing => return Ok(None),
        Offered::File(path) => {
            let read = std::fs::read_to_string(path).map_err(|error| {
                format!("mcf: the credential file could not be read\n  {path}: {error}")
            })?;
            (
                read.trim().to_owned(),
                Held::File {
                    path: PathBuf::from(path),
                },
            )
        }
        Offered::Variable(name) => (
            look_up(name)
                .ok_or_else(|| {
                    format!(
                        "mcf: {name} is not set, so there is no credential to offer\n  MCF reads \
                         an environment variable only when it is named, and this one holds nothing"
                    )
                })?
                .trim()
                .to_owned(),
            Held::Environment {
                variable: name.to_owned(),
            },
        ),
    };
    if token.is_empty() {
        return Err(format!(
            "mcf: {origin} holds nothing\n  an empty credential is not a credential, and \
             offering one would produce a refusal nobody could explain"
        ));
    }
    Ok(Some(Credential::new(Secret::new(token), origin)))
}

fn what_is_lying_around(look_up: &dyn Fn(&str) -> Option<String>) -> String {
    let seen = credentials::sightings(look_up, None, &|_| None);
    if seen.is_empty() {
        return "  MCF looked for a credential on this machine and found none.\n  Offer one with \
                --token-from <file> or --token-from-env <VARIABLE>."
            .to_owned();
    }
    let mut lines = vec!["  MCF has looked, and used nothing:".to_owned()];
    for sighting in &seen {
        lines.push(format!("    {}", sighting.describe()));
        if let Held::Environment { variable } = &sighting.origin {
            lines.push(format!("    offer it with --token-from-env {variable}"));
        }
    }
    lines.join("\n")
}

fn plan_lines(plan: &Plan) -> Vec<String> {
    {
        let available = plan.available;
        plan.verdicts
            .iter()
            .map(|(name, verdict)| match verdict {
                Verdict::Fits { needs, headroom } => format!(
                    "  {name} — fits: needs {} of {} usable, {} left",
                    needs.0, available.0, headroom.0
                ),
                Verdict::FitsWithoutContextHeadroom {
                    needs,
                    longest_context,
                } => format!(
                    "  {name} — fits at a shorter context: {} at {PLANNING_CONTEXT} tokens is \
                     more than this machine has; {longest_context} tokens would fit",
                    needs.0
                ),
                Verdict::DoesNotFit { needs, short_by } => format!(
                    "  {name} — does NOT fit: needs {}, which is {} more than this machine has",
                    needs.0, short_by.0
                ),
            })
            .collect()
    }
}

fn record_plan(listing: &Listing, plan: &Plan, at: Timestamp) {
    let Some(path) = mcf_record::journal::default_path() else {
        return;
    };
    let Ok(mut journal) = Journal::open(&path) else {
        return;
    };
    let body = Value::map([
        ("repository", Value::text(listing.reference.repository())),
        (
            "revision",
            match listing.revision.as_deref() {
                Some(revision) => Value::text(revision),
                None => Value::Null,
            },
        ),
        (
            "plan",
            fitment::planned(&plan.verdicts, PLANNING_CONTEXT, plan.available),
        ),
    ]);
    let _written = journal.append(&Record::new(EntryKind::FitmentPlanned, at, body));
}

fn offer(listing: &Listing, planned: &std::result::Result<Plan, String>) -> String {
    let mut lines = vec![format!(
        "{} publishes {} file(s) at {}",
        listing.reference.repository(),
        listing.entries.len(),
        listing
            .revision
            .as_deref()
            .unwrap_or("a revision the hub did not name")
    )];
    lines.push(mcf_hub::licence::describe(
        listing
            .declared_licence
            .as_deref()
            .and_then(mcf_hub::licence::recognize)
            .as_ref(),
    ));
    for variant in listing.variants() {
        let parts = match variant.parts {
            0 | 1 => String::new(),
            parts if variant.whole => format!(", in {parts} files"),
            parts => format!(
                ", in {parts} files — and the repository publishes fewer than its names declare, \
                 so an engine could load none of it"
            ),
        };
        let digest = if variant.digested {
            ""
        } else if variant.parts > 1 {
            " (the hub declares no digest for one of these)"
        } else {
            " (the hub declares no digest for this one)"
        };
        lines.push(format!(
            "  {} — {} bytes{parts}{digest}",
            variant.name, variant.bytes
        ));
    }
    match planned {
        Ok(plan) => {
            lines.push(format!(
                "\nat {PLANNING_CONTEXT} tokens of context, on this machine:"
            ));
            lines.extend(plan_lines(plan));
        }
        Err(why) => lines.push(format!(
            "\nMCF cannot say which of these would run here: {why}"
        )),
    }
    lines.push(format!(
        "\nnothing was acquired: name the file you want, as\n  mcf pull {}:<file>",
        listing.reference.repository()
    ));
    lines.join("\n")
}

fn render(
    acquired: &Acquired,
    listing: &Listing,
    sidecar: &Path,
    provenance: &Provenance,
    recorded: Result<&PathBuf, &Failure>,
    plan_now: &std::result::Result<String, String>,
) -> String {
    let verification = match &acquired.verification {
        Verification::Digest { digest } => format!("verified against the hub's digest: {digest}"),
        Verification::LengthOnly { digest } => format!(
            "HELD, NOT VERIFIED: the hub declared no digest, so all MCF can say is that {} bytes \
             arrived and their digest is {digest}",
            acquired.bytes
        ),
    };
    let mut lines = vec![
        format!("acquired {}", acquired.path.display()),
        format!("  {} bytes, {verification}", acquired.bytes),
        format!(
            "  from {} at {}",
            listing.reference.repository(),
            listing
                .revision
                .as_deref()
                .unwrap_or("a revision the hub did not name")
        ),
        format!(
            "  {}",
            mcf_hub::licence::describe(provenance.licence().known())
        ),
        format!("  provenance beside it: {}", sidecar.display()),
    ];
    if let Some(source) = provenance.source() {
        lines.push(format!(
            "  made from {}, which MCF has not fetched and cannot vouch for",
            source.origin()
        ));
        for transformation in provenance.transformations() {
            lines.push(format!("    {transformation}"));
        }
    }
    if acquired.attempts > 1 {
        lines.push(format!(
            "  it took {} transfers{}",
            acquired.attempts,
            if acquired.resumed {
                ", continuing where each stopped"
            } else {
                ", starting again each time"
            }
        ));
    }
    match plan_now {
        Ok(verdict) => {
            lines.push(format!(
                "  and now that it is here, at {PLANNING_CONTEXT} tokens of context:"
            ));
            lines.push(format!("  {}", verdict.trim_start()));
        }
        Err(why) => lines.push(format!(
            "  whether it will run here is a question MCF cannot answer: {why}"
        )),
    }
    match recorded {
        Ok(path) => lines.push(format!("  recorded in {}", path.display())),
        Err(failure) => lines.push(format!(
            "  NOT recorded: {failure}\n   the model is on the disk with its provenance; what is \
             missing is the record that it arrived"
        )),
    }
    lines.join("\n")
}

fn refused(what: &str, failure: &Failure) -> Response {
    Response {
        text: crate::say::refusal(what, failure),
        served: false,
    }
}

#[cfg(test)]
pub(crate) fn licence_of(listing: &Listing) -> Option<mcf_core::provenance::Licence> {
    listing
        .declared_licence
        .as_deref()
        .and_then(mcf_hub::licence::recognize)
}

#[cfg(test)]
mod tests;
