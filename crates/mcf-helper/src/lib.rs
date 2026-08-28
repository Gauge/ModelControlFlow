//! The privileged helper: one named operation, performed, and then gone
//! (B-190, D35, §6.32, §XVII, A26).
//!
//! **What this program is.** A separate executable that does exactly one of
//! three things and exits. §6.32 requires MCF's privileged surface be
//! enumerable, auditable and short; D35 enumerated it from a machine rather
//! than from documentation ([findings.md] F15), and this is that list as a
//! program. There is no daemon here, no configuration, no plugin, no shell and
//! nothing read from the environment: the arguments are the whole input.
//!
//! **Why it is a separate crate.** It links `mcf-core` and nothing else. The
//! daemon never holds elevated rights; it starts this, which does one thing and
//! dies. A helper that shared the daemon's dependency tree would be a helper
//! nobody can audit at a glance, and *auditable* is half of what §6.32 asks
//! for.
//!
//! **The three operations, and why each is here.**
//!
//! 1. `governor <name>` — set every processor's frequency governor and print
//!    what each one was, so the caller can put it back. §6.39's ladder and
//!    §3.25's environment control; A27 makes the restoration part of the
//!    operation, which is why the previous value is an *output* rather than
//!    something the caller is expected to have read first.
//! 2. `accelerator exclusive|shared <index>` — take or release a device's
//!    exclusive compute mode, for D8's exclusive laboratory.
//! 3. `energy` — read the processor's energy counter. The awkward one §7.39
//!    anticipated: a *read* that needs elevation, because this counter was made
//!    root-only after it was shown to leak what a machine is doing. D11 makes
//!    energy first-class, and without this there is no energy figure for a
//!    processor at all.
//!
//! **What it refuses.** Everything else, including a governor name the machine
//! does not offer: the value written is chosen from what
//! `scaling_available_governors` says, never passed through from an argument.
//! A helper that writes what it is told is a helper that writes anything.
//!
//! **`--under <path>` names a fixture, not a file.** Every path is built from
//! fixed components under it, so a laboratory can point the helper at a
//! scratch directory and watch what it does (D26 — build the observable). It
//! cannot be used to write somewhere of the caller's choosing, because the
//! caller never names a file.
//!
//! **It says what it did on stdout, one `key: value` a line**, and what it
//! refused on stderr with the classified category. The caller records it: this
//! program writes to no record, because a privileged program that writes to the
//! operator's record is a second thing to audit.
//!
//! [findings.md]: ../../../doc/findings.md

use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-helper");

/// What the helper will do, and nothing else (D35, §6.32).
///
/// A constant rather than a match arm somewhere, so that
/// `checks/tests/the_privileged_surface_is_short.rs` can compare it against
/// D35's list and fail when the two drift.
pub const OPERATIONS: &[(&str, &str)] = &[
    (
        "governor",
        "set every processor's frequency governor, and say what each one was",
    ),
    (
        "accelerator",
        "take or release a device's exclusive compute mode",
    ),
    (
        "energy",
        "read the processor's energy counter, which is root-only on some systems",
    ),
];

/// Reads the arguments and does the one thing they name.
///
/// Public because the laboratory drives it: A13 asks for a scenario per
/// category MCF's code constructs, and a scenario that produced one by hand
/// would be a scenario about a mock (D26). The binary is a `main` that calls
/// this and prints; nothing else links it.
///
/// # Errors
///
/// `config.invalid` for anything that is not one of [`OPERATIONS`] with the
/// arguments it takes, and whatever the operation itself produces.
pub fn run(arguments: &[&str]) -> Result<Vec<String>> {
    // The machine, and only the machine. A caller cannot tell this program
    // where the machine is.
    //
    // It could once: `--under` rebased every fixed path, and the *shipped*
    // binary honoured it. With no privilege that is harmless, which is why it
    // survived. With any privilege it is a hole exactly the size of the
    // privilege — `energy --under <a tree you control>` reads any file as
    // root through a symlink, and the governor operation writes one. This
    // program exists to be given privilege (D35), so the seam cannot be in it
    // (§6.32, B-190, F50).
    //
    // Refused rather than ignored: a caller who asked for something and did
    // not get it must be told (A2).
    if arguments.contains(&"--under") {
        return Err(Failure::new(
            Category::ConfigInvalid,
            Attribution::User,
            Disposition::Refused,
            Subsystem::new("mcf-helper"),
            "this program reads and writes the machine's own paths and cannot be pointed \
             elsewhere: a helper that can be told where the machine is, is a helper that reads \
             and writes anything it is given privilege for",
        ));
    }
    run_under(Path::new("/"), arguments)
}

/// The same, against a stated root.
///
/// Not reachable from the binary and not reachable from an argument: the root
/// is a parameter, so the only callers are the laboratory and this crate's own
/// tests, which is what lets a privileged program be exercised without letting
/// it near the machine (D26).
///
/// # Errors
///
/// As [`run`].
pub fn run_under(root: &Path, arguments: &[&str]) -> Result<Vec<String>> {
    let (operation, rest) = arguments.split_first().ok_or_else(usage)?;
    let rest: Vec<&str> = rest.to_vec();
    let root = root.to_path_buf();

    match *operation {
        "governor" => {
            let [name] = rest.as_slice() else {
                return Err(usage());
            };
            governor(&root, name)
        }
        "accelerator" => {
            let [mode, index] = rest.as_slice() else {
                return Err(usage());
            };
            accelerator(mode, index)
        }
        "energy" => {
            if rest.is_empty() {
                energy(&root)
            } else {
                Err(usage())
            }
        }
        other => Err(Failure::new(
            Category::ConfigInvalid,
            Attribution::User,
            Disposition::Refused,
            WHERE,
            "that is not an operation this helper performs",
        )
        .with_context("asked_for", other.to_owned())
        .with_context(
            "performs",
            OPERATIONS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

fn usage() -> Failure {
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "the arguments do not name one of this helper's operations",
    )
    .with_context(
        "usage",
        "mcf-helper governor <name> | accelerator exclusive|shared <index> | energy",
    )
}

/// Sets every processor's governor, and says what each one was.
///
/// The name is checked against what the machine says it offers before anything
/// is written, and the value written is the machine's own string rather than
/// the caller's: a helper that writes what it is told is a helper that writes
/// anything (§6.32).
///
/// Every processor, not one: a machine with half its cores on `powersave` is a
/// machine whose measurements depend on which core the work landed on, which is
/// exactly the condition §3.4 exists to remove.
fn governor(root: &Path, wanted: &str) -> Result<Vec<String>> {
    let processors = processors(root)?;
    let offered = available(root, processors.first().ok_or_else(no_governor)?)?;
    let name = offered
        .iter()
        .find(|known| known.as_str() == wanted)
        .ok_or_else(|| {
            Failure::new(
                Category::ConfigInvalid,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "this machine does not offer that governor",
            )
            .with_context("asked_for", wanted.to_owned())
            .with_context("offers", offered.join(", "))
        })?;

    let mut said = Vec::new();
    let mut changed = Vec::new();
    for processor in &processors {
        let path = governor_path(root, processor);
        let was = match read(&path) {
            Ok(was) => was,
            // A failure part-way through is partial whether it happened on the
            // read or on the write: what matters to the caller is the same
            // either way — some processors were changed and they have to go
            // back (A4, A27).
            Err(failure) => return Err(partial(failure, &changed)),
        };
        // A4: what was already set is not a change, and saying so keeps a
        // restoration from putting back something that was never altered.
        if was == *name {
            said.push(format!("{processor}: already {was}"));
            continue;
        }
        match std::fs::write(&path, name.as_bytes()) {
            Ok(()) => {
                changed.push((processor.clone(), was.clone()));
                said.push(format!("{processor}: was {was}, now {name}"));
            }
            Err(error) => {
                return Err(partial(
                    Failure::new(
                        category_for(&error),
                        Attribution::Machine,
                        Disposition::Partial,
                        WHERE,
                        "the governor could not be set on every processor",
                    )
                    .with_context("processor", processor.clone())
                    .with_context("path", path.display().to_string())
                    .with_context("os_error", error.to_string()),
                    &changed,
                ));
            }
        }
    }
    said.push(format!("governor: {name}"));
    Ok(said)
}

/// The tool an operation needs is not on this machine.
///
/// Public because the laboratory reproduces it (A13): the helper refuses an
/// unprivileged caller before it looks for the tool, so this failure is
/// reachable only from a privileged process — which the suite is not and B19
/// says must not be. The scenario constructs it through the same function the
/// helper uses, so the two cannot drift.
#[must_use]
pub fn no_vendor_tool(tool: &str) -> Failure {
    Failure::new(
        Category::PlatformPrivilegeUnavailable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the vendor's tool an operation needs is not on this machine",
    )
    .with_context("tool", tool.to_owned())
    .with_context(
        "what_to_do",
        "install the vendor's management tool, or run the laboratory without an exclusive \
         device and let the result carry the contention as a condition (§3.4)",
    )
}

/// A failure that happened part-way through, carrying what has to be put back.
///
/// A4: the caller is told what was already changed, because a restoration that
/// does not know what was altered is a restoration that cannot be made (A27).
fn partial(failure: Failure, changed: &[(String, String)]) -> Failure {
    if changed.is_empty() {
        return failure;
    }
    failure.with_context(
        "already_changed",
        changed
            .iter()
            .map(|(processor, was)| format!("{processor}={was}"))
            .collect::<Vec<_>>()
            .join(","),
    )
}

/// Takes or releases a device's exclusive compute mode.
///
/// Through the vendor's own tool, because the mode is the vendor's: there is no
/// file to write, and a helper that spoke a proprietary driver's protocol
/// directly would be a helper nobody can audit. `nothing_acquired_is_ever_run`
/// declares this as one of the few places MCF starts a process, with what it
/// starts and why that is not an artifact.
///
/// Refused before the tool is run when this process is not root: the vendor's
/// tool is documented as requiring it, and asking anyway would mean a refusal
/// nobody can distinguish from a device that is not there (A2).
fn accelerator(mode: &str, index: &str) -> Result<Vec<String>> {
    let wanted = match mode {
        "exclusive" => "EXCLUSIVE_PROCESS",
        "shared" => "DEFAULT",
        _ => return Err(usage()),
    };
    if !index.chars().all(|character| character.is_ascii_digit()) {
        return Err(usage());
    }

    // SAFETY-adjacent, and the reason this is not `unsafe`: reading this
    // process's own effective user is what `id -u` does, and `std` exposes it
    // through the environment-free path only on some platforms — so the check
    // is made from what the filesystem says about a root-only file rather than
    // from a syscall this crate would have to declare `unsafe` for.
    if !is_root() {
        return Err(Failure::new(
            Category::PlatformPrivilegeDenied,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "changing a device's compute mode needs elevation and this helper has none",
        )
        .with_context("mode", wanted.to_owned())
        .with_context("device", index.to_owned())
        .with_context(
            "what_to_do",
            "install the helper with the rights D35 names, or run the laboratory without an \
             exclusive device and let the result carry the contention as a condition (§3.4)",
        ));
    }

    let outcome = std::process::Command::new("nvidia-smi")
        .args(["-i", index, "-c", wanted])
        .output()
        .map_err(|_| no_vendor_tool("nvidia-smi"))?;
    if !outcome.status.success() {
        return Err(Failure::new(
            Category::PlatformPrivilegeDenied,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "the vendor's tool refused to change the compute mode",
        )
        .with_context("device", index.to_owned())
        .with_context("mode", wanted.to_owned())
        .with_context(
            "said",
            String::from_utf8_lossy(&outcome.stderr).trim().to_owned(),
        ));
    }
    Ok(vec![
        format!("device: {index}"),
        format!("compute_mode: {wanted}"),
    ])
}

/// Reads the processor's energy counter, in microjoules, per domain.
///
/// A counter rather than a rate: it wraps, and the difference between two
/// readings is the caller's arithmetic. D11 makes energy first-class; what
/// makes it *honest* is that the wrap is the caller's problem rather than
/// something this program smooths over.
fn energy(root: &Path) -> Result<Vec<String>> {
    let powercap = root.join("sys/class/powercap");
    let mut said = Vec::new();
    let reading = std::fs::read_dir(&powercap).map_err(|error| {
        Failure::new(
            category_for(&error),
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "this machine publishes no energy counters",
        )
        .with_context("path", powercap.display().to_string())
        .with_context("os_error", error.to_string())
    })?;

    let mut domains: Vec<PathBuf> = reading
        .filter_map(|entry| Some(entry.ok()?.path()))
        .collect();
    domains.sort();
    for domain in domains {
        let counter = domain.join("energy_uj");
        if !counter.exists() {
            continue;
        }
        let name = std::fs::read_to_string(domain.join("name")).map_or_else(
            |_| {
                domain
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            },
            |text| text.trim().to_owned(),
        );
        match read(&counter) {
            Ok(microjoules) => said.push(format!("{name}: {microjoules}")),
            // One unreadable domain is not a failed read: A4 keeps the ones
            // that were read, and the caller is told which was not.
            Err(failure) => said.push(format!("{name}: unreadable — {failure}")),
        }
    }

    if said.is_empty() {
        return Err(Failure::new(
            Category::PlatformMechanismUnavailable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "this machine publishes no energy counters MCF can read",
        )
        .with_context("path", powercap.display().to_string()));
    }
    Ok(said)
}

/// Every processor the machine publishes a governor for, in order.
fn processors(root: &Path) -> Result<Vec<String>> {
    let cpus = root.join("sys/devices/system/cpu");
    let reading = std::fs::read_dir(&cpus).map_err(|error| {
        Failure::new(
            category_for(&error),
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "this machine does not publish its processors where they are expected",
        )
        .with_context("path", cpus.display().to_string())
        .with_context("os_error", error.to_string())
    })?;

    let mut found: Vec<String> = reading
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().to_string_lossy().into_owned();
            let rest = name.strip_prefix("cpu")?;
            rest.chars().all(|c| c.is_ascii_digit()).then_some(name)
        })
        .filter(|processor| governor_path(root, processor).exists())
        .collect();
    found.sort();
    if found.is_empty() {
        return Err(no_governor());
    }
    Ok(found)
}

fn governor_path(root: &Path, processor: &str) -> PathBuf {
    root.join("sys/devices/system/cpu")
        .join(processor)
        .join("cpufreq/scaling_governor")
}

fn available(root: &Path, processor: &str) -> Result<Vec<String>> {
    let path = root
        .join("sys/devices/system/cpu")
        .join(processor)
        .join("cpufreq/scaling_available_governors");
    Ok(read(&path)?.split_whitespace().map(str::to_owned).collect())
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map(|text| text.trim().to_owned())
        .map_err(|error| {
            Failure::new(
                category_for(&error),
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the machine would not say",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string())
        })
}

/// Which classification a filesystem error is.
///
/// `permission.denied` when the kernel said no, which is the answer that means
/// *this helper was not installed with the rights D35 names* — a different
/// thing from a machine that has no such file, and an operator acts on them
/// differently (A2).
fn category_for(error: &std::io::Error) -> Category {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => Category::PlatformPrivilegeDenied,
        std::io::ErrorKind::NotFound => Category::PlatformMechanismUnavailable,
        _ => Category::ResourceDiskReadonly,
    }
}

fn no_governor() -> Failure {
    Failure::new(
        Category::PlatformMechanismUnavailable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "this machine publishes no frequency governor",
    )
    .with_context(
        "looked_in",
        "sys/devices/system/cpu/cpu*/cpufreq/scaling_governor".to_owned(),
    )
    .with_context(
        "what_to_do",
        "nothing: a machine with no governor is one MCF measures as it finds it, and the \
         governor becomes an unknown condition rather than a controlled one (§3.4, A7)",
    )
}

/// Whether this process is root, asked of the filesystem rather than of a
/// syscall.
///
/// `/proc/self/status` names the effective user, and reading a file is
/// something this crate can do without an `unsafe` block or a dependency —
/// which are the two things §6.32 would rather a privileged program did not
/// have. A machine with no `/proc` reads as *not root*, which is the safe
/// direction: the operation is refused and the operator is told why.
fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status").is_ok_and(|status| {
        status.lines().any(|line| {
            line.strip_prefix("Uid:")
                .and_then(|rest| rest.split_whitespace().nth(1))
                .is_some_and(|effective| effective == "0")
        })
    })
}

#[cfg(test)]
mod tests;
