#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

struct Reaches {
    module: &'static str,
    subcommand: Option<&'static str>,
}

const REACHES: &[Reaches] = &[
    Reaches {
        module: "crates/mcf-cli/src/serve.rs",
        subcommand: None,
    },
    Reaches {
        module: "crates/mcf-cli/src/run.rs",
        subcommand: Some("run"),
    },
    Reaches {
        module: "crates/mcf-cli/src/bench.rs",
        subcommand: Some("bench"),
    },
    Reaches {
        module: "crates/mcf-cli/src/probe.rs",
        subcommand: Some("probe"),
    },
    Reaches {
        module: "crates/mcf-cli/src/examine.rs",
        subcommand: Some("examine"),
    },
    Reaches {
        module: "crates/mcf-cli/src/measure.rs",
        subcommand: Some("measure"),
    },
    Reaches {
        module: "crates/mcf-cli/src/acquire.rs",
        subcommand: Some("offered"),
    },
    Reaches {
        module: "crates/mcf-cli/src/failures.rs",
        subcommand: Some("failures"),
    },
    Reaches {
        module: "crates/mcf-cli/src/hosting.rs",
        subcommand: Some("settings"),
    },
    Reaches {
        module: "crates/mcf-cli/src/hosting.rs",
        subcommand: Some("host"),
    },
    Reaches {
        module: "crates/mcf-cli/src/hosting.rs",
        subcommand: Some("hosted"),
    },
    Reaches {
        module: "crates/mcf-cli/src/hosting.rs",
        subcommand: Some("unhost"),
    },
    Reaches {
        module: "crates/mcf-cli/src/acquire.rs",
        subcommand: Some("acquire"),
    },
    Reaches {
        module: "crates/mcf-cli/src/crosscheck.rs",
        subcommand: Some("cross-check"),
    },
    Reaches {
        module: "crates/mcf-cli/src/eval.rs",
        subcommand: Some("eval"),
    },
    Reaches {
        module: "crates/mcf-cli/src/prompt.rs",
        subcommand: Some("prompt"),
    },
    Reaches {
        module: "crates/mcf-cli/src/provision.rs",
        subcommand: Some("provision"),
    },
    Reaches {
        module: "crates/mcf-cli/src/tui.rs",
        subcommand: Some("tui"),
    },
    Reaches {
        module: "crates/mcf-cli/src/desk.rs",
        subcommand: Some("desk"),
    },
];

const HELPER: &str = "tier_private_runtime_dir";

fn tier_scripts() -> Vec<PathBuf> {
    let scripts = mcf_checks::workspace::root().join("scripts");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&scripts)
        .expect("the scripts directory is in the tree")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|kind| kind == "sh")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("check-"))
        })
        .collect();
    found.sort();
    found
}

fn instructions(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with('#'))
}

fn modules_that_reach() -> Vec<String> {
    let root = mcf_checks::workspace::root();
    let source = root.join("crates/mcf-cli/src");
    let mut found = Vec::new();
    let mut stack = vec![source];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|kind| kind != "rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if instructions(&text).any(|line| {
                line.contains("socket_path()") && !line.contains("pub(crate) fn socket_path")
            }) {
                let relative = path
                    .strip_prefix(mcf_checks::workspace::root())
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_module_that_reaches_a_daemon_is_written_down() {
    let found = modules_that_reach();
    let mut unknown: Vec<&String> = found
        .iter()
        .filter(|module| !REACHES.iter().any(|known| known.module == module.as_str()))
        .collect();
    unknown.sort();
    assert!(
        unknown.is_empty(),
        "a CLI module consults the control socket and this check does not know which \
         subcommand it is, so a tier could drive it and inherit a daemon unnoticed (F104): \
         {unknown:#?}"
    );

    let mut stale: Vec<&str> = REACHES
        .iter()
        .map(|known| known.module)
        .filter(|module| !found.iter().any(|seen| seen == module))
        .collect();
    stale.sort_unstable();
    assert!(
        stale.is_empty(),
        "this check watches modules that no longer consult the socket; a stale list is a \
         list nobody trusts: {stale:#?}"
    );
}

#[test]
fn no_tier_lets_a_daemon_answer_for_the_binary_it_built() {
    let mut offenders = Vec::new();
    for path in tier_scripts() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        let invokes = |subcommand: &str| {
            instructions(&source).any(|line| {
                (line.contains("$mcf\"") || line.contains("${mcf}"))
                    && line.contains(&format!(" {subcommand} "))
            })
        };
        let driven: Vec<&str> = REACHES
            .iter()
            .filter_map(|reaches| reaches.subcommand)
            .filter(|subcommand| invokes(subcommand))
            .collect();
        if driven.is_empty() {
            continue;
        }
        let isolates = instructions(&source).any(|line| line.contains(HELPER))
            && instructions(&source).any(|line| line.contains("export XDG_RUNTIME_DIR="));
        if !isolates {
            offenders.push(format!(
                "{name}: drives {driven:?} and does not export a runtime directory of its own \
                 via {HELPER}"
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "a scheduled tier drives a subcommand that consults the control socket without \
         bringing a socket of its own, so a daemon built from other source can answer for the \
         binary the tier just built (F104, §3.12): {offenders:#?}"
    );
}

#[test]
fn the_helper_bounds_the_socket_path() {
    let library =
        std::fs::read_to_string(mcf_checks::workspace::root().join("scripts/lib-tiers.sh"))
            .expect("the tier library is in the tree");
    assert!(
        library.contains(&format!("{HELPER}()")),
        "the helper the tiers call no longer exists (F104)"
    );
    assert!(
        library.contains("108"),
        "the helper no longer bounds the socket path against `sun_path`, so a machine with a \
         long TMPDIR would silently fail to isolate (F104)"
    );
}

fn shell(command: &str) -> Option<String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn the_reported_build_is_the_binary_that_ran() {
    let identifier = mcf_core::build_identity::identifier();
    let Some((version, reported)) = identifier.split_once('+') else {
        panic!("the build identifier no longer names an instrument: {identifier}");
    };
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "the build identifier no longer starts with the version"
    );
    if reported == "unknown" {
        return;
    }

    let Ok(binary) = std::env::current_exe() else {
        return;
    };
    let Some(independent) = shell(&format!(
        "sha256sum {} 2>/dev/null | cut -c1-12",
        Path::new(&binary).display()
    )) else {
        return;
    };
    if independent.is_empty() {
        eprintln!("no sha256sum on this machine; the digest was not cross-checked");
        return;
    }
    assert_eq!(
        reported, independent,
        "the build MCF reports is not the digest of the binary that reported it"
    );
}
