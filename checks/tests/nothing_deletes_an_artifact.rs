//! Every deletion in MCF is one somebody declared.
//!
//! B-027's condition is that *no code path deletes an artifact without an
//! explicit, recorded authorization*, and §3.11 gives the reason: an artifact
//! is not a cache entry. It is what a measurement was made against, and
//! re-acquiring it is not always possible — a pinned repository can be
//! withdrawn, gated or relicensed between one week and the next (DEC-038). A
//! tool that deletes to reclaim space has quietly decided that disk is worth
//! more than evidence.
//!
//! The condition cannot be checked by looking for the *intent* to reclaim
//! space, because nobody writes that. What can be checked is the act: every
//! call that destroys a file in MCF's non-test code appears in the list below,
//! with what it deletes and why that is not an artifact. Adding a deletion
//! means adding a line here, which is the point — the line is where somebody
//! has to write down what they are destroying.
//!
//! Only `mcf_hub::store::purge` deletes an artifact, and its signature will not
//! let a caller reach it without an [`Authorization`], which cannot be
//! constructed without a plan somebody previewed and a reason they stated.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// A file that destroys something, and what it destroys.
struct Deletes {
    file: &'static str,
    /// How many calls it makes, so that a fourth one added to a file that
    /// already had three is still a line somebody has to change.
    calls: usize,
    /// What is being destroyed, and why it is not an artifact.
    what: &'static str,
}

/// Every deletion in MCF's non-test code.
const DECLARED: &[Deletes] = &[
    Deletes {
        file: "crates/mcf-cli/src/eval.rs",
        calls: 1,
        what: "the scratch directory the laboratory wrote a model's answer into, once every \
               task has been checked. What is removed is text a model emitted seconds earlier \
               and the program MCF wrapped around it — never an artifact MCF acquired, and \
               never the record, which keeps what the answers did (B-027)",
    },
    Deletes {
        file: "crates/mcf-serve/src/probes/run.rs",
        calls: 1,
        what: "the picture the vision probe just drew, once the turn that was shown it is \
               over. MCF computes those bytes itself — the shapes are drawn by \
               `probes::vision`, not read from anywhere — so what is removed is a temporary \
               MCF made, never a file it acquired, and the record keeps what the model said \
               about it rather than the file (B-027)",
    },
    Deletes {
        file: "crates/mcf-cli/src/provision.rs",
        calls: 1,
        what: "a provisioned component's prefix, in `remove`, which carries a reason and is \
               recorded before the directory goes — a component MCF built, never an artifact \
               it acquired (B-367, A27)",
    },
    Deletes {
        file: "crates/mcf-desk/build.rs",
        calls: 1,
        what: "a stale `libmcffont.a` in this build's own output directory, before the \
               archiver writes a new one. `ar rcs` adds members rather than replacing an \
               archive, so an object from an earlier build would otherwise survive into this \
               one — which is the opposite of what a reproducible build wants. What is \
               destroyed is a build product under `OUT_DIR` that this build script created \
               and is about to recreate (B-409, A27)",
    },
    Deletes {
        file: "crates/mcf-cli/src/serve.rs",
        calls: 1,
        what: "a control socket with nothing behind it, before starting a daemon that would \
               otherwise refuse to bind over it. A killed daemon leaves the file, and this \
               machine reached that state more than once. A socket is a name for a running \
               process rather than a thing anybody stored: it holds no bytes, and the one \
               removed here has already been shown to answer nothing (B-408, A27)",
    },
    Deletes {
        file: "crates/mcf-serve/src/served.rs",
        calls: 2,
        what: "the Unix socket the provisioned server listens on — once before binding, in \
               case a killed daemon left one behind, and once when the server is dropped. A \
               socket is a name for a running process, not a thing anybody stored: it holds \
               no bytes, it is created by MCF a moment earlier under MCF's own runtime \
               directory, and leaving one behind is the litter B58 is about (A27, §3.11)",
    },
    Deletes {
        file: "crates/mcf-serve/src/orphans.rs",
        calls: 2,
        what: "the socket file of an engine server whose daemon was gone, once the server \
               is stopped, and a socket named for a daemon that is not in the process \
               table. Both are names the kernel has already forgotten (B-574, A27); the \
               model files the servers held are not touched",
    },
    Deletes {
        file: "crates/mcf-serve/src/configured.rs",
        calls: 1,
        what: "a model's derived configuration, in `forget`, which is somebody undoing a \
               decision somebody else made (D43). What goes is MCF's note about how to \
               address a model — never the model, and never the record: the `model_configured` \
               entry that says the decision was taken outlives the file, the way \
               `ArtifactRemoved` outlives an artifact (A1, §3.11)",
    },
    Deletes {
        file: "crates/mcf-hub/src/store.rs",
        calls: 1,
        what: "an artifact on the shelf, in `purge`, which is the one place MCF destroys one \
               and takes an authorization to do it (B-027)",
    },
    Deletes {
        file: "crates/mcf-hub/src/fetch.rs",
        calls: 3,
        what: "MCF's own partial transfer — bytes that never became an artifact, discarded \
               when they turn out to be a mixture of two files (B-021)",
    },
    Deletes {
        file: "crates/mcf-serve/src/daemon.rs",
        calls: 2,
        what: "the daemon's own control socket — one left behind by a process that died, \
               and its own on the way out. A socket is a name the kernel gave this process, \
               not something anybody's model is in (A27, B-030)",
    },
    Deletes {
        file: "crates/mcf-lab/src/world.rs",
        calls: 3,
        what: "a laboratory scratch directory this process made, and the leavings of \
               laboratory processes that died (A27)",
    },
    Deletes {
        file: "crates/mcf-lab/src/catalogue/record.rs",
        calls: 1,
        what: "a file the laboratory itself just filed in a scratch content store, removed so \
               that a directory can take its place — which is how *there and unreadable* is \
               constructed, and which must not come back as the same answer as *never kept* \
               (F105, D26, A27)",
    },
    Deletes {
        file: "crates/mcf-record/src/overhead.rs",
        calls: 1,
        what: "the file the overhead measurement itself wrote, removed by the measurement \
               that created it (A27)",
    },
    Deletes {
        file: "crates/mcf-record/src/restore.rs",
        calls: 3,
        what: "a file MCF created while changing the environment, being put back the way it \
               was found (A27, §6.39)",
    },
];

/// What a call to destroy a file looks like.
const DESTROYS: &[&str] = &["remove_file", "remove_dir_all", "remove_dir"];

/// Nothing deletes anything except where it is written down.
#[test]
fn every_deletion_is_declared() {
    let root = mcf_checks::workspace::root();
    for file in non_test_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        let calls = destroying_calls(&read(&file));
        let declared = DECLARED.iter().find(|d| d.file == relative);
        match declared {
            None => assert_eq!(
                calls, 0,
                "{relative} destroys files and nothing says what: every deletion in MCF is \
                 declared in checks/tests/nothing_deletes_an_artifact.rs, with what it \
                 destroys and why that is not an artifact (B-027, §3.11)"
            ),
            Some(declared) => assert_eq!(
                calls, declared.calls,
                "{relative} makes {calls} deleting calls and declares {}: {}",
                declared.calls, declared.what
            ),
        }
    }
}

/// And nothing is declared that has since stopped deleting — a stale line would
/// be a budget somebody could spend later without saying so.
#[test]
fn nothing_is_declared_that_no_longer_deletes() {
    let root = mcf_checks::workspace::root();
    for declared in DECLARED {
        let path = root.join(declared.file);
        assert!(
            path.exists(),
            "{} is declared here and no longer exists",
            declared.file
        );
        assert_eq!(
            destroying_calls(&read(&path)),
            declared.calls,
            "{} no longer makes the {} deleting calls declared for it",
            declared.file,
            declared.calls
        );
        assert!(
            !declared.what.is_empty(),
            "{} declares no reason",
            declared.file
        );
    }
}

/// The one place that destroys an artifact will not do it without an
/// authorization, and an authorization cannot be had without a plan and a
/// reason.
///
/// The compiler holds this; what a source check adds is that the shape has not
/// been loosened — a `purge` that took a path, or an `Authorization` with a
/// public field, would satisfy every test above while removing the discipline.
#[test]
fn purging_requires_an_authorization_nobody_can_conjure() {
    let source = read(&mcf_checks::workspace::root().join("crates/mcf-hub/src/store.rs"));
    let code = code_only(&source);
    assert!(
        code.contains(
            "pub fn purge(removed: &Removed, authorization: &Authorization, plan: &Plan)"
        ),
        "`purge` no longer takes an authorization and the plan it was given for (B-027)"
    );
    assert!(
        code.contains("pub fn given(plan: &Plan, reason: &str) -> Result<Self>"),
        "an authorization is no longer made from a plan and a stated reason (B-027, §3.11)"
    );
    assert!(
        code.contains("struct Authorization {\n    doomed: Vec<Doomed>,\n    reason: String,\n}"),
        "`Authorization`'s fields are no longer private, so one can be conjured without a plan"
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// Calls that destroy a file, in code that ships rather than in prose or in a
/// test module.
fn destroying_calls(source: &str) -> usize {
    code_only(&ships(source))
        .lines()
        .map(|line| {
            DESTROYS
                .iter()
                .filter(|call| line.contains(&format!("{call}(")))
                .count()
        })
        .sum()
}

/// A file with its inline test module cut off.
///
/// A test that removes its own scratch directory is a test doing its job, and
/// several modules keep theirs at the bottom of the same file. The claim here
/// is about the code that ships.
fn ships(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(at) => source.get(..at).unwrap_or(source).to_owned(),
        None => source.to_owned(),
    }
}

/// A file with its documentation removed.
///
/// The prose names what it forbids in order to say it is absent.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under `crates/*/src` that is not itself a test.
///
/// Tests delete their own scratch directories constantly and that is what a
/// test should do; the claim here is about the code that ships.
fn non_test_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(&root.join("crates"), &mut found);
    found.retain(|path| {
        let text = path.display().to_string();
        !text.contains("/tests/") && !text.ends_with("tests.rs")
    });
    found.sort();
    found
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            collect(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
