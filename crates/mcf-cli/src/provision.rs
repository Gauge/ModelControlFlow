//! `mcf provision`: a component MCF installs, builds and pins itself, in an
//! environment it controls (B-367, D39, DEC-052, F30).
//!
//! **The command, not the build.** What a build is — the container, the
//! pinned image, the script, the record — lives in
//! [`mcf_serve::provisioning`], because the daemon builds too, when a window
//! holds a model that has no engine to run it; one builder, two callers
//! (A22, B-072). What is here is what makes provisioning a *command*: naming
//! a component, listing what can be built and what is, and removing one with
//! a reason.
//!
//! **A component that is not named is the one this machine needs.** `mcf
//! provision` with nothing after it builds what [`mcf_serve::engines::required`]
//! says a model here would run on — the same answer the window acts on when
//! a model is held with no engine, so the headless path can do what the window
//! does (A22).

use std::path::PathBuf;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use mcf_core::component::{COMPONENTS, Component};
use mcf_record::journal::EntryKind;
use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};
use mcf_serve::provisioning::{self, Outcome, is_complete, prefix_for, short};

use crate::Response;

/// Where a component lands when the operator does not say.
///
/// Under the data home, beside the models and the record — which on this
/// machine the operator has already pointed at the large drive. `--into` names
/// anywhere else.
pub(crate) fn default_root() -> Option<PathBuf> {
    crate::models::default_root().map(|models| {
        models.parent().map_or_else(
            || provisioning::root_under(&models),
            provisioning::root_under,
        )
    })
}

/// What a daemon that is *already running* says it can reach.
///
/// **Asked, not started.** Listing what MCF can build is a question about the
/// disk, and a question about the disk that started a daemon would be MCF
/// doing work nobody asked for (§3.8). So this connects where something is
/// already listening and gives up quietly everywhere else.
///
/// What it adds is the one thing the disk cannot answer: whether a prefix that
/// exists is a build MCF can actually reach as an engine. A directory is not a
/// binary, and treating the two as one is F31.
fn reachable_engines() -> std::collections::BTreeMap<String, bool> {
    let mut found = std::collections::BTreeMap::new();
    let Some(socket) = crate::serve::socket_path() else {
        return found;
    };
    let Ok(connection) = UnixStream::connect(&socket) else {
        return found;
    };
    let mut connection = connection;
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    if writeln!(connection, "{}", Request::Components.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return found;
    }
    let mut line = String::new();
    if BufReader::new(&connection).read_line(&mut line).is_err() {
        return found;
    }
    let Ok(answer) = Answer::read(&line) else {
        return found;
    };
    if !answer.served {
        return found;
    }
    if let Some(listed) = answer.body.get("components").and_then(Value::as_list) {
        for held in listed {
            if let Some(name) = held.get("name").and_then(Value::as_text) {
                found.insert(
                    name.to_owned(),
                    matches!(held.get("usable_engine"), Some(Value::Bool(true))),
                );
            }
        }
    }
    found
}

/// Lists what can be provisioned and what is.
pub(crate) fn list(into: Option<&str>) -> Response {
    let root = match root_from(into) {
        Ok(root) => root,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let reachable = reachable_engines();
    let mut lines = vec![format!(
        "MCF can provision {} component(s); prefixes under {}",
        COMPONENTS.len(),
        root.display()
    )];
    for component in COMPONENTS {
        let prefix = prefix_for(component, &root);
        let state = if is_complete(&prefix) {
            "provisioned"
        } else if prefix.exists() {
            "INCOMPLETE — a run stopped partway; provision it again or remove it"
        } else {
            "not provisioned"
        };
        lines.push(format!(
            "  {}@{}  —  {state}\n    {}\n    image {} ({})",
            component.name,
            short(component),
            component.role,
            component.image,
            component.image_digest,
        ));
        // Only where a daemon is up to be asked: silence here is "nobody was
        // asked", which is not the same as "it cannot be reached".
        if let Some(usable) = reachable.get(component.name) {
            lines.push(if *usable {
                "    the daemon reaches this as an engine".to_owned()
            } else {
                "    the daemon does NOT reach this as an engine".to_owned()
            });
        }
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}

/// The component to build: the one named, or the engine this machine needs.
fn chosen_component(name: Option<&str>) -> Result<&'static Component, String> {
    let Some(name) = name else {
        let backend = mcf_serve::engines::backend_present();
        return mcf_serve::engines::required(backend).ok_or_else(|| {
            "mcf: MCF's component table names no engine for this machine — adding one is a \
             change to MCF, not a setting"
                .to_owned()
        });
    };
    COMPONENTS
        .iter()
        .find(|component| component.name == name)
        .ok_or_else(|| {
            format!(
                "mcf: MCF does not know how to provision {name}\n  it knows: {}\n  adding one is a \
                 change to MCF, not a configuration (§5)",
                COMPONENTS
                    .iter()
                    .map(|component| component.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// What a build came to, whichever side of the socket built it.
///
/// The daemon's final line and the library's [`Outcome`] say the same things;
/// this is the one shape the command reports from, so the two paths cannot
/// drift apart in what they show (B-072).
enum Came {
    /// The prefix was complete before anything ran.
    Already { prefix: PathBuf },
    /// A build ran to the end.
    Built {
        prefix: PathBuf,
        log: PathBuf,
        toolchain: String,
        recorded: Result<PathBuf, String>,
        /// Whether a daemon now reaches it as an engine — known only when a
        /// daemon built it, since only a daemon holds engines.
        usable_engine: Option<bool>,
    },
    /// The refusal, already worded for a person.
    Refused(String),
}

/// Provisions one component — or, unnamed, the one this machine needs.
///
/// **Through the daemon when one is up.** A daemon holds the engines it found
/// at start and rediscovers them after a build *it* ran; a build the command
/// ran beside it was a prefix on disk and no engine on the socket until the
/// daemon restarted (F149). So where a daemon is listening and the build is
/// bound for its own root, the command asks the daemon to build, and streams
/// what it says. `--into` names a root the daemon does not look under, so
/// that build stays local and says so.
pub(crate) fn run(name: Option<&str>, into: Option<&str>) -> Response {
    let component = match chosen_component(name) {
        Ok(component) => component,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let root = match root_from(into) {
        Ok(root) => root,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    // Unnamed, the choice is stated: a build the operator did not name is a
    // choice MCF made, and §3.15 wants it visible.
    let chosen = if name.is_none() {
        format!(
            "{} is what a model on this machine would run on, so that is what is built\n",
            component.name
        )
    } else {
        String::new()
    };

    let daemon = crate::serve::socket_path().and_then(|socket| UnixStream::connect(socket).ok());
    let (came, where_built) = match daemon {
        Some(connection) if into.is_none() => (through_daemon(connection, name), "by the daemon"),
        Some(_) => (
            locally(component, &root),
            "here, outside the daemon's root — the daemon that is up does not look there",
        ),
        None => (
            locally(component, &root),
            "here; no daemon is up to hold it yet",
        ),
    };
    report(component, &chosen, where_built, came)
}

/// Builds in this process, printing each of the build's lines as it comes.
fn locally(component: &'static Component, root: &std::path::Path) -> Came {
    let prefix = prefix_for(component, root);
    // Each line the build prints, as it prints it, on the error stream —
    // which is where progress goes so that the outcome below stays the one
    // thing on standard output. The prefix's log keeps every line.
    let mut progress = |line: &str| eprintln!("  {line}");
    match provisioning::provision(component, &prefix, &mut progress) {
        Ok(Outcome::Already { prefix }) => Came::Already { prefix },
        Ok(Outcome::Built(built)) => Came::Built {
            prefix: built.prefix,
            log: built.log,
            toolchain: built.toolchain,
            recorded: built.recorded.map_err(|failure| failure.to_string()),
            usable_engine: None,
        },
        Err(failure) => Came::Refused(crate::say::refusal(
            &format!("{} was not provisioned", component.name),
            &failure,
        )),
    }
}

/// Asks the daemon that answered to build, and follows the build line by line.
///
/// No read deadline: a build is as long as the compiler makes it, and a
/// timeout here would report a working build as a dead daemon.
fn through_daemon(mut connection: UnixStream, name: Option<&str>) -> Came {
    let request = Request::Provision {
        component: name.map(str::to_owned),
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Came::Refused("mcf: the daemon is up but the request could not be sent".to_owned());
    }
    for read in BufReader::new(&connection).lines() {
        let Ok(read) = read else { break };
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served {
            return Came::Refused(format!(
                "mcf: not provisioned\n  {}",
                crate::say::refused_because(&answer.body)
            ));
        }
        if let Some(doing) = answer.body.get("doing").and_then(Value::as_text) {
            eprintln!("  {doing}");
        }
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            return came_from(&answer.body);
        }
    }
    Came::Refused(
        "mcf: the daemon stopped answering before the build ended\n  what it built, if anything, \
         is in its log; `mcf provision --list` shows whether the prefix is complete"
            .to_owned(),
    )
}

/// The daemon's final line, read back into what the library would have said.
fn came_from(body: &Value) -> Came {
    let path = |key: &str| body.get(key).and_then(Value::as_text).map(PathBuf::from);
    let Some(prefix) = path("prefix") else {
        return Came::Refused(format!(
            "mcf: the daemon ended the build without naming a prefix: {}",
            body.to_line()
        ));
    };
    if matches!(body.get("already"), Some(Value::Bool(true))) {
        return Came::Already { prefix };
    }
    Came::Built {
        prefix,
        log: path("log").unwrap_or_default(),
        toolchain: body
            .get("toolchain")
            .and_then(Value::as_text)
            .unwrap_or_default()
            .to_owned(),
        recorded: match body.get("recorded") {
            Some(Value::Text(at)) => Ok(PathBuf::from(at)),
            Some(other) => Err(crate::say::refused_because(other)),
            None => Err("the daemon did not say".to_owned()),
        },
        usable_engine: match body.get("usable_engine") {
            Some(Value::Bool(reached)) => Some(*reached),
            _ => None,
        },
    }
}

/// One wording for both builders.
fn report(component: &Component, chosen: &str, where_built: &str, came: Came) -> Response {
    match came {
        Came::Already { prefix } => Response {
            text: format!(
                "{chosen}{}@{} is already provisioned at {}\n  remove it first to provision it \
                 again (`mcf provision --remove {}`)",
                component.name,
                short(component),
                prefix.display(),
                component.name,
            ),
            served: true,
        },
        Came::Built {
            prefix,
            log,
            toolchain,
            recorded,
            usable_engine,
        } => {
            let mut lines = vec![
                format!("provisioned {}@{}", component.name, short(component)),
                format!("  into    {}", prefix.display()),
                format!("  image   {} ({})", component.image, component.image_digest),
                format!("  built   {}", component.targets.join(", ")),
                "  packages, exactly:".to_owned(),
            ];
            lines.extend(toolchain.lines().map(|line| format!("    {line}")));
            lines.push(format!(
                "  provenance beside it: {}",
                prefix.join("mcf-provenance.json").display()
            ));
            lines.push(format!("  log kept at {}", log.display()));
            lines.push(match recorded {
                Ok(path) => format!("  recorded in {}", path.display()),
                Err(why) => format!("  NOT RECORDED: {why}"),
            });
            lines.push(format!("  built {where_built}"));
            match usable_engine {
                Some(true) => lines.push("  the daemon now reaches it as an engine".to_owned()),
                Some(false) => lines.push(
                    "  the daemon does NOT reach it as an engine — a library builds and is never \
                     one; an engine here is a build that landed wrong"
                        .to_owned(),
                ),
                None => {}
            }
            Response {
                text: format!("{chosen}{}", lines.join("\n")),
                served: true,
            }
        }
        Came::Refused(text) => Response {
            text,
            served: false,
        },
    }
}

/// Removes a provisioned component, and says so in the record first.
pub(crate) fn remove(name: &str, because: Option<&str>, into: Option<&str>) -> Response {
    let Some(component) = COMPONENTS.iter().find(|component| component.name == name) else {
        return Response {
            text: format!("mcf: there is no provisionable component called {name}"),
            served: false,
        };
    };
    let Some(reason) = because else {
        return Response {
            text: "mcf: a removal carries its reason: --because <why>".to_owned(),
            served: false,
        };
    };
    let root = match root_from(into) {
        Ok(root) => root,
        Err(text) => {
            return Response {
                text,
                served: false,
            };
        }
    };
    let prefix = prefix_for(component, &root);
    if !prefix.exists() {
        return Response {
            text: format!("{} is not provisioned at {}", name, prefix.display()),
            served: true,
        };
    }

    // The record before the removal: if the removal half fails, a recorded
    // intention beside a still-present prefix beats a removed prefix nobody
    // wrote down (A1's ordering).
    let entry = Value::map([
        ("component", Value::text(component.name)),
        ("commit", Value::text(component.commit)),
        ("prefix", Value::text(prefix.display().to_string())),
        ("reason", Value::text(reason)),
    ]);
    let recorded = provisioning::record(EntryKind::ComponentRemoved, &entry);

    match std::fs::remove_dir_all(&prefix) {
        Ok(()) => Response {
            text: format!(
                "removed {}, because: {reason}\n  {} is gone; the base image is shared and \
                 stays\n  {}",
                name,
                prefix.display(),
                match recorded {
                    Ok(path) => format!("recorded in {}", path.display()),
                    Err(failure) => format!("NOT RECORDED: {failure}"),
                }
            ),
            served: true,
        },
        Err(error) => Response {
            text: format!(
                "mcf: {} could not be removed\n  {error}\n  the removal is recorded as asked \
                 for; the prefix is still there",
                prefix.display()
            ),
            served: false,
        },
    }
}

fn root_from(into: Option<&str>) -> Result<PathBuf, String> {
    match into {
        Some(named) => Ok(PathBuf::from(named)),
        None => default_root().ok_or_else(|| {
            "mcf: there is nowhere to provision into — neither XDG_DATA_HOME nor HOME is set, \
             and MCF does not invent a place (A7); --into names one"
                .to_owned()
        }),
    }
}

#[cfg(test)]
mod tests;
