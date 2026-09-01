//! `mcf list` and `mcf rm`: what this machine is holding, and how something
//! stops being held (B-029, B-027, §III).
//!
//! **Where models live.** `$XDG_DATA_HOME/mcf/models`, falling back to
//! `$HOME/.local/share/mcf/models` — beside the record, by the same rule, and
//! `None` when neither is set. A7 applied to a path: MCF does not invent a
//! place to keep somebody's models any more than it invents a place to write
//! their evidence.
//!
//! **`rm` is two commands and looks like one.** Run without `--because`, it
//! previews: it says exactly what would go, how much it weighs, and whether it
//! could be brought back, and it removes nothing. Run with a reason, it
//! authorizes that plan and moves the artifact to a shelf beside the store.
//! Neither form deletes anything: §3.11 makes reclaiming space a decision, and
//! `--purge` is where somebody makes it.
//!
//! **Nothing here pulls.** `mcf pull` needs a transport, and the vendoring
//! decision a network stack requires has not been made (B-021, DEC-011). The
//! usage text says so rather than omitting the command, because an operator who
//! cannot find `pull` should learn why it is absent rather than wonder.

use std::path::{Path, PathBuf};

use mcf_hub::store::{self, Authorization, Held, Plan};
use mcf_record::journal::Journal;

use crate::Response;

/// The variable that says where models go.
///
/// A list of absolute paths separated the way every path list on this platform
/// is separated, in preference order: the **first** is where a new acquisition
/// goes unless one is named, and **all** of them are searched for what is held.
///
/// An environment variable rather than a configuration file, and one variable
/// rather than a set of them, because §5 refuses MCF a configuration language
/// and a list in a variable is the platform's own idiom rather than a language.
/// Where a machine wants this to persist, the shell profile is where a machine
/// persists an environment variable.
pub(crate) const STORES: &str = "MCF_MODELS";

/// Everywhere this machine keeps models, in preference order.
///
/// `MCF_MODELS` when it is set, and the platform's data home otherwise. A
/// relative path in the list is **dropped and named** rather than resolved
/// against whatever directory MCF happened to be started in: a data path that
/// depends on the caller's working directory is a data path that moves (A7).
///
/// Empty when nothing says where to put anything — neither the variable nor
/// `XDG_DATA_HOME` nor `HOME` — which is the same refusal the record makes in
/// the same situation rather than an invented location.
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

/// Any relative path in the list, which MCF will not resolve for the caller.
///
/// Returned rather than logged, so the surface that has an operator's attention
/// is the one that tells them a store they named is being ignored (A2).
#[must_use]
pub(crate) fn ignored_stores() -> Vec<PathBuf> {
    match std::env::var_os(STORES) {
        None => Vec::new(),
        Some(named) => std::env::split_paths(&named)
            .filter(|path| !path.as_os_str().is_empty() && !path.is_absolute())
            .collect(),
    }
}

/// Where a new acquisition goes when nobody names a store.
#[must_use]
pub(crate) fn default_root() -> Option<PathBuf> {
    stores().into_iter().next()
}

/// The store the platform would put models in, ignoring what anybody asked for.
///
/// `None` when neither `XDG_DATA_HOME` nor `HOME` is set, which is the same
/// answer `mcf_record::journal::default_path` gives and for the same reason.
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

/// Everything held, across every store, in the order the stores are searched.
///
/// A model in two stores appears twice, which is the honest answer: A1 forbids
/// dropping the second, and an operator with two copies has a fact worth
/// knowing rather than a duplicate MCF should quietly resolve.
#[must_use]
pub(crate) fn held_everywhere() -> Vec<(PathBuf, Vec<Held>)> {
    stores()
        .into_iter()
        .filter(|root| root.exists())
        .filter_map(|root| store::held(&root).ok().map(|holding| (root, holding)))
        .collect()
}

/// Which store a path is in, if it is in one.
fn store_holding(path: &Path) -> Option<PathBuf> {
    stores().into_iter().find(|store| path.starts_with(store))
}

/// Where a removal shelves what it removes.
///
/// Beside the store rather than inside it, so that a shelved artifact is not
/// listed as held — and on the same filesystem, so that shelving is a rename
/// and putting something back costs nothing.
#[must_use]
fn shelf_beside(root: &Path) -> PathBuf {
    root.with_file_name("removed")
}

/// What this machine is holding, across every store it keeps models in.
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
            // The same sentence `mcf check` uses for the same state: a store
            // nobody has created is not an empty one, and two surfaces saying
            // it differently is two answers to one situation (A6).
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

    // A store the operator named and MCF will not use, said where they are
    // looking rather than in a log nobody reads (A2).
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

/// What a listing says.
///
/// Every artifact, with what is known about where it came from and what is not.
/// A7 in the surface: *nothing beside it says where it came from* is a
/// different line from a repository name, and an operator reading a list should
/// be able to see which of their models MCF can account for.
fn render(root: &Path, holding: &[Held]) -> String {
    // Counted apart, because a projector is not a model: counting it among
    // them said sixteen where there were fifteen and a companion, and every
    // sweep of *every model on this machine* had one entry that could only
    // ever fail (A7).
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
        // §III asks that terms be legible *before use*, and this is where an
        // operator sees a model before using it (B-023).
        lines.push(format!("    {}", held.terms()));
    }
    lines.join("\n")
}

/// Removes an artifact, or says what removing it would do.
///
/// `reason` is the authorization: without one this previews and removes
/// nothing. `purge` deletes what was shelved, in the same authorized act, which
/// is the only way anything in MCF is destroyed.
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
    // The shelf sits beside the store the artifact is *in*, not beside the
    // default one: shelving is a rename, and a rename across filesystems is a
    // copy that can half-happen (B-027, A4).
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

/// The record a removal is written to, opened before anything moves.
///
/// A removal nobody recorded is one nobody can account for, so this failing is
/// a reason not to remove rather than a detail to report afterwards (A1).
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

/// What an operator sees before they have decided anything.
fn preview_text(plan: &Plan, purge: bool) -> String {
    let ending = if purge {
        "nothing was removed. --purge deletes what a removal shelves, and it needs the same \
         authorization: add --because \"<why>\""
    } else {
        "nothing was removed. To go ahead, say why: --because \"<why>\""
    };
    format!("{}\n{ending}", plan.describe())
}

/// A name the operator gave, as a path.
///
/// An absolute path is taken as given — an operator who names a file means that
/// file. Anything else is under the store, which is where `mcf list` said it
/// was.
/// Where a name to be removed is, across every store.
///
/// An absolute path is itself. A relative one is looked for in each store in
/// order, and the **first that exists** is what is removed — with the whole
/// list searched rather than only the default store, since a removal that could
/// not find what `mcf list` had just shown would be the surfaces disagreeing
/// about what this machine holds (A6).
///
/// A name in no store resolves against the first, so that what a preview says
/// would be removed is a path an operator can read and correct.
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
