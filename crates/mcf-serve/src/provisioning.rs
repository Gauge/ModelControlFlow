//! Building a component MCF installs, builds and pins itself, in an
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
//! **One builder, two callers.** `mcf provision` builds from the command
//! line, and the daemon builds when a window holds a model that has no engine
//! to run it. Both come here, so what a build does — the image it pulls, the
//! script it writes, the record it leaves — is one thing whichever surface
//! asked (A22, B-072). The command-line half that is a *command* — naming a
//! component, listing, removing with a reason — stays in `mcf-cli`.
//!
//! **What may be provisioned is a table, not a language.** §5 forbids a
//! configuration language and §3.13 forbids generality nobody asked for, so
//! the components are declared in [`mcf_core::component`], the way the
//! helper's operations and the engine's architectures are: adding one is
//! editing that file, and each entry answers *what claim can MCF make once
//! this exists*.
//!
//! **Two failure shapes, one honest category.** A machine without `podman` and
//! a build that did not complete are both the same statement — the mechanism
//! this decision rests on did not deliver — and the `detail` says which way.
//! F30 stated the first as a condition a machine can lack; this is where that
//! statement becomes a classified refusal rather than a stack trace.

use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};

use mcf_core::component::{Component, Packaging};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-serve::provisioning");

/// What a build left behind.
#[derive(Debug)]
pub struct Provisioned {
    /// Where it landed.
    pub prefix: PathBuf,
    /// The build's own words, kept.
    pub log: PathBuf,
    /// The packages the container resolved, one a line, exactly.
    pub toolchain: String,
    /// The record it was written into — or why it was not, which is reported
    /// rather than swallowed: a build nobody can find in the record is a
    /// build that will be repeated (A1).
    pub recorded: core::result::Result<PathBuf, Failure>,
}

/// What asking for a build came to.
#[derive(Debug)]
pub enum Outcome {
    /// Built now.
    Built(Provisioned),
    /// It was already there, complete, and nothing was done.
    Already {
        /// Where.
        prefix: PathBuf,
    },
}

/// Install the named packages, quietly, without prompting.
fn install_packages(packaging: Packaging, packages: &str) -> String {
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

/// Write down exactly what was installed. A version that is not recorded is
/// a condition of the artifact nobody can restate (§3.4).
fn record_packages(packaging: Packaging, packages: &str) -> String {
    match packaging {
        Packaging::Dnf => format!("rpm -q {packages} glibc > /work/toolchain.txt"),
        Packaging::Apt => format!(
            "dpkg-query -W -f='${{Package}} ${{Version}}\\n' {packages} libc6 \
             > /work/toolchain.txt"
        ),
    }
}

/// Where components land under one MCF home: beside the models and the
/// record.
#[must_use]
pub fn root_under(mcf_home: &Path) -> PathBuf {
    mcf_home.join("provisioned")
}

/// The prefix one component builds into: name and short commit, so that two
/// pins of the same component are two directories and neither overwrites the
/// other (A1).
#[must_use]
pub fn prefix_for(component: &Component, root: &Path) -> PathBuf {
    root.join(format!("{}@{}", component.name, short(component)))
}

/// Whether a prefix holds a finished build.
///
/// Complete means the provenance beside it, which is what the builder writes
/// last — not merely a directory, which is what a run that stopped partway
/// also leaves.
#[must_use]
pub fn is_complete(prefix: &Path) -> bool {
    prefix.join("mcf-provenance.json").is_file()
}

/// The first twelve characters of the pin, for a directory name and a line.
#[must_use]
pub fn short(component: &Component) -> &str {
    component.commit.get(..12).unwrap_or(component.commit)
}

/// Provisions one component into a prefix: one container run, then the
/// records.
///
/// `progress` hears the build's own output a line at a time, so that a
/// surface can show a build going rather than a build that may have hung; the
/// same lines go into the log beside the prefix.
///
/// # Errors
///
/// A classified refusal: no `podman`, a prefix that could not be written, a
/// container that did not produce the component, or a checkout that landed
/// somewhere other than the pin.
pub fn provision(
    component: &Component,
    prefix: &Path,
    progress: &mut dyn FnMut(&str),
) -> core::result::Result<Outcome, Failure> {
    let podman = which_podman()?;

    if is_complete(prefix) {
        return Ok(Outcome::Already {
            prefix: prefix.to_path_buf(),
        });
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

    let log = build_in_container(component, prefix, &podman, progress)?;
    let toolchain = verified(component, prefix)?;
    let provenance = provenance_of(component, prefix, &toolchain);
    std::fs::write(
        prefix.join("mcf-provenance.json"),
        format!("{}\n", provenance.to_line()),
    )
    .map_err(|error| broke(&error))?;
    let recorded = record(EntryKind::ComponentProvisioned, &provenance);

    Ok(Outcome::Built(Provisioned {
        prefix: prefix.to_path_buf(),
        log,
        toolchain,
        recorded,
    }))
}

/// One `podman run --rm` over the pinned image, its output kept in the prefix
/// and heard as it comes.
fn build_in_container(
    component: &Component,
    prefix: &Path,
    podman: &Path,
    progress: &mut dyn FnMut(&str),
) -> core::result::Result<PathBuf, Failure> {
    // By digest: the tag is what a person reads, the digest is what runs.
    let pinned = format!(
        "{}@{}",
        component.image.split(':').next().unwrap_or(component.image),
        component.image_digest
    );
    let log_path = prefix.join("provision.log");
    // One open file, two handles: the build's error stream writes it
    // directly and its output comes through this process, and a clone of a
    // handle shares the offset, so the two interleave in order rather than
    // overwriting each other (A1). Truncated, not appended, so that a run
    // stopped partway leaves nothing of itself in this run's log.
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| {
            Failure::new(
                Category::RecordUnwritable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the provisioning log could not be created",
            )
            .with_context("os_error", error.to_string())
        })?;
    let errors = log.try_clone().map_err(|error| broke(&error))?;

    // Podman's *image store* is not the prefix, and must not follow MCF's data
    // home. Rootless podman keeps its store under `$XDG_DATA_HOME`, and on this
    // machine that is the large content drive — a filesystem that will not do
    // the ownership changes an overlay store needs, so the very first
    // provisioning failed pulling the image (F31). The prefix, bind-mounted,
    // lives there without trouble. So the child sees the platform default for
    // its store and the operator's choice for the output, which is the division
    // that holds: the store is podman's and shared; the prefix is MCF's and
    // removable.
    let mut child = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
        .arg("-v")
        .arg(format!("{}:/work:z", prefix.display()))
        .arg(pinned)
        .arg("bash")
        .arg("/work/provision.sh")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(errors)
        .spawn()
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

    if let Some(output) = child.stdout.take() {
        for line in BufReader::new(output).lines().map_while(Result::ok) {
            let _kept = writeln!(log, "{line}");
            progress(&line);
        }
    }
    let status = child.wait().map_err(|error| {
        Failure::new(
            Category::PlatformMechanismUnavailable,
            Attribution::Machine,
            Disposition::Aborted,
            WHERE,
            "podman could not be waited for",
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
fn verified(component: &Component, prefix: &Path) -> core::result::Result<String, Failure> {
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

/// The recipe as a script: install, record, clone, pin, configure, build.
///
/// `safe.directory` is passed per invocation rather than configured: the
/// prefix is a bind mount whose files present as another owner inside the
/// container, and git refuses a repository it thinks somebody else owns. The
/// exception is scoped to the one directory and lives only as long as the
/// command, which is exactly as far as it should reach.
///
/// **Each stage announces itself, and the build's own lines pass through.**
/// The script's output is what a surface shows while it waits, and a window
/// that heard nothing for the minutes a build takes could not tell it from a
/// hang (A2). The configure and build logs are still written whole into the
/// prefix — `tee` copies rather than diverts — and `pipefail` keeps a failed
/// cmake a failed script.
#[must_use]
pub fn script_for(component: &Component) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         # Written by `mcf provision {name}`; what ran is part of what is recorded.\n\
         set -o errexit -o nounset -o pipefail\n\
         echo 'installing the toolchain'\n\
         {install}\n\
         {record}\n\
         rm -rf /work/source /work/build\n\
         echo 'fetching the source at {commit}'\n\
         git clone -q {source} /work/source\n\
         git -c safe.directory=/work/source -C /work/source checkout -q {commit}\n\
         git -c safe.directory=/work/source -C /work/source rev-parse HEAD > /work/commit.txt\n\
         echo 'configuring'\n\
         cmake -S /work/source -B /work/build {configure} 2>&1 | tee /work/configure.log\n\
         echo 'building'\n\
         cmake --build /work/build -j --target {targets} 2>&1 | tee /work/build.log\n",
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
fn which_podman() -> core::result::Result<PathBuf, Failure> {
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

/// Writes one entry about a component into the record.
///
/// # Errors
///
/// `record.unwritable` where there is no record, or it will not take the
/// entry.
pub fn record(kind: EntryKind, body: &Value) -> core::result::Result<PathBuf, Failure> {
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
