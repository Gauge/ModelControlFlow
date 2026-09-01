//! `mcf provision`: a component MCF installs, builds and pins itself, in an
//! environment it controls (B-367, D39, DEC-052, F30).
//!
//! **What "controlled" means here was decided by measurement, not preference.**
//! F30 provisioned the same component two ways. The host-tool route failed
//! D39's first condition on its very first use — the build silently picked up
//! a cmake from a pyenv shim, recorded nowhere, different from what the system
//! says — and the container route left container storage byte-identical while
//! producing a binary that runs on the host and agrees exactly. So: a rootless
//! container, base image pinned by digest, source cloned at a pinned commit,
//! everything landing in one prefix the operator can point anywhere, and the
//! exact package set written into the prefix beside what it built.
//!
//! **What may be provisioned is a table, not a language.** §5 forbids a
//! configuration language and §3.13 forbids generality nobody asked for, so
//! the components are declared here in code, the way the helper's operations
//! and the engine's architectures are: adding one is editing this file, and
//! each entry answers *what claim can MCF make once this exists* — the first
//! entry's answer is B-368's, an oracle anyone can reproduce.
//!
//! **Two failure shapes, one honest category.** A machine without `podman` and
//! a build that did not complete are both the same statement — the mechanism
//! this decision rests on did not deliver — and the `detail` says which way.
//! F30 stated the first as a condition a machine can lack; this is where that
//! statement becomes a classified refusal rather than a stack trace.

use std::path::{Path, PathBuf};

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use mcf_core::component::{COMPONENTS, Component, Packaging};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

const WHERE: Subsystem = Subsystem::new("mcf-cli::provision");
/// Install the named packages, quietly, without prompting.
///
/// A free function rather than a method: [`Packaging`] is the catalogue's type
/// now, and writing shell is this crate's business rather than the
/// catalogue's.
fn install_packages(packaging: Packaging, packages: &str) -> String {
    {
        match packaging {
            Packaging::Dnf => format!("dnf -q install -y {packages}"),
            // `update` first, because a Debian image ships no package lists and
            // an install without one fails on every name.
            Packaging::Apt => format!(
                "export DEBIAN_FRONTEND=noninteractive\n\
                 apt-get -qq update > /dev/null\n\
                 apt-get -qq install -y --no-install-recommends {packages} > /dev/null"
            ),
        }
    }
}

/// Write down exactly what was installed. A version that is not recorded is
/// a condition of the artifact nobody can restate (§3.4).
fn record_packages(packaging: Packaging, packages: &str) -> String {
    {
        match packaging {
            Packaging::Dnf => format!("rpm -q {packages} glibc > /work/toolchain.txt"),
            Packaging::Apt => format!(
                "dpkg-query -W -f='${{Package}} ${{Version}}\\n' {packages} libc6 \
                 > /work/toolchain.txt"
            ),
        }
    }
}

/// Where a component lands when the operator does not say.
///
/// Under the data home, beside the models and the record — which on this
/// machine the operator has already pointed at the large drive. `--into` names
/// anywhere else.
pub(crate) fn default_root() -> Option<PathBuf> {
    crate::models::default_root().map(|models| {
        models
            .parent()
            .map_or_else(|| models.join("provisioned"), |mcf| mcf.join("provisioned"))
    })
}

/// The prefix one component builds into: name and short commit, so that two
/// pins of the same component are two directories and neither overwrites the
/// other (A1).
fn prefix_for(component: &Component, root: &Path) -> PathBuf {
    let short: String = component.commit.chars().take(12).collect();
    root.join(format!("{}@{short}", component.name))
}

/// Lists what can be provisioned and what is.
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
        let state = if prefix.join("mcf-provenance.json").is_file() {
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

/// Provisions one component.
pub(crate) fn run(name: &str, into: Option<&str>) -> Response {
    let Some(component) = COMPONENTS.iter().find(|component| component.name == name) else {
        return Response {
            text: format!(
                "mcf: MCF does not know how to provision {name}\n  it knows: {}\n  adding one \
                 is a change to MCF, not a configuration (§5)",
                COMPONENTS
                    .iter()
                    .map(|component| component.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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

    match provision(component, &prefix) {
        Ok(provisioned) => Response {
            text: provisioned,
            served: true,
        },
        Err(failure) => Response {
            text: crate::say::refusal(&format!("{} was not provisioned", component.name), &failure),
            served: false,
        },
    }
}

/// The provisioning itself: one container run, then the records.
fn provision(component: &Component, prefix: &Path) -> Result<String, Failure> {
    let podman = which_podman()?;

    if prefix.join("mcf-provenance.json").is_file() {
        return Ok(format!(
            "{}@{} is already provisioned at {}\n  remove it first to provision it again \
             (`mcf provision --remove {}`)",
            component.name,
            short(component),
            prefix.display(),
            component.name,
        ));
    }
    std::fs::create_dir_all(prefix).map_err(|error| {
        Failure::new(
            Category::ResourceDiskReadonly,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the prefix could not be created",
        )
        .with_context("prefix", prefix.display().to_string())
        .with_context("os_error", error.to_string())
    })?;

    // The whole recipe as one script, written into the prefix so that what ran
    // is part of what is recorded — a build whose steps are only in MCF's
    // source is a build somebody has to read MCF to restate (§3.12).
    std::fs::write(prefix.join("provision.sh"), script_for(component)).map_err(|error| {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the provisioning script could not be written into the prefix",
        )
        .with_context("os_error", error.to_string())
    })?;

    let log_path = build_in_container(component, prefix, &podman)?;
    let toolchain = verified(component, prefix)?;
    let provenance = provenance_of(component, prefix, &toolchain);
    std::fs::write(
        prefix.join("mcf-provenance.json"),
        format!("{}\n", provenance.to_line()),
    )
    .map_err(|error| broke(&error))?;
    let recorded = record(EntryKind::ComponentProvisioned, &provenance);

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
    lines.push(format!("  log kept at {}", log_path.display()));
    lines.push(match recorded {
        Ok(path) => format!("  recorded in {}", path.display()),
        Err(failure) => format!("  NOT RECORDED: {failure}"),
    });
    Ok(lines.join("\n"))
}

/// The first twelve characters of the pin, for a directory name and a line.
fn short(component: &Component) -> &str {
    component.commit.get(..12).unwrap_or(component.commit)
}

/// One `podman run --rm` over the pinned image, its output kept in the prefix.
fn build_in_container(
    component: &Component,
    prefix: &Path,
    podman: &Path,
) -> Result<PathBuf, Failure> {
    // By digest: the tag is what a person reads, the digest is what runs.
    let pinned = format!(
        "{}@{}",
        component.image.split(':').next().unwrap_or(component.image),
        component.image_digest
    );
    let log_path = prefix.join("provision.log");
    let log = std::fs::File::create(&log_path).map_err(|error| {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the provisioning log could not be created",
        )
        .with_context("os_error", error.to_string())
    })?;

    // Podman's *image store* is not the prefix, and must not follow MCF's data
    // home. Rootless podman keeps its store under `$XDG_DATA_HOME`, and on this
    // machine that is the large content drive — a filesystem that will not do
    // the ownership changes an overlay store needs, so the very first
    // provisioning failed pulling the image (F31). The prefix, bind-mounted,
    // lives there without trouble. So the child sees the platform default for
    // its store and the operator's choice for the output, which is the division
    // that holds: the store is podman's and shared; the prefix is MCF's and
    // removable.
    let status = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{}:/work:z", prefix.display()))
        .arg(pinned)
        .arg("bash")
        .arg("/work/provision.sh")
        .stdout(log.try_clone().map_err(|error| broke(&error))?)
        .stderr(log)
        .status()
        .map_err(|error| {
            Failure::new(
                Category::PlatformMechanismUnavailable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "podman could not be started",
            )
            .with_context("os_error", error.to_string())
        })?;

    if status.success() {
        return Ok(log_path);
    }
    Err(Failure::new(
        Category::PlatformMechanismUnavailable,
        Attribution::Machine,
        Disposition::Aborted,
        WHERE,
        "the controlled environment did not produce the component",
    )
    .with_context("exit", status.to_string())
    .with_context("log", log_path.display().to_string())
    .with_context(
        "what_to_do",
        "the log holds the build's own words; the prefix is safe to remove and the run safe \
         to repeat",
    ))
}

/// What the run pinned, read back out of the prefix rather than assumed: the
/// commit the checkout landed on and the packages dnf resolved are what was
/// *got*, and the recipe is only what was asked for (A21).
fn verified(component: &Component, prefix: &Path) -> Result<String, Failure> {
    let commit = std::fs::read_to_string(prefix.join("commit.txt")).unwrap_or_default();
    mcf_core::provenance::checked_out(component.commit, &commit)?;
    Ok(std::fs::read_to_string(prefix.join("toolchain.txt")).unwrap_or_default())
}

/// Everything a rerun needs, as one record.
fn provenance_of(component: &Component, prefix: &Path, toolchain: &str) -> Value {
    Value::map([
        ("component", Value::text(component.name)),
        ("role", Value::text(component.role)),
        ("image", Value::text(component.image)),
        ("image_digest", Value::text(component.image_digest)),
        ("source", Value::text(component.source)),
        ("commit", Value::text(component.commit)),
        (
            "packages",
            Value::List(
                toolchain
                    .lines()
                    .map(|line| Value::text(line.trim()))
                    .collect(),
            ),
        ),
        (
            "targets",
            Value::List(
                component
                    .targets
                    .iter()
                    .map(|target| Value::text(*target))
                    .collect(),
            ),
        ),
        ("prefix", Value::text(prefix.display().to_string())),
    ])
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
            text: "mcf: a removal carries its reason: --because <why> (§3.11, A27)".to_owned(),
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
    let recorded = record(EntryKind::ComponentRemoved, &entry);

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

/// The recipe as a script: install, record, clone, pin, configure, build.
///
/// `safe.directory` is passed per invocation rather than configured: the
/// prefix is a bind mount whose files present as another owner inside the
/// container, and git refuses a repository it thinks somebody else owns. The
/// exception is scoped to the one directory and lives only as long as the
/// command, which is exactly as far as it should reach.
fn script_for(component: &Component) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         # Written by `mcf provision {name}`; what ran is part of what is recorded.\n\
         set -o errexit -o nounset -o pipefail\n\
         {install}\n\
         {record}\n\
         rm -rf /work/source /work/build\n\
         git clone -q {source} /work/source\n\
         git -c safe.directory=/work/source -C /work/source checkout -q {commit}\n\
         git -c safe.directory=/work/source -C /work/source rev-parse HEAD > /work/commit.txt\n\
         cmake -S /work/source -B /work/build {configure} > /work/configure.log 2>&1\n\
         cmake --build /work/build -j --target {targets} > /work/build.log 2>&1\n",
        name = component.name,
        install = install_packages(component.packaging, &component.packages.join(" ")),
        record = record_packages(component.packaging, &component.packages.join(" ")),
        source = component.source,
        commit = component.commit,
        configure = component
            .configure
            .iter()
            .map(|flag| shell_quoted(flag))
            .collect::<Vec<_>>()
            .join(" "),
        targets = component.targets.join(" "),
    )
}

/// One argument, safe to paste into a shell.
///
/// The configure flags are written into a script and run by bash, so an
/// argument holding a shell metacharacter is a command. `CMAKE_CUDA_ARCHITECTURES`
/// takes a semicolon-separated list, and unquoted it ended the cmake command
/// and made `120` the next one — exit 127, after a configure that reported
/// success while silently dropping the flag (F128). Quoting every argument
/// rather than that one keeps the next flag from finding the same hole.
fn shell_quoted(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\\''"))
}

/// Where `podman` is, or the refusal F30 promised.
fn which_podman() -> Result<PathBuf, Failure> {
    let candidates = ["/usr/bin/podman", "/usr/local/bin/podman"];
    for candidate in candidates {
        let path = Path::new(candidate);
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
    }
    Err(Failure::new(
        Category::PlatformMechanismUnavailable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "this machine has no podman, and a controlled environment is a container (DEC-052)",
    )
    .with_context("looked_at", candidates.join(", "))
    .with_context(
        "what_to_do",
        "install podman from the platform's own repository; MCF will not build with the \
         host's ambient tools — F30 measured what that route silently does",
    ))
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

fn record(kind: EntryKind, body: &Value) -> Result<PathBuf, Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "there is nowhere to record the provisioning",
        ));
    };
    let mut journal = Journal::open(&path)?;
    journal.append(&Record::new(kind, Timestamp::now(), body.clone()))?;
    Ok(path)
}

fn broke(error: &std::io::Error) -> Failure {
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Aborted,
        WHERE,
        "what was provisioned could not be written down beside it",
    )
    .with_context("os_error", error.to_string())
}

#[cfg(test)]
mod tests;
