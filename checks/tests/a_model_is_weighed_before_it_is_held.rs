//! Nothing loads a model without first weighing it against free memory
//! (B-372, A2, A22).
//!
//! MCF's own engine dequantizes to `f32`. A quantized file that is comfortable
//! on disk becomes several times its size in memory, so *can this machine hold
//! this model* is a question with an arithmetic answer, and
//! [`fits_dequantized`] is where that arithmetic lives. It names both numbers
//! when it refuses, which is the whole point: a machine that cannot afford a
//! model should hear so from MCF.
//!
//! **What went wrong, and why a check.** The console weighed a model before
//! loading it and the daemon did not. Both serve the same request — `mcf run`
//! reaches the stand-in either way, in-process when nothing is listening and
//! over the socket when something is — so the same model was refused by one
//! path and loaded by the other, which is exactly the disagreement A22 and
//! B-072 exist to prevent. The unguarded path read the whole file into memory
//! and allocated several times its size again. On a machine without room for
//! that, nothing was reported by anybody: the kernel chose a process and ended
//! it, which was sometimes MCF and sometimes whatever else the operator had
//! open. A failure MCF causes and does not report is the plainest form of A2,
//! and it is worse than usual here because the process that pays is not
//! necessarily the one that asked (F136).
//!
//! **What this check holds.** Every place in shipped code that hands bytes to
//! the stand-in loader sits in a file that also weighs the model, by one of
//! the two names that arithmetic is reached by. That is a coarse instrument —
//! it proves the guard is *present*, not that it is *upstream of* the
//! allocation on every path through the file — and a coarse instrument is
//! still what caught this: the defect was a file with no guard in it at all.
//! Where a load site is deliberately unweighed, it is declared below with the
//! reason, so that adding one is a decision somebody writes down rather than
//! an omission nobody notices.
//!
//! [`fits_dequantized`]: mcf_standin::gguf::Model::fits_dequantized

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// How a file reaches the memory arithmetic, whatever wraps it.
const WEIGHS: &[&str] = &["fits_dequantized", "fits_in_memory", "examined("];

/// What hands bytes to the loader.
const LOADS: &str = "llama::load(";

/// A file that loads a model without weighing it, and why that is allowed.
struct Unweighed {
    file: &'static str,
    /// Why no arithmetic is owed here.
    because: &'static str,
}

/// The declared exceptions.
///
/// Both are laboratories rather than serving paths: they exist to find out
/// what MCF does with a file, and a refusal computed before the experiment
/// would answer a different question than the one being asked.
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

/// The workspace root, from this file's location.
fn root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .unwrap_or_else(|| panic!("the checks crate sits in the workspace"))
        .to_path_buf()
}

/// Every `.rs` file under `crates/`, excluding test trees.
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
        // The loader's own definition is not a call site.
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
    // A declaration that has outlived its file is a reason nobody is reading
    // any more, and an exception list nobody prunes stops meaning anything.
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
    // The specific disagreement F136 was. Named on its own because the sweep
    // above would pass again the moment somebody moved the guard out of the
    // daemon into a file that merely mentions it.
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
    // **The other half of the same defect.** Which devices exist is asked once
    // when the daemon starts, because a build's capabilities do not change
    // while it sits on the disk and §3.13 keeps idle free. How much memory is
    // free was sampled in the same breath and it does change constantly: a
    // resident model takes it, stopping one gives it back. Planning a
    // placement from the start-up figure says a second model fits beside the
    // first, because the first was not there when the number was taken (A21).
    //
    // So the resolver is never handed the stored table directly. Anything that
    // plans against free memory goes through the accessor that re-reads it.
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
