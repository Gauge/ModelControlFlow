use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};

use mcf_core::component::{Component, Packaging};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

const WHERE: Subsystem = Subsystem::new("mcf-serve::provisioning");

#[derive(Debug)]
pub struct Provisioned {
    pub prefix: PathBuf,
    pub log: PathBuf,
    pub toolchain: String,
    pub recorded: core::result::Result<PathBuf, Failure>,
}

#[derive(Debug)]
pub enum Outcome {
    Built(Provisioned),
    Already { prefix: PathBuf },
}

fn install_packages(packaging: Packaging, packages: &str) -> String {
    match packaging {
        Packaging::Dnf => format!("dnf -q install -y {packages}"),
        Packaging::Apt => format!(
            "export DEBIAN_FRONTEND=noninteractive\n\
             apt-get -qq update > /dev/null\n\
             apt-get -qq install -y --no-install-recommends {packages} > /dev/null"
        ),
    }
}

fn record_packages(packaging: Packaging, packages: &str) -> String {
    match packaging {
        Packaging::Dnf => format!("rpm -q {packages} glibc > /work/toolchain.txt"),
        Packaging::Apt => format!(
            "dpkg-query -W -f='${{Package}} ${{Version}}\\n' {packages} libc6 \
             > /work/toolchain.txt"
        ),
    }
}

#[must_use]
pub fn root_under(mcf_home: &Path) -> PathBuf {
    mcf_home.join("provisioned")
}

#[must_use]
pub fn prefix_for(component: &Component, root: &Path) -> PathBuf {
    root.join(format!("{}@{}", component.name, short(component)))
}

#[must_use]
pub fn is_complete(prefix: &Path) -> bool {
    prefix.join("mcf-provenance.json").is_file()
}

#[must_use]
pub fn short(component: &Component) -> &str {
    component.commit.get(..12).unwrap_or(component.commit)
}

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

fn build_in_container(
    component: &Component,
    prefix: &Path,
    podman: &Path,
    progress: &mut dyn FnMut(&str),
) -> core::result::Result<PathBuf, Failure> {
    let pinned = format!(
        "{}@{}",
        component.image.split(':').next().unwrap_or(component.image),
        component.image_digest
    );
    let log_path = prefix.join("provision.log");
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

fn verified(component: &Component, prefix: &Path) -> core::result::Result<String, Failure> {
    let commit = std::fs::read_to_string(prefix.join("commit.txt")).unwrap_or_default();
    mcf_core::provenance::checked_out(component.commit, &commit)?;
    Ok(std::fs::read_to_string(prefix.join("toolchain.txt")).unwrap_or_default())
}

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

fn shell_quoted(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\\''"))
}

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
