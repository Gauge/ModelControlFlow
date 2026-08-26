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

/// Where this machine keeps its models.
///
/// `None` when neither `XDG_DATA_HOME` nor `HOME` is set, which is the same
/// answer `mcf_record::journal::default_path` gives and for the same reason.
#[must_use]
pub(crate) fn default_root() -> Option<PathBuf> {
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

/// Where a removal shelves what it removes.
///
/// Beside the store rather than inside it, so that a shelved artifact is not
/// listed as held — and on the same filesystem, so that shelving is a rename
/// and putting something back costs nothing.
#[must_use]
fn shelf_beside(root: &Path) -> PathBuf {
    root.with_file_name("removed")
}

/// What this machine is holding.
pub(crate) fn list() -> Response {
    let Some(root) = default_root() else {
        return Response {
            text: "mcf: there is nowhere to look — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    if !root.exists() {
        return Response {
            text: format!(
                "no models: {} does not exist yet\n\
                 \x20 nothing has been acquired on this machine",
                root.display()
            ),
            served: true,
        };
    }

    match store::held(&root) {
        Err(failure) => Response {
            text: format!(
                "mcf: the model store could not be read\n{}",
                crate::say::beneath(&failure)
            ),
            served: false,
        },
        Ok(holding) if holding.is_empty() => Response {
            text: format!("no models: {} is empty", root.display()),
            served: true,
        },
        Ok(holding) => Response {
            text: render(&root, &holding),
            served: true,
        },
    }
}

/// What a listing says.
///
/// Every artifact, with what is known about where it came from and what is not.
/// A7 in the surface: *nothing beside it says where it came from* is a
/// different line from a repository name, and an operator reading a list should
/// be able to see which of their models MCF can account for.
fn render(root: &Path, holding: &[Held]) -> String {
    let mut lines = vec![format!(
        "{} model file(s) in {}",
        holding.len(),
        root.display()
    )];
    let mut unaccounted = 0_usize;
    for held in holding {
        lines.push(format!("  {}", held.describe()));
        if held.provenance.is_err() {
            unaccounted = unaccounted.saturating_add(1);
        }
    }
    if unaccounted > 0 {
        lines.push(format!(
            "\n{unaccounted} of them cannot say where they came from. That is a state, not a \
             defect —\nbut a measurement against one of them carries the same gap (§3.6)."
        ));
    }
    lines.join("\n")
}

/// Removes an artifact, or says what removing it would do.
///
/// `reason` is the authorization: without one this previews and removes
/// nothing. `purge` deletes what was shelved, in the same authorized act, which
/// is the only way anything in MCF is destroyed.
pub(crate) fn remove(names: &[&str], reason: Option<&str>, purge: bool) -> Response {
    let Some(root) = default_root() else {
        return Response {
            text: "mcf: there is nowhere to look — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    let paths: Vec<PathBuf> = names.iter().map(|name| resolve(&root, name)).collect();
    let shelf = shelf_beside(&root);

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
fn resolve(root: &Path, name: &str) -> PathBuf {
    let given = Path::new(name);
    if given.is_absolute() {
        given.to_path_buf()
    } else {
        root.join(given)
    }
}
