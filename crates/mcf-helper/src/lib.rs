use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-helper");

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

pub fn run(arguments: &[&str]) -> Result<Vec<String>> {
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
            Err(failure) => return Err(partial(failure, &changed)),
        };
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

fn accelerator(mode: &str, index: &str) -> Result<Vec<String>> {
    let wanted = match mode {
        "exclusive" => "EXCLUSIVE_PROCESS",
        "shared" => "DEFAULT",
        _ => return Err(usage()),
    };
    if !index.chars().all(|character| character.is_ascii_digit()) {
        return Err(usage());
    }

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
