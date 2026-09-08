#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

const WEIGHS: &[&str] = &["fits_dequantized", "fits_in_memory", "examined("];

const LOADS: &str = "llama::load(";

struct Unweighed {
    file: &'static str,
    because: &'static str,
}

const ALLOWED: &[Unweighed] = &[
    Unweighed {
        file: "crates/mcf-lab/src/catalogue/artifact.rs",
        because: "the artifact laboratory feeds MCF deliberately malformed files to record what \
                  it does with them; the files are fixtures measured in kilobytes, and a memory \
                  refusal computed first would pre-empt the reading being observed",
    },
    Unweighed {
        file: "crates/mcf-standin/examples/load.rs",
        because: "an example, not a shipped path: it exists to show the loader being called and \
                  is run by hand on a file the operator names",
    },
    Unweighed {
        file: "crates/mcf-standin/examples/margins.rs",
        because: "an example, as above",
    },
];

fn root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .unwrap_or_else(|| panic!("the checks crate sits in the workspace"))
        .to_path_buf()
}

fn sources(at: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "tests" || name == "target" {
                continue;
            }
            sources(&path, into);
        } else if path.extension().is_some_and(|held| held == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn nothing_loads_a_model_it_has_not_weighed() {
    let root = root();
    let mut found = Vec::new();
    sources(&root.join("crates"), &mut found);
    assert!(!found.is_empty(), "the source tree was not found");

    let mut unweighed = Vec::new();
    for path in found {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if path.ends_with("mcf-standin/src/llama.rs") {
            continue;
        }
        if !text.contains(LOADS) {
            continue;
        }
        if WEIGHS.iter().any(|held| text.contains(held)) {
            continue;
        }
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if ALLOWED.iter().any(|held| held.file == relative) {
            continue;
        }
        unweighed.push(relative);
    }

    assert!(
        unweighed.is_empty(),
        "these load a model without weighing it against free memory first, so a machine \
         without room for one is not told — the kernel ends some process instead, and not \
         necessarily this one (B-372, A2, F136): {unweighed:?}\n\nEither weigh it — \
         `fits_dequantized` against what the platform says is free — or declare it in \
         ALLOWED with the reason it is owed no arithmetic."
    );
}

#[test]
fn every_declared_exception_still_exists() {
    let root = root();
    for held in ALLOWED {
        let path = root.join(held.file);
        assert!(
            path.exists(),
            "{} is declared as loading a model unweighed, and there is no such file — remove \
             the declaration (the reason given was: {})",
            held.file,
            held.because
        );
    }
}

#[test]
fn the_serving_path_and_the_console_agree() {
    let root = root();
    let daemon_side = std::fs::read_to_string(root.join("crates/mcf-serve/src/generation.rs"))
        .expect("the serving path is readable");
    let console_side =
        std::fs::read_to_string(root.join("crates/mcf-cli/src/run.rs")).expect("run is readable");
    for (which, text) in [("the daemon", &daemon_side), ("the console", &console_side)] {
        assert!(
            WEIGHS.iter().any(|held| text.contains(held)),
            "{which} loads a model into memory without weighing it; both serve `mcf run` and \
             they have to refuse the same models (A22, B-072, F136)"
        );
    }
}

#[test]
fn a_placement_is_planned_against_memory_as_it_is_now() {
    let daemon = std::fs::read_to_string(root().join("crates/mcf-serve/src/daemon.rs"))
        .expect("the daemon is readable");
    assert!(
        !daemon.contains("resolve(&self.engines,"),
        "a placement is being planned from the engine table sampled at start-up, whose free \
         memory figure describes a machine that stopped existing the moment anything was \
         loaded — use `engines_now()`, which re-reads it (A21, F136)"
    );
    assert!(
        daemon.contains("fn engines_now("),
        "the accessor that re-reads free memory is gone, so nothing re-reads it (F136)"
    );
}
