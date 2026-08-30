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

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;

use crate::Response;

const WHERE: Subsystem = Subsystem::new("mcf-cli::provision");

/// How a base image installs and reports its packages.
///
/// The recipe used to say `dnf` and `rpm` outright, which was true of the one
/// image there was. The CUDA toolkit ships on Ubuntu, and a second component
/// made the assumption visible by failing on it — `dnf: command not found`,
/// exit 127, before a single file was compiled (F128).
#[derive(Clone, Copy)]
pub(crate) enum Packaging {
    /// Fedora and its relatives.
    Dnf,
    /// Debian and its relatives, which is what the CUDA images are built on.
    Apt,
}

impl Packaging {
    /// Install the named packages, quietly, without prompting.
    fn install(self, packages: &str) -> String {
        match self {
            Self::Dnf => format!("dnf -q install -y {packages}"),
            // `update` first, because a Debian image ships no package lists and
            // an install without one fails on every name.
            Self::Apt => format!(
                "export DEBIAN_FRONTEND=noninteractive\n\
                 apt-get -qq update > /dev/null\n\
                 apt-get -qq install -y --no-install-recommends {packages} > /dev/null"
            ),
        }
    }

    /// Write down exactly what was installed. A version that is not recorded is
    /// a condition of the artifact nobody can restate (§3.4).
    fn record(self, packages: &str) -> String {
        match self {
            Self::Dnf => format!("rpm -q {packages} glibc > /work/toolchain.txt"),
            Self::Apt => format!(
                "dpkg-query -W -f='${{Package}} ${{Version}}\\n' {packages} libc6 \
                 > /work/toolchain.txt"
            ),
        }
    }
}

/// One component MCF knows how to provision.
pub(crate) struct Component {
    /// The name the operator types.
    pub name: &'static str,
    /// What having it lets MCF claim.
    pub role: &'static str,
    /// The base image, pinned by digest — the tag beside it is for a reader.
    pub image: &'static str,
    pub image_digest: &'static str,
    /// Where the source comes from, and exactly which of it.
    pub source: &'static str,
    pub commit: &'static str,
    /// The packages the build needs, installed inside the container and
    /// recorded with their exact versions.
    pub packages: &'static [&'static str],
    /// How this image installs them.
    pub packaging: Packaging,
    /// How it is configured and what is built.
    pub configure: &'static [&'static str],
    pub targets: &'static [&'static str],
}

/// Everything MCF can provision.
pub(crate) const COMPONENTS: &[Component] = &[
    Component {
        name: "llama.cpp",
        role: "the reference implementation MCF's own engine is checked against (B-368): \
           tokenizers compared exactly, generations at a measured margin, embeddings \
           at a measured floor",
        image: "registry.fedoraproject.org/fedora:44",
        image_digest: "sha256:5a4a491c33973b8173e6134d6f00e77f27cebef581c9b34420b2b6183a6398df",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "925e1179947ea0c0ebfb0032df18af3a729822be",
        packages: &["gcc-c++", "cmake", "git", "make"],
        packaging: Packaging::Dnf,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            // Portable rather than tuned: a provisioned binary is a condition of
            // measurements, and `-march=native` would make it a condition nobody
            // can restate on another machine (§3.4).
            "-DGGML_NATIVE=OFF",
            // Self-contained, because the artifact outlives the container that
            // built it. A shared build bakes the *container's* library path into
            // every binary — `/work/build/bin`, a directory that exists nowhere on
            // the host — so the first provisioned oracle loaded nothing without an
            // incantation (F31). What is provisioned must run where it lands.
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
        ],
        // The server is the one reference tool that exposes the model's
        // distribution — `n_probs` on its completion endpoint — which is what a
        // comparison of logits rather than texts needs (B-373). Nothing else in
        // the reference prints a logit.
        targets: &[
            "llama-tokenize",
            "llama-completion",
            "llama-embedding",
            "llama-server",
        ],
    },
    Component {
        name: "llama.cpp-cuda",
        role: "the same reference, built with a CUDA backend, so a measurement can \
           be taken on the GPU as well as the CPU. Without it MCF's engine \
           reports no devices and every timing on this machine is a CPU timing \
           whether or not a card is installed (F127, F128)",
        // A CUDA toolkit image, because nvcc is what the backend needs and the
        // Fedora image beside this one carries none. Compiling needs the toolkit;
        // it does not need a GPU, so this build is as reproducible as the other.
        image: "docker.io/nvidia/cuda:12.9.1-devel-ubuntu24.04",
        image_digest: "sha256:020bc241a628776338f4d4053fed4c38f6f7f3d7eb5919fecb8de313bb8ba47c",
        source: "https://github.com/ggml-org/llama.cpp.git",
        // The SAME commit as the CPU build. Two backends of one source are
        // comparable; two backends of two sources are not, and putting one against
        // the other is the whole point of having both.
        commit: "925e1179947ea0c0ebfb0032df18af3a729822be",
        packages: &["build-essential", "cmake", "git"],
        packaging: Packaging::Apt,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DGGML_NATIVE=OFF",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
            "-DGGML_CUDA=ON",
            // The architectures compiled for are a condition of the artifact, the
            // way `-march` would be, so they are stated rather than left to the
            // toolkit's default. 89 is Ada, 120 is Blackwell — the card here is the
            // latter, and a binary that ran only here would be one nobody could
            // restate a measurement with (§3.4).
            "-DCMAKE_CUDA_ARCHITECTURES=89;120",
            // Static, for the same reason `BUILD_SHARED_LIBS=OFF` is: what is
            // provisioned must run where it lands (F31). The first CUDA build
            // linked the container's libcudart.so.12 and would not start on this
            // host, which carries CUDA 13. cudart alone was not enough — cuBLAS and
            // NCCL were still dynamic, and NCCL is for spreading one model across
            // several cards, which this is not doing. The driver library is the one
            // thing that must come from the machine, and it does.
            "-DCMAKE_CUDA_RUNTIME_LIBRARY=Static",
            "-DGGML_STATIC=ON",
            "-DGGML_CUDA_NCCL=OFF",
        ],
        targets: &[
            "llama-tokenize",
            "llama-completion",
            "llama-embedding",
            "llama-server",
        ],
    },
    Component {
        name: "SDL3",
        role: "a window, keyboard and mouse events, and a 2D renderer for the desktop \
               application (B-405). It is the ONLY thing vendored for it: MCF draws every \
               panel, table and button itself, with the layout the terminal console already \
               uses, so no widget toolkit is admitted and no font library is needed — SDL \
               carries an 8x8 font of its own",
        image: "registry.fedoraproject.org/fedora:44",
        image_digest: "sha256:5a4a491c33973b8173e6134d6f00e77f27cebef581c9b34420b2b6183a6398df",
        source: "https://github.com/libsdl-org/SDL.git",
        // release-3.4.14.
        commit: "147a8ee32dbf9ac02f3794964490687b6bbda1bc",
        packages: &[
            "gcc",
            "cmake",
            "git",
            "make",
            // The windowing systems SDL talks to. Loaded at runtime rather than
            // linked, so the built library runs on a machine with either.
            "libX11-devel",
            "libXext-devel",
            "libXrandr-devel",
            "libXcursor-devel",
            "libXfixes-devel",
            "libXi-devel",
            "libXScrnSaver-devel",
            "libXtst-devel",
            "libxkbcommon-devel",
            "wayland-devel",
            "wayland-protocols-devel",
            "mesa-libGL-devel",
            "mesa-libEGL-devel",
        ],
        packaging: Packaging::Dnf,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DSDL_SHARED=OFF",
            // Static, for the same reason every other provisioned artifact is:
            // what is provisioned must run where it lands (F31).
            "-DSDL_STATIC=ON",
            // Position-independent, because what links it is a Rust binary and
            // Rust links a position-independent executable. Without this the
            // archive builds, and then the link fails on a relocation nobody
            // reading the recipe would have predicted.
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            // Everything below is off because MCF does not use it — and because
            // each one is a part of the tree that is NOT zlib. Switching them
            // off is not tidiness: it is what makes the shipped tree almost
            // entirely one licence, and the finding in vendored.md rests on
            // this exact list.
            //
            //   HIDAPI   tri-licensed, one option being GPL-3.0
            //   VULKAN   pulls a Khronos header under Apache-2.0
            //   OPENVR   Valve's, BSD-3-Clause
            //   TESTS    public domain, and not shipped anyway
            "-DSDL_HIDAPI=OFF",
            "-DSDL_HIDAPI_JOYSTICK=OFF",
            "-DSDL_VULKAN=OFF",
            "-DSDL_RENDER_VULKAN=OFF",
            "-DSDL_OPENVR=OFF",
            "-DSDL_TESTS=OFF",
            "-DSDL_EXAMPLES=OFF",
            // Subsystems a measuring instrument has no use for. Less code is
            // less to verify and less to go wrong.
            "-DSDL_AUDIO=OFF",
            "-DSDL_CAMERA=OFF",
            "-DSDL_HAPTIC=OFF",
            "-DSDL_JOYSTICK=OFF",
            "-DSDL_SENSOR=OFF",
            "-DSDL_POWER=OFF",
        ],
        targets: &["SDL3-static"],
    },
];

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
        install = component.packaging.install(&component.packages.join(" ")),
        record = component.packaging.record(&component.packages.join(" ")),
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
