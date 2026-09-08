use std::path::{Path, PathBuf};

use mcf_hub::store::{self, Authorization, Held, Plan};
use mcf_record::journal::Journal;

use crate::Response;

pub(crate) const STORES: &str = "MCF_MODELS";

#[must_use]
pub(crate) fn stores() -> Vec<PathBuf> {
    if let Some(named) = std::env::var_os(STORES) {
        let listed: Vec<PathBuf> = std::env::split_paths(&named)
            .filter(|path| path.is_absolute() && !path.as_os_str().is_empty())
            .collect();
        if !listed.is_empty() {
            return listed;
        }
    }
    platform_store().into_iter().collect()
}

#[must_use]
pub(crate) fn ignored_stores() -> Vec<PathBuf> {
    match std::env::var_os(STORES) {
        None => Vec::new(),
        Some(named) => std::env::split_paths(&named)
            .filter(|path| !path.as_os_str().is_empty() && !path.is_absolute())
            .collect(),
    }
}

#[must_use]
pub(crate) fn default_root() -> Option<PathBuf> {
    stores().into_iter().next()
}

#[must_use]
pub(crate) fn platform_store() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/share"))
        })?;
    Some(base.join("mcf").join("models"))
}

#[must_use]
pub(crate) fn held_everywhere() -> Vec<(PathBuf, Vec<Held>)> {
    stores()
        .into_iter()
        .filter(|root| root.exists())
        .filter_map(|root| store::held(&root).ok().map(|holding| (root, holding)))
        .collect()
}

fn store_holding(path: &Path) -> Option<PathBuf> {
    stores().into_iter().find(|store| path.starts_with(store))
}

#[must_use]
fn shelf_beside(root: &Path) -> PathBuf {
    root.with_file_name("removed")
}

pub(crate) fn list() -> Response {
    let stores = stores();
    if stores.is_empty() {
        return Response {
            text: format!(
                "mcf: there is nowhere to look — none of {STORES}, XDG_DATA_HOME or HOME says \
                 where models go"
            ),
            served: false,
        };
    }

    let mut lines = Vec::new();
    let mut found = 0_usize;
    let mut unaccounted = 0_usize;
    for root in &stores {
        if !root.exists() {
            lines.push(format!(
                "no models: {} does not exist yet\n\
                 \x20 nothing has been acquired there",
                root.display()
            ));
            continue;
        }
        match store::held(root) {
            Err(failure) => {
                lines.push(format!("{} — could not be read", root.display()));
                lines.push(crate::say::beneath(&failure));
            }
            Ok(holding) if holding.is_empty() => {
                lines.push(format!("no models: {} is empty", root.display()));
            }
            Ok(holding) => {
                found = found.saturating_add(holding.len());
                unaccounted = unaccounted.saturating_add(
                    holding
                        .iter()
                        .filter(|held| held.provenance.is_err())
                        .count(),
                );
                lines.push(render(root, &holding));
            }
        }
    }

    for ignored in ignored_stores() {
        lines.push(format!(
            "\n{STORES} names {}, which is not an absolute path: MCF will not resolve a store \
             against whatever directory it was started in (A7)",
            ignored.display()
        ));
    }

    if found == 0 && stores.len() > 1 {
        lines.push(format!(
            "\nnothing is held in any of the {} stores {STORES} names",
            stores.len()
        ));
    }
    if unaccounted > 0 {
        lines.push(format!(
            "\n{unaccounted} of them cannot say where they came from. That is a state, not a \
             defect —\nbut a measurement against one of them carries the same gap (§3.6)."
        ));
    }

    Response {
        text: lines.join("\n"),
        served: true,
    }
}

fn render(root: &Path, holding: &[Held]) -> String {
    let companions = holding.iter().filter(|held| held.companion).count();
    let mut lines = vec![match companions {
        0 => format!("{} model(s) in {}", holding.len(), root.display()),
        _ => format!(
            "{} model(s) in {}, and {companions} companion file(s) that belong to one rather \
             than being one",
            holding.len().saturating_sub(companions),
            root.display()
        ),
    }];
    for held in holding {
        lines.push(format!("  {}", held.describe()));
        lines.push(format!("    {}", held.terms()));
    }
    lines.join("\n")
}

#[allow(
    clippy::too_many_lines,
    reason = "one removal, in the four acts B-027 requires — resolve, preview, \
              authorize, record — and splitting them would put the authorization \
              somewhere other than beside what it authorizes"
)]
pub(crate) fn remove(names: &[&str], reason: Option<&str>, purge: bool) -> Response {
    let Some(root) = default_root() else {
        return Response {
            text: format!(
                "mcf: there is nowhere to look — none of {STORES}, XDG_DATA_HOME or HOME says \
                 where models go"
            ),
            served: false,
        };
    };
    let paths: Vec<PathBuf> = names.iter().map(|name| resolve(&root, name)).collect();
    let shelf = paths
        .first()
        .and_then(|path| store_holding(path))
        .map_or_else(|| shelf_beside(&root), |store| shelf_beside(&store));

    let plan = match store::preview(&paths, &shelf) {
        Ok(plan) => plan,
        Err(failure) => {
            return Response {
                text: format!(
                    "mcf: nothing was removed\n{}",
                    crate::say::beneath(&failure)
                ),
                served: false,
            };
        }
    };

    let Some(reason) = reason else {
        return Response {
            text: preview_text(&plan, purge),
            served: true,
        };
    };

    let authorization = match Authorization::given(&plan, reason) {
        Ok(authorization) => authorization,
        Err(failure) => {
            return Response {
                text: format!(
                    "mcf: nothing was removed\n{}",
                    crate::say::beneath(&failure)
                ),
                served: false,
            };
        }
    };

    let (mut journal, journal_path) = match the_record() {
        Ok(both) => both,
        Err(response) => return response,
    };

    let removed = match store::remove(
        &plan,
        &authorization,
        &mut journal,
        mcf_core::time::Timestamp::now(),
    ) {
        Ok(removed) => removed,
        Err(failure) => {
            return Response {
                text: format!(
                    "mcf: nothing was removed\n{}",
                    crate::say::beneath(&failure)
                ),
                served: false,
            };
        }
    };

    let mut lines = vec![format!(
        "removed {} file(s), {} bytes, because: {reason}",
        removed.shelved.len(),
        removed.bytes
    )];
    for (path, why) in &removed.refused {
        lines.push(format!("  NOT removed: {} — {why}", path.display()));
    }

    if purge {
        match store::purge(&removed, &authorization, &plan) {
            Ok(freed) => lines.push(format!(
                "purged: {freed} bytes are gone and cannot be brought back"
            )),
            Err(failure) => {
                lines.push(format!(
                    "the purge did not finish\n{}",
                    crate::say::beneath(&failure)
                ));
                return Response {
                    text: lines.join("\n"),
                    served: false,
                };
            }
        }
    } else {
        lines.push(format!(
            "shelved in {}\n\x20 nothing was deleted: put it back by moving it, or delete it \
             with --purge",
            shelf.display()
        ));
    }
    lines.push(format!("recorded in {}", journal_path.display()));

    Response {
        text: lines.join("\n"),
        served: removed.complete(),
    }
}

fn the_record() -> std::result::Result<(Journal, PathBuf), Response> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(Response {
            text: "mcf: nothing was removed — there is nowhere to record it, and a removal \
                   nobody recorded is one nobody can account for (A1)"
                .to_owned(),
            served: false,
        });
    };
    match Journal::open(&path) {
        Ok(journal) => Ok((journal, path)),
        Err(failure) => Err(Response {
            text: format!(
                "mcf: nothing was removed — the record could not be opened, and the record goes \
                 first (A1)\n{}",
                crate::say::beneath(&failure)
            ),
            served: false,
        }),
    }
}

fn preview_text(plan: &Plan, purge: bool) -> String {
    let ending = if purge {
        "nothing was removed. --purge deletes what a removal shelves, and it needs the same \
         authorization: add --because \"<why>\""
    } else {
        "nothing was removed. To go ahead, say why: --because \"<why>\""
    };
    format!("{}\n{ending}", plan.describe())
}

fn resolve(root: &Path, name: &str) -> PathBuf {
    let given = Path::new(name);
    if given.is_absolute() {
        return given.to_path_buf();
    }
    stores()
        .into_iter()
        .map(|store| store.join(given))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| root.join(given))
}

pub(crate) fn read_prefix(path: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if mcf_standin::gguf::parse(&prefix).is_ok() {
            return Some(prefix);
        }
        if take >= held {
            return None;
        }
    }
    None
}

pub(crate) fn resolve_named(named: &str) -> Result<Option<PathBuf>, Vec<PathBuf>> {
    let given = Path::new(named);
    if given.is_file() {
        return Ok(Some(given.to_path_buf()));
    }
    let relative = named.replace(':', "/");
    let found: Vec<PathBuf> = stores()
        .into_iter()
        .map(|root| root.join(&relative))
        .filter(|candidate| candidate.is_file())
        .collect();
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.into_iter().next()),
        _ => Err(found),
    }
}

pub(crate) fn ambiguous(named: &str, found: &[PathBuf]) -> String {
    let mut lines = vec![format!(
        "mcf: {named} names {} files, in different stores:",
        found.len()
    )];
    for path in found {
        lines.push(format!("  {}", path.display()));
    }
    lines.push(
        "  name one of those paths. MCF will not choose: they are two artifacts with two \
         provenances, and holding whichever one MCF reached first would be a hold nobody \
         could account for"
            .to_owned(),
    );
    lines.join("\n")
}
