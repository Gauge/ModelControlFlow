#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

#[test]
fn the_daemon_has_no_heartbeat() {
    for file in sources("crates/mcf-serve/src") {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        assert!(
            !source.contains("thread::spawn(move || loop"),
            "{}: a spawned loop is a heartbeat, and an idle MCF must have none (B4, B-187)",
            file.display()
        );
    }
}

#[test]
fn a_counter_is_read_because_somebody_asked() {
    let source = read("crates/mcf-core/src/hardware/nvml.rs");
    assert!(
        source.contains("nvmlDeviceGetTemperature"),
        "this check is about the temperature read; if it has moved, the check must move with it"
    );
    let mut callers = Vec::new();
    for file in sources("crates") {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        let is_test = file.display().to_string().contains("tests")
            || source.contains("#[cfg(test)]") && source.contains("fn read_through");
        if !is_test
            && (source.contains("Machine::read()") || source.contains("Machine::read_through("))
        {
            callers.push(file);
        }
    }
    for caller in &callers {
        let named = caller.display().to_string();
        assert!(
            named.contains("mcf-cli") || named.contains("mcf-core/src/hardware"),
            "{named} reads the machine, and only a command the operator ran may: a read from \
             the serving path would make MCF one of the competitors it reports (§3.8, B4)"
        );
    }
    assert!(
        !callers.is_empty(),
        "no caller was found at all, so this check is reading the wrong tree"
    );
}

#[test]
fn the_benchmark_subsystem_is_not_in_the_serving_path() {
    let manifest = read("crates/mcf-serve/Cargo.toml");
    assert!(
        !manifest.contains("mcf-bench"),
        "B-108: a benchmark subsystem that the daemon can reach is one whose idle cost has to \
         be argued about; one it cannot name has none by construction (§6.18)"
    );
}

fn sources(relative: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&mcf_checks::workspace::root().join(relative), &mut found);
    found
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
