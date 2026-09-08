use std::path::PathBuf;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use mcf_core::component::{COMPONENTS, Component};
use mcf_record::journal::EntryKind;
use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};
use mcf_serve::provisioning::{self, Outcome, is_complete, prefix_for, short};

use crate::Response;

pub(crate) fn default_root() -> Option<PathBuf> {
    crate::models::default_root().map(|models| {
        models.parent().map_or_else(
            || provisioning::root_under(&models),
            provisioning::root_under,
        )
    })
}

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

enum Came {
    Already {
        prefix: PathBuf,
    },
    Built {
        prefix: PathBuf,
        log: PathBuf,
        toolchain: String,
        recorded: Result<PathBuf, String>,
        usable_engine: Option<bool>,
    },
    Refused(String),
}

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

fn locally(component: &'static Component, root: &std::path::Path) -> Came {
    let prefix = prefix_for(component, root);
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
