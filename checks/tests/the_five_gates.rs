#![allow(clippy::panic)]

use std::path::{Path, PathBuf};

use mcf_core::authorization::{Asking, GATED, Gated};

#[test]
fn every_gate_is_where_it_says_it_is() {
    for gate in GATED {
        match gate.asking() {
            Asking::ByCommand { command, .. } => {
                let surface =
                    read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/main.rs"));
                let verb = command.strip_prefix("mcf ").unwrap_or(command);
                assert!(
                    surface.contains(&format!("\"{verb}\"")),
                    "{gate} says it is asked by `{command}` and the surface has no such command"
                );
            }
            Asking::NoPathExists { .. } => {}
            other => panic!("{gate} is asked in a way this check does not know: {other:?}"),
        }
    }
}

#[test]
fn destruction_needs_a_reason_and_a_second_word_to_delete() {
    let Asking::ByCommand { and, .. } = Gated::Destruction.asking() else {
        panic!("destruction stopped being asked by a command without this check changing");
    };
    assert!(and.contains("--purge"), "{and}");

    let removal = read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/models.rs"));
    assert!(
        removal.contains("let Some(reason) = reason else"),
        "`rm` no longer previews when no reason is given (B-027, §3.11)"
    );
    assert!(
        removal.contains("nothing was deleted"),
        "`rm` no longer says that it deleted nothing"
    );

    let store = read(&mcf_checks::workspace::root().join("crates/mcf-hub/src/store.rs"));
    assert!(
        store.contains(
            "pub fn purge(removed: &Removed, authorization: &Authorization, plan: &Plan)"
        ),
        "purging no longer takes the authorization it was given for"
    );
}

#[test]
fn a_large_download_happens_only_when_it_is_named() {
    let Asking::ByCommand { and, .. } = Gated::LargeIrrecoverableUse.asking() else {
        panic!("acquisition stopped being asked by a command without this check changing");
    };
    assert!(and.contains("named"), "{and}");

    let pull = read(&mcf_checks::workspace::root().join("crates/mcf-cli/src/pull.rs"));
    assert!(
        pull.contains("let Some(wanted) = reference.file.clone() else"),
        "`pull` no longer requires a file to be named before it acquires one (§6.14)"
    );
    assert!(
        pull.contains("nothing was acquired"),
        "`pull` no longer says that it acquired nothing"
    );
}

#[test]
fn nothing_listens_where_another_machine_could_reach_it() {
    let Asking::ByCommand { command, and } = Gated::NetworkExposure.asking() else {
        panic!(
            "network exposure is claimed absent while a hold can be opened to the network \
             (B-577): the gate §6.12 asks for is owed"
        );
    };
    assert!(command == "mcf host", "{command}");
    assert!(
        and.contains("--open on") && and.contains("API key"),
        "{and}"
    );
    let gated: &[(&str, &str)] = &[
        (
            "crates/mcf-serve/src/hosting.rs",
            "the address a hold's engine is told to bind: every address this machine has only \
             where the person turned the hold open, and never without a key (B-577)",
        ),
        (
            "crates/mcf-serve/src/served.rs",
            "the engine started with that address, which is the loopback one unless the hold \
             was opened (B-577)",
        ),
    ];

    let declared: &[(&str, &str)] = &[
        (
            "crates/mcf-lab/src/serving.rs",
            "the laboratory's hub on a socket: a port the kernel chooses on 127.0.0.1, so that \
             code which talks to an operating system can be tested against something that \
             answers",
        ),
        (
            "crates/mcf-serve/src/hosting.rs",
            "asking whether a port is free before a hosted engine is told to bind it. The \
             bind here is the *question* — the listener is dropped immediately and nothing is \
             served from it — and it exists because an engine that cannot bind exits with a \
             status and no sentence, which would be reported as the wrong failure (A2). What \
             a hosted model itself listens on is 127.0.0.1 and only that: putting somebody's \
             model on their network is a decision they make, not one MCF makes for them \
             (§3.7, B-416)",
        ),
    ];

    let root = mcf_checks::workspace::root();
    let mut reachable = Vec::new();
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        for line in code_only(&ships(&read(&file))).lines() {
            let trimmed = line.trim();
            if !trimmed.contains("bind(") {
                continue;
            }
            if trimmed.contains("UnixListener") {
                continue;
            }
            let by_name = trimmed.contains("LOOPBACK") && loopback_is_loopback(&root);
            let loopback = trimmed.contains("127.0.0.1") || trimmed.contains("[::1]") || by_name;
            let allowed = declared.iter().any(|(file, _)| *file == relative);
            let gated_here = gated.iter().any(|(file, _)| *file == relative)
                && (trimmed.contains("fn bind(") || trimmed.contains("settings.bind()"));
            if !(gated_here || loopback && allowed) {
                reachable.push(format!("{relative}: {trimmed}"));
            }
        }
    }

    assert!(
        reachable.is_empty(),
        "MCF binds something that is not a Unix socket or a declared loopback port, so \
         network exposure is now a capability and needs the gate §6.12 asks for (B-036, \
         B-039): {reachable:#?}"
    );

    for (file, what) in declared {
        let source = code_only(&ships(&read(&root.join(file))));
        assert!(
            source.contains("bind("),
            "{file} is declared as a listener and no longer binds anything: {what}"
        );
    }
}

#[test]
fn untrusted_execution_is_the_absence_the_other_check_holds() {
    let Asking::NoPathExists { why } = Gated::UntrustedExecution.asking() else {
        panic!(
            "MCF has learned to run something it acquired, and §6.4's gate is now owed \
             rather than unnecessary (B-025)"
        );
    };
    assert!(why.contains("B-025"), "{why}");
    assert!(
        mcf_checks::workspace::root()
            .join("checks/tests/nothing_acquired_is_ever_run.rs")
            .exists(),
        "the check that holds this absence is gone and the claim is now unheld"
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
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

const DECLARED: &[(&str, &str)] = &[
    (
        "crates/mcf-hub/src/wire.rs",
        "the one outbound connection MCF makes: to the hub a person named, when `mcf pull` is \
     typed. Bytes come in; what goes out is the request for them, and A16 already gates \
     the act as a large irrecoverable use (B-021, B-322)",
    ),
    (
        "crates/mcf-lab/src/serving.rs",
        "the laboratory waking its own accept loop on the loopback address so it can stop: a \
     connection to this process from this process, which reaches no other machine and \
     carries nothing (B-028, D26)",
    ),
    (
        "crates/mcf-serve/src/served.rs",
        "asking a hosted engine on 127.0.0.1 whether it has finished loading. A model is \
     loaded before it answers, and on a large one that is tens of seconds during which the \
     engine refuses everything — so treating *started* as *ready* would hand a caller a \
     server that says no to everything. It is one GET to a process on this machine that MCF \
     itself started a moment earlier, and it carries nothing out (B-416, §3.7)",
    ),
    (
        "crates/mcf-serve/src/takes.rs",
        "asking the same hosted engine on 127.0.0.1, once it answers, what it takes for the \
     model it holds — which media reach it and what its template does with a switch. Two \
     requests to a process on this machine that MCF started, about a model on this machine; \
     what goes out is the question, and the turn it asks the engine to render holds no \
     person's words (B-449, §3.7)",
    ),
];

fn loopback_is_loopback(root: &std::path::Path) -> bool {
    let source = read(&root.join("crates/mcf-serve/src/hosting.rs"));
    source
        .lines()
        .filter(|line| line.contains("pub const LOOPBACK"))
        .any(|line| line.contains("\"127.0.0.1\""))
}

#[test]
fn nothing_sends_anything_anywhere() {
    let Asking::NoPathExists { why } = Gated::Publication.asking() else {
        panic!(
            "MCF has learned to send something somewhere, and A24's gate is now owed rather \
             than unnecessary: publication cannot be undone, so the act needs the itemized \
             confirmation B-160 builds"
        );
    };
    assert!(why.contains("no destination"), "{why}");

    let root = mcf_checks::workspace::root();
    let mut sending = Vec::new();
    for file in shipped_sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        if DECLARED.iter().any(|(named, _)| *named == relative) {
            continue;
        }
        for line in code_only(&ships(&read(&file))).lines() {
            let trimmed = line.trim();
            for shape in ["TcpStream::connect", "reqwest::", "ureq::", "UdpSocket::"] {
                if trimmed.contains(shape) {
                    sending.push(format!("{relative}: {trimmed}"));
                }
            }
        }
    }
    assert!(
        sending.is_empty(),
        "shipped code opens an outbound connection somewhere undeclared. Publication is the \
         one gated act that cannot be undone (A24), and MCF's claim that it has no path to it \
         is what this check holds:\n{sending:#?}"
    );
}
